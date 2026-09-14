# Data Model: Located Hierarchy API

## No new public type

Unlike spec `1000` (which introduced `LocatedValue<'m>`), this feature
introduces **no new public entity**. It reuses `query::LocatedValue<'m>`
(spec `1000`, `crates/core/src/query.rs:197`) exactly as-is:

```rust
pub struct LocatedValue<'m> {
    pub value: Cow<'m, str>,
    pub line: usize,
}
```

Every field's meaning and invariant is identical to spec `1000`'s
data-model.md — spec.md's "Located Hierarchy Value" Key Entity is
deliberately the same shape as spec `1000`'s "Located Value," not a new one,
per this feature's Assumptions. `line`'s meaning for hierarchy navigation
extends the existing definition consistently: the 1-based position, among
all segments in the message, of the *child* segment occurrence a value was
read from (never the parent's line).

## Function signatures (new, `crates/core/src/hierarchy.rs`)

```rust
/// Location-aware counterpart to `execute_hierarchy` (spec 008). Same
/// preconditions, same per-parent-occurrence bounded scan, same
/// flattened-across-parents grouping — every value execute_hierarchy would
/// return is paired with its source child segment occurrence's 1-based line
/// number.
pub fn execute_hierarchy_located<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
    profile: Option<&HierarchyProfile>,
) -> Result<Vec<Vec<query::LocatedValue<'m>>>, QueryError>;
```

No new error type: reuses `QueryError` (spec `007`) unchanged — still
exactly one variant, `QueryError::NonNumericComparison`, for the same
non-numeric-ordering-filter case `execute_hierarchy` already handles this
way on either the parent's or the child's `SEG_IDX` filter clause.

## Internal helper signatures (new, private to `hierarchy.rs`)

These do not cross the module's public boundary — listed because
`execute_hierarchy_located`'s correctness depends directly on them, per
research.md #1's delegation design.

```rust
/// _indexed sibling of direct_children_of_type: identical selection logic,
/// additionally pairing each returned child span with its own 1-based line
/// number (its position among all of scan.segments) -- captured within the
/// same bounded forward scan, not a second pass. direct_children_of_type
/// becomes a thin wrapper: `.into_iter().map(|(_, span)| span).collect()`.
fn direct_children_of_type_indexed<'m>(
    scan: &ScanResult<'m>,
    profile: &HierarchyProfile,
    parent_span: SegmentSpan,
    parent_node: usize,
    cseg: &str,
) -> Vec<(usize, SegmentSpan)>;

/// _indexed sibling of apply_child_index: identical SEG_IDX
/// (Numeric/Last/Star/Filter) selection logic, operating on and returning
/// (usize, SegmentSpan) pairs so the line survives child-index selection.
/// apply_child_index becomes a thin wrapper that tags input spans with a
/// placeholder line (unused by its own logic) and strips it from the output.
fn apply_child_index_indexed<'m>(
    scan: &ScanResult<'m>,
    candidates: Vec<(usize, SegmentSpan)>,
    index: Option<&SegIndex<'_>>,
) -> Result<Vec<(usize, SegmentSpan)>, QueryError>;
```

| Existing function | Becomes |
|---|---|
| `direct_children_of_type` (hierarchy.rs:275) | Thin wrapper over `direct_children_of_type_indexed`, unchanged signature/behavior — every existing unit test (lines 426-507) passes unmodified. |
| `apply_child_index` (hierarchy.rs:331) | Thin wrapper over `apply_child_index_indexed`, unchanged signature/behavior. |

## Invariants

- `execute_hierarchy_located` never fabricates a `LocatedValue` for a value
  that was not actually extracted — identical to spec `1000`'s
  `execute_located` invariant, and to `execute_hierarchy`'s own existing
  "no match -> `Ok(vec![])`" behavior (FR-006).
- `line` is stable across repeated calls against the same
  `ScanResult`/`HierarchyProfile` pair — no interior mutability, no
  randomness (mirrors spec `008`'s determinism precedent, carried forward
  unchanged).
- All values sharing one matched child segment occurrence carry the same
  `line` (FR-004) — `line` is attached per child occurrence, not per
  sub-segment position, identical to spec `1000`'s FR-005 for non-hierarchy
  extraction.
- Stripping every `LocatedValue.line` from `execute_hierarchy_located`'s
  output reproduces `execute_hierarchy`'s output exactly, value-for-value
  and group-for-group (FR-002) — the executable form of this feature's core
  correctness claim, mirroring spec `1000`'s contract for `execute_located`
  vs. `execute`.

**Relationships**: `LocatedValue<'m>`'s `value` field borrows the same
`Cow<'m, str>` `execute_hierarchy`'s existing output already produces
(decoded per spec `1001`) — no new lifetime or borrowing relationship beyond
what `execute_hierarchy` already establishes against `ScanResult<'m>`.

## State / lifecycle

No new state. `execute_hierarchy_located` is a pure function of its
`scan`/`path`/`profile` inputs, called after the same
`scan()` → `parse()` → (optionally) `HierarchyProfile::from_json()` pipeline
`execute_hierarchy` already requires — no new setup, no new struct that
outlives the call, no caching.

## Relationship to existing entities

| Existing entity (spec) | Relationship |
|---|---|
| `ScanResult<'m>` (spec `005`) | Read-only input; `segments`' existing document order is this feature's sole source of line numbers (research.md #1), identical to spec `1000`'s use of it. Unmodified. |
| `CompiledPath<'_>` (spec `006`) | Read-only input; `path.child` MUST be `Some(_)` for hierarchy-mode selection — when `None`, `execute_hierarchy_located` delegates to `query::execute_located` (spec `1000`) unchanged, mirroring `execute_hierarchy`'s own existing flat-path delegation to `query::execute`. |
| `HierarchyProfile` (spec `008`) | Read-only input, `Option`-wrapped exactly as `execute_hierarchy` already requires; `None` yields `Ok(vec![])` per FR-006/spec `008`'s FR-009 static-mode fallback, unchanged. |
| `QueryError` (spec `007`) | Reused unchanged as the error type. |
| `LocatedValue<'m>` (spec `1000`, `query.rs`) | Reused unchanged as the success value's element type — no new type, per this feature's core design decision. |
| `resolve_field_values_located` (spec `1000`, `query.rs`, `pub(crate)`) | Reused unchanged for the final field-resolution step (research.md #2) — the same function `query::execute_located` already calls, called here once per selected child occurrence instead. |
| `direct_children_of_type`, `apply_child_index` (spec `008`/`010`, private to `hierarchy.rs`) | Each refactored into a thin wrapper over a new `_indexed` sibling (research.md #1) — signature and behavior unchanged, all existing unit tests pass unmodified. |
| `resolve_occurrence_node` (spec `010`, private to `hierarchy.rs`) | Reused unchanged — ambiguous parent-side resolution has no dependency on child-side line tracking; `execute_hierarchy_located` calls it exactly where `execute_hierarchy` already does. |
