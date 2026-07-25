//! Single-pass segment/delimiter scanner (spec 005-message-scanner).
//!
//! Reads the field separator (MSH-1) and encoding characters (MSH-2) from the
//! message's own MSH segment and produces byte offsets only — no owned copy of
//! any field, component, or segment text.

/// The five characters governing a message's structure, resolved once from its
/// own MSH-1/MSH-2 rather than hardcoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelimiterSet {
    pub field: u8,
    pub component: u8,
    pub repetition: u8,
    pub escape: u8,
    pub subcomponent: u8,
}

/// One segment's boundary within the message. Segment name is never stored —
/// derive it via [`ScanResult::segment_name`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentSpan {
    pub start: usize,
    pub end: usize,
}

/// Which of the five MSH-declared characters a [`DelimiterOccurrence`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterKind {
    Field,
    Component,
    Repetition,
    Escape,
    Subcomponent,
}

/// One located delimiter character within the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelimiterOccurrence {
    pub segment_index: usize,
    pub offset: usize,
    pub kind: DelimiterKind,
}

/// The scanner's success output for one message: a borrowed reference to the
/// input plus offset-only structure. Never copies field/component text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult<'a> {
    pub message: &'a str,
    pub delimiters: DelimiterSet,
    pub segments: Vec<SegmentSpan>,
    pub delimiter_occurrences: Vec<DelimiterOccurrence>,
}

impl<'a> ScanResult<'a> {
    /// The borrowed 3-byte segment name for a span produced by this same scan.
    pub fn segment_name(&self, segment: &SegmentSpan) -> &'a str {
        &self.message[segment.start..segment.start + 3]
    }
}

/// A structural failure returned instead of a [`ScanResult`] — never a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanError {
    /// The message does not begin with the 3-character segment name `MSH`.
    MissingMsh { offset: usize },
    /// The first segment is `MSH` but ends before a complete 4-character MSH-2.
    TruncatedMsh { offset: usize },
    /// A later segment does not begin with a recognizable segment name.
    UnrecognizedSegment { offset: usize, segment_index: usize },
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanError::MissingMsh { offset } => {
                write!(f, "message does not start with a valid MSH segment (offset {offset})")
            }
            ScanError::TruncatedMsh { offset } => {
                write!(f, "MSH segment is truncated before MSH-2 completes (offset {offset})")
            }
            ScanError::UnrecognizedSegment { offset, segment_index } => {
                write!(
                    f,
                    "segment {segment_index} at offset {offset} has an unrecognized name"
                )
            }
        }
    }
}

impl std::error::Error for ScanError {}

/// Scans `message` and returns either a complete offset map or the first
/// structural error encountered. Never panics.
pub fn scan(message: &str) -> Result<ScanResult<'_>, ScanError> {
    let bytes = message.as_bytes();

    if bytes.len() < 3 || &bytes[0..3] != b"MSH" {
        return Err(ScanError::MissingMsh { offset: 0 });
    }

    let msh_end = first_terminator(bytes).unwrap_or(bytes.len());
    if msh_end < 8 {
        return Err(ScanError::TruncatedMsh { offset: msh_end });
    }

    let delimiters = DelimiterSet {
        field: bytes[3],
        component: bytes[4],
        repetition: bytes[5],
        escape: bytes[6],
        subcomponent: bytes[7],
    };

    // Pass 1: count segments and delimiter occurrences, validating segment
    // names along the way, without allocating anything.
    let mut segment_count = 0usize;
    let mut delimiter_count = 0usize;
    let mut segment_index = 0usize;
    let mut error: Option<ScanError> = None;

    for_each_segment(bytes, |start, end| {
        let name_end = std::cmp::min(start + 3, end);
        if segment_index > 0 && !is_recognizable_segment_name(&bytes[start..name_end]) {
            error = Some(ScanError::UnrecognizedSegment {
                offset: start,
                segment_index,
            });
            return false;
        }
        segment_count += 1;
        for &b in &bytes[start..end] {
            if classify(b, &delimiters).is_some() {
                delimiter_count += 1;
            }
        }
        segment_index += 1;
        true
    });

    if let Some(e) = error {
        return Err(e);
    }

    // Pass 2: fill two pre-sized buffers — the only two heap allocations in a
    // successful scan (data-model.md, research.md #3/#6).
    let mut segments = Vec::with_capacity(segment_count);
    let mut delimiter_occurrences = Vec::with_capacity(delimiter_count);
    let mut segment_index = 0usize;

    for_each_segment(bytes, |start, end| {
        segments.push(SegmentSpan { start, end });
        for (offset, &b) in bytes.iter().enumerate().take(end).skip(start) {
            if let Some(kind) = classify(b, &delimiters) {
                delimiter_occurrences.push(DelimiterOccurrence {
                    segment_index,
                    offset,
                    kind,
                });
            }
        }
        segment_index += 1;
        true
    });

    Ok(ScanResult {
        message,
        delimiters,
        segments,
        delimiter_occurrences,
    })
}

/// Walks `bytes` calling `f(start, end)` for each segment span, splitting on
/// `\r`, `\n`, or `\r\n`. Stops early if `f` returns `false`. A trailing
/// terminator with nothing after it does not produce an empty final segment.
fn for_each_segment(bytes: &[u8], mut f: impl FnMut(usize, usize) -> bool) {
    let len = bytes.len();
    let mut i = 0usize;
    let mut seg_start = 0usize;
    while i < len {
        match bytes[i] {
            b'\r' => {
                if !f(seg_start, i) {
                    return;
                }
                i += 1;
                if i < len && bytes[i] == b'\n' {
                    i += 1;
                }
                seg_start = i;
            }
            b'\n' => {
                if !f(seg_start, i) {
                    return;
                }
                i += 1;
                seg_start = i;
            }
            _ => i += 1,
        }
    }
    if seg_start < len {
        f(seg_start, len);
    }
}

fn first_terminator(bytes: &[u8]) -> Option<usize> {
    bytes.iter().position(|&b| b == b'\r' || b == b'\n')
}

fn classify(byte: u8, d: &DelimiterSet) -> Option<DelimiterKind> {
    if byte == d.field {
        Some(DelimiterKind::Field)
    } else if byte == d.component {
        Some(DelimiterKind::Component)
    } else if byte == d.repetition {
        Some(DelimiterKind::Repetition)
    } else if byte == d.escape {
        Some(DelimiterKind::Escape)
    } else if byte == d.subcomponent {
        Some(DelimiterKind::Subcomponent)
    } else {
        None
    }
}

/// spec 001's tightened `SEG` grammar rule: `ALPHA ALPHANUM ALPHANUM` (research.md #7).
fn is_recognizable_segment_name(name: &[u8]) -> bool {
    fn is_alphanum_upper(b: u8) -> bool {
        b.is_ascii_uppercase() || b.is_ascii_digit()
    }
    name.len() == 3
        && name[0].is_ascii_uppercase()
        && is_alphanum_upper(name[1])
        && is_alphanum_upper(name[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ALLOC_COUNT: Cell<usize> = const { Cell::new(0) };
    }

    struct CountingAllocator;

    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOC_COUNT.with(|c| c.set(c.get() + 1));
            System.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            ALLOC_COUNT.with(|c| c.set(c.get() + 1));
            System.realloc(ptr, layout, new_size)
        }
    }

    #[global_allocator]
    static GLOBAL: CountingAllocator = CountingAllocator;

    fn count_allocs(f: impl FnOnce()) -> usize {
        ALLOC_COUNT.with(|c| c.get()); // touch the thread-local once so first-access lazy init happens outside the measured window
        let before = ALLOC_COUNT.with(|c| c.get());
        f();
        ALLOC_COUNT.with(|c| c.get()) - before
    }

    // T014 (US1, SC-004): allocation count is independent of field/component/
    // repetition count — it varies only with segment count.
    #[test]
    fn allocation_count_independent_of_field_count() {
        let small = "MSH|^~\\&|A|B\nPID|1\n";

        let mut large_body = String::new();
        for i in 0..500 {
            large_body.push_str(&format!("{i}^{i}~{i}&{i}|"));
        }
        let large = format!("MSH|^~\\&|A|B\nPID|{large_body}\n");

        let small_allocs = count_allocs(|| {
            let result = scan(small).unwrap();
            std::hint::black_box(&result);
        });
        let large_allocs = count_allocs(|| {
            let result = scan(&large).unwrap();
            std::hint::black_box(&result);
        });

        assert_eq!(small_allocs, 2, "small message should perform exactly 2 allocations");
        assert_eq!(
            large_allocs, 2,
            "large message (500x more fields/components/repetitions/subcomponents) should still perform exactly 2 allocations"
        );
    }

    // T015 (US1): ScanResult::segment_name returns the correct borrowed slice.
    #[test]
    fn segment_name_returns_first_segment() {
        let message = "MSH|^~\\&|A|B\nPID|1\n";
        let result = scan(message).unwrap();
        assert_eq!(result.segment_name(&result.segments[0]), "MSH");
    }

    #[test]
    fn segment_name_returns_later_segment() {
        let message = "MSH|^~\\&|A|B\nPID|1\n";
        let result = scan(message).unwrap();
        assert_eq!(result.segment_name(&result.segments[1]), "PID");
    }

    // T024 (US3): scan() must return a Result for any input, never panic —
    // including every truncation point of a valid MSH and non-ASCII payload data.
    #[test]
    fn scan_never_panics_on_pathological_input() {
        let mut inputs: Vec<String> = vec![
            String::new(),
            "M".to_string(),
            "\r".to_string(),
            "\n".to_string(),
            "\r\n".to_string(),
            "\r\r\r".to_string(),
            "MSH|^~\\&|Motörhead\nPID|1\n".to_string(),
        ];

        let full = "MSH|^~\\&|SENDAPP|SENDFAC\n";
        for i in 0..=full.len() {
            inputs.push(full[..i].to_string());
        }

        for input in inputs {
            // Discarding the Result is deliberate: this test only asserts
            // scan() returns rather than panics, for every input above.
            let _ = scan(&input);
        }
    }
}
