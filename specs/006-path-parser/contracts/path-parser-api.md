# Contract: `hl7pet-core` PATH Parser Public API

The interface Roadmap specs `007` (query execution) and `008` (lazy hierarchy
navigation) build on. Types are defined in [data-model.md](../data-model.md); this
document is the implementation-facing contract (signatures, error semantics,
invariants) — it is the authority `crates/core/src/parser.rs` MUST implement.

## Module

`hl7pet_core::parser`

## Public function

```rust
pub fn parse(path: &str) -> Result<CompiledPath<'_>, ParseError>;
```

**Preconditions**: none — `parse` accepts any `&str`, including the empty string and
strings with no valid PATH structure. Classifying malformed input via `ParseError` is
the function's job, not the caller's job to pre-validate (spec.md FR-006, mirroring
`hl7pet_core::scanner::scan`'s own precondition-free contract).

**Postconditions on `Ok(CompiledPath)`**:
- The input string matched the `PATH` production in
  `specs/001-path-grammar-spec/contracts/path-grammar.md` in its entirety — no
  unconsumed trailing characters (`ParseErrorKind::TrailingInput` otherwise).
- `result.segment` and, when present, `result.field`/`result.child` losslessly
  represent every piece of the matched grammar (spec.md FR-002-FR-005) — nothing in the
  input PATH is discarded or normalized away.
- No text from `path` is copied into `CompiledPath` where a borrowed `&'_ str` slice
  would do (spec.md FR-011); the only heap allocation, if any, is `FilterClause.values`
  when a filter clause is present (data-model.md).
- Calling `parse` again with the same `path` value always returns an
  observably-identical `CompiledPath` (spec.md FR-009 — parsing is a pure function of
  the string alone).

**Postconditions on `Err(ParseError)`**:
- No partial `CompiledPath` is ever returned alongside an error — the `Result` is
  exclusive (spec.md FR-007).
- `ParseError.kind` and `ParseError.offset` together identify the exact grammar rule
  violated and its position in `path` (spec.md FR-008) — see data-model.md's
  `ParseErrorKind` table for the variant-to-condition mapping.
- `parse` MUST NOT panic for any `&str` input, including empty strings, strings with
  interior NUL bytes, or malformed UTF-8-adjacent byte sequences that are nonetheless
  valid `&str` content (Constitution Principle III).

## Public types

Re-exported from `hl7pet_core::parser`:

- `CompiledPath<'a>` — see data-model.md. `Eq`, `Clone` (cheap: no owned heap data
  except an already-allocated `Vec` inside an optional `FilterClause`, itself `Clone`).
- `SegmentExpr<'a>`, `SegIndex<'a>`, `FieldExpr`, `FieldIndex`, `FilterClause<'a>`,
  `FilterOperator`, `ChildPath<'a>` — see data-model.md. All `Eq`, `Clone`.
- `ParseError` — see data-model.md. Implements `std::error::Error` and `Display` (a
  human-readable message including `ParseErrorKind` and `offset`) via a manual `impl`,
  no error-derive crate dependency, matching `hl7pet_core::scanner::ScanError`'s
  precedent.
- `ParseErrorKind` — see data-model.md. `Copy`, `Eq`, exhaustive enum (no catch-all
  variant — a syntax condition this parser doesn't yet distinguish should surface as a
  new named variant in a future change, not be silently folded into a generic one).

## What this contract explicitly does NOT provide (deferred to later specs)

- Evaluating a `CompiledPath` against a message's scanned offsets to extract values —
  spec `007` (query execution). This module never reads `hl7pet_core::scanner`'s types
  or output.
- Resolving the hierarchy operator's parent→child navigation semantics — spec `008`.
  This module only recognizes `SEGMENT_EXPR -> CHILD_PATH` as syntax and captures both
  halves distinctly; it does not know what `->` *means*.
- Multi-hop hierarchy chaining (`ORC[1] -> OBR[1] -> OBX-5`) — not yet part of the
  grammar this parser implements. A second `" -> "` is rejected
  (`ParseErrorKind::MultipleHierarchyHops`), not silently accepted or truncated.
- Escape-sequence decoding of filter values or any other PATH substring — spec `1001`'s
  scope is `getValue`/`getFirstValue` output, not PATH syntax; this parser's `FILTER`
  `VALUE` class excludes the characters that would need escaping in the first place
  (`contracts/path-grammar.md` Note #7), so there is nothing to decode here regardless.
- Semantic/range validation of `SEG_IDX`/`FIELD_IDX` numeric values (e.g. whether `0` is
  meaningful) — any syntactically valid `NUMBER` parses successfully; matching real
  data is spec `007`'s concern (`contracts/path-grammar.md` Non-Goals).
