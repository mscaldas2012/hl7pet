# Contract: `hl7pet-core` Query Execution Public API

The interface downstream Roadmap specs (`008` hierarchy navigation, and eventually the
`6000`-range language bindings) build on. Types are defined in
[data-model.md](../data-model.md); this document is the implementation-facing contract
(signatures, error semantics, invariants) — it is the authority
`crates/core/src/query.rs` MUST implement.

## Module

`hl7pet_core::query`

## Public function

```rust
pub fn execute<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
) -> Result<Vec<Vec<&'m str>>, QueryError>;
```

**Preconditions**: `path.child` MUST be `None`. Calling `execute` with a `CompiledPath`
whose `child` is `Some(_)` is a programmer error (this executor never resolves
hierarchy navigation, spec.md FR-011) — implementations MAY panic or MAY treat it as
equivalent to ignoring `child` and evaluating only `segment`/`field`; either is
acceptable since spec `008` owns defining actual behavior for the hierarchy form.
Aside from this, `execute` has no precondition on `scan` or the non-hierarchy shape of
`path` — any combination of a well-formed `ScanResult` and a non-hierarchy
`CompiledPath` (including ones that structurally cannot match anything, e.g. a segment
name never present in the message) must produce a `Result`, never panic
(Constitution Principle III, SC-003).

**Postconditions on `Ok(values)`**:
- `values.len()` is at most the number of segment occurrences `path.segment` resolved
  to (data-model.md's Occurrence Candidate resolution) — `0` when nothing matched, for
  any reason: the segment type is absent, an explicit segment/field index is out of
  range, or a filter matched nothing (research.md #2, verified against the real Scala
  engine — none of these throw).
- Each `values[i]` present is non-empty: exactly one entry (the full raw segment
  content) when `path.field` is `None` (research.md #5), or one entry per resolved
  field repetition otherwise (research.md #7's numbering). A matched segment
  occurrence whose requested field-repetition index (`FieldIndex::Numeric`) is out of
  range contributes **no entry at all** to `values` — verified against the real Scala
  engine, which collapses this to no match entirely rather than an occurrence with an
  empty inner list.
- Every `&'m str` in `values` borrows directly from `scan.message` — no owned/copied
  substrings anywhere in the return value (FR-013, Constitution Principle II).
- An entry is the empty string `""`, not omitted and not an error, when the requested
  field/component/subcomponent *number* is beyond what that specific occurrence
  contains (spec.md FR-009(e)) — distinct from an out-of-range repetition *index*
  (previous bullet), which drops the occurrence rather than keeping an empty string.
- `values` is ordered by message order of the matched segment occurrences, and within
  each occurrence, by ascending repetition position (spec.md FR-006).

**Postconditions on `Err(QueryError)`**:
- No partial `values` is ever returned alongside an error — the `Result` is exclusive
  (spec.md FR-001), matching `hl7pet_core::scanner::scan` and `hl7pet_core::parser::parse`'s
  own precedent.
- Returned only for the one structural-precondition case data-model.md's `QueryError`
  enumerates: a non-numeric operand compared with an ordering filter operator (`>`,
  `>=`, `<`, `<=`). Never returned for "the segment type isn't in this message," "an
  explicit segment/field index is out of range," or "the filter matched nothing" —
  those are all `Ok(vec![])` (research.md #2's Verification note: confirmed against
  the real Scala engine, none of these throw there either).
- `execute` MUST NOT panic for any `&ScanResult`/non-hierarchy `&CompiledPath`
  combination, including pathological ones (empty message-derived `ScanResult`,
  filters that reference field numbers absent from every candidate, non-ASCII field
  content) — Constitution Principle III, SC-003.

## Public types

Re-exported from `hl7pet_core::query`:

- `QueryError` — see data-model.md. `Eq`, `Clone`/`Copy` where its fields allow.
  Implements `std::error::Error` and `Display` via a manual `impl`, no error-derive
  crate dependency, matching `ScanError`/`ParseError`'s precedent. Exhaustive — no
  catch-all variant.

No new success type is introduced — `execute`'s success value is the plain
`Vec<Vec<&'m str>>` described in data-model.md's "Query Result shape" section, chosen
to match `fixtures/schemas/conformance-vector.schema.json`'s existing `getValue`
representation exactly (research.md #1) rather than wrap it in a project-specific
newtype with no behavior of its own to add.

## Derived convenience: `getFirstValue`-shaped access

Not a separate function requiring its own contract — any caller (including this
spec's own conformance test) derives it from `execute`'s result with no additional
message walk:

```rust
execute(scan, path)?.first().and_then(|reps| reps.first()).copied()
// Option<&'m str> — None when values is empty, matching the getFirstValue
// vectors' `"expected": null` convention.
```

## What this contract explicitly does NOT provide (deferred to later specs)

- Resolving `CompiledPath.child` (hierarchy `->` navigation) — spec `008`. This module
  never inspects `child`'s contents beyond the precondition note above.
- Escape-sequence decoding of any extracted value (e.g. `\F\`, `\H\`) — spec `1001`'s
  scope; `execute` returns raw substrings exactly as they appear in the message
  (spec.md FR-012).
- 1-based source line numbers alongside extracted values — spec `1000`
  (`located-extraction-api`), which the Roadmap says depends on offset data already
  tracked by the message scanner (spec `005`) and, transitively, this executor's
  occurrence resolution; computing or exposing line numbers is not this spec's job.
- Any batch/streaming multi-message or multi-PATH API — Migration Plan stretch goals,
  not Phase 2.
- Range/semantic validation of `field_num`/`comp_num`/`subcomp_num` beyond "does this
  occurrence structurally contain it" (spec `001`'s grammar Non-Goals already ruled
  out numeric-range validation at parse time; this executor does not add it at
  evaluation time either).
