//! Call-level precondition errors for `hl7pet-arrow` (spec
//! 6002-arrow-integration, contracts/arrow-api.md). Reuses `hl7pet`'s own
//! `Hl7PathError`/`Hl7ProfileError` exception types (imported at the PyO3
//! boundary via `py.import("hl7pet")`, the same pattern
//! `crates/python/src/lib.rs`'s `get_value_hierarchy` already uses for the
//! stdlib `json` module) rather than defining a parallel exception
//! hierarchy -- `hl7pet_arrow` depends on `hl7pet` at the Python level
//! (`pyproject.toml`), not at the Cargo level (`crates/python` is a
//! `cdylib` with no `lib`/`rlib` output another crate could link against).
//!
//! Per-row structural failure (a scan or query-evaluation error for one
//! particular message) is never raised here -- it's the `status: "error"`
//! field in `result_schema`'s Result Struct (FR-010). Only call-level
//! preconditions (an invalid PATH, an empty PATH list, a hierarchy PATH
//! with no profile) raise, and always before any row is processed.

use pyo3::exceptions::{PyImportError, PyValueError};
use pyo3::prelude::*;

fn hl7pet_exception(py: Python<'_>, class_name: &str, msg: String) -> PyErr {
    match py
        .import("hl7pet")
        .and_then(|module| module.getattr(class_name))
    {
        Ok(class) => PyErr::from_value(class.call1((msg,)).unwrap_or_else(|_| {
            // Constructing the exception instance itself should never fail
            // for a plain string argument; fall back to the class object so
            // `raise` still produces *some* `hl7pet.<class_name>`-typed
            // error rather than panicking.
            class
        })),
        Err(_) => PyImportError::new_err(format!(
            "hl7pet_arrow requires the 'hl7pet' package to be installed \
             (needed for {class_name}): {msg}"
        )),
    }
}

/// Raised for a syntactically invalid PATH expression, mirroring the plain
/// `hl7pet` binding's `Hl7PathError` (contracts/arrow-api.md).
pub(crate) fn hl7pet_path_error(py: Python<'_>, msg: impl Into<String>) -> PyErr {
    hl7pet_exception(py, "Hl7PathError", msg.into())
}

/// Raised for a hierarchy-mode PATH with a missing/invalid profile,
/// mirroring the plain `hl7pet` binding's `Hl7ProfileError`
/// (contracts/arrow-api.md).
pub(crate) fn hl7pet_profile_error(py: Python<'_>, msg: impl Into<String>) -> PyErr {
    hl7pet_exception(py, "Hl7ProfileError", msg.into())
}

/// Raised for an empty `paths` list passed to `extract_values` (FR-008). A
/// plain builtin `ValueError`, not an `Hl7*Error` -- there is no
/// `hl7pet-core` equivalent to mirror (contracts/arrow-api.md).
pub(crate) fn empty_paths_error() -> PyErr {
    PyValueError::new_err("paths must not be empty")
}
