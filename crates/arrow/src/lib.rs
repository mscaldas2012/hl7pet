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

/// Scans `message` if present. A `None` message (a null row in the
/// Message Column, FR-007) is never scanned -- the caller (`extract_one`)
/// treats it as `"no_match"`, not an error, since there is no message
/// content to have failed to scan (contracts/arrow-api.md).
fn scan_message<'m>(
    message: Option<&'m str>,
) -> Option<Result<hl7pet_core::ScanResult<'m>, hl7pet_core::ScanError>> {
    message.map(hl7pet_core::scan)
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
/// uses) -- only when `compiled` is actually a hierarchy PATH; `validate_path`
/// already guarantees `profile.is_some()` in that case.
fn build_hierarchy_profile(
    py: Python<'_>,
    compiled: &hl7pet_core::CompiledPath<'_>,
    profile: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<hl7pet_core::HierarchyProfile>> {
    if compiled.child.is_none() {
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
    let hierarchy_profile = build_hierarchy_profile(py, &compiled, profile)?;

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

#[pymodule]
fn _hl7pet_arrow(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(extract_value, m)?)?;
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
}
