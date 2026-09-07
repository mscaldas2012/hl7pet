//! Runs every conformance vector under `fixtures/vectors/escapes/` against
//! `hl7pet_core::query::execute`, per spec 1001-escape-sequence-decoding
//! SC-001: proves every documented escape-sequence type decodes correctly.
//! Kept as its own family, separate from `fixtures/vectors/path/`, since no
//! message in that pre-existing corpus contains an escape sequence at all
//! (spec.md Assumptions) — mirrors `query_vectors.rs`'s loading pattern.

use std::fs;
use std::path::{Path, PathBuf};

use hl7pet_core::{execute, parse, scan};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct EscapeVector {
    id: String,
    path: String,
    message_ref: String,
    method: String,
    expected: Value,
    escape_types: Vec<String>,
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn load_vectors(file_name: &str) -> Vec<EscapeVector> {
    let file = fixtures_root().join("vectors").join("escapes").join(file_name);
    let content =
        fs::read_to_string(&file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("parsing {}: {e}", file.display()))
}

fn load_message(message_ref: &str) -> String {
    let file = fixtures_root().join(message_ref);
    fs::read_to_string(&file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()))
}

const ALL_ESCAPE_TYPES: &[&str] = &["F", "S", "T", "R", "E", "H_N", "X", "Z"];

#[test]
fn escape_vectors_decode_to_expected_value() {
    let mut checked = 0;
    let mut covered: Vec<String> = Vec::new();
    for vector in load_vectors("valid.json") {
        dispatch(&vector);
        covered.extend(vector.escape_types.iter().cloned());
        checked += 1;
    }
    assert!(checked > 0, "expected at least one escape vector to run");

    // SC-001: every documented escape-sequence type has at least one vector.
    for escape_type in ALL_ESCAPE_TYPES {
        assert!(
            covered.iter().any(|t| t == escape_type),
            "no vector exercises escape_type {escape_type:?} — SC-001 requires 8/8 coverage"
        );
    }
}

fn dispatch(vector: &EscapeVector) {
    let message = load_message(&vector.message_ref);
    let scan_result = scan(&message)
        .unwrap_or_else(|e| panic!("{}: scanning {} failed: {e}", vector.id, vector.message_ref));
    let compiled = parse(&vector.path)
        .unwrap_or_else(|e| panic!("{}: parsing {:?} failed: {e}", vector.id, vector.path));

    let values = execute(&scan_result, &compiled)
        .unwrap_or_else(|e| panic!("{}: execute() returned unexpected error {e}", vector.id));

    match vector.method.as_str() {
        "getValue" => assert_get_value(&vector.id, &values, &vector.expected),
        "getFirstValue" => assert_get_first_value(&vector.id, &values, &vector.expected),
        other => panic!("{}: unknown method {other:?}", vector.id),
    }
}

fn assert_get_value(id: &str, values: &[Vec<std::borrow::Cow<'_, str>>], expected: &Value) {
    match expected {
        Value::Null => assert!(values.is_empty(), "{id}: expected no match, got {values:?}"),
        Value::Array(outer) => {
            let expected_shape: Vec<Vec<String>> = outer
                .iter()
                .map(|inner| {
                    inner
                        .as_array()
                        .unwrap_or_else(|| panic!("{id}: expected getValue array-of-arrays"))
                        .iter()
                        .map(|v| v.as_str().unwrap().to_string())
                        .collect()
                })
                .collect();
            let actual: Vec<Vec<String>> = values
                .iter()
                .map(|inner| inner.iter().map(|s| s.as_ref().to_string()).collect())
                .collect();
            assert_eq!(actual, expected_shape, "{id}: getValue mismatch");
        }
        other => panic!("{id}: unexpected expected shape for getValue: {other:?}"),
    }
}

fn assert_get_first_value(id: &str, values: &[Vec<std::borrow::Cow<'_, str>>], expected: &Value) {
    let actual = values.first().and_then(|reps| reps.first()).map(|v| v.as_ref());
    match expected {
        Value::Null => assert!(actual.is_none(), "{id}: expected None, got {actual:?}"),
        Value::String(s) => assert_eq!(actual, Some(s.as_str()), "{id}: getFirstValue mismatch"),
        other => panic!("{id}: unexpected expected shape for getFirstValue: {other:?}"),
    }
}
