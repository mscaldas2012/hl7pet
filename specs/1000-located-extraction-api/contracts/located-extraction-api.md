# Contract: `hl7pet-core` Located Extraction Public API

The interface downstream consumers (the `hl7pet` dev CLI today; eventually the
`6000`-range Python/JNI bindings, Constitution Principle IV) build on. Types are
defined in [data-model.md](../data-model.md); this document is the
implementation-facing contract (signatures, error semantics, invariants) — it is
the authority `crates/core/src/query.rs` MUST implement for this feature, exactly
as `007-query-execution/contracts/query-api.md` is for `execute()`.

## Module

`hl7pet_core::query` (re-exported from crate root, alongside `execute`/`QueryError`)

## Public type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatedValue<'m> {
    pub value: &'m str,
    pub line: usize,
}
```

See data-model.md for field semantics and invariants.

## Public functions

```rust
pub fn execute_located<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
) -> Result<Vec<Vec<LocatedValue<'m>>>, QueryError>;

pub fn first_located<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
) -> Result<Option<LocatedValue<'m>>, QueryError>;
```

### `execute_located`

**Preconditions**: Identical to `execute()`'s (`query-api.md`): `path.child`
MUST be `None`. Hierarchy-form `CompiledPath`s are spec `008`'s domain, not this
function's — implementations MAY panic or MAY ignore `child`, matching
whatever `execute()` itself does for the same input, since this feature makes
no independent decision here.

**Postconditions on `Ok(values)`** — for every position `[i][j]`:
- `values[i][j].value` is byte-for-byte identical to what
  `execute(scan, path)?[i][j]` would return for the same `scan`/`path`
  (spec.md FR-003, SC-002). `execute_located` and `execute` MUST always agree
  on shape: `values.len()` equals `execute()`'s outer length, and
  `values[i].len()` equals `execute()`'s corresponding inner length, for every
  `i`.
- `values[i][j].line` is the 1-based position, among all segments in
  `scan.message`, of the segment occurrence that produced `values[i][j].value`
  (spec.md FR-004). Every `LocatedValue` within the same `values[i]` group
  shares one `line` (spec.md FR-005) — one segment occurrence, one line,
  regardless of how many field/component/subcomponent values it yields.
- `values` is empty when nothing matches, for any of the same reasons
  `execute()` already returns `Ok(vec![])` (absent segment type, out-of-range
  index, filter matched nothing) — never an error, never a fabricated
  `LocatedValue` (spec.md FR-007).
- Ordering matches `execute()`'s: message order of matched segment
  occurrences, then ascending repetition position within each.

**Postconditions on `Err(QueryError)`**: Identical trigger and shape to
`execute()`'s — the one non-numeric-ordering-filter case (`query-api.md`). No
partial `values` accompanies an error.

**Relationship to `execute`**: `execute_located` is not merely "similar to"
`execute` — for the same `scan`/`path`, `execute_located(scan, path)?
.into_iter().map(|group| group.into_iter().map(|lv| lv.value).collect())
.collect::<Vec<Vec<_>>>()` MUST equal `execute(scan, path)?` exactly. This is
the executable form of spec.md FR-003/SC-002 and is what the new integration
test (`located_vectors.rs`) verifies directly.

### `first_located`

**Preconditions**: Same as `execute_located`.

**Postconditions on `Ok(value)`**:
- `Some(lv)` where `lv` equals `execute_located(scan, path)?[0][0]` whenever
  that entry exists (spec.md FR-002, User Story 3) — i.e. the first value in
  document order, exactly as the CLI's existing `--first` flag already
  selects from `execute()`'s output today.
- `None` exactly when `execute_located(scan, path)?` would be empty, or its
  first group would be empty — never a fabricated value or line (spec.md
  Acceptance Scenario, User Story 3 #2).

**Postconditions on `Err(QueryError)`**: Identical to `execute_located`'s.

## Non-goals (explicitly out of contract)

- Hierarchy-mode PATHs (`path.child = Some(_)`) — spec `008`'s domain,
  untouched by this contract (spec.md Assumptions).
- Any new error variant, byte-offset field, or column/sub-line position —
  `LocatedValue` carries exactly `value` and `line`, nothing else
  (data-model.md's Key Entity definition).
- Changing `execute()`'s own signature, behavior, or output in any way
  (Constitution Principle I read together with the Backward-Compatible-
  Additions convention, `ROADMAP.md`).
