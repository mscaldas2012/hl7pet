# Contract: `hl7pet-core` Scanner Public API

The interface Roadmap specs `006` (PATH parser) and `007` (query execution) build on.
Types are defined in [data-model.md](../data-model.md); this document is the
implementation-facing contract (signatures, error semantics, invariants) — it is the
authority `crates/core/src/scanner.rs` MUST implement.

## Module

`hl7pet_core::scanner`

## Public function

```rust
pub fn scan(message: &str) -> Result<ScanResult<'_>, ScanError>;
```

**Preconditions**: none — `scan` accepts any `&str`, including empty strings and
strings with no valid HL7 structure. It is the function's job to classify malformed
input via `ScanError`, not the caller's job to pre-validate (spec.md FR-006/FR-007).

**Postconditions on `Ok(ScanResult)`**:
- `result.segments` is non-empty (at minimum, the MSH segment itself).
- `result.segments[0]` is the MSH segment; `result.delimiters` was resolved from it.
- `result.delimiter_occurrences` is sorted ascending by `offset`.
- Every `DelimiterOccurrence.segment_index` is a valid index into `result.segments`.
- For a message using the standard delimiters, `result.delimiters == DelimiterSet {
  field: b'|', component: b'^', repetition: b'~', escape: b'\\', subcomponent: b'&'
  }` and segment/delimiter offsets are identical to what a hardcoded-delimiter scan of
  the same message would produce (FR-005 / SC-001 — this is a testable equality, not
  just a design intention).

**Postconditions on `Err(ScanError)`**:
- No partial `ScanResult` is ever returned alongside an error — the `Result` is
  exclusive (FR-006's "produces no offset map").
- The specific `ScanError` variant and its `offset` field identify the exact structural
  problem and location (FR-007) — see data-model.md's `ScanError` table for the
  variant-to-condition mapping.
- `scan` MUST NOT panic for any `&str` input, including empty strings, strings with
  interior NUL bytes, or non-ASCII UTF-8 content in positions other than the five
  delimiter bytes themselves (Constitution Principle III).

## Public types

Re-exported from `hl7pet_core::scanner`:

- `ScanResult<'a>` — see data-model.md.
- `ScanError` — see data-model.md. Implements `std::error::Error` and `Display` (a
  human-readable message including the variant's `offset`) via a manual `impl`, no
  dependency on an error-derive crate (research.md #2 — zero runtime deps).
- `DelimiterSet` — see data-model.md. `Copy`, `Eq` (needed for the FR-005 equality
  postcondition above, and for tests to assert against a literal standard-delimiter
  value).
- `SegmentSpan` — see data-model.md. `Copy`, `Eq`.
- `DelimiterOccurrence` — see data-model.md. `Copy`, `Eq`.
- `DelimiterKind` — see data-model.md. `Copy`, `Eq`, exhaustive 5-variant enum (no
  catch-all `Other` variant — adding a sixth kind is a deliberate future change, not
  something callers should silently tolerate via a wildcard match today).

## Helper method

```rust
impl<'a> ScanResult<'a> {
    pub fn segment_name(&self, segment: &SegmentSpan) -> &'a str;
}
```

Returns the borrowed 3-byte segment name slice (research.md #5) — the one sanctioned
way to read a segment's name, so callers never hand-roll the `start..start+3` slicing
themselves. Panics only if given a `SegmentSpan` that did not originate from this same
`ScanResult` (a programmer error, not a data-validity concern — out of scope for
`Result`-based error handling per Constitution Principle III's own carve-out for
"violated structural preconditions" that are the caller's bug, not the input's).

## What this contract explicitly does NOT provide (deferred to later specs)

- PATH expression evaluation — spec `006`/`007`.
- Escape-sequence decoding of field values — spec `1001`.
- Hierarchy/parent-child navigation — spec `008`.
- 1-based source line numbers — spec `1000` (`located-extraction-api`), which the
  Roadmap says depends on "offset data already tracked internally by the message
  scanner" — this contract's byte offsets are exactly that raw material, but computing
  or exposing line numbers is not this spec's job.
- Any batch/streaming multi-message API — Migration Plan stretch goals, not Phase 2.
