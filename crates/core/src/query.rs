//! Query executor (spec 007-query-execution).
//!
//! Navigates a scanner's `ScanResult` (spec 005) offsets using a parser's
//! `CompiledPath` (spec 006) to extract the actual value(s) a PATH addresses —
//! the piece neither prior spec provides. Resolves segment/field index
//! selectors and filter clauses, reproducing the existing Scala engine's
//! `getValue`/`getFirstValue` output byte-for-byte for every non-hierarchy
//! PATH in the shared regression suite (spec 003). Hierarchy navigation
//! (`CompiledPath.child`) is spec 008's responsibility, not this module's.
//!
//! An out-of-range segment or field index is *not* an error here — verified
//! against the real Scala engine (research.md #2), `getValue`/`getFirstValue`
//! return no match for an out-of-range index rather than throwing, so this
//! executor represents that the same way as any other "no data" outcome:
//! `Ok(vec![])`. The one genuine error is a non-numeric operand compared with
//! an ordering filter operator (FR-008) — the one case the real engine does
//! *not* handle gracefully (an uncaught `NumberFormatException`).

use crate::parser::{CompiledPath, FieldExpr, FieldIndex, FilterClause, FilterOperator, SegIndex};
use crate::scanner::{DelimiterSet, ScanResult, SegmentSpan};

/// The executor's one non-panic failure output (data-model.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    /// An ordering operator (`>`, `>=`, `<`, `<=`) was applied where the
    /// target sub-value or one of the filter's literal values does not parse
    /// as a number.
    NonNumericComparison { operator: FilterOperator },
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::NonNumericComparison { operator } => write!(
                f,
                "cannot apply ordering operator {operator:?} to a non-numeric operand"
            ),
        }
    }
}

impl std::error::Error for QueryError {}

/// Executes `path` (non-hierarchy form — `path.child` is ignored, spec 008's
/// responsibility) against `scan`, returning the outer/inner value shape
/// `fixtures/schemas/conformance-vector.schema.json` already uses for
/// `getValue`: outer = matched segment occurrences in message order, inner =
/// resolved field repetitions (or a single full-segment entry when `path.field`
/// is `None`). Empty when nothing matched, for any reason — never an error. A
/// candidate occurrence whose requested field index has no matching
/// repetition contributes no entry at all (verified against the real Scala
/// engine: `OBX-5[5]` beyond the repetitions present collapses to no match
/// entirely, not an empty inner list) — distinct from FR-009(e)'s "component/
/// subcomponent beyond what's present," which still yields a single
/// empty-string entry.
pub fn execute<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
) -> Result<Vec<Vec<&'m str>>, QueryError> {
    let candidates = resolve_segment_candidates(scan, path.segment.name, path.segment.index.as_ref())?;

    let mut result = Vec::with_capacity(candidates.len());
    for span in &candidates {
        let segment_content = &scan.message[span.start..span.end];
        let segment_name = scan.segment_name(span);
        let values = resolve_field_values(path.field.as_ref(), segment_content, segment_name, &scan.delimiters);
        // A candidate whose requested field index has no matching repetition
        // (e.g. FieldIndex::Numeric beyond what's present) contributes no
        // entry at all -- verified against the real Scala engine, which
        // collapses this to no match entirely rather than an empty inner
        // list (research.md #2). This is distinct from FR-009(e)'s "beyond
        // what's present" component/subcomponent case, which still yields a
        // single empty-string entry (`values` is non-empty, just `""`).
        if !values.is_empty() {
            result.push(values);
        }
    }
    Ok(result)
}

/// Resolves a segment index selector (`SegIndex`) against the ordered set of
/// segment occurrences in `scan` whose name matches `segment_name`. An
/// explicit `Numeric`/`Last` index beyond what's present yields zero
/// candidates, not an error (research.md #2, verified against the real Scala
/// engine).
pub(crate) fn resolve_segment_candidates<'m>(
    scan: &ScanResult<'m>,
    segment_name: &str,
    index: Option<&SegIndex<'_>>,
) -> Result<Vec<SegmentSpan>, QueryError> {
    let matching: Vec<SegmentSpan> = scan
        .segments
        .iter()
        .filter(|span| scan.segment_name(span) == segment_name)
        .copied()
        .collect();

    match index {
        None | Some(SegIndex::Star) => Ok(matching),
        Some(SegIndex::Numeric(n)) => {
            let idx = *n as usize;
            Ok(if idx >= 1 && idx <= matching.len() {
                vec![matching[idx - 1]]
            } else {
                vec![]
            })
        }
        Some(SegIndex::Last) => Ok(matching.last().copied().into_iter().collect()),
        Some(SegIndex::Filter(clause)) => {
            let mut selected = Vec::new();
            for span in matching {
                if filter_matches(scan, &span, clause)? {
                    selected.push(span);
                }
            }
            Ok(selected)
        }
    }
}

/// Evaluates `clause` against one candidate segment occurrence's content,
/// reusing `extract_component`'s field/component/subcomponent navigation
/// (research.md #3 — one navigation path, not two).
pub(crate) fn filter_matches<'m>(
    scan: &ScanResult<'m>,
    span: &SegmentSpan,
    clause: &FilterClause<'_>,
) -> Result<bool, QueryError> {
    let segment_content = &scan.message[span.start..span.end];
    let segment_name = scan.segment_name(span);
    let raw_field = field_at(
        segment_name,
        segment_content,
        scan.delimiters.field,
        clause.field_num,
    );
    let target_value = extract_component(raw_field, &scan.delimiters, clause.component, clause.subcomponent);

    let is_ordering = matches!(
        clause.operator,
        FilterOperator::Gt | FilterOperator::Ge | FilterOperator::Lt | FilterOperator::Le
    );

    if is_ordering {
        let target_num: f64 = target_value
            .parse()
            .map_err(|_| QueryError::NonNumericComparison { operator: clause.operator })?;
        for value in &clause.values {
            let filter_num: f64 = value
                .parse()
                .map_err(|_| QueryError::NonNumericComparison { operator: clause.operator })?;
            let matches = match clause.operator {
                FilterOperator::Gt => target_num > filter_num,
                FilterOperator::Ge => target_num >= filter_num,
                FilterOperator::Lt => target_num < filter_num,
                FilterOperator::Le => target_num <= filter_num,
                FilterOperator::Eq | FilterOperator::Ne => unreachable!("guarded by is_ordering"),
            };
            if matches {
                return Ok(true);
            }
        }
        Ok(false)
    } else {
        for value in &clause.values {
            let matches = match clause.operator {
                FilterOperator::Eq => target_value == *value,
                FilterOperator::Ne => target_value != *value,
                _ => unreachable!("guarded by is_ordering"),
            };
            if matches {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Resolves the value(s) `field_expr` addresses within one matched segment
/// occurrence. `None` (no field expression) returns the full raw segment
/// content, unsplit, as a single-element vec (FR-002, research.md #5).
pub(crate) fn resolve_field_values<'m>(
    field_expr: Option<&FieldExpr>,
    segment_content: &'m str,
    segment_name: &str,
    delimiters: &DelimiterSet,
) -> Vec<&'m str> {
    match field_expr {
        None => vec![segment_content],
        Some(fe) => {
            let raw_field = field_at(segment_name, segment_content, delimiters.field, fe.field_num);
            let repetitions = split_bytes(raw_field, delimiters.repetition);
            select_by_field_index(&repetitions, fe.index)
                .into_iter()
                .map(|rep| extract_component(rep, delimiters, fe.component, fe.subcomponent))
                .collect()
        }
    }
}

/// Returns the raw (unsplit-by-repetition) content of `field_num` within
/// `segment_content`. Handles the MSH-1 special case: for the `MSH` segment,
/// the field separator character itself is `MSH-1` (it can't appear as a
/// normal split token since it *is* the delimiter), so every other MSH field
/// number is shifted by one relative to a plain `split_bytes` index.
fn field_at<'m>(segment_name: &str, segment_content: &'m str, field_sep: u8, field_num: u32) -> &'m str {
    if segment_name == "MSH" {
        if field_num == 1 {
            return &segment_content[3..4];
        }
        let fields = split_bytes(segment_content, field_sep);
        let idx = (field_num - 1) as usize;
        return fields.get(idx).copied().unwrap_or("");
    }
    let fields = split_bytes(segment_content, field_sep);
    fields.get(field_num as usize).copied().unwrap_or("")
}

/// Resolves a field index selector (`FieldIndex`) against a field's
/// `~`-delimited repetitions. An explicit `Numeric` index beyond what's
/// present yields zero repetitions, not an error (research.md #2).
fn select_by_field_index<'m>(repetitions: &[&'m str], index: Option<FieldIndex>) -> Vec<&'m str> {
    match index {
        None | Some(FieldIndex::Star) => repetitions.to_vec(),
        Some(FieldIndex::Numeric(n)) => {
            let idx = n as usize;
            if idx >= 1 && idx <= repetitions.len() {
                vec![repetitions[idx - 1]]
            } else {
                vec![]
            }
        }
        Some(FieldIndex::Last) => repetitions.last().copied().into_iter().collect(),
    }
}

/// Splits `content` on `component`/`subcomponent` delimiters down to the
/// requested level, returning the empty string (not an error) when the
/// requested level is beyond what `content` actually contains (FR-009(e)).
fn extract_component<'m>(
    content: &'m str,
    delimiters: &DelimiterSet,
    component: Option<u32>,
    subcomponent: Option<u32>,
) -> &'m str {
    let Some(comp_num) = component else {
        return content;
    };
    let components = split_bytes(content, delimiters.component);
    let comp_value = one_based_get(&components, comp_num);

    let Some(subcomp_num) = subcomponent else {
        return comp_value;
    };
    let subcomponents = split_bytes(comp_value, delimiters.subcomponent);
    one_based_get(&subcomponents, subcomp_num)
}

/// Looks up `index` (1-based; `0` or beyond `items.len()` is out of range) in
/// `items`, returning `""` — not a panic — when out of range.
fn one_based_get<'m>(items: &[&'m str], index: u32) -> &'m str {
    if index == 0 {
        return "";
    }
    items.get((index - 1) as usize).copied().unwrap_or("")
}

fn split_bytes(s: &str, delim: u8) -> Vec<&str> {
    s.split(delim as char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::scanner::scan;
    use crate::test_alloc::count_allocs;

    // --- US1 (T013): extract_component's field/component/subcomponent levels ---

    #[test]
    fn extract_component_field_only_returns_full_field() {
        let delimiters = standard_delimiters();
        assert_eq!(extract_component("A^B^C", &delimiters, None, None), "A^B^C");
    }

    #[test]
    fn extract_component_field_and_component() {
        let delimiters = standard_delimiters();
        assert_eq!(extract_component("A^B^C", &delimiters, Some(2), None), "B");
    }

    #[test]
    fn extract_component_field_component_and_subcomponent() {
        let delimiters = standard_delimiters();
        assert_eq!(extract_component("A^B&C&D^E", &delimiters, Some(2), Some(2)), "C");
    }

    // FR-009(e): a requested component/subcomponent beyond what's present is
    // absent (empty string), never an error and never a panic.
    #[test]
    fn extract_component_beyond_present_is_empty_not_error() {
        let delimiters = standard_delimiters();
        assert_eq!(extract_component("A^B", &delimiters, Some(5), None), "");
        assert_eq!(extract_component("A^B", &delimiters, Some(1), Some(5)), "");
    }

    #[test]
    fn extract_component_zero_index_is_empty_not_panic() {
        let delimiters = standard_delimiters();
        assert_eq!(extract_component("A^B", &delimiters, Some(0), None), "");
    }

    // --- US2 (T016): SegIndex/FieldIndex resolution ---

    #[test]
    fn seg_index_numeric_selects_correct_occurrence() {
        let message = multi_obx_message();
        let scan_result = scan(&message).unwrap();
        let candidates =
            resolve_segment_candidates(&scan_result, "OBX", Some(&SegIndex::Numeric(2))).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(scan_result.segment_name(&candidates[0]), "OBX");
        let content = &scan_result.message[candidates[0].start..candidates[0].end];
        assert!(content.contains("Second"));
    }

    #[test]
    fn seg_index_last_selects_final_occurrence_regardless_of_count() {
        let message = multi_obx_message();
        let scan_result = scan(&message).unwrap();
        let candidates =
            resolve_segment_candidates(&scan_result, "OBX", Some(&SegIndex::Last)).unwrap();
        assert_eq!(candidates.len(), 1);
        let content = &scan_result.message[candidates[0].start..candidates[0].end];
        assert!(content.contains("Third"));
    }

    #[test]
    fn seg_index_star_or_omitted_selects_every_occurrence_in_message_order() {
        let message = multi_obx_message();
        let scan_result = scan(&message).unwrap();
        let star = resolve_segment_candidates(&scan_result, "OBX", Some(&SegIndex::Star)).unwrap();
        let omitted = resolve_segment_candidates(&scan_result, "OBX", None).unwrap();
        assert_eq!(star.len(), 3);
        assert_eq!(star, omitted);
    }

    // research.md #2: an explicit out-of-range segment index resolves to zero
    // candidates, never a panic and never Err (verified against the real
    // Scala engine).
    #[test]
    fn seg_index_numeric_out_of_range_is_empty_not_error() {
        let message = multi_obx_message();
        let scan_result = scan(&message).unwrap();
        let candidates =
            resolve_segment_candidates(&scan_result, "OBX", Some(&SegIndex::Numeric(5))).unwrap();
        assert!(candidates.is_empty());
    }

    #[test]
    fn seg_index_numeric_out_of_range_when_segment_absent_entirely() {
        let message = multi_obx_message();
        let scan_result = scan(&message).unwrap();
        let candidates =
            resolve_segment_candidates(&scan_result, "ZZZ", Some(&SegIndex::Numeric(1))).unwrap();
        assert!(candidates.is_empty());
    }

    // research.md #2 (verified against the real Scala engine): a matched
    // segment occurrence whose requested field-repetition index is out of
    // range contributes no entry at all -- not an empty inner list.
    #[test]
    fn execute_drops_occurrence_whose_field_index_is_out_of_range() {
        let message = "MSH|^~\\&|A|B|A|B|20260101000000||ORU^R01|1|P|2.5.1\n\
                        OBX|1|ST|X||IgG~IgM||||||F\n"
            .to_string();
        let scan_result = scan(&message).unwrap();
        let path = parse("OBX-5[5]").unwrap();
        let result = execute(&scan_result, &path).unwrap();
        assert!(result.is_empty(), "expected no match at all, got {result:?}");
    }

    #[test]
    fn field_index_numeric_selects_correct_repetition() {
        let repetitions = ["IgG", "IgM", "IgA"];
        let selected = select_by_field_index(&repetitions, Some(FieldIndex::Numeric(2)));
        assert_eq!(selected, vec!["IgM"]);
    }

    #[test]
    fn field_index_last_selects_final_repetition() {
        let repetitions = ["IgG", "IgM", "IgA"];
        let selected = select_by_field_index(&repetitions, Some(FieldIndex::Last));
        assert_eq!(selected, vec!["IgA"]);
    }

    #[test]
    fn field_index_star_or_omitted_selects_every_repetition_in_order() {
        let repetitions = ["IgG", "IgM"];
        assert_eq!(
            select_by_field_index(&repetitions, Some(FieldIndex::Star)),
            vec!["IgG", "IgM"]
        );
        assert_eq!(select_by_field_index(&repetitions, None), vec!["IgG", "IgM"]);
    }

    // research.md #2: an explicit out-of-range field/repetition index resolves
    // to zero repetitions, never an error.
    #[test]
    fn field_index_numeric_out_of_range_is_empty_not_error() {
        let repetitions = ["IgG", "IgM"];
        let selected = select_by_field_index(&repetitions, Some(FieldIndex::Numeric(5)));
        assert!(selected.is_empty());
    }

    // --- US3 (T018): FilterClause evaluation ---

    fn filter_example_message() -> String {
        "MSH|^~\\&|A|B|A|B|20260101000000||ORU^R01|1|P|2.5.1\n\
         OBX|1|ST|94500-6^Name^LN||Positive||||||F\n\
         OBX|2|ST|85477-8^Name^LN||Negative||||||F\n"
            .to_string()
    }

    fn eq_clause<'a>(field_num: u32, component: Option<u32>, value: &'a str) -> FilterClause<'a> {
        FilterClause {
            field_num,
            component,
            subcomponent: None,
            operator: FilterOperator::Eq,
            values: vec![value],
        }
    }

    #[test]
    fn filter_eq_matches_correct_occurrence() {
        let message = filter_example_message();
        let scan_result = scan(&message).unwrap();
        let clause = eq_clause(3, Some(1), "85477-8");
        let matches: Vec<bool> = scan_result
            .segments
            .iter()
            .filter(|s| scan_result.segment_name(s) == "OBX")
            .map(|s| filter_matches(&scan_result, s, &clause).unwrap())
            .collect();
        assert_eq!(matches, vec![false, true]);
    }

    #[test]
    fn filter_ne_matches_non_matching_occurrence() {
        let message = filter_example_message();
        let scan_result = scan(&message).unwrap();
        let clause = FilterClause {
            field_num: 3,
            component: Some(1),
            subcomponent: None,
            operator: FilterOperator::Ne,
            values: vec!["85477-8"],
        };
        let matches: Vec<bool> = scan_result
            .segments
            .iter()
            .filter(|s| scan_result.segment_name(s) == "OBX")
            .map(|s| filter_matches(&scan_result, s, &clause).unwrap())
            .collect();
        assert_eq!(matches, vec![true, false]);
    }

    #[test]
    fn filter_orvalues_matches_if_any_value_succeeds() {
        let message = filter_example_message();
        let scan_result = scan(&message).unwrap();
        let clause = FilterClause {
            field_num: 3,
            component: Some(1),
            subcomponent: None,
            operator: FilterOperator::Eq,
            values: vec!["94500-6", "85477-8"],
        };
        let matches: Vec<bool> = scan_result
            .segments
            .iter()
            .filter(|s| scan_result.segment_name(s) == "OBX")
            .map(|s| filter_matches(&scan_result, s, &clause).unwrap())
            .collect();
        assert_eq!(matches, vec![true, true]);
    }

    #[test]
    fn filter_subcomponent_target_navigates_correctly() {
        let message = "MSH|^~\\&|A|B|A|B|20260101000000||ORU^R01|1|P|2.5.1\n\
                        OBR|1|X|Y|94500-6^Name&Extended^LN\n"
            .to_string();
        let scan_result = scan(&message).unwrap();
        let clause = FilterClause {
            field_num: 4,
            component: Some(2),
            subcomponent: Some(2),
            operator: FilterOperator::Eq,
            values: vec!["Extended"],
        };
        let span = scan_result
            .segments
            .iter()
            .find(|s| scan_result.segment_name(s) == "OBR")
            .unwrap();
        assert!(filter_matches(&scan_result, span, &clause).unwrap());
    }

    #[test]
    fn filter_ordering_operators_compare_numerically_in_both_directions() {
        let message = "MSH|^~\\&|A|B|A|B|20260101000000||ORU^R01|1|P|2.5.1\n\
                        OBX|1|NM|X||50||||||F\n"
            .to_string();
        let scan_result = scan(&message).unwrap();
        let span = scan_result
            .segments
            .iter()
            .find(|s| scan_result.segment_name(s) == "OBX")
            .unwrap();

        let gt = FilterClause {
            field_num: 5,
            component: None,
            subcomponent: None,
            operator: FilterOperator::Gt,
            values: vec!["10"],
        };
        assert!(filter_matches(&scan_result, span, &gt).unwrap());

        let lt = FilterClause {
            field_num: 5,
            component: None,
            subcomponent: None,
            operator: FilterOperator::Lt,
            values: vec!["10"],
        };
        assert!(!filter_matches(&scan_result, span, &lt).unwrap());

        let ge = FilterClause {
            field_num: 5,
            component: None,
            subcomponent: None,
            operator: FilterOperator::Ge,
            values: vec!["50"],
        };
        assert!(filter_matches(&scan_result, span, &ge).unwrap());

        let le = FilterClause {
            field_num: 5,
            component: None,
            subcomponent: None,
            operator: FilterOperator::Le,
            values: vec!["50"],
        };
        assert!(filter_matches(&scan_result, span, &le).unwrap());
    }

    // FR-008: a non-numeric operand compared with an ordering operator is a
    // distinguishable error, never a silent `false` and never a panic —
    // verified against the real Scala engine, which throws an uncaught
    // `NumberFormatException` here (research.md #4).
    #[test]
    fn filter_ordering_operator_against_non_numeric_returns_comparison_error() {
        let message = filter_example_message();
        let scan_result = scan(&message).unwrap();
        let span = scan_result
            .segments
            .iter()
            .find(|s| scan_result.segment_name(s) == "OBX")
            .unwrap();
        let clause = FilterClause {
            field_num: 5,
            component: None,
            subcomponent: None,
            operator: FilterOperator::Gt,
            values: vec!["100"],
        };
        match filter_matches(&scan_result, span, &clause) {
            Err(QueryError::NonNumericComparison { operator: FilterOperator::Gt }) => {}
            other => panic!("expected NonNumericComparison, got {other:?}"),
        }
    }

    // --- SC-004 (T011): at most one pass over matched content ---

    fn padded_message(padding: usize) -> String {
        let mut msg = "MSH|^~\\&|A|B|A|B|20260101000000||ORU^R01|1|P|2.5.1\nPID|1\n".to_string();
        for i in 0..padding {
            msg.push_str(&format!("OBR|{i}|X|Y\n"));
        }
        msg.push_str("OBX|1|ST|X||TargetValue||||||F\n");
        msg
    }

    #[test]
    fn execute_single_pass_allocation_count_independent_of_segment_count() {
        let path = parse("OBX-5").expect("OBX-5 must parse");

        let small_message = padded_message(0);
        let large_message = padded_message(500);

        let small_allocs = count_allocs(|| {
            let scan_result = scan(&small_message).unwrap();
            let result = execute(&scan_result, &path).unwrap();
            std::hint::black_box(&result);
        });
        let large_allocs = count_allocs(|| {
            let scan_result = scan(&large_message).unwrap();
            let result = execute(&scan_result, &path).unwrap();
            std::hint::black_box(&result);
        });

        assert_eq!(
            small_allocs, large_allocs,
            "allocation count must not scale with unrelated segment count — \
             execute() must not re-scan the full message per query"
        );
    }

    fn standard_delimiters() -> DelimiterSet {
        DelimiterSet {
            field: b'|',
            component: b'^',
            repetition: b'~',
            escape: b'\\',
            subcomponent: b'&',
        }
    }

    fn multi_obx_message() -> String {
        "MSH|^~\\&|A|B|A|B|20260101000000||ORU^R01|1|P|2.5.1\n\
         OBX|1|ST|X||First||||||F\n\
         OBX|2|ST|X||Second||||||F\n\
         OBX|3|ST|X||Third||||||F\n"
            .to_string()
    }
}
