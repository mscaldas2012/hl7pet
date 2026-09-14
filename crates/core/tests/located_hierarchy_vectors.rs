//! Runs every single-hop conformance vector under `fixtures/vectors/hierarchy/`
//! that carries `expected_lines` metadata against
//! `hl7pet_core::execute_hierarchy_located`, per spec
//! 011-located-hierarchy-api FR-010: reuse the line-number metadata already
//! present (but previously unused) in the shared fixtures corpus rather than
//! deriving new expected line numbers independently. Mirrors
//! `hierarchy_vectors.rs`'s loader/exclusion conventions exactly.
//!
//! Multi-hop vectors (`hier-009`, `hier-010`) are excluded for the same
//! reason `hierarchy_vectors.rs` excludes them (spec 006's parser already
//! rejects a second `" -> "` hop outright) — this feature does not change
//! that scope.

use std::fs;
use std::path::{Path, PathBuf};

use hl7pet_core::{execute_hierarchy_located, parse, scan, HierarchyProfile, LocatedValue};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct HierarchyFlags {
    #[serde(rename = "buildHierarchy")]
    build_hierarchy: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LocatedHierarchyVector {
    id: String,
    path: String,
    profile_ref: String,
    message_ref: String,
    method: String,
    flags: Option<HierarchyFlags>,
    expected: Value,
    expected_lines: Option<Value>,
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn load_vectors(file_name: &str) -> Vec<LocatedHierarchyVector> {
    let file = fixtures_root().join("vectors").join("hierarchy").join(file_name);
    let content =
        fs::read_to_string(&file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("parsing {}: {e}", file.display()))
}

fn load_message(message_ref: &str) -> String {
    let file = fixtures_root().join(message_ref);
    fs::read_to_string(&file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()))
}

fn load_profile(profile_ref: &str) -> String {
    let file = fixtures_root().join(profile_ref);
    fs::read_to_string(&file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()))
}

/// True for a PATH with more than one `" -> "` hop — out of scope for this
/// spec, and unparseable by spec 006's parser regardless (identical to
/// `hierarchy_vectors.rs`'s own exclusion).
fn is_multi_hop(path: &str) -> bool {
    path.matches(" -> ").count() > 1
}

#[test]
fn all_located_hierarchy_vectors_with_expected_lines_match() {
    for vector in load_vectors("basic.json").into_iter().chain(load_vectors("complex.json")) {
        if is_multi_hop(&vector.path) || vector.expected_lines.is_none() {
            continue;
        }
        dispatch(&vector);
    }
}

fn dispatch(vector: &LocatedHierarchyVector) {
    let message = load_message(&vector.message_ref);
    let scan_result =
        scan(&message).unwrap_or_else(|e| panic!("{}: scanning {} failed: {e}", vector.id, vector.message_ref));
    let compiled =
        parse(&vector.path).unwrap_or_else(|e| panic!("{}: parsing {:?} failed: {e}", vector.id, vector.path));

    let build_hierarchy = vector.flags.as_ref().and_then(|f| f.build_hierarchy).unwrap_or(true);
    let profile = if build_hierarchy {
        let profile_json = load_profile(&vector.profile_ref);
        Some(
            HierarchyProfile::from_json(&profile_json)
                .unwrap_or_else(|e| panic!("{}: parsing profile {}: {e}", vector.id, vector.profile_ref)),
        )
    } else {
        None
    };

    let values = execute_hierarchy_located(&scan_result, &compiled, profile.as_ref())
        .unwrap_or_else(|e| panic!("{}: execute_hierarchy_located() returned unexpected error {e}", vector.id));

    match vector.method.as_str() {
        "getValue" => assert_get_value(&vector.id, &values, &vector.expected, vector.expected_lines.as_ref()),
        other => panic!("{}: unexpected method {other:?} for a located-hierarchy vector", other),
    }
}

fn assert_get_value(id: &str, values: &[Vec<LocatedValue<'_>>], expected: &Value, expected_lines: Option<&Value>) {
    match expected {
        Value::Null => assert!(values.is_empty(), "{id}: expected no match, got {values:?}"),
        Value::Array(outer) => {
            let actual_values: Vec<Vec<String>> =
                values.iter().map(|inner| inner.iter().map(|lv| lv.value.to_string()).collect()).collect();
            let expected_values: Vec<Vec<String>> = outer
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
            assert_eq!(actual_values, expected_values, "{id}: value mismatch");

            let actual_lines: Vec<Vec<usize>> =
                values.iter().map(|inner| inner.iter().map(|lv| lv.line).collect()).collect();
            let expected_lines = expected_lines.unwrap_or_else(|| panic!("{id}: missing expected_lines"));
            let expected_lines: Vec<Vec<usize>> = expected_lines
                .as_array()
                .unwrap_or_else(|| panic!("{id}: expected_lines must be an array-of-arrays"))
                .iter()
                .map(|inner| {
                    inner
                        .as_array()
                        .unwrap_or_else(|| panic!("{id}: expected_lines must be an array-of-arrays"))
                        .iter()
                        .map(|v| v.as_u64().unwrap() as usize)
                        .collect()
                })
                .collect();
            assert_eq!(actual_lines, expected_lines, "{id}: line mismatch");
        }
        other => panic!("{id}: unexpected expected shape for getValue: {other:?}"),
    }
}
