//! The per-path Result Struct (data-model.md, resolving FR-010): each
//! requested PATH's per-row outcome is a 2-field Arrow struct, `{value:
//! List<List<Utf8>>, status: Utf8}`. `status` is one of `"ok"` (a match --
//! `value` is non-null), `"no_match"` (row scanned fine, nothing matched),
//! or `"error"` (the row could not be evaluated -- a scan failure or a
//! non-numeric filter comparison; `value` is null either way). This lets a
//! caller distinguish "no data" from "structurally invalid" per-row
//! without catching an exception mid-batch (Constitution Principle III,
//! extended to the columnar case).

use std::borrow::Cow;
use std::sync::Arc;

use arrow::array::{ArrayRef, ListBuilder, StringBuilder, StructArray};
use arrow::datatypes::Field;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RowStatus {
    Ok,
    NoMatch,
    Error,
}

impl RowStatus {
    fn as_str(self) -> &'static str {
        match self {
            RowStatus::Ok => "ok",
            RowStatus::NoMatch => "no_match",
            RowStatus::Error => "error",
        }
    }
}

/// One requested PATH's per-row outcome: `None` whenever the paired
/// `RowStatus != RowStatus::Ok`; `Some(rows)` carries the matched segment
/// occurrences (outer `Vec`) and field repetitions (inner `Vec`),
/// identical nesting to the plain `hl7pet` binding's `list[list[str]]`.
pub(crate) type RowOutcome<'m> = (Option<Vec<Vec<Cow<'m, str>>>>, RowStatus);

/// Builds one Result Struct array from a per-row outcome list, in row
/// order.
pub(crate) fn build_result_struct_array<'m>(outcomes: Vec<RowOutcome<'m>>) -> StructArray {
    let mut value_builder: ListBuilder<ListBuilder<StringBuilder>> =
        ListBuilder::new(ListBuilder::new(StringBuilder::new()));
    let mut status_builder = StringBuilder::new();

    for (value, status) in &outcomes {
        match value {
            Some(rows) => {
                let occurrences = value_builder.values();
                for repetitions in rows {
                    let reps = occurrences.values();
                    for repetition in repetitions {
                        reps.append_value(repetition.as_ref());
                    }
                    occurrences.append(true);
                }
                value_builder.append(true);
            }
            None => value_builder.append(false),
        }
        status_builder.append_value(status.as_str());
    }

    let value_array: ArrayRef = Arc::new(value_builder.finish());
    let status_array: ArrayRef = Arc::new(status_builder.finish());

    // Field metadata derived directly from the built arrays' own data
    // types, rather than hand-constructed separately, so it can never
    // drift out of sync with what the builders actually produced.
    let value_field = Arc::new(Field::new("value", value_array.data_type().clone(), true));
    let status_field = Arc::new(Field::new(
        "status",
        status_array.data_type().clone(),
        false,
    ));

    StructArray::from(vec![(value_field, value_array), (status_field, status_array)])
}
