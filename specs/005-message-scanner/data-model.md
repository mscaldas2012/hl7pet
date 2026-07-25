# Data Model: Message Scanner

Entities carried over from [spec.md](spec.md)'s Key Entities section, made concrete
against the design decisions in [research.md](research.md). Types are given in Rust
since this spec's deliverable is Rust source (`crates/core/src/scanner.rs`); see
[contracts/scanner-api.md](contracts/scanner-api.md) for the full public API surface.

## DelimiterSet

The five characters resolved once per message from its own MSH-1/MSH-2 (research.md
has no open question here — this is FR-002/FR-003 made concrete).

| Field | Type | Notes |
|---|---|---|
| `field` | `u8` | From MSH-1 — the single character immediately after the segment name `MSH`. |
| `component` | `u8` | From MSH-2, position 1. |
| `repetition` | `u8` | From MSH-2, position 2. |
| `escape` | `u8` | From MSH-2, position 3. |
| `subcomponent` | `u8` | From MSH-2, position 4. |

`u8` rather than `char`: HL7 delimiters are always single-byte ASCII per the standard
and every prior spec's fixtures; borrowing raw bytes throughout (see `ScanResult`
below) makes `u8` the natural comparison type and avoids UTF-8 decoding on the hot
path. FR-005's byte-identical-to-hardcoded requirement for standard messages is
trivially satisfied since `DelimiterSet { field: b'|', component: b'^', repetition:
b'~', escape: b'\\', subcomponent: b'&' }` is exactly what a hardcoded scanner would
have used.

## SegmentSpan

One segment's boundary within the message (research.md #5 — no stored name).

| Field | Type | Notes |
|---|---|---|
| `start` | `usize` | Byte offset of the segment's first byte (the start of its 3-character name). |
| `end` | `usize` | Byte offset one past the segment's last content byte, *before* its terminator (`\r`/`\n`/`\r\n`, FR-008). |

Segment name is never stored — derive via `&message[span.start..span.start + 3]`
(research.md #5). A helper method on `ScanResult` (contracts/scanner-api.md) provides
this without callers re-deriving the slicing logic themselves.

## DelimiterKind

| Variant | Corresponds to |
|---|---|
| `Field` | `DelimiterSet.field` |
| `Component` | `DelimiterSet.component` |
| `Repetition` | `DelimiterSet.repetition` |
| `Escape` | `DelimiterSet.escape` |
| `Subcomponent` | `DelimiterSet.subcomponent` |

All five are tracked as occurrences per research.md #4.

## DelimiterOccurrence

One located delimiter character within the message.

| Field | Type | Notes |
|---|---|---|
| `segment_index` | `usize` | Index into `ScanResult.segments` — which segment this occurrence falls within. |
| `offset` | `usize` | Absolute byte offset in the message (not relative to the segment). |
| `kind` | `DelimiterKind` | Which of the five delimiter characters this is. |

Stored in a single message-wide `Vec<DelimiterOccurrence>` per research.md #3, in
ascending `offset` order (a natural consequence of the left-to-right scan) — so a
caller wanting "all delimiters in segment N" filters or binary-searches by
`segment_index` rather than the scanner maintaining a separate per-segment index.

## ScanResult

The scanner's success output for one message (spec.md's "Scan Result / Offset Map"
Key Entity).

| Field | Type | Notes |
|---|---|---|
| `message` | `&'a str` | Borrowed reference to the original input — never copied (Principle II). |
| `delimiters` | `DelimiterSet` | Resolved once from this message's own MSH-1/MSH-2 (FR-002/FR-003). |
| `segments` | `Vec<SegmentSpan>` | One entry per segment, in message order; length = segment count. |
| `delimiter_occurrences` | `Vec<DelimiterOccurrence>` | Every delimiter character location across the whole message, ascending `offset` order. |

Lifetime `'a` ties `ScanResult` to the input `&str`'s lifetime — it cannot outlive the
message it was scanned from, which is what makes the zero-copy design sound (there is
no ownership to manage, only a borrow).

Total heap allocations for a successful scan: exactly 2 (`segments`, `delimiter_
occurrences`), each allocated once at its final size (research.md #3, #6) —
this is the concrete mechanism behind SC-004.

## ScanError

The scanner's failure output (spec.md's "Structural Error" Key Entity), returned
instead of a `ScanResult` per Constitution Principle III — never a panic.

| Variant | Fields | Corresponds to (spec.md FR-006) |
|---|---|---|
| `MissingMsh` | `{ offset: usize }` | Message does not begin with the 3-character segment name `MSH`. `offset` is always `0`. |
| `TruncatedMsh` | `{ offset: usize }` | First segment is `MSH` but shorter than the minimum length needed to contain MSH-1 and a complete 4-character MSH-2. `offset` is the byte position where the segment ends prematurely. |
| `UnrecognizedSegment` | `{ offset: usize, segment_index: usize }` | A later segment does not begin with a recognizable segment name (research.md #7). `offset` is the segment's start byte; `segment_index` is its position among already-scanned segments. |

Every variant carries enough to satisfy FR-007 (specific problem + byte offset)
without a caller needing to re-scan to find where things went wrong.

## Relationships / State

```text
scan(message: &str) -> Result<ScanResult<'_>, ScanError>
```

There is no mutable state and no intermediate "in-progress scan" object exposed
publicly — `scan` is a pure function from input text to either a complete
`ScanResult` or the first `ScanError` encountered (FR-006 conditions are checked in
message order; the first violation found is returned, per the Edge Cases' "identify
which segment and byte offset" requirement — the scanner does not attempt to collect
multiple errors in one pass).
