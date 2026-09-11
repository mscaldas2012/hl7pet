//! Python exception hierarchy for `hl7pet-core` structural failures (spec
//! 6000-python-bindings-automation, research.md #3, contracts/python-api.md,
//! data-model.md's Exceptions table).
//!
//! One subclass per `hl7pet-core` error enum crossed at the FFI boundary —
//! each is a genuine structural precondition failure the core itself
//! already distinguishes from "no data" (Constitution Principle III).
//! `Hl7PetError` is never raised directly; it exists only so a caller can
//! `except Hl7PetError` broadly across every structural failure this
//! package defines.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

create_exception!(
    _hl7pet,
    Hl7PetError,
    PyException,
    "Base class for every hl7pet structural-failure exception. Never raised directly."
);
create_exception!(
    _hl7pet,
    Hl7ScanError,
    Hl7PetError,
    "Raised when an HL7 message fails to scan (e.g. missing/truncated MSH, unrecognized segment)."
);
create_exception!(
    _hl7pet,
    Hl7PathError,
    Hl7PetError,
    "Raised when a PATH expression fails to parse, or is used with a shape the called method does not support (e.g. a hierarchy PATH passed to get_value)."
);
create_exception!(
    _hl7pet,
    Hl7QueryError,
    Hl7PetError,
    "Raised when a PATH filter applies an ordering operator (>, >=, <, <=) to a non-numeric operand."
);
create_exception!(
    _hl7pet,
    Hl7ProfileError,
    Hl7PetError,
    "Raised when a hierarchy hl7pet_core::segmentDefinition profile fails to parse."
);

pub(crate) fn scan_error(e: hl7pet_core::ScanError) -> PyErr {
    Hl7ScanError::new_err(e.to_string())
}

pub(crate) fn parse_error(e: hl7pet_core::ParseError) -> PyErr {
    Hl7PathError::new_err(e.to_string())
}

pub(crate) fn query_error(e: hl7pet_core::QueryError) -> PyErr {
    Hl7QueryError::new_err(e.to_string())
}

pub(crate) fn profile_error(e: hl7pet_core::ProfileError) -> PyErr {
    Hl7ProfileError::new_err(e.to_string())
}

/// Registers the exception hierarchy on the compiled extension module.
pub(crate) fn register(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("Hl7PetError", py.get_type::<Hl7PetError>())?;
    m.add("Hl7ScanError", py.get_type::<Hl7ScanError>())?;
    m.add("Hl7PathError", py.get_type::<Hl7PathError>())?;
    m.add("Hl7QueryError", py.get_type::<Hl7QueryError>())?;
    m.add("Hl7ProfileError", py.get_type::<Hl7ProfileError>())?;
    Ok(())
}
