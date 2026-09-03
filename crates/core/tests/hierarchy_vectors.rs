//! Runs every single-hop conformance vector under `fixtures/vectors/hierarchy/`
//! against `hl7pet_core::execute_hierarchy`, per spec 008-lazy-hierarchy-nav
//! FR-010 and the existing `conformance-vector.schema.json` (spec 002/003,
//! reused as-is — no new schema, plan.md Structure Decision).
//!
//! Two vectors (`hier-009`, `hier-010`) use a two-hop PATH
//! (`"OBR[1] -> OBX[3] -> NTE-3"`) added by spec 002/003 anticipating
//! spec 002 Section B.2's multi-hop recommendation. Spec 008's Clarifications
//! deferred multi-hop chaining to a future spec, and spec 006's parser
//! already rejects a second `" -> "` outright (`MultipleHierarchyHops`) — so
//! these two vectors cannot be parsed at all under this spec's scope, let
//! alone executed. They are skipped here, not silently miscounted as
//! passing; whichever future spec implements multi-hop chaining is the
//! right place to exercise them (`hier-010` in particular is already the
//! right shape for that spec's own conformance suite).

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct HierarchyFlags {
    #[serde(rename = "buildHierarchy")]
    build_hierarchy: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct HierarchyVector {
    id: String,
    path: String,
    profile_ref: String,
    message_ref: String,
    method: String,
    flags: Option<HierarchyFlags>,
    expected: Value,
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn load_vectors(file_name: &str) -> Vec<HierarchyVector> {
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
/// spec (multi-hop deferred, spec.md Clarifications), and unparseable by
/// spec 006's parser regardless.
fn is_multi_hop(path: &str) -> bool {
    path.matches(" -> ").count() > 1
}

#[test]
fn all_hierarchy_vectors_execute_to_expected_value() {
    for vector in load_vectors("basic.json").into_iter().chain(load_vectors("complex.json")) {
        if is_multi_hop(&vector.path) {
            continue;
        }
        dispatch(&vector);
    }
}

fn dispatch(vector: &HierarchyVector) {
    let message = load_message(&vector.message_ref);
    let scan_result = hl7pet_core::scan(&message)
        .unwrap_or_else(|e| panic!("{}: scanning {} failed: {e}", vector.id, vector.message_ref));
    let compiled = hl7pet_core::parse(&vector.path)
        .unwrap_or_else(|e| panic!("{}: parsing {:?} failed: {e}", vector.id, vector.path));

    let build_hierarchy = vector.flags.as_ref().and_then(|f| f.build_hierarchy).unwrap_or(true);
    let profile = if build_hierarchy {
        let profile_json = load_profile(&vector.profile_ref);
        Some(
            hl7pet_core::HierarchyProfile::from_json(&profile_json)
                .unwrap_or_else(|e| panic!("{}: parsing profile {}: {e}", vector.id, vector.profile_ref)),
        )
    } else {
        None
    };

    let values = hl7pet_core::execute_hierarchy(&scan_result, &compiled, profile.as_ref())
        .unwrap_or_else(|e| panic!("{}: execute_hierarchy() returned unexpected error {e}", vector.id));

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
            let actual: Vec<Vec<String>> = values
                .iter()
                .map(|inner| inner.iter().map(|s| s.to_string()).collect())
                .collect();
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
            assert_eq!(actual, expected_shape, "{id}: getValue mismatch");
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
