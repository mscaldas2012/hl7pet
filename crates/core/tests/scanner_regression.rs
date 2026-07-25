//! Regression test for spec 005-message-scanner FR-005/SC-001: every message
//! already used by specs 001-003 (all standard-delimiter) must resolve to
//! exactly the delimiters a hardcoded scanner would have used, and must scan
//! without error — proving dynamic MSH-1/MSH-2 reading changes nothing for
//! the common case.

use std::path::Path;

use hl7pet_core::{scan, DelimiterSet};

/// Fixed list of messages already used by specs 001-003, predating this
/// spec's own `scanner-*.hl7` fixtures. Hardcoded rather than globbed so this
/// test never accidentally tries to scan spec 005's own non-standard-
/// delimiter or malformed-MSH fixtures, which are deliberately not
/// standard-delimiter messages.
const PRE_EXISTING_MESSAGES: &[&str] = &[
    "baseline.hl7",
    "basic-hierarchy.hl7",
    "complex-hierarchy.hl7",
    "filter-example.hl7",
    "multi-obx.hl7",
    "multi-repetition.hl7",
    "unrecognized-segment.hl7",
];

const STANDARD_DELIMITERS: DelimiterSet = DelimiterSet {
    field: b'|',
    component: b'^',
    repetition: b'~',
    escape: b'\\',
    subcomponent: b'&',
};

#[test]
fn standard_delimiter_corpus_has_zero_regressions() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/messages");

    for name in PRE_EXISTING_MESSAGES {
        let path = fixtures.join(name);
        let message = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));

        let result = scan(&message)
            .unwrap_or_else(|e| panic!("{name}: expected successful scan, got error: {e}"));

        assert_eq!(
            result.delimiters, STANDARD_DELIMITERS,
            "{name}: resolved delimiters must match what a hardcoded scanner would have used"
        );
    }
}
