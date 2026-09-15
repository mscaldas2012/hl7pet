//! PyO3/Arrow extension module wrapping `hl7pet-core` for columnar
//! (PyArrow/PySpark) extraction (spec 6002-arrow-integration). Every
//! function here scans/parses/executes against `hl7pet-core` directly (no
//! eager hierarchy build, no intermediate object model -- Constitution
//! Principle II) and converts to Arrow exactly once, at this outer
//! boundary. A per-row structural failure never raises mid-batch -- it's
//! the `status: "error"` field in `result_schema`'s Result Struct
//! (FR-010); only call-level preconditions (an invalid PATH, an empty
//! PATH list, a hierarchy PATH with no profile) raise, and always before
//! any row is processed. See contracts/arrow-api.md for the full
//! behavioral contract.

mod convert;
mod errors;
mod result_schema;

use std::borrow::Cow;

use arrow::array::Array;
use pyo3::prelude::*;

/// Parses `path` (mapping a `ParseError` through `errors::hl7pet_path_error`)
/// and, if it's a hierarchy PATH (`compiled.child.is_some()`) with no
/// `profile` supplied, raises `errors::hl7pet_profile_error` immediately --
/// a call-level precondition, checked before any row is touched
/// (contracts/arrow-api.md). Does not itself build a `HierarchyProfile`
/// from `profile`; that happens once per call in `extract_value`/
/// `extract_values`, not per validation.
fn validate_path<'p>(
    py: Python<'_>,
    path: &'p str,
    profile: Option<&Bound<'_, PyAny>>,
) -> PyResult<hl7pet_core::CompiledPath<'p>> {
    let compiled = hl7pet_core::parse(path).map_err(|e| errors::hl7pet_path_error(py, e.to_string()))?;
    if compiled.child.is_some() && profile.is_none() {
        return Err(errors::hl7pet_profile_error(
            py,
            format!("hierarchy PATH '{path}' requires a profile"),
        ));
    }
    Ok(compiled)
}

// A `thread_local!` counter (not a shared `static`) so parallel `#[test]`
// functions -- each on its own thread by default -- never interfere with
// each other's count, mirroring `crates/core/src/test_alloc.rs`'s
// counting-allocator harness (specs 005/006's SC-004 precedent).
#[cfg(test)]
thread_local! {
    static SCAN_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_scan_count() {
    SCAN_CALLS.with(|c| c.set(0));
}

#[cfg(test)]
pub(crate) fn scan_count() -> usize {
    SCAN_CALLS.with(|c| c.get())
}

/// Scans `message` if present. A `None` message (a null row in the
/// Message Column, FR-007) is never scanned -- the caller (`extract_one`)
/// treats it as `"no_match"`, not an error, since there is no message
/// content to have failed to scan (contracts/arrow-api.md). Test builds
/// count every real scan via `SCAN_CALLS`, used by `test_scan_count`
/// (SC-002) to prove `extract_rows_for_paths` scans each message exactly
/// once regardless of requested-PATH count.
fn scan_message<'m>(
    message: Option<&'m str>,
) -> Option<Result<hl7pet_core::ScanResult<'m>, hl7pet_core::ScanError>> {
    message.map(|m| {
        #[cfg(test)]
        SCAN_CALLS.with(|c| c.set(c.get() + 1));
        hl7pet_core::scan(m)
    })
}

/// The one shared per-row, per-requested-PATH extraction step both
/// `extract_value` and `extract_values` call: `scanned` is `scan_message`'s
/// output for this row (called once per row by the caller, not once per
/// PATH -- this is what makes `extract_values`' one-scan-per-message
/// guarantee, SC-002, structural rather than conventional). Dispatches to
/// `hl7pet_core::execute`/`execute_hierarchy` per `compiled.child`, and
/// maps the result through `convert::row_outcome`.
fn extract_one<'m>(
    scanned: Option<&Result<hl7pet_core::ScanResult<'m>, hl7pet_core::ScanError>>,
    compiled: &hl7pet_core::CompiledPath<'_>,
    profile: Option<&hl7pet_core::HierarchyProfile>,
) -> (
    Option<Vec<Vec<Cow<'m, str>>>>,
    result_schema::RowStatus,
) {
    match scanned {
        None => (None, result_schema::RowStatus::NoMatch),
        Some(Err(_)) => (None, result_schema::RowStatus::Error),
        Some(Ok(scan)) => {
            let result = if compiled.child.is_some() {
                hl7pet_core::execute_hierarchy(scan, compiled, profile)
            } else {
                hl7pet_core::execute(scan, compiled)
            };
            convert::row_outcome(result)
        }
    }
}

/// Builds `hl7pet_core::HierarchyProfile` from a Python `profile` dict once
/// per call, re-serializing it to JSON via the stdlib `json` module (the
/// same pattern `crates/python/src/lib.rs`'s `get_value_hierarchy` already
/// uses) -- only when at least one requested PATH is hierarchy-mode
/// (`needs_profile`); `validate_path` already guarantees `profile.is_some()`
/// whenever that's true. Shared by `extract_value` (one PATH, so
/// `needs_profile = compiled.child.is_some()`) and `extract_values` (built
/// at most once and reused across every hierarchy-mode PATH in `paths`,
/// per spec.md's "one profile per call" Assumption).
fn build_hierarchy_profile(
    py: Python<'_>,
    needs_profile: bool,
    profile: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<hl7pet_core::HierarchyProfile>> {
    if !needs_profile {
        return Ok(None);
    }
    let profile = profile.expect("validate_path guarantees Some for a hierarchy PATH");
    let json_mod = py.import("json")?;
    let profile_json: String = json_mod.call_method1("dumps", (profile,))?.extract()?;
    Ok(Some(
        hl7pet_core::HierarchyProfile::from_json(&profile_json)
            .map_err(|e| errors::hl7pet_profile_error(py, e.to_string()))?,
    ))
}

/// Single-PATH extraction over a column of messages (Story 1, FR-001).
/// See contracts/arrow-api.md.
#[pyfunction]
#[pyo3(signature = (messages, path, profile=None))]
fn extract_value(
    py: Python<'_>,
    messages: pyo3_arrow::PyArray,
    path: &str,
    profile: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let compiled = validate_path(py, path, profile)?;
    let hierarchy_profile = build_hierarchy_profile(py, compiled.child.is_some(), profile)?;

    let mut outcomes = Vec::with_capacity(messages.array().len());
    for message in convert::messages_iter(messages.array().as_ref())? {
        let scanned = scan_message(message);
        outcomes.push(extract_one(
            scanned.as_ref(),
            &compiled,
            hierarchy_profile.as_ref(),
        ));
    }

    let struct_array = result_schema::build_result_struct_array(outcomes);
    let py_array = pyo3_arrow::PyArray::from_array_ref(std::sync::Arc::new(struct_array));
    let bound = py_array.to_pyarrow(py)?;
    let unbound = bound.unbind();
    Ok(unbound)
}

/// The pure per-message-column extraction loop `extract_values` runs:
/// scans each message exactly once (`scan_message`), then calls
/// `extract_one` once per `compiled_paths` entry, reusing that one scan --
/// this is what makes SC-002 ("one scan per message regardless of PATH
/// count") hold structurally rather than conventionally. Factored out of
/// `extract_values` itself (no PyO3 types) so `test_scan_count` can
/// exercise the real production code path without needing a live Python
/// interpreter in the test. Returns one outcome list per `compiled_paths`
/// entry, in row order (column-major: `result[path_index][row_index]`).
fn extract_rows_for_paths<'m>(
    messages: impl Iterator<Item = Option<&'m str>>,
    compiled_paths: &[hl7pet_core::CompiledPath<'_>],
    profile: Option<&hl7pet_core::HierarchyProfile>,
) -> Vec<Vec<(Option<Vec<Vec<Cow<'m, str>>>>, result_schema::RowStatus)>> {
    let mut per_path_outcomes: Vec<Vec<(Option<Vec<Vec<Cow<'m, str>>>>, result_schema::RowStatus)>> =
        (0..compiled_paths.len()).map(|_| Vec::new()).collect();

    for message in messages {
        let scanned = scan_message(message);
        for (compiled, outcomes) in compiled_paths.iter().zip(per_path_outcomes.iter_mut()) {
            outcomes.push(extract_one(scanned.as_ref(), compiled, profile));
        }
    }

    per_path_outcomes
}

/// Multi-PATH extraction over a column of messages, computed from a
/// single scan per message regardless of how many PATHs are requested
/// (Story 2, FR-002, SC-002). See contracts/arrow-api.md.
#[pyfunction]
#[pyo3(signature = (messages, paths, profile=None))]
fn extract_values(
    py: Python<'_>,
    messages: pyo3_arrow::PyArray,
    paths: Vec<String>,
    profile: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    if paths.is_empty() {
        return Err(errors::empty_paths_error());
    }

    let compiled_paths: Vec<hl7pet_core::CompiledPath<'_>> = paths
        .iter()
        .map(|p| validate_path(py, p, profile))
        .collect::<PyResult<_>>()?;
    let needs_profile = compiled_paths.iter().any(|c| c.child.is_some());
    let hierarchy_profile = build_hierarchy_profile(py, needs_profile, profile)?;

    let per_path_outcomes = extract_rows_for_paths(
        convert::messages_iter(messages.array().as_ref())?,
        &compiled_paths,
        hierarchy_profile.as_ref(),
    );

    let fields_and_arrays: Vec<(arrow::datatypes::FieldRef, arrow::array::ArrayRef)> = paths
        .iter()
        .zip(per_path_outcomes.into_iter())
        .map(|(path, outcomes)| {
            let inner = result_schema::build_result_struct_array(outcomes);
            let field = std::sync::Arc::new(arrow::datatypes::Field::new(
                path.as_str(),
                inner.data_type().clone(),
                false,
            ));
            let array: arrow::array::ArrayRef = std::sync::Arc::new(inner);
            (field, array)
        })
        .collect();

    let struct_of_structs = arrow::array::StructArray::from(fields_and_arrays);
    let py_array = pyo3_arrow::PyArray::from_array_ref(std::sync::Arc::new(struct_of_structs));
    let bound = py_array.to_pyarrow(py)?;
    let unbound = bound.unbind();
    Ok(unbound)
}

#[pymodule]
fn _hl7pet_arrow(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(extract_value, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(extract_values, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASELINE_MESSAGE: &str = include_str!("../../../fixtures/messages/baseline.hl7");

    // Story 1 Acceptance Scenario 3 / FR-007: a null message row is
    // "no_match", never "error" -- there is no message content to have
    // failed to scan.
    #[test]
    fn null_message_row_is_no_match_not_error() {
        let scanned = scan_message(None);
        let compiled = hl7pet_core::parse("MSH-12").expect("valid path");
        let (value, status) = extract_one(scanned.as_ref(), &compiled, None);
        assert!(value.is_none());
        assert_eq!(status, result_schema::RowStatus::NoMatch);
    }

    // Story 1 Acceptance Scenario 3: a PATH matching nothing in a
    // well-formed message is "no_match", never an `Err`/"error".
    #[test]
    fn path_matching_nothing_is_no_match_not_error() {
        let scanned = scan_message(Some(BASELINE_MESSAGE));
        let compiled = hl7pet_core::parse("ZZZ-1").expect("valid path");
        let (value, status) = extract_one(scanned.as_ref(), &compiled, None);
        assert!(value.is_none());
        assert_eq!(status, result_schema::RowStatus::NoMatch);
    }

    // A structurally malformed message maps to "error", distinct from
    // "no_match" (FR-010).
    #[test]
    fn malformed_message_row_is_error() {
        let scanned = scan_message(Some("not an hl7 message"));
        let compiled = hl7pet_core::parse("MSH-12").expect("valid path");
        let (value, status) = extract_one(scanned.as_ref(), &compiled, None);
        assert!(value.is_none());
        assert_eq!(status, result_schema::RowStatus::Error);
    }

    // A matching PATH against a well-formed message is "ok" with a
    // non-empty value.
    #[test]
    fn matching_path_is_ok_with_value() {
        let scanned = scan_message(Some(BASELINE_MESSAGE));
        let compiled = hl7pet_core::parse("MSH-12").expect("valid path");
        let (value, status) = extract_one(scanned.as_ref(), &compiled, None);
        assert_eq!(status, result_schema::RowStatus::Ok);
        assert_eq!(value, Some(vec![vec![Cow::Borrowed("2.5.1")]]));
    }

    // SC-002 / Story 2 Acceptance Scenario 2: proves the real production
    // code path (`extract_rows_for_paths`, which `extract_values` calls
    // verbatim) scans each message exactly once, independent of how many
    // PATHs are requested. A `thread_local!` counter (not a shared
    // `static`) so this is safe under `cargo test`'s default parallel
    // test threads, mirroring `crates/core/src/test_alloc.rs`'s
    // counting-allocator harness.
    #[test]
    fn extract_values_scans_each_message_exactly_once_regardless_of_path_count() {
        reset_scan_count();
        let messages = [Some(BASELINE_MESSAGE), Some(BASELINE_MESSAGE), None];
        let compiled: Vec<_> = ["MSH-12", "PID-5.1", "MSH-9"]
            .iter()
            .map(|p| hl7pet_core::parse(p).expect("valid path"))
            .collect();
        let outcomes = extract_rows_for_paths(messages.into_iter(), &compiled, None);

        assert_eq!(outcomes.len(), 3, "one outcome list per requested path");
        for path_outcomes in &outcomes {
            assert_eq!(path_outcomes.len(), 3, "one outcome per input row");
        }
        // 2 real messages scanned once each; the null row is never scanned
        // (FR-007) -- independent of the 3 requested paths.
        assert_eq!(scan_count(), 2);
    }

    #[test]
    fn scan_count_is_independent_of_requested_path_count() {
        let messages = || [Some(BASELINE_MESSAGE), Some(BASELINE_MESSAGE)].into_iter();

        reset_scan_count();
        let one_path = vec![hl7pet_core::parse("MSH-12").expect("valid path")];
        let _ = extract_rows_for_paths(messages(), &one_path, None);
        let scans_for_one_path = scan_count();

        reset_scan_count();
        let four_paths: Vec<_> = ["MSH-12", "PID-5.1", "MSH-9", "PID-3.1"]
            .iter()
            .map(|p| hl7pet_core::parse(p).expect("valid path"))
            .collect();
        let _ = extract_rows_for_paths(messages(), &four_paths, None);
        let scans_for_four_paths = scan_count();

        assert_eq!(scans_for_one_path, 2);
        assert_eq!(scans_for_four_paths, scans_for_one_path);
    }
}
