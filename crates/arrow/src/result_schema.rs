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

use arrow::array::{
    ArrayRef, ArrayBuilder, ListBuilder, StringBuilder, StructArray, StructBuilder, UInt64Builder,
};
use arrow::datatypes::{DataType, Field, Fields};

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

/// One matched segment occurrence in a **located** result: its field
/// repetitions (`value`, possibly more than one for a repeating field) and
/// the single 1-based source line all of them share -- `hl7pet-core`'s own
/// `LocatedValue` already guarantees every repetition within one occurrence
/// carries the same line, so this folds that redundancy away rather than
/// reproducing it per repetition.
pub(crate) struct LocatedOccurrence<'m> {
    pub(crate) value: Vec<Cow<'m, str>>,
    pub(crate) line: u64,
}

/// One requested PATH's per-row outcome, located variant: `None` whenever
/// the paired `RowStatus != RowStatus::Ok`; `Some(occurrences)` is one
/// entry per matched segment occurrence, in message order.
pub(crate) type LocatedRowOutcome<'m> = (Option<Vec<LocatedOccurrence<'m>>>, RowStatus);

/// Builds one **located** Result Struct array from a per-row outcome list,
/// in row order: `{values: List<Struct<value: List<Utf8>, line: UInt64>>,
/// status: Utf8}` (data-model.md's located extension -- `line` lives once
/// per occurrence, not once per repetition, since repetitions within an
/// occurrence always share it).
pub(crate) fn build_located_result_struct_array<'m>(
    outcomes: Vec<LocatedRowOutcome<'m>>,
) -> StructArray {
    let occurrence_fields = Fields::from(vec![
        Field::new(
            "value",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            false,
        ),
        Field::new("line", DataType::UInt64, false),
    ]);
    let occurrence_field_builders: Vec<Box<dyn ArrayBuilder>> = vec![
        Box::new(ListBuilder::new(StringBuilder::new())),
        Box::new(UInt64Builder::new()),
    ];
    let occurrence_builder = StructBuilder::new(occurrence_fields.clone(), occurrence_field_builders);
    let mut values_builder: ListBuilder<StructBuilder> = ListBuilder::new(occurrence_builder);
    let mut status_builder = StringBuilder::new();

    for (occurrences, status) in &outcomes {
        match occurrences {
            Some(occurrences) => {
                let occurrence_builder = values_builder.values();
                for occurrence in occurrences {
                    let reps = occurrence_builder
                        .field_builder::<ListBuilder<StringBuilder>>(0)
                        .expect("field 0 is the `value` ListBuilder<StringBuilder>");
                    for repetition in &occurrence.value {
                        reps.values().append_value(repetition.as_ref());
                    }
                    reps.append(true);

                    let line = occurrence_builder
                        .field_builder::<UInt64Builder>(1)
                        .expect("field 1 is the `line` UInt64Builder");
                    line.append_value(occurrence.line);

                    occurrence_builder.append(true);
                }
                values_builder.append(true);
            }
            None => values_builder.append(false),
        }
        status_builder.append_value(status.as_str());
    }

    let values_array: ArrayRef = Arc::new(values_builder.finish());
    let status_array: ArrayRef = Arc::new(status_builder.finish());

    let values_field = Arc::new(Field::new("values", values_array.data_type().clone(), true));
    let status_field = Arc::new(Field::new(
        "status",
        status_array.data_type().clone(),
        false,
    ));

    StructArray::from(vec![(values_field, values_array), (status_field, status_array)])
}
