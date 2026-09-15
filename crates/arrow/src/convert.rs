//! Conversions between `hl7pet-core`'s output types and this crate's Arrow
//! representation: the messages-array input (data-model.md's Message
//! Column) and the per-row outcome mapping that feeds `result_schema`
//! (data-model.md's Result Struct, resolving FR-010).

use std::borrow::Cow;

use arrow::array::{Array, LargeStringArray, StringArray};
use pyo3::exceptions::PyTypeError;
use pyo3::PyResult;

use crate::result_schema::RowStatus;

/// Maps one requested PATH's `hl7pet_core::execute`/`execute_hierarchy`
/// result for an already-scanned message to a Result Struct outcome
/// (data-model.md): a query-evaluation failure (a non-numeric filter
/// comparison) and a scan failure both fold into the single `"error"`
/// status (data-model.md's merged status, not split by cause); an empty
/// match is `"no_match"`; a non-empty match is `"ok"`.
pub(crate) fn row_outcome<'m>(
    execute_result: Result<Vec<Vec<Cow<'m, str>>>, hl7pet_core::QueryError>,
) -> (Option<Vec<Vec<Cow<'m, str>>>>, RowStatus) {
    match execute_result {
        Err(_) => (None, RowStatus::Error),
        Ok(rows) if rows.is_empty() => (None, RowStatus::NoMatch),
        Ok(rows) => (Some(rows), RowStatus::Ok),
    }
}

/// Iterates the `messages` Message Column input (data-model.md), yielding
/// `None` for a null row (FR-007) and `Some(&str)` otherwise. Accepts
/// either `Utf8` (`StringArray`) or `LargeUtf8` (`LargeStringArray`), the
/// two Arrow string representations `pyarrow.array([...])` of Python `str`
/// can produce.
pub(crate) fn messages_iter<'a>(
    array: &'a dyn Array,
) -> PyResult<Box<dyn Iterator<Item = Option<&'a str>> + 'a>> {
    if let Some(utf8) = array.as_any().downcast_ref::<StringArray>() {
        Ok(Box::new(utf8.iter()))
    } else if let Some(large_utf8) = array.as_any().downcast_ref::<LargeStringArray>() {
        Ok(Box::new(large_utf8.iter()))
    } else {
        Err(PyTypeError::new_err(format!(
            "messages must be a Utf8 or LargeUtf8 Arrow array, got {:?}",
            array.data_type()
        )))
    }
}
