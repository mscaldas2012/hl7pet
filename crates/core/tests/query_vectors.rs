//! Runs every non-hierarchy conformance vector under `fixtures/vectors/path/`
//! against `hl7pet_core::query::execute`, per spec 007-query-execution FR-010
//! and the existing `conformance-vector.schema.json` (spec 001, reused as-is —
//! no new schema, plan.md Structure Decision). Hierarchy vectors (path
//! containing `" -> "`) are out of scope here (spec.md Assumptions, spec 008).

use std::fs;
use std::path::{Path, PathBuf};

use hl7pet_core::{execute, parse, scan, QueryError};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct PathVector {
    id: String,
    path: String,
    message_ref: String,
    method: String,
    expected: Value,
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn load_vectors(file_name: &str) -> Vec<PathVector> {
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

const NONNUMERIC_COMPARISON_SENTINEL: &str = "ERROR:NonNumericComparison";

#[test]
fn non_hierarchy_valid_vectors_execute_to_expected_value() {
    for vector in load_vectors("valid.json") {
        if is_hierarchy(&vector.path) {
            continue;
        }
        dispatch(&vector);
    }
}

fn dispatch(vector: &PathVector) {
    let message = load_message(&vector.message_ref);
    let scan_result = scan(&message)
        .unwrap_or_else(|e| panic!("{}: scanning {} failed: {e}", vector.id, vector.message_ref));
    let compiled = parse(&vector.path)
        .unwrap_or_else(|e| panic!("{}: parsing {:?} failed: {e}", vector.id, vector.path));

    if vector.expected == Value::String(NONNUMERIC_COMPARISON_SENTINEL.to_string()) {
        match execute(&scan_result, &compiled) {
            Err(QueryError::NonNumericComparison { .. }) => {}
            other => panic!(
                "{}: expected Err(QueryError::NonNumericComparison), got {other:?}",
                vector.id
            ),
        }
        return;
    }

    let values = execute(&scan_result, &compiled)
        .unwrap_or_else(|e| panic!("{}: execute() returned unexpected error {e}", vector.id));

    match vector.method.as_str() {
        "getValue" => assert_get_value(&vector.id, &values, &vector.expected),
        "getFirstValue" => assert_get_first_value(&vector.id, &values, &vector.expected),
        other => panic!("{}: unknown method {other:?}", vector.id),
    }
}

fn assert_get_value(id: &str, values: &[Vec<&str>], expected: &Value) {
    match expected {
        Value::Null => assert!(values.is_empty(), "{id}: expected no match, got {values:?}"),
        Value::Array(outer) => {
            let actual: Vec<Vec<&str>> = values.to_vec();
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
            assert_eq!(
                actual
                    .iter()
                    .map(|inner| inner.iter().map(|s| s.to_string()).collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
                expected_shape,
                "{id}: getValue mismatch"
            );
        }
        other => panic!("{id}: unexpected expected shape for getValue: {other:?}"),
    }
}

fn assert_get_first_value(id: &str, values: &[Vec<&str>], expected: &Value) {
    let actual = values.first().and_then(|reps| reps.first()).copied();
    match expected {
        Value::Null => assert!(actual.is_none(), "{id}: expected None, got {actual:?}"),
        Value::String(s) => assert_eq!(actual, Some(s.as_str()), "{id}: getFirstValue mismatch"),
        other => panic!("{id}: unexpected expected shape for getFirstValue: {other:?}"),
    }
}
