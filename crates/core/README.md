# hl7pet-core

Pure Rust HL7 v2 engine (`HL7-PET-Rust-Migration-Plan.md`). Zero FFI
dependencies; `serde`/`serde_json` (pure Rust, no system/C-library build
step) are the one runtime dependency, used only for hierarchy profile
parsing (spec 008) and never exposed across this crate's public API.

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
- Lazy hierarchy navigation (`src/hierarchy.rs`, spec
  [`008-lazy-hierarchy-nav`](../../specs/008-lazy-hierarchy-nav/)): resolves
  a `CompiledPath`'s `child` (the `->` hop) against a `ScanResult` and a new
  `HierarchyProfile` (parsed from a `segmentDefinition` JSON document), via a
  bounded, per-parent-occurrence forward scan — never a full-message tree.
  Single-hop only (multi-hop chaining is deferred to a future spec); the real
  Scala engine's documented child-index bug (unfiltered by type, un-rebased)
  is fixed rather than reproduced, a documented Breaking Change (see
  `../../ROADMAP.md`).

`src/test_alloc.rs` (`cfg(test)` only) holds the crate's single shared
counting-allocator harness — only one `#[global_allocator]` can exist per
test binary, so both modules' allocation-count tests (spec 005/006 SC-004)
call it rather than each declaring their own.

## Performance validation

`benches/` (spec [`009-core-perf-validation`](../../specs/009-core-perf-validation/))
benchmarks `scan`/`execute`/`execute_hierarchy` against the shared
`fixtures/messages/perf/` corpus and compares the results against the Scala
engine's own JMH benchmarks (`specs/004-scala-baseline-bench/harness/`) on
the same corpus. A hand-written `Instant`-sampling harness, not `criterion` —
see research.md #2 — with its own standalone allocator (`benches/common/alloc.rs`,
separate from `src/test_alloc.rs`, since each `cargo bench` target is its own
binary):

```bash
cargo bench -p hl7pet-core                                  # writes rust-results-*.json
python3 specs/009-core-perf-validation/scripts/compare_results.py <run-dir>
```

See [`../../specs/009-core-perf-validation/quickstart.md`](../../specs/009-core-perf-validation/quickstart.md)
for the full walkthrough (including the paired Scala run) and
[`../../specs/009-core-perf-validation/contracts/comparison-artifact-schema.md`](../../specs/009-core-perf-validation/contracts/comparison-artifact-schema.md)
for the output format. The 2026-09-04 run found the Rust core beats Scala on
every comparable metric (throughput/latency 2-2600x, allocated bytes/op
1.2-449x — the extreme end is hierarchy navigation, where Scala rebuilds a
full parse tree per call and Rust's bounded scan doesn't) — see
`specs/009-core-perf-validation/tasks.md`'s results summary for the full
breakdown, and `ROADMAP.md` for the headline numbers.

## Build and test

```bash
cargo build --workspace
cargo test -p hl7pet-core
```

`cargo test -p hl7pet-core --lib` runs unit tests (delimiter resolution,
PATH grammar productions, index/filter resolution, allocation-count,
panic-safety, hierarchy bounded-scan/profile-parsing) for all four modules.
`cargo test -p hl7pet-core --test scanner_vectors` / `--test parser_vectors` /
`--test query_vectors` / `--test hierarchy_vectors` run every conformance
vector under `../../fixtures/vectors/{scanner,path,hierarchy}/`
respectively (`query_vectors` executes every non-hierarchy `path` vector,
`hierarchy_vectors` every single-hop `hierarchy` vector, end-to-end: scan,
parse, execute); `--test scanner_regression` confirms zero behavior change
across the pre-existing standard-delimiter corpus.

See [`../../specs/005-message-scanner/quickstart.md`](../../specs/005-message-scanner/quickstart.md) /
[`../../specs/006-path-parser/quickstart.md`](../../specs/006-path-parser/quickstart.md) /
[`../../specs/007-query-execution/quickstart.md`](../../specs/007-query-execution/quickstart.md) /
[`../../specs/008-lazy-hierarchy-nav/quickstart.md`](../../specs/008-lazy-hierarchy-nav/quickstart.md)
for the full validation walkthroughs and
[`../../specs/005-message-scanner/contracts/scanner-api.md`](../../specs/005-message-scanner/contracts/scanner-api.md) /
[`../../specs/006-path-parser/contracts/path-parser-api.md`](../../specs/006-path-parser/contracts/path-parser-api.md) /
[`../../specs/007-query-execution/contracts/query-api.md`](../../specs/007-query-execution/contracts/query-api.md) /
[`../../specs/008-lazy-hierarchy-nav/contracts/hierarchy-api.md`](../../specs/008-lazy-hierarchy-nav/contracts/hierarchy-api.md)
for the public API contracts.
