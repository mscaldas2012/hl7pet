# Data Model: Located Extraction API

## `LocatedValue<'m>` (new)

The one new entity this feature introduces — spec.md's "Located Value" made
concrete. Pairs a value `execute()` would already return with the 1-based
line number of the segment occurrence it came from.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatedValue<'m> {
    pub value: &'m str,
    pub line: usize,
}
```

| Field   | Type      | Meaning |
|---------|-----------|---------|
| `value` | `&'m str` | Identical in content to the corresponding entry `execute()` would return for the same PATH/message (spec.md FR-003) — borrowed from `scan.message`, never copied. |
| `line`  | `usize`   | 1-based position of the source segment occurrence among all segments in the message (spec.md FR-004), i.e. `1 + segment's index in ScanResult.segments`. Always `>= 1`. |

**Invariants**:
- Never constructed for a value that was not actually extracted — no
  "placeholder" or fabricated `LocatedValue` exists (spec.md FR-007).
- `line` is stable across repeated calls against the same `ScanResult` (no
  interior mutability, no randomness) — mirrors spec `008`'s determinism
  precedent for hierarchy navigation.
- All values sharing one segment occurrence (e.g. multiple components pulled
  from the same field) carry the same `line` (spec.md FR-005) — `line` is
  attached per occurrence, not per sub-segment position.

**Relationships**: Wraps a `&'m str` value with the same lifetime and origin
as `execute()`'s existing `Vec<Vec<&'m str>>` output — no new lifetime or
borrowing relationship beyond what `execute()` already establishes against
`ScanResult<'m>`.

## Function signatures

```rust
/// Location-aware counterpart to `execute` (spec 007). Same preconditions,
/// same non-hierarchy scope, same outer/inner occurrence-then-repetition
/// shape — every `&'m str` execute() would return is paired with its
/// source segment occurrence's 1-based line number.
pub fn execute_located<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
) -> Result<Vec<Vec<LocatedValue<'m>>>, QueryError>;

/// Location-aware counterpart to the CLI's existing `--first` convenience
/// (there is no dedicated `execute_first` in the core today — see
/// research.md #2). Returns the first located value `execute_located` would
/// produce, or `None` when nothing matches.
pub fn first_located<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
) -> Result<Option<LocatedValue<'m>>, QueryError>;
```

No new error type: both functions return the existing `QueryError` (spec
`007`) unchanged — still exactly one variant,
`QueryError::NonNumericComparison`, for the same non-numeric-ordering-filter
case `execute()` already handles this way.

## State / lifecycle

Neither function introduces any new state. Both are pure functions of their
`scan`/`path` inputs, called after the same `scan()` → `parse()` pipeline
`execute()` already requires (spec `005` → spec `006` → this feature) — no
new setup, no new struct that outlives the call, no caching.

## Relationship to existing entities

| Existing entity (spec) | Relationship |
|---|---|
| `ScanResult<'m>` (spec `005`) | Read-only input; `segments` field's existing document order is this feature's sole source of line numbers (research.md #1). Unmodified. |
| `CompiledPath<'_>` (spec `006`) | Read-only input; `execute_located`/`first_located` share the exact same precondition as `execute()` — `path.child` MUST be `None` (non-hierarchy). Unmodified. |
| `QueryError` (spec `007`) | Reused unchanged as the error type for both new functions. |
| `resolve_segment_candidates`, `resolve_field_values` (spec `007`, `pub(crate)`) | `resolve_field_values` reused unchanged (already returns `Vec<&'m str>` per occurrence, which is all `execute_located` needs per matched span). `resolve_segment_candidates` reused unchanged too — its indexed logic is factored into a new sibling, `resolve_segment_candidates_indexed`, which it now delegates to (research.md #1's refinement) — so `execute()` and `hierarchy.rs`'s existing call sites see no change at all. |
