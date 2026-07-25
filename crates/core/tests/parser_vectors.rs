//! Runs every conformance vector under `fixtures/vectors/path/` against
//! `hl7pet_core::parse`, per spec 006-path-parser FR-001/FR-012 and the
//! existing `conformance-vector.schema.json` (spec 001, reused as-is — no new
//! schema, plan.md Structure Decision). This spec checks accept/reject only:
//! a vector's `expected` field is the literal string `"INVALID"` for a
//! rejected PATH, or anything else (array/string/null) for one that must
//! parse successfully — the *value* an evaluator would extract is spec 007's
//! concern, not this one's. A curated subset also asserts specific compiled
//! fields (US3, T017).

use std::path::{Path, PathBuf};

use hl7pet_core::{parse, FilterOperator};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct PathVector {
    id: String,
    path: String,
    expected: Value,
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn load_vectors(file_name: &str) -> Vec<PathVector> {
    let file = fixtures_root().join("vectors").join("path").join(file_name);
    let content =
        std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("parsing {}: {e}", file.display()))
}

fn is_invalid_sentinel(expected: &Value) -> bool {
    expected.as_str() == Some("INVALID")
}

#[test]
fn valid_paths_parse_successfully() {
    for vector in load_vectors("valid.json") {
        assert!(
            !is_invalid_sentinel(&vector.expected),
            "{}: valid.json entry has expected == \"INVALID\", which belongs in invalid.json",
            vector.id
        );
        parse(&vector.path).unwrap_or_else(|e| {
            panic!("{}: expected {:?} to parse, got error {e}", vector.id, vector.path)
        });
    }
}

#[test]
fn invalid_paths_are_rejected() {
    for vector in load_vectors("invalid.json") {
        assert!(
            is_invalid_sentinel(&vector.expected),
            "{}: invalid.json entry does not have expected == \"INVALID\"",
            vector.id
        );
        if let Ok(compiled) = parse(&vector.path) {
            panic!(
                "{}: expected {:?} to be rejected, but it parsed as {compiled:?}",
                vector.id, vector.path
            );
        }
    }
}

/// US3 (T017): a curated subset asserts specific `CompiledPath` fields, not
/// just accept/reject — proving the compiled representation is structured
/// data (spec.md Acceptance Scenarios 1-3), not just a validated string.
#[test]
fn compiled_fields_match_expected_shape_for_representative_paths() {
    let bare = parse("PID[1]-5").expect("PID[1]-5 must parse");
    assert_eq!(bare.segment.name, "PID");
    assert_eq!(
        bare.segment.index,
        Some(hl7pet_core::SegIndex::Numeric(1))
    );
    assert_eq!(bare.field.unwrap().field_num, 5);

    let filter = parse("OBX[@3.1='94500-6']-5").expect("filter PATH must parse");
    let clause = match filter.segment.index {
        Some(hl7pet_core::SegIndex::Filter(ref c)) => c,
        other => panic!("expected a filter clause, got {other:?}"),
    };
    assert_eq!(clause.field_num, 3);
    assert_eq!(clause.component, Some(1));
    assert_eq!(clause.subcomponent, None);
    assert_eq!(clause.operator, FilterOperator::Eq);
    assert_eq!(clause.values, vec!["94500-6"]);

    let hierarchy = parse("OBR[1] -> OBX-5").expect("hierarchy PATH must parse");
    assert!(hierarchy.field.is_none());
    let child = hierarchy.child.expect("hierarchy PATH must have a child");
    assert_eq!(child.segment.name, "OBX");
    assert_eq!(child.field.unwrap().field_num, 5);
}

/// T017 (US3, FR-012): the 3 new vectors this spec adds must compile to the
/// exact structured shape their id's grammar feature implies — tied to the
/// vector file's own `path` string (not a hardcoded duplicate), so this test
/// breaks if the fixture's path ever changes without updating this
/// assertion.
#[test]
fn new_vectors_compile_to_expected_structured_shape() {
    let vectors = load_vectors("valid.json");
    let find = |id: &str| -> String {
        vectors
            .iter()
            .find(|v| v.id == id)
            .unwrap_or_else(|| panic!("vector {id} not found in valid.json"))
            .path
            .clone()
    };

    // path-filter-orvalues: OBX[@3.1='94500-6||85477-8']-5 — two OR'd values.
    let orvalues_path = find("path-filter-orvalues");
    let orvalues = parse(&orvalues_path).unwrap();
    let clause = match orvalues.segment.index {
        Some(hl7pet_core::SegIndex::Filter(ref c)) => c,
        other => panic!("expected a filter clause, got {other:?}"),
    };
    assert_eq!(clause.values, vec!["94500-6", "85477-8"]);

    // path-filter-subcomponent: OBR[@4.2.2='Extended']-4.1 — filter target
    // has a populated subcomponent.
    let subcomp_path = find("path-filter-subcomponent");
    let subcomp = parse(&subcomp_path).unwrap();
    let clause = match subcomp.segment.index {
        Some(hl7pet_core::SegIndex::Filter(ref c)) => c,
        other => panic!("expected a filter clause, got {other:?}"),
    };
    assert_eq!(clause.field_num, 4);
    assert_eq!(clause.component, Some(2));
    assert_eq!(clause.subcomponent, Some(2));
    assert_eq!(clause.values, vec!["Extended"]);

    // path-filter-operator-whitespace: OBX[@3.1 = '94500-6']-5 — parses to
    // the identical structured shape as the no-whitespace form (everything
    // but `source`, which is the differing raw input string itself).
    let whitespace_path = find("path-filter-operator-whitespace");
    let whitespace = parse(&whitespace_path).unwrap();
    let no_whitespace = parse("OBX[@3.1='94500-6']-5").unwrap();
    assert_eq!(whitespace.segment, no_whitespace.segment);
    assert_eq!(whitespace.field, no_whitespace.field);
    assert_eq!(whitespace.child, no_whitespace.child);
}
