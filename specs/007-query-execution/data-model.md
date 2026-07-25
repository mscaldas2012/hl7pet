# Data Model: Query Execution

Entities carried over from [spec.md](spec.md)'s Key Entities section, made concrete
against the design decisions in [research.md](research.md). Types are given in Rust
since this spec's deliverable is Rust source (`crates/core/src/query.rs`); see
[contracts/query-api.md](contracts/query-api.md) for the full public API surface.
Inputs (`ScanResult`, `CompiledPath` and its constituents) are defined by specs `005`/
`006` and are not redefined here — only referenced.

## QueryError

The executor's non-panic failure output — returned instead of a result per
Constitution Principle III, reserved for the one genuine structural precondition
this spec has (research.md #2's Verification note). An out-of-range segment or field
index is deliberately **not** a variant here: verified against the real Scala engine,
`getValue`/`getFirstValue` return no match for an out-of-range index rather than
throwing, so this executor represents that case as `Ok(vec![])` (Query Result shape,
below), the same as any other "no data" outcome — a single-variant enum, not the
3-variant design an earlier draft of this spec's planning considered before that
verification.

| Variant | Fields | Corresponds to |
|---|---|---|
| `NonNumericComparison` | `operator: FilterOperator` | A `FilterClause` uses an ordering operator (`Gt`/`Ge`/`Lt`/`Le`) and either the target sub-value or one of the filter's literal values fails to parse as a number (research.md #4, spec.md FR-008). Verified: the real Scala engine throws an uncaught `NumberFormatException` here rather than handling it gracefully — this executor surfaces a typed `Err` instead of reproducing that crash. |

`QueryError` mirrors `ScanError`/`ParseError`'s existing precedent: `Copy` (its one
field, `FilterOperator`, is already `Copy`), `Eq`, implements `std::error::Error` and
`Display` via a manual `impl` (no error-derive crate dependency, keeping
`hl7pet-core` at zero runtime deps). Exhaustive — no catch-all variant, matching
`ScanError`/`ParseErrorKind`'s convention that a newly discovered condition gets a new
named variant rather than falling into a generic one.

## Query Result shape

Not a distinct named struct — per research.md #1, the executor's success output is
`Vec<Vec<&'a str>>`, tied to the source message's lifetime `'a` (the same lifetime
`ScanResult<'a>` carries). This is deliberately the same shape
`fixtures/schemas/conformance-vector.schema.json`'s `expected` field already uses for
`getValue`:

- **Outer dimension**: one entry per matched segment occurrence, in message order,
  *except* an occurrence whose requested field-repetition index is out of range,
  which contributes no entry at all (verified against the real Scala engine: this
  collapses to no match entirely, not an occurrence with an empty inner list). Empty
  when zero occurrences matched, for any reason — segment type absent, an explicit
  segment index out of range, or a filter matching nothing (research.md #2(b)/(c)/(d),
  all represented identically, verified against the real Scala engine).
- **Inner dimension**: one entry per field repetition matched within that occurrence
  (research.md #7's repetition numbering), or exactly one entry — the full segment
  content unsplit — when `CompiledPath.field` is `None` (research.md #5). An entry is
  the empty string `""`, not omitted, when the requested field/component/subcomponent
  number is beyond what that occurrence contains (spec.md FR-009(e)).

`getFirstValue`'s shape (`Option<&'a str>`) is a derivation, not a stored type:
`result.first().and_then(|reps| reps.first()).copied()`.

## Occurrence Candidate (internal, not part of the public API)

An intermediate value used while resolving a `SegIndex`/`FilterClause` against
`ScanResult.segments` — not exposed to callers, listed here because it is the
concrete mechanism `contracts/query-api.md`'s algorithm description depends on.

| Field | Type | Notes |
|---|---|---|
| `segment` | `SegmentSpan` (spec `005`) | The candidate segment occurrence's byte span. |
| `occurrence_number` | `u32` | 1-based position among same-named segment occurrences only (research.md #7), used to resolve `SegIndex::Numeric`/`Last`. |

Produced by filtering `ScanResult.segments` to those whose `ScanResult::segment_name`
matches `CompiledPath.segment.name`, in message order — the same filter both plain
positional resolution (`SegIndex::Numeric`/`Last`/`Star`) and filter-clause candidate
iteration (`SegIndex::Filter`) walk over, per research.md #3's shared-navigation
decision.

## Relationship to specs `005`/`006`'s types

```text
CompiledPath<'p>  (spec 006, borrows PATH string)
  - segment: SegmentExpr<'p> { name, index: Option<SegIndex<'p>> }
  - field:   Option<FieldExpr>

execute(scan: &ScanResult<'m>, path: &CompiledPath<'p>):
  1. filter ScanResult<'m>.segments (spec 005) by name match against
     CompiledPath.segment.name, in message order
       -> Occurrence Candidates (this spec, internal)
  2. resolve CompiledPath.segment.index (SegIndex) against those candidates
       -> the subset of candidates this query targets
  3. for each targeted candidate, extract_subvalue(...) (this spec, internal,
     research.md #3) splits its segment content using
     ScanResult<'m>.delimiters (spec 005's DelimiterSet) down to the level
     CompiledPath.field (FieldExpr) requests
  4. returns Result<Vec<Vec<&'m str>>, QueryError>
```

The output's `Vec<Vec<&'m str>>` borrows the message lifetime `'m` (from
`ScanResult<'m>`), independent of the PATH string's lifetime `'p` (from
`CompiledPath<'p>`) — the two are unrelated per specs `005`/`006`'s own designs, and
this spec's function signature must reflect that (see `contracts/query-api.md`).

The output's lifetime is tied to the message (`ScanResult<'m>`'s `'m`), not the PATH
string (`CompiledPath<'p>`'s `'p`) — the two are independent per specs `005`/`006`'s
own designs, and this spec's function signature must reflect that (see
`contracts/query-api.md`).
