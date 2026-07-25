# Quickstart: Query Execution

Validates spec.md's user stories end-to-end: a single-value PATH returns the correct
field/component/subcomponent (US1), segment and field index selectors resolve to the
right occurrence(s) (US2), and a filter clause selects matching segment occurrence(s)
(US3) — all validated byte-for-byte against the existing Scala engine's behavior via
the shared regression suite (spec `003`).

## Prerequisites

- Rust `stable` toolchain — same as specs `005`/`006`, no pinned MSRV.
- No JVM, Scala, or Maven required to *run* this spec's own tests — only needed if
  regenerating the 6 new conformance vectors' expected values (research.md #6), which
  reuses spec `004`'s existing Maven Central dependency setup.
- Specs `005` (scanner) and `006` (parser) already implemented — this spec is a new
  sibling module, not a rewrite of either (plan.md Project Structure).

## 1. Build with the new query module

```bash
cargo build --workspace
```

**Expected outcome**: `crates/core` compiles cleanly with zero warnings, now exporting
`hl7pet_core::query::{execute, QueryError}` alongside the existing
`hl7pet_core::scanner` and `hl7pet_core::parser` modules.

## 2. Run unit tests (individual resolution rules)

```bash
cargo test -p hl7pet-core --lib query
```

**Expected outcome**: unit tests colocated in `query.rs` pass — these cover each
`SegIndex`/`FieldIndex` resolution form, filter evaluation (including OR'd values,
subcomponent targets, and each of the six operators), and each `QueryError` condition
independently of the shared fixtures corpus.

## 3. Run the conformance vector suite against `fixtures/vectors/path/valid.json` (US1, US2, US3)

```bash
cargo test -p hl7pet-core --test query_vectors
```

**Expected outcome**: for every non-hierarchy entry (path not containing `" -> "`),
scanning `message_ref`, parsing `path`, and executing the query reproduces `expected`
exactly for the vector's declared `method` — 19 vectors exercised (14 existing minus 1
hierarchy vector, plus 6 new additions from FR-014/research.md #6). This is the single
command that proves SC-001.

## 4. Confirm a non-numeric ordering comparison surfaces as an error, never a panic

```bash
cargo test -p hl7pet-core --lib -- comparison
```

**Expected outcome**: an ordering operator (`>`, `>=`, `<`, `<=`) against a
non-numeric operand returns `Err(QueryError::NonNumericComparison)` — confirmed
against this spec's new `path-filter-nonnumeric-ordering` vector, which documents
that the real Scala engine throws an uncaught `NumberFormatException` here (research.md
#4) rather than handling it gracefully; this executor deliberately surfaces a typed
error instead of reproducing that crash. Never a panic — `cargo test` reporting a
panic as a failed test with a backtrace makes this a strong negative-case check.

## 5. Confirm "no data present" — including out-of-range indices — never surfaces as an error

```bash
cargo test -p hl7pet-core --lib -- absent no_match out_of_range
```

**Expected outcome**: a segment type entirely absent from the message, an explicit
segment or field index beyond what's actually present, and a filter matching zero
candidate occurrences all return `Ok(vec![])` — confirmed against the existing
`path-zero-values-nonexistent` vector's `"expected": null` and this spec's new
`path-segidx-out-of-range`/`path-fieldidx-out-of-range`/`path-filter-no-match`
vectors, all verified live against the real Scala engine (research.md #2) to return
no match there too, never `Err`.

## 6. Confirm at-most-one-pass extraction (SC-004)

```bash
cargo test -p hl7pet-core --lib -- single_pass
```

**Expected outcome**: a dedicated test (mirroring spec `005`'s allocation-counting
precedent, `crates/core/src/test_alloc.rs`) confirms executing a query against a
message with many repetitions/components performs at most one traversal of each
matched segment occurrence's content — not a re-scan per repetition or per filter
candidate.

## 7. Corpus validation still passes with the extended `path` family

```bash
python3 fixtures/scripts/validate_corpus.py
```

**Expected outcome**: spec `003`'s existing validation script accepts the 6 new
entries in `fixtures/vectors/path/valid.json` without any script or schema change —
they conform to the same `conformance-vector.schema.json` the existing 14 vectors
already use (plan.md Structure Decision).
