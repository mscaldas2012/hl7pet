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

#[pymodule]
fn _hl7pet_arrow(_py: Python<'_>, _m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
