# hl7pet-core

Pure Rust HL7 v2 engine, zero FFI dependencies (`HL7-PET-Rust-Migration-Plan.md`).

Currently implements:

- The message scanner (`src/scanner.rs`, spec
  [`005-message-scanner`](../../specs/005-message-scanner/)): a single-pass,
  zero-copy scan producing segment and delimiter byte offsets only, reading
  the field separator and encoding characters from each message's own
  MSH-1/MSH-2.
- The PATH parser (`src/parser.rs`, spec
  [`006-path-parser`](../../specs/006-path-parser/)): a hand-written
  recursive-descent parser compiling a PATH string into a reusable
  `CompiledPath` per `specs/001-path-grammar-spec/contracts/path-grammar.md`,
  or a located `ParseError` — never a panic. Independent of the scanner; both
  are pure, dependency-free components of the engine.
- The query executor (`src/query.rs`, spec
  [`007-query-execution`](../../specs/007-query-execution/)): navigates a
  scanner `ScanResult`'s offsets using a parser `CompiledPath` to extract the
  value(s) a PATH addresses, resolving segment/field index selectors and
  filter clauses, reproducing the existing Scala engine's
  `getValue`/`getFirstValue` output byte-for-byte for every non-hierarchy
  PATH. An out-of-range segment/field index is not an error (verified against
  the real engine — it returns no match); the one genuine error is a
  non-numeric operand compared with an ordering filter operator.

`src/test_alloc.rs` (`cfg(test)` only) holds the crate's single shared
counting-allocator harness — only one `#[global_allocator]` can exist per
test binary, so both modules' allocation-count tests (spec 005/006 SC-004)
call it rather than each declaring their own.

## Build and test

```bash
cargo build --workspace
cargo test -p hl7pet-core
```

`cargo test -p hl7pet-core --lib` runs unit tests (delimiter resolution,
PATH grammar productions, index/filter resolution, allocation-count,
panic-safety) for all three modules.
`cargo test -p hl7pet-core --test scanner_vectors` / `--test parser_vectors` /
`--test query_vectors` run every conformance vector under
`../../fixtures/vectors/scanner/` and `../../fixtures/vectors/path/`
respectively (`query_vectors` executes every non-hierarchy `path` vector
end-to-end: scan, parse, execute); `--test scanner_regression` confirms zero
behavior change across the pre-existing standard-delimiter corpus.

See [`../../specs/005-message-scanner/quickstart.md`](../../specs/005-message-scanner/quickstart.md) /
[`../../specs/006-path-parser/quickstart.md`](../../specs/006-path-parser/quickstart.md) /
[`../../specs/007-query-execution/quickstart.md`](../../specs/007-query-execution/quickstart.md)
for the full validation walkthroughs and
[`../../specs/005-message-scanner/contracts/scanner-api.md`](../../specs/005-message-scanner/contracts/scanner-api.md) /
[`../../specs/006-path-parser/contracts/path-parser-api.md`](../../specs/006-path-parser/contracts/path-parser-api.md) /
[`../../specs/007-query-execution/contracts/query-api.md`](../../specs/007-query-execution/contracts/query-api.md)
for the public API contracts.
