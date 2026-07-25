# Quickstart: PATH Parser

Validates spec.md's user stories end-to-end: malformed PATHs rejected at parse time
with a precise reason (US1), a parsed PATH reused without re-parsing (US2), and the
compiled result exposing structured fields rather than an opaque validated string
(US3).

## Prerequisites

- Rust `stable` toolchain — same as spec `005`, no pinned MSRV.
- No JVM, Scala, or Maven required to *run* this spec's own tests — only needed if
  regenerating the 4 new conformance vectors' expected values (research.md #2), which
  reuses spec `004`'s existing Maven Central dependency setup.

## 1. Build with the new parser module

```bash
cargo build --workspace
```

**Expected outcome**: `crates/core` compiles cleanly with zero warnings, now exporting
`hl7pet_core::parser::{parse, CompiledPath, ParseError, ...}` alongside the existing
`hl7pet_core::scanner` module.

## 2. Run unit tests (individual grammar productions)

```bash
cargo test -p hl7pet-core --lib
```

**Expected outcome**: unit tests colocated in `parser.rs` pass — these cover each
`SEG_IDX`/`FIELD_IDX` form, filter clauses (including OR'd values and subcomponents),
the hierarchy hop, and each `ParseErrorKind` condition independently of the shared
fixtures corpus.

## 3. Run the conformance vector suite against `fixtures/vectors/path/` (US1, US3)

```bash
cargo test -p hl7pet-core --test parser_vectors
```

**Expected outcome**: every entry in `fixtures/vectors/path/valid.json` parses
successfully (`Ok(CompiledPath)`), and every entry in `invalid.json` (identified by
`expected == "INVALID"`) is rejected (`Err(ParseError)`) — 21 vectors total after this
spec's 4 additions (FR-012): 14 valid, 7 invalid. This is the single command that
proves SC-001 and SC-002 together.

## 4. Confirm zero panics across every invalid vector (US1, spec.md FR-006)

```bash
cargo test -p hl7pet-core --test parser_vectors -- invalid
```

**Expected outcome**: each invalid vector produces a specific `ParseErrorKind` and
`offset` rather than a panic (`cargo test` reports a panic as a failed test with a
backtrace, making this a strong negative-case check) — no partial `CompiledPath` is
ever observed alongside an error.

## 5. Confirm a compiled PATH is reusable without re-parsing (US2, SC-004)

```bash
cargo test -p hl7pet-core --lib -- reuse_without_reparse
```

**Expected outcome**: a unit test parses a single PATH once, then passes shared
references to the resulting `CompiledPath` to multiple simulated call sites, confirming
no API on `CompiledPath` triggers re-parsing (research.md #5). Full throughput
comparison against re-parsing repeatedly is out of scope here — deferred to spec `009`
alongside the rest of the core's baseline comparison, once spec `007`'s evaluator
exists to exercise real reuse.

## 6. Confirm the compiled representation exposes structured fields (US3)

```bash
cargo test -p hl7pet-core --lib -- compiled_shape
```

**Expected outcome**: parsing `OBX[@3.1='94500-6']-5` yields a `CompiledPath` whose
`FilterClause` has `field_num == 3`, `component == Some(1)`, `operator ==
FilterOperator::Eq`, and `values == vec!["94500-6"]` as individually readable fields —
not just confirmation that the string was syntactically valid.

## 7. Corpus validation still passes with the extended `path` family

```bash
python3 fixtures/scripts/validate_corpus.py
```

**Expected outcome**: spec `003`'s existing validation script accepts the 4 new
entries in `fixtures/vectors/path/valid.json`/`invalid.json` without any script or
schema change — they conform to the same `conformance-vector.schema.json` the
existing 17 vectors already use (plan.md Structure Decision). Grammar-production
coverage (spec `003`'s coverage report) stays at 12/12 or improves if a new vector
exercises a production combination the original 17 didn't (e.g. `FILTER` +
subcomponent together).
