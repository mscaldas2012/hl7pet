# Contract: `hl7pet-core` Located Hierarchy Public API

The interface downstream consumers (the Python binding, module `6000`-`6999`;
eventually a Java binding) build on. Types are defined in
[data-model.md](../data-model.md); this document is the implementation-facing
contract (signatures, error semantics, invariants) — it is the authority
`crates/core/src/hierarchy.rs` MUST implement for this feature, exactly as
`008-lazy-hierarchy-nav/contracts/hierarchy-api.md` is for `execute_hierarchy`
and `1000-located-extraction-api/contracts/located-extraction-api.md` is for
`execute_located`.

## Module

`hl7pet_core::hierarchy` (re-exported from crate root, alongside
`execute_hierarchy`/`HierarchyProfile`/`ProfileError`)

## Public function

```rust
pub fn execute_hierarchy_located<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
    profile: Option<&HierarchyProfile>,
) -> Result<Vec<Vec<query::LocatedValue<'m>>>, QueryError>;
```

**Preconditions**: Identical to `execute_hierarchy`'s
(`008-lazy-hierarchy-nav/contracts/hierarchy-api.md`) — no additional
precondition on `scan` or `profile` beyond what that contract already states.
Any combination, including a `profile` that recognizes neither the parent
nor the child segment type, or a `scan` with zero occurrences of either, MUST
produce a `Result`, never panic (Constitution Principle III).

**Behavior**:
1. `path.child.is_none()` — delegates to `query::execute_located(scan, path)`
   (spec `1000`) unchanged, mirroring `execute_hierarchy`'s own existing
   delegation to `query::execute` for the same case. `profile` is accepted
   but ignored, identical rationale to `execute_hierarchy`'s.
2. `path.child.is_some()` and `profile.is_none()` — returns `Ok(vec![])`,
   identical to `execute_hierarchy`'s FR-009 static-mode fallback: the whole
   `->` expression yields no match, and it is never independently evaluated
   as two flat paths.
3. `path.child.is_some()` and `profile.is_some(p)` — resolves via the exact
   same steps `execute_hierarchy` already performs (parent selection →
   per-parent bounded direct-child scan, type-filtered → per-parent
   `csegIdx` resolution → concatenation across matching parents, in document
   order), with each selected child span additionally carrying its own
   1-based line number (data-model.md's `_indexed` helpers), before the
   final field-resolution step calls `query::resolve_field_values_located`
   instead of `query::resolve_field_values`.

**Postconditions on `Ok(values)`** — for every position `[i][j]`:
- `values[i][j].value` is byte-for-byte identical to what
  `execute_hierarchy(scan, path, profile)?[i][j]` would return for the same
  `scan`/`path`/`profile` (spec.md FR-002). `execute_hierarchy_located` and
  `execute_hierarchy` MUST always agree on shape: `values.len()` equals
  `execute_hierarchy()`'s outer length, and `values[i].len()` equals its
  corresponding inner length, for every `i`.
- `values[i][j].line` is the 1-based position, among all segments in
  `scan.message`, of the *child* segment occurrence that produced
  `values[i][j].value` (spec.md FR-003) — never the parent occurrence's
  line. Every `LocatedValue` within the same `values[i]` group shares one
  `line` (spec.md FR-004) — one child segment occurrence, one line,
  regardless of how many field/component/subcomponent values it yields.
- `values` is empty for any of the same reasons `execute_hierarchy` already
  returns `Ok(vec![])` (no profile, absent parent type, zero direct
  children, out-of-range child `SEG_IDX`, ambiguous-parent-type occurrence
  with no legal position given the real message structure) — all
  represented identically, never distinguished from each other, never a
  fabricated `LocatedValue` (spec.md FR-006).
- Ordering matches `execute_hierarchy`'s: children flattened across matching
  parent occurrences in document order, then ascending repetition position
  within each (spec.md FR-005) — the located entry point introduces no new
  grouping of its own (research.md #3).
- Every `LocatedValue.value` borrows directly from `scan.message`
  (`Cow::Borrowed`) unless spec `1001`'s escape decoding rewrote it
  (`Cow::Owned`) — identical to `execute_hierarchy`'s existing output
  (Constitution Principle II).

**Postconditions on `Err(QueryError)`**: Identical trigger and shape to
`execute_hierarchy`'s — `QueryError::NonNumericComparison` for an ordering
operator applied to a non-numeric operand, on either side's `SEG_IDX` filter
clause. No new error variant exists for located hierarchy queries. No
partial `values` accompanies an error.

**Relationship to `execute_hierarchy`**: `execute_hierarchy_located` is not
merely "similar to" `execute_hierarchy` — for the same
`scan`/`path`/`profile`,
`execute_hierarchy_located(scan, path, profile)?.into_iter().map(|group|
group.into_iter().map(|lv| lv.value).collect()).collect::<Vec<Vec<_>>>()`
MUST equal `execute_hierarchy(scan, path, profile)?` exactly. This is the
executable form of spec.md FR-002/SC-002 and is what the new integration
test (`located_hierarchy_vectors.rs`) verifies directly.

## Python binding surface (`crates/python/src/lib.rs`, secondary module: Language Bindings `6000`-range)

```python
def get_value_hierarchy_located(
    message: str, path: str, profile: dict[str, Any], build_hierarchy: bool = True
) -> list[list[LocatedValue]] | None: ...
```

Mirrors `get_value_hierarchy`'s existing profile-handling (JSON
re-serialization via the stdlib `json` module, `build_hierarchy` toggle) and
`get_value_located`'s existing empty-to-`None` collapse and per-value
`LocatedValue::from` wrapping (research.md #4). Raises the same typed
exceptions `get_value_hierarchy` already raises
(`Hl7ScanError`/`Hl7PathError`/`Hl7QueryError`/`Hl7ProfileError`) for the
same structural failures — no new exception type.

## Non-goals (explicitly out of contract)

- Multi-hop `->` chaining — unchanged Non-Goal carried forward from spec
  `008`; `CompiledPath.child`'s type stays non-recursive.
- Any new error variant, byte-offset field, column, or sub-line position —
  `LocatedValue` carries exactly `value` and `line`, nothing else, per
  data-model.md.
- A located counterpart to a "first hierarchy value" convenience — no such
  non-located method exists today for hierarchy mode, so none is added here
  either (spec.md Assumptions).
- Changing `execute_hierarchy`'s or `get_value_hierarchy`'s own signature,
  behavior, or output in any way (Constitution Principle I read together
  with the Backward-Compatible-Additions convention, `ROADMAP.md`).
