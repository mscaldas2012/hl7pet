# HL7-PET Roadmap

This is a hand-maintained index of module → spec-number ranges. Spec Kit itself has
no concept of "module" — every `/speckit-specify` call creates a flat, globally
numbered folder under `specs/`. This file is the convention we use to keep specs
grouped by module anyway, since specs from different modules will otherwise
interleave in a single ascending sequence.

Nothing in `.specify/` reads this file automatically — it's for humans (and for
Claude) to stay oriented, and it should be updated whenever a module range fills
up or a new module is added.

## How to use this

When starting a new spec for a module, pass both flags to
`.specify/scripts/bash/create-new-feature.sh` (or let `/speckit-specify` do it,
telling it which module/number to use):

```bash
.specify/scripts/bash/create-new-feature.sh --number 1000 --short-name field-extraction "..."
```

This produces `specs/1000-field-extraction/`. Use the next free number within the
module's range; update the "Next free" column below after each spec is created.

## Module ranges

Ranges are 1000-wide, with Rust Core first since it's the foundation everything
else in the migration builds on.

| Range       | Module                        | Maps to (current Scala)                                   | Next free |
|-------------|--------------------------------|-------------------------------------------------------------|-----------|
| 0-999       | Rust Core / Engine Migration    | Migration Plan Phases 1-3, 6 (scanner, PATH parser, hierarchy, perf) | 10        |
| 1000-1999   | Parsing & Extraction            | `HL7ParseUtils`, `HL7StaticParser`, PATH grammar             | 1002      |
| 2000-2999   | Validation                      | `StructureValidator`, `RulesValidator`, `BatchValidator`      | 2000      |
| 3000-3999   | De-identification                | `DeIdentifier`                                                | 3000      |
| 4000-4999   | Transformation                   | (new — no current Scala equivalent)                          | 4000      |
| 5000-5999   | File & Batch Utilities           | `HL7FileUtils`                                                | 5000      |
| 6000-6999   | Language Bindings                | Migration Plan Phase 4-5 (Arrow, PyO3, JNI/JNA)               | 6000      |
| 9000-9999   | Cross-cutting / Infra            | tooling, CI, docs, benchmarking harness                       | 9000      |

7000-8999 is intentionally unassigned buffer for a future module.

Specs within a module are numbered sequentially by ones (1, 2, 3, ...), not
spaced out — inserting a spec later just takes the next free number rather
than a reserved gap.

Note: the 0-999 range starts numbering at 1, not 0, so the first spec reads
`001-...` rather than `000-...`.

### 0-999: Rust Core / Engine Migration -- planned specs

The Scala library referenced throughout lives at
[github.com/mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet)
(see `HL7-PET-Rust-Migration-Plan.md` > Repository Layout). This repo has no
hard dependency on it (no submodule, no build-time fetch) -- any fixtures or
benchmark numbers needed from it are downloaded once and committed here as
static data.

| #   | Short name              | Migration plan phase | Scope |
|-----|--------------------------|-----------------------|-------|
| 001 | `path-grammar-spec`      | Phase 1               | Formal PATH grammar specification + conformance test vectors, derived from `SPEC.md` section 3.1 |
| 002 | `hierarchy-semantics`    | Phase 1               | Document hierarchy-mode semantics (`segmentDefinition`-driven parent→child navigation, cardinality, `->` operator) |
| 003 | `regression-suite`       | Phase 1               | Shared golden-message corpus (`fixtures/`) + expected outputs exported from the Scala baseline |
| 004 | `scala-baseline-bench`   | Phase 1               | Benchmark harness capturing the Scala engine's current throughput/memory/allocation/latency numbers, to compare against later |
| 005 | `message-scanner`        | Phase 2               | Single-pass segment/delimiter scanner, offsets only, no field/component allocations. MUST read the field separator from MSH-1 and the encoding characters (component/repetition/escape/subcomponent) from MSH-2 rather than hardcoding `\|` and `^~\&` -- fixes the Scala engine's "MSH-1/MSH-2 must be standard" limitation (`SPEC.md` §7). For messages using standard delimiters (the common case), output is unchanged; this only changes behavior for non-standard-delimiter messages, which previously mis-parsed or errored. Needs its own conformance vectors: at least one message with non-default delimiters, plus malformed-MSH error cases. |
| 006 | `path-parser`            | Phase 2               | Hand-written PATH parser/state machine replacing regex; compiles paths into reusable query objects |
| 007 | `query-execution`        | Phase 2               | Navigate offsets to extract values; validated against the `003` regression suite |
| 008 | `lazy-hierarchy-nav`     | Phase 3               | Contextual parent→child navigation without building a full tree |
| 009 | `core-perf-validation`   | Phase 6               | Benchmark `005`-`008` against the `004` Scala baseline; confirm zero-copy/lazy targets are met |

Next spec in this module starts at **010**.

### 1000-1999: Parsing & Extraction -- planned specs

| #    | Short name              | Scope |
|------|--------------------------|-------|
| 1000 | `located-extraction-api` | New capability (no current Scala equivalent): a location-aware extraction API returning each value paired with its 1-based source line number, e.g. `getValueLocated(path)`. MUST be additive -- existing `getValue`/`getFirstValue` stay unchanged, this is a new method alongside them, per the Backward-Compatible-Additions convention below. Depends on offset data already tracked internally by the message scanner (spec `005`) and query executor (spec `007`). Conformance vectors for this can reuse the line-number metadata collected in spec `001` (FR-008) rather than re-deriving it. |
| 1001 | `escape-sequence-decoding` | Fixes the Scala engine's "no escaped character support" limitation (`SPEC.md` §7: `\H\`, `\N\`, `\F\`, `\S\`, `\R\`, `\E\`, `\X..\` hex, custom `\Zxxx\`). Unlike `1000`, this is a **deliberate exception** to Backward-Compatible Additions: rather than a parallel method, `getValue`/`getFirstValue` gain a `decodeEscapes` parameter defaulting to `true` (decode). Passing `false` explicitly disables decoding and reproduces today's raw-passthrough behavior exactly -- so the opt-out always exists, it just isn't what a caller gets by not specifying anything. The default still changes existing callers' output for any value containing an escape sequence, so per Constitution Principle I this still requires a MAJOR version bump and a documented migration guide (which should tell callers who need the old behavior unconditionally to pass `decodeEscapes=false`) -- this spec's plan.md MUST include both. Note for the Java binding (spec `6000`-range): Java has no native default-parameter syntax, so this likely needs an explicit overload (`getValue(path)` vs `getValue(path, decodeEscapes)`) rather than a single method with a default. |

Next spec in this module starts at **1002**.

## Conventions

### Backward-Compatible Additions

When a new capability has no equivalent in the current Scala API (or would
require changing an existing method's return type), it MUST be added as a
new method alongside the existing ones, not by changing an existing
method's signature or return shape. Existing callers of `getValue`,
`getFirstValue`, etc. must never be forced to change to pick up an
unrelated improvement. This mirrors the single-call-vs-batched API decision
already made for the Python/Java bindings in
`HL7-PET-Rust-Migration-Plan.md` Phase 5.

Exception: when the current behavior is being treated as a bug fix rather
than a missing capability (e.g. spec `1001`, escape-sequence decoding), a
breaking change to an existing method's default behavior is acceptable in
place of a parallel method -- but only with an explicit, documented
decision to that effect (not by default), a parameter that lets a caller
opt back into the old behavior explicitly, and the MAJOR version bump and
migration guide Constitution Principle I requires for breaking changes.

### Documented Breaking Changes

Running registry of every deliberate breaking change decided so far, each requiring
a MAJOR version bump and migration guide per Constitution Principle I. Kept in one
place so `/speckit-plan` runs for later specs can check this list rather than
rediscover each decision independently.

| Spec | Change | Why it's breaking |
|---|---|---|
| `001` | `SEG` requires an alphabetic first character (was `[A-Z0-9]{3}`, any 3 chars) | A previously syntax-valid segment name like `999` is no longer valid |
| `001` | `SEG_IDX`/`FIELD_IDX` reject non-numeric/non-`$LAST`/non-`*` content at parse time (was accepted by regex, then crashed at evaluation with an uncaught exception) | Behavior changes from "crash at runtime" to "rejected at parse time" for inputs like `PID[ABC]-1` |
| `001` | `OPERATOR` restricted to `=`, `!=`, `>`, `>=`, `<`, `<=` (was any 1-2 chars from `[!><=]`, e.g. `==` matched syntactically then crashed) | Same shift from runtime crash to parse-time rejection |
| `1001` | `getValue`/`getFirstValue` decode escape sequences by default (`decodeEscapes: Boolean = true`) | Changes returned value content for any message containing an escape sequence, unless the caller passes `decodeEscapes=false` |

## Status

| Spec # | Module    | Short name          | Status |
|--------|-----------|----------------------|--------|
| 001    | Rust Core | `path-grammar-spec`  | Complete — grammar finalized (`contracts/path-grammar.md`), 17 conformance vectors authored and verified against the real Scala library with zero discrepancies, scope boundary vs. spec `002` cross-referenced. Ready for spec `006` (Rust PATH parser) to build against. |
| 002    | Rust Core | `hierarchy-semantics` | Complete — `contracts/hierarchy-semantics.md` documents `segmentDefinition`-driven nearest-enclosing-ancestor tree construction, single-hop `->` evaluation, and the `None`-vs-`Some(empty)` result-shape distinction, all verified against the real Scala source (a local checkout, not just `SPEC.md` prose) and against 9/9 verifiable conformance vectors (`hier-001`–`hier-009`) with zero discrepancies. Also found and documented a real, apparently-untested engine limitation: numeric child indices in `->` (e.g. `OBX[2]`) are unfiltered by type and un-rebased (no `-1` adjustment), confirmed exactly against the real library. Both deferred decisions resolved: profile required but no eager full-tree build (FR-004); multi-level chaining recommended as a Backward-Compatible Addition to spec `001`'s `CHILD_PATH`, gated on a falsifiable O(message size) performance claim (FR-005) — spec `001`'s grammar doc cross-references this. Ready for spec `008` (Rust lazy hierarchy navigation) to build against. |
| 003    | Rust Core | `regression-suite`   | Complete — `fixtures/` corpus created at the repo root (`messages/` 7 files, `profiles/` 2 files, `vectors/path/` 17 vectors, `vectors/hierarchy/` 10 vectors, `schemas/` both vector schemas), consolidated byte-for-byte from specs `001`/`002` with originals left untouched. `fixtures/scripts/validate_corpus.py` (schema conformance, reference resolution, corpus-wide id uniqueness, coverage report) wired into `.github/workflows/fixtures-validation.yml` (this repo's first CI workflow), scoped to `fixtures/**`. Coverage confirmed at 12/12 grammar productions and 10/10 hierarchy semantic rules, zero gaps. Ready for spec `005` (message scanner) and later Rust core work to consume `fixtures/` directly. |
| 004    | Rust Core | `scala-baseline-bench` | Complete — Maven/Java harness (JMH-driven, `specs/004-scala-baseline-bench/harness/`) benchmarking `gov.cdc:hl7-pet_2.13:1.2.11` (Maven Central, no auth, no vendored source — verified via `verify-no-vendored-source.sh`) for parsing (`retrieveFirstSegmentOf`/`retrieveMultipleSegments`) and extraction (`getValue`/`getFirstValue`) throughput, latency (p50/p95), memory, and allocations across a 27-message interim synthetic corpus (`interim-v1`). First baseline committed at `baseline/2026-07-24/` (`manifest.json` + JMH's native `jmh-results.json`), consumable via the dependency-free `baseline/read-baseline-example.py` with no JVM/Maven/Scala needed. Reproducibility (SC-003) verified at `forks(3)`: 16/20 benchmark/message-type combinations within ±10% across repeated runs (avg. deviation 5.25%); the 4 outliers are consistently the fastest calls in the suite (`retrieveFirstSegmentOf`, 2-4µs/op, up to ~18% swing) — documented in `baseline/README.md` as expected JVM-microbenchmark noise at that time scale, not a harness defect. Built with its own interim synthetic corpus (predates spec `003` landing); migrating to the shared `fixtures/` corpus is a natural follow-up, not done in this PR. Ready for spec `009` (core-perf-validation) to compare the Rust core against. |
| 005    | Rust Core | `message-scanner` | Implemented — first Rust engine code in the migration: workspace `Cargo.toml` + `crates/core` (`hl7pet-core`, zero runtime deps) with `scan()` in `crates/core/src/scanner.rs`. Single-pass, two-allocation-per-scan (`Vec<SegmentSpan>` + `Vec<DelimiterOccurrence>`) design reads the field separator from MSH-1 and all four encoding characters from MSH-2 dynamically, fixing the Scala engine's fixed-MSH limitation (`SPEC.md` §7) for non-standard-delimiter messages while a dedicated regression test confirms zero behavior change across the full pre-existing standard-delimiter corpus. Malformed MSH/segment-name conditions return a located `ScanError` (`MissingMsh`/`TruncatedMsh`/`UnrecognizedSegment`), never a panic — verified by a dedicated panic-safety test. Allocation-count independence from field/component/repetition count (SC-004) verified by a counting-allocator unit test. New `fixtures/vectors/scanner/` vector family (8 vectors, `scan-001`-`scan-008`) registered in `fixtures/scripts/validate_corpus.py`; full corpus validation passes (35/35 vectors, `scanner` correctly reported as a family with no coverage dimension, per spec `003` FR-007). All 8 `quickstart.md` steps and `cargo clippy --workspace --all-targets` pass clean. Ready for spec `006` (PATH parser) to build against `hl7pet_core::scan`'s offset output (contracts/scanner-api.md). |
| 006    | Rust Core | `path-parser` | Implemented — hand-written recursive-descent parser (`crates/core/src/parser.rs`, no `nom`/`pest`, zero new runtime deps) compiling a PATH string into a reusable `CompiledPath`, or a located `ParseError`, strictly per spec `001`'s grammar (`contracts/path-grammar.md`). All 6 syntax-tightening rules spec `001` documented (alpha-first `SEG`, parse-time `SEG_IDX`/`FIELD_IDX` rejection, six-token `OPERATOR` set) are now enforced in code for the first time, not just documented. `fixtures/vectors/path/` extended from 17 to 21 vectors (14 valid, 7 invalid — 3 new valid: OR'd filter values, filter subcomponent, whitespace-tolerant operator; 1 new invalid: multi-hop hierarchy rejection), all verified against the real Scala library (`gov.cdc:hl7-pet_2.13:1.2.11`, spec `004`'s Maven Central dependency) per research.md #2 — with one refinement discovered in the process: the real Scala engine returns `None` for the whitespace-operator form, confirming grammar Note #4 is a genuinely new capability this parser adds (not pre-existing behavior), so that one vector's expected value is sourced from its verified no-whitespace equivalent rather than re-running an unsupported string against Scala (documented in `tasks.md` Notes). Never panics (dedicated panic-safety test over pathological/non-ASCII input); a parse call always returns exactly a `CompiledPath` or a `ParseError`, never both. `CompiledPath` reuse-without-reparse and parse-purity both proven by dedicated allocation-counting and determinism tests (SC-004/FR-009). Full corpus validation passes (39/39 vectors, 12/12 grammar-production coverage); `cargo clippy --workspace --all-targets` and all 7 `quickstart.md` steps pass clean. Multi-hop `->` chaining explicitly rejected (deferred to spec `008` per spec `001`'s grammar Non-Goals). Ready for spec `007` (query execution) to build against `hl7pet_core::parse`'s output (`contracts/path-parser-api.md`). |
| 007    | Rust Core | `query-execution` | Implemented — query executor (`crates/core/src/query.rs`, new sibling module to specs `005`/`006`'s `scanner.rs`/`parser.rs`, no new Cargo dependency) navigating spec `005`'s scanned offsets using spec `006`'s compiled PATHs: resolves segment/field index selectors (`Numeric`/`$LAST`/`*`/omitted) and filter clauses (all six operators, OR'd values, subcomponent targets) sharing one `extract_subvalue`-style navigation path for both direct extraction and filter evaluation. A significant finding from verifying against the real Scala library (`gov.cdc:hl7-pet_2.13:1.2.11`, spec `004`'s dependency) before writing any Rust code: an out-of-range segment or field index is **not** an error there (`getValue`/`getFirstValue` return no match, confirmed live for both `OBX[5]-5` and `OBX-5[5]`) — this corrected an earlier planning draft that had assumed otherwise from a literal reading of the Constitution's Principle III example, so `QueryError` ended up with a single variant, `NonNumericComparison` (the one case the real engine does *not* handle gracefully — an uncaught `NumberFormatException` for a non-numeric ordering comparison, which this executor surfaces as a typed error instead of reproducing the crash). A second real bug caught by the same verification-driven test suite during implementation: a matched segment occurrence whose field-repetition index is out of range must be dropped from the result entirely (`Ok(vec![])`), not kept as an occurrence with an empty inner list — the real engine collapses fully to no match. `fixtures/vectors/path/valid.json` extended from 14 to 20 entries (6 new, all Scala-verified live: `path-segment-only`, `path-segidx-out-of-range`, `path-fieldidx-out-of-range`, `path-filter-no-match`, `path-filter-multi-match`, `path-filter-nonnumeric-ordering`); full corpus validation passes (45/45 vectors, 12/12 `path` grammar-production coverage). 22 new unit tests plus the `query_vectors` integration test (19 non-hierarchy vectors executed end-to-end: scan → parse → execute) all pass alongside the full pre-existing suite (43 unit tests crate-wide); `cargo clippy --workspace --all-targets` clean; all 7 `quickstart.md` steps pass. Hierarchy navigation (`CompiledPath.child`) and escape decoding are explicitly out of scope, deferred to specs `008`/`1001`. Ready for spec `008` (lazy hierarchy navigation) to build against `execute()`'s output for non-hierarchy sub-paths. |

## Notes

- If a module's range fills up (unlikely at 1000 slots), extend it into the
  7000-8999 buffer rather than renumbering existing specs.
- Cross-module specs (e.g. a change that touches both Validation and
  De-identification) belong to whichever module owns the primary change; note the
  secondary module in that spec's own `spec.md`.
- This file complements, but does not replace, `.specify/memory/constitution.md`
  (project-wide non-negotiable principles) and `HL7-PET-Rust-Migration-Plan.md`
  (the Rust core's phased build-out, which the 0-999 and 6000-6999 ranges above
  track).
