//! Runs every conformance vector under `fixtures/vectors/scanner/` against
//! `hl7pet_core::scan`, per spec 005-message-scanner FR-010 and
//! contracts/scanner-conformance-vector.schema.json. One #[test] per vector
//! file so `cargo test -- <name>` can target a single user story's vectors
//! (spec 005 quickstart.md steps 4-6).

use std::path::{Path, PathBuf};

use hl7pet_core::{scan, DelimiterKind, DelimiterSet, ScanError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ExpectedDelimiters {
    field: String,
    component: String,
    repetition: String,
    escape: String,
    subcomponent: String,
}

impl ExpectedDelimiters {
    fn as_delimiter_set(&self) -> DelimiterSet {
        fn byte_of(s: &str) -> u8 {
            assert_eq!(s.len(), 1, "delimiter character must be exactly one byte: {s:?}");
            s.as_bytes()[0]
        }
        DelimiterSet {
            field: byte_of(&self.field),
            component: byte_of(&self.component),
            repetition: byte_of(&self.repetition),
            escape: byte_of(&self.escape),
            subcomponent: byte_of(&self.subcomponent),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ExpectedSegment {
    start: usize,
    end: usize,
}

#[derive(Debug, Deserialize)]
struct ExpectedDelimiterOccurrence {
    segment_index: usize,
    offset: usize,
    kind: String,
}

impl ExpectedDelimiterOccurrence {
    fn kind_matches(&self, actual: DelimiterKind) -> bool {
        matches!(
            (self.kind.as_str(), actual),
            ("Field", DelimiterKind::Field)
                | ("Component", DelimiterKind::Component)
                | ("Repetition", DelimiterKind::Repetition)
                | ("Escape", DelimiterKind::Escape)
                | ("Subcomponent", DelimiterKind::Subcomponent)
        )
    }
}

#[derive(Debug, Deserialize)]
struct ExpectedError {
    kind: String,
    offset: usize,
    #[serde(default)]
    segment_index: Option<usize>,
}

impl ExpectedError {
    fn matches(&self, actual: &ScanError) -> bool {
        match (self.kind.as_str(), actual) {
            ("MissingMsh", ScanError::MissingMsh { offset }) => *offset == self.offset,
            ("TruncatedMsh", ScanError::TruncatedMsh { offset }) => *offset == self.offset,
            (
                "UnrecognizedSegment",
                ScanError::UnrecognizedSegment {
                    offset,
                    segment_index,
                },
            ) => *offset == self.offset && Some(*segment_index) == self.segment_index,
            _ => false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ScannerVector {
    id: String,
    message_ref: String,
    #[serde(default)]
    #[allow(dead_code)]
    description: Option<String>,
    #[serde(default)]
    expected_delimiters: Option<ExpectedDelimiters>,
    #[serde(default)]
    expected_segments: Option<Vec<ExpectedSegment>>,
    #[serde(default)]
    expected_delimiter_occurrences: Option<Vec<ExpectedDelimiterOccurrence>>,
    #[serde(default)]
    expected_error: Option<ExpectedError>,
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn load_vectors(file_name: &str) -> Vec<(PathBuf, ScannerVector)> {
    let path = fixtures_root().join("vectors").join("scanner").join(file_name);
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let records: Vec<ScannerVector> =
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));
    records.into_iter().map(|r| (path.clone(), r)).collect()
}

/// Shared assertion logic for one vector file, run by each of the three
/// per-family tests below (standard delimiters, non-standard delimiters,
/// malformed MSH — spec 005 quickstart.md steps 4-6).
fn assert_vectors(vectors: &[(PathBuf, ScannerVector)]) {
    let fixtures = fixtures_root();

    for (file, vector) in vectors {
        let message_path = fixtures.join(&vector.message_ref);
        let message = std::fs::read_to_string(&message_path).unwrap_or_else(|e| {
            panic!(
                "{}: {}: failed to read message_ref {}: {e}",
                file.display(),
                vector.id,
                message_path.display()
            )
        });

        let result = scan(&message);

        if let Some(expected_error) = &vector.expected_error {
            let err = match result {
                Err(e) => e,
                Ok(_) => panic!(
                    "{}: {}: expected error {:?} but scan succeeded",
                    file.display(),
                    vector.id,
                    expected_error
                ),
            };
            assert!(
                expected_error.matches(&err),
                "{}: {}: expected error {:?}, got {:?}",
                file.display(),
                vector.id,
                expected_error,
                err
            );
            continue;
        }

        let scan_result = result.unwrap_or_else(|e| {
            panic!(
                "{}: {}: expected success but scan failed: {e}",
                file.display(),
                vector.id
            )
        });

        if let Some(expected_delimiters) = &vector.expected_delimiters {
            assert_eq!(
                scan_result.delimiters,
                expected_delimiters.as_delimiter_set(),
                "{}: {}: delimiter mismatch",
                file.display(),
                vector.id
            );
        }

        if let Some(expected_segments) = &vector.expected_segments {
            assert_eq!(
                scan_result.segments.len(),
                expected_segments.len(),
                "{}: {}: segment count mismatch",
                file.display(),
                vector.id
            );
            for (actual, expected) in scan_result.segments.iter().zip(expected_segments) {
                assert_eq!(
                    (actual.start, actual.end),
                    (expected.start, expected.end),
                    "{}: {}: segment span mismatch",
                    file.display(),
                    vector.id
                );
            }
        }

        if let Some(expected_occurrences) = &vector.expected_delimiter_occurrences {
            assert_eq!(
                scan_result.delimiter_occurrences.len(),
                expected_occurrences.len(),
                "{}: {}: delimiter occurrence count mismatch",
                file.display(),
                vector.id
            );
            for (actual, expected) in scan_result
                .delimiter_occurrences
                .iter()
                .zip(expected_occurrences)
            {
                assert_eq!(
                    actual.segment_index, expected.segment_index,
                    "{}: {}: delimiter occurrence segment_index mismatch",
                    file.display(),
                    vector.id
                );
                assert_eq!(
                    actual.offset, expected.offset,
                    "{}: {}: delimiter occurrence offset mismatch",
                    file.display(),
                    vector.id
                );
                assert!(
                    expected.kind_matches(actual.kind),
                    "{}: {}: delimiter occurrence kind mismatch (expected {}, got {:?})",
                    file.display(),
                    vector.id,
                    expected.kind,
                    actual.kind
                );
            }
        }
    }
}

#[test]
fn standard_delimiters() {
    assert_vectors(&load_vectors("standard-delimiters.json"));
}

#[test]
fn non_standard_delimiters() {
    assert_vectors(&load_vectors("non-standard-delimiters.json"));
}

#[test]
fn malformed_msh() {
    assert_vectors(&load_vectors("malformed-msh.json"));
}
