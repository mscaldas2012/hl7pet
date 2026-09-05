//! Runs every conformance vector under `fixtures/vectors/path/` that carries
//! `expected_lines` metadata (spec 001 FR-008) against
//! `hl7pet_core::query::execute_located`, per spec 1000-located-extraction-api
//! FR-009: reuse the line-number metadata the shared fixtures corpus already
//! records rather than deriving new expected line numbers independently.
//! Covers both US1 (single-occurrence vectors, e.g. `path-msh12`) and US2
//! (multi-occurrence/filtered vectors, e.g. `path-obx5-occurrences`,
//! `path-filter-multi-match`) in one dispatcher — both exercise the same
//! `execute_located` code path at different cardinalities (tasks.md's
//! Organization note). Hierarchy vectors (`" -> "` in the path) are out of
//! scope entirely (spec.md Assumptions, spec 008), matching
//! `query_vectors.rs`'s own exclusion.

use std::fs;
use std::path::{Path, PathBuf};

use hl7pet_core::{execute_located, parse, scan, LocatedValue};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct LocatedPathVector {
    id: String,
    path: String,
    message_ref: String,
    method: String,
    expected: Value,
    expected_lines: Option<Value>,
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn load_vectors(file_name: &str) -> Vec<LocatedPathVector> {
    let file = fixtures_root().join("vectors").join("path").join(file_name);
    let content =
        fs::read_to_string(&file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("parsing {}: {e}", file.display()))
}

fn load_message(message_ref: &str) -> String {
    let file = fixtures_root().join(message_ref);
    fs::read_to_string(&file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()))
}

fn is_hierarchy(path: &str) -> bool {
    path.contains(" -> ")
}

/// US1 scope: at most one matched segment occurrence. `getFirstValue`
/// vectors are always single-occurrence by definition; `getValue` vectors
/// qualify only when `expected`'s outer array has zero or one entries.
fn is_single_occurrence(vector: &LocatedPathVector) -> bool {
    match vector.method.as_str() {
        "getFirstValue" => true,
        "getValue" => match &vector.expected {
            Value::Null => true,
            Value::Array(outer) => outer.len() <= 1,
            _ => false,
        },
        _ => false,
    }
}

#[test]
fn single_occurrence_vectors_report_expected_value_and_line() {
    let mut checked = 0;
    for vector in load_vectors("valid.json") {
        if is_hierarchy(&vector.path) || vector.expected_lines.is_none() || !is_single_occurrence(&vector) {
            continue;
        }
        dispatch(&vector);
        checked += 1;
    }
    assert!(
        checked > 0,
        "expected at least one single-occurrence vector with expected_lines to run"
    );
}

/// T012 (US2): every remaining `expected_lines`-bearing vector this file's
/// other test doesn't already cover — multi-occurrence and filtered-to-
/// several vectors, e.g. `path-obx5-occurrences` (3 occurrences) and
/// `path-filter-multi-match`.
#[test]
fn multi_occurrence_vectors_report_expected_value_and_line_per_occurrence() {
    let mut checked = 0;
    for vector in load_vectors("valid.json") {
        if is_hierarchy(&vector.path) || vector.expected_lines.is_none() || is_single_occurrence(&vector) {
            continue;
        }
        dispatch(&vector);
        checked += 1;
    }
    assert!(
        checked > 0,
        "expected at least one multi-occurrence vector with expected_lines to run"
    );
}

fn dispatch(vector: &LocatedPathVector) {
    let message = load_message(&vector.message_ref);
    let scan_result = scan(&message)
        .unwrap_or_else(|e| panic!("{}: scanning {} failed: {e}", vector.id, vector.message_ref));
    let compiled = parse(&vector.path)
        .unwrap_or_else(|e| panic!("{}: parsing {:?} failed: {e}", vector.id, vector.path));

    let located = execute_located(&scan_result, &compiled)
        .unwrap_or_else(|e| panic!("{}: execute_located() returned unexpected error {e}", vector.id));

    match vector.method.as_str() {
        "getValue" => assert_get_value_located(&vector.id, &located, &vector.expected, vector.expected_lines.as_ref().unwrap()),
        "getFirstValue" => assert_get_first_value_located(&vector.id, &located, &vector.expected, vector.expected_lines.as_ref().unwrap()),
        other => panic!("{}: unknown method {other:?}", vector.id),
    }
}

fn assert_get_value_located(id: &str, located: &[Vec<LocatedValue<'_>>], expected: &Value, expected_lines: &Value) {
    match (expected, expected_lines) {
        (Value::Null, _) => assert!(located.is_empty(), "{id}: expected no match, got {located:?}"),
        (Value::Array(outer_values), Value::Array(outer_lines)) => {
            assert_eq!(outer_values.len(), outer_lines.len(), "{id}: expected/expected_lines shape mismatch");
            assert_eq!(located.len(), outer_values.len(), "{id}: occurrence-count mismatch");
            for (i, (inner_values, inner_lines)) in outer_values.iter().zip(outer_lines).enumerate() {
                let inner_values = inner_values.as_array().unwrap_or_else(|| panic!("{id}: expected array-of-arrays"));
                let inner_lines = inner_lines.as_array().unwrap_or_else(|| panic!("{id}: expected_lines array-of-arrays"));
                assert_eq!(located[i].len(), inner_values.len(), "{id}: value-count mismatch at occurrence {i}");
                for (j, lv) in located[i].iter().enumerate() {
                    let expected_value = inner_values[j].as_str().unwrap();
                    let expected_line = inner_lines[j].as_u64().unwrap() as usize;
                    assert_eq!(lv.value, expected_value, "{id}: value mismatch at [{i}][{j}]");
                    assert_eq!(lv.line, expected_line, "{id}: line mismatch at [{i}][{j}]");
                }
            }
        }
        other => panic!("{id}: unexpected expected/expected_lines shape for getValue: {other:?}"),
    }
}

fn assert_get_first_value_located(id: &str, located: &[Vec<LocatedValue<'_>>], expected: &Value, expected_lines: &Value) {
    let actual = located.first().and_then(|group| group.first());
    match (expected, expected_lines) {
        (Value::Null, _) => assert!(actual.is_none(), "{id}: expected None, got {actual:?}"),
        (Value::String(s), Value::Array(lines)) => {
            let lv = actual.unwrap_or_else(|| panic!("{id}: expected a value, got none"));
            assert_eq!(lv.value, s.as_str(), "{id}: getFirstValue value mismatch");
            let expected_line = lines
                .first()
                .unwrap_or_else(|| panic!("{id}: expected_lines must have one entry for getFirstValue"))
                .as_u64()
                .unwrap() as usize;
            assert_eq!(lv.line, expected_line, "{id}: getFirstValue line mismatch");
        }
        other => panic!("{id}: unexpected expected/expected_lines shape for getFirstValue: {other:?}"),
    }
}
