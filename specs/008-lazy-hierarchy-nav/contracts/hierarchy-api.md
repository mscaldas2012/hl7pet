# Contract: `hl7pet-core` Hierarchy Navigation Public API

The interface downstream Roadmap specs (`009` core-perf-validation, a future
multi-hop spec, and eventually the `6000`-range language bindings) build on. Types
are given in [data-model.md](../data-model.md); this document is the
implementation-facing contract (signatures, error semantics, invariants) —
the authority `crates/core/src/hierarchy.rs` MUST implement.

## Module

`hl7pet_core::hierarchy`

## Public functions

```rust
impl HierarchyProfile {
    pub fn from_json(json: &str) -> Result<Self, ProfileError>;
}

pub fn execute_hierarchy<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
    profile: Option<&HierarchyProfile>,
) -> Result<Vec<Vec<&'m str>>, QueryError>;
```

**Why `profile` is `Option`, not required**: mirrors spec `002` Section A.5's
"no profile" (static mode) as a real, distinct, callable state (FR-009) rather than
forcing every caller — including ones who only ever pass flat, non-hierarchy
`CompiledPath`s — to supply a `HierarchyProfile` they will never use. A caller who
knows in advance they only have flat paths should keep calling spec `007`'s
`execute()` directly; `execute_hierarchy` is the superset entry point for callers
(e.g. the `hl7pet` CLI, and later bindings) who don't want to branch on
`path.child.is_some()` themselves.

**Preconditions**: None beyond spec `007`'s existing ones for a flat `path` (no
`child`). For a hierarchy `path` (`child: Some(_)`), there is no additional
precondition on `scan` or `profile` — any combination, including a `profile` that
recognizes neither the parent nor the child segment type, or a `scan` with zero
occurrences of either, must produce a `Result`, never panic (Constitution
Principle III, SC-004).

**Behavior**:
1. `path.child.is_none()` — delegates to `query::execute(scan, path)` unchanged.
   `profile` is accepted but ignored (FR-009's "no profile" boundary is specifically
   about `->` expressions; a flat path never needed a profile in the first place).
2. `path.child.is_some()` and `profile.is_none()` — returns `Ok(vec![])` (FR-009):
   the whole `->` expression yields no match, matching spec `002` Section A.5
   exactly. The parent and child sides are never independently evaluated as flat
   paths.
3. `path.child.is_some()` and `profile.is_some(p)` — resolves per
   [data-model.md](../data-model.md)'s "Relationship to specs `005`-`007`'s types"
   walk-through: parent selection (spec `007`, unchanged) → per-parent bounded
   direct-child scan, type-filtered to `child.segment.name` (research.md #1,
   FR-003) → per-parent `csegIdx` resolution (data-model.md's corrected table,
   FR-007) → concatenation across matching parents, in document order → `child.field`
   resolution (spec `007`'s `resolve_field_values`, unchanged).

**Postconditions on `Ok(values)`**:
- Same shape and same conventions as spec `007`'s `execute()` (outer = matched
  occurrences in message order, inner = field repetitions; an occurrence whose
  requested repetition index is out of range contributes no entry at all; a
  field/component/subcomponent number beyond what's present is an empty-string
  entry, not an omission) — this spec introduces no new shape, only a new selection
  step ahead of the same shape (FR-001).
- `values` is empty for any of: the parent selector matched zero occurrences; a
  matching parent had zero direct children of the requested child type; no profile
  was supplied; the child's own `csegIdx`/`FieldIndex` was out of range for its
  scope. All represented identically as `Ok(vec![])` (FR-006), never distinguished
  from each other in the return value — matching spec `007`'s existing "no data
  present" philosophy, not spec `002` Section A.2's `None`-vs-`Some(empty)` Scala
  distinction (that distinction has no Rust representative here, same as it has
  none in spec `007`'s `execute()` for the analogous flat-path cases).
- Every `&'m str` borrows directly from `scan.message` — no owned/copied
  substrings anywhere in the return value (Constitution Principle II), identical to
  spec `007`.
- No line of the message is visited outside the range research.md #1's bounded scan
  actually needs — no full-message tree is constructed, cached, or retained across
  calls (FR-003, FR-013, SC-002).

**Postconditions on `Err(QueryError)`**:
- Returned only for `QueryError::NonNumericComparison` (research.md #3) — an
  ordering operator applied to a non-numeric operand, on either the parent's or the
  child's `SEG_IDX` filter clause. No new error variant exists for hierarchy
  queries.
- `execute_hierarchy` MUST NOT panic for any `ScanResult`/`CompiledPath`/
  `Option<&HierarchyProfile>` combination, including a profile that recognizes
  neither side's segment type, a message with zero occurrences of the parent type,
  or a message ending immediately after the parent occurrence's own line
  (Constitution Principle III, SC-004).

## Public types

Re-exported from `hl7pet_core::hierarchy`:

- `HierarchyProfile` — opaque (data-model.md); `from_json` is its only public
  constructor. No `serde`/`serde_json` type appears in this struct's public shape or
  in `ProfileError` (FR-014) — verified by this module having no `pub` item whose
  type path includes `serde_json`.
- `ProfileError` — see data-model.md. `Clone`, `Eq`, implements `std::error::Error`
  and `Display` via a manual `impl`. Exhaustive — no catch-all variant.

No new success type — `execute_hierarchy`'s success value is `Vec<Vec<&'m str>>`,
identical to spec `007`'s `execute()`. `QueryError` is re-used, not re-exported
again (already public from `hl7pet_core::query`).

## What this contract explicitly does NOT provide (deferred, per spec.md Clarifications)

- Multi-hop `->` chaining (`ORC[1] -> OBR[1] -> OBX-5`) — deferred to a future spec
  (spec.md User Story 3). `CompiledPath.child`'s type (`ChildPath`, spec `006`)
  stays non-recursive; `execute_hierarchy` has no notion of a child's own child.
- Reproducing the original Scala engine's child-index bug (spec `002` Section A.4) —
  deliberately not done; see data-model.md's corrected resolution table and
  `ROADMAP.md`'s Documented Breaking Changes entry for spec `008`.
- Cardinality validation of any kind (`[m..n]` enforcement) — Roadmap 2000-2999's
  `StructureValidator`, unrelated to navigation (spec `002` Section A.3, carried
  forward unchanged by this spec).
- Resolving an **ambiguous parent-side type** — a segment type used as a `->`
  expression's parent that occupies more than one position in `segmentDefinition` —
  via history-dependent disambiguation. `node_for` (data-model.md) returns `None`
  for this case, folding into FR-006's "no qualifying children" outcome, same as an
  absent type. A segment type repeating at multiple positions is otherwise fully
  supported and common (research.md #2) — this limitation applies only when *that
  specific type* is itself used as a parent, which no existing vector's parent side
  ever is.
- Escape-sequence decoding of any extracted value — spec `1001`'s scope, unchanged
  from spec `007`'s existing boundary.
- Any batch/streaming multi-message or multi-PATH API, or Arrow-oriented output —
  later Migration Plan phases, not Phase 3.
