//! Python counterpart to `hl7pet_core::LocatedValue` (spec 1000, contracts/
//! python-api.md). `value` is always a freshly Python-owned `String` — the
//! one conversion the FFI boundary requires (Constitution Principle II,
//! research.md #3's sibling decision for exceptions).

use pyo3::prelude::*;

#[pyclass(module = "hl7pet", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub(crate) struct LocatedValue {
    #[pyo3(get)]
    value: String,
    #[pyo3(get)]
    line: usize,
}

#[pymethods]
impl LocatedValue {
    fn __repr__(&self) -> String {
        format!("LocatedValue(value={:?}, line={})", self.value, self.line)
    }
}

impl From<hl7pet_core::LocatedValue<'_>> for LocatedValue {
    fn from(v: hl7pet_core::LocatedValue<'_>) -> Self {
        LocatedValue { value: v.value.into_owned(), line: v.line }
    }
}
