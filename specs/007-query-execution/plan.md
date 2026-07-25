# Implementation Plan: Query Execution

**Branch**: `007-query-execution` | **Date**: 2026-07-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/007-query-execution/spec.md`

## Summary

Implement `hl7pet-core`'s query executor: given a scanner's `ScanResult` (spec `005`,
offsets only) and a parser's `CompiledPath` (spec `006`, structured but message-blind),
navigate the scanned offsets to produce the actual value(s) a PATH addresses — the
piece neither prior spec provides. The executor resolves segment-occurrence and
field-repetition index selectors (`Numeric`, `$LAST`, `*`/omitted), evaluates filter
clauses by reusing the same field/component/subcomponent navigation used for direct
extraction, and reproduces the existing Scala engine's `getValue`/`getFirstValue`
output byte-for-byte for every standard-delimiter, non-hierarchy vector in the shared
regression suite (spec `003`, `fixtures/vectors/path/`). Hierarchy navigation
(`CompiledPath.child`, spec `008`) and escape decoding (spec `1001`) are out of scope,
per spec.md's Assumptions.

## Technical Context

**Language/Version**: Rust, stable channel, edition 2021 — same as specs `005`/`006`
(`crates/core`'s existing crate); no new MSRV decision.

**Primary Dependencies**: None at runtime — the executor lives in the existing
dependency-free `hl7pet-core` crate, as a new sibling module to `scanner.rs`/
`parser.rs`. `serde`/`serde_json` (already a dev-dependency) are reused by the new
integration test to deserialize `fixtures/vectors/path/valid.json`.

**Storage**: N/A — inputs are an in-memory `ScanResult<'a>` (borrows the message) and
a `CompiledPath<'_>` (borrows the PATH string); output borrows from the message only.

**Testing**: `cargo test` — unit tests colocated in `query.rs` for individual
resolution rules (segment index forms, field index forms, filter evaluation,
out-of-range/comparison-failure conditions), plus a new integration test
(`crates/core/tests/query_vectors.rs`) that, for every entry in
`fixtures/vectors/path/valid.json` whose `path` does not contain the hierarchy operator
(`" -> "`, out of scope per spec.md Assumptions), scans `message_ref`, parses `path`,
executes the query, and asserts the result matches `expected` exactly for the vector's
declared `method` (research.md #1).

**Target Platform**: Any platform Rust `stable` supports — pure computation over
already-scanned `&str` data, no OS-specific behavior, same as specs `005`/`006`.

**Project Type**: Library. Extends the existing `hl7pet-core` crate (`crates/core/`);
no new workspace member.

**Performance Goals**: No numeric throughput target in this spec — full comparative
benchmarking against the Scala baseline (spec `004`) remains deferred to spec `009`,
once specs `005`-`008` together produce something comparable to the Scala baseline's
measured `getValue`/`getFirstValue` calls (the precedent specs `005`/`006`'s plans
already set). This spec's own performance claim (SC-004) is structural: extraction
performs at most one pass over each matched segment occurrence's content, verified by
a dedicated allocation/iteration-counting test rather than a wall-clock benchmark.

**Constraints**: The executor MUST NOT panic for any valid `ScanResult` /
non-hierarchy `CompiledPath` combination (SC-003, Constitution Principle III); a query
call MUST produce exactly one of a result or an error, never both (FR-001); MUST NOT
copy message substrings where a borrowed slice suffices (FR-013, Constitution
Principle II); MUST match the Scala engine's `getValue`/`getFirstValue` output
byte-for-byte for every applicable regression-suite vector (FR-010).

**Scale/Scope**: Operates on one `(ScanResult, CompiledPath)` pair per call. Validated
against `fixtures/vectors/path/valid.json`'s 13 non-hierarchy vectors (spec `006`
extended the family to 14 total, 1 of which — `path-childpath-hierarchy` — is out of
scope here) plus 6 new vectors this spec adds (FR-014, research.md #6).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Applies? | Assessment |
|---|---|---|
| I. Path Contract Stability | **Yes** | This spec is where the PATH contract's *evaluation semantics* (not just syntax, which spec `006` owns) become observable for the first time — index-selector resolution order, filter-match semantics, and the getValue/getFirstValue output shape all become load-bearing here. FR-010's byte-for-byte parity requirement against the existing Scala engine is the direct enforcement mechanism; any drift would silently break the contract this principle protects. |
| II. Zero-Copy & Lazy Evaluation | **Yes** | Every extracted value is a borrowed slice of the original message (FR-013, data-model.md `QueryError`/output types); no field, component, or segment text is copied. The executor only walks the segment occurrences and delimiter positions `scan()` already located — it re-derives split points from `ScanResult`'s existing offsets rather than re-scanning the message from scratch. |
| III. Explicit, Exception-Free Data Absence | **Yes** | Verified empirically against the real Scala library before writing any Rust code (research.md #2's Verification note): an out-of-range segment or field index does **not** throw there — `getValue`/`getFirstValue` return no match, the same as a segment type absent entirely or a filter matching nothing. All of these are represented as an *empty* result (`Ok(vec![])`), never an error, confirmed both against the existing conformance vector `path-zero-values-nonexistent` (`expected: null`) and a live check of `OBX[5]-5`/`OBX-5[5]` against out-of-range indices. The one genuine error this executor introduces, `QueryError::NonNumericComparison`, corresponds to the one case the real engine does *not* handle gracefully (an uncaught `NumberFormatException` for a non-numeric ordering comparison) — Principle III reserves `Err` for exactly this kind of violated precondition, not for ordinary "no data present." A field/component/subcomponent number beyond what a matched occurrence contains is likewise absence (empty string), not an error (FR-009(e)) — same category as "genuinely empty field." |
| IV. Multi-Language Interoperability | Not yet applicable | No FFI boundary exists yet in `crates/core` (bindings are Migration Plan Phase 5) — same assessment specs `005`/`006` made. |
| V. Conformance Through Declarative Profiles & Documented Limitations | Partially | Not profile-driven (no `segmentDefinition` involved — unrelated to the 2000-2999 Validation module). Documents rather than silently mishandles a real scope boundary: hierarchy navigation (`CompiledPath.child`) is explicitly unresolved by this executor (FR-011), not silently ignored or partially evaluated. |
| Performance & Portability Standards | **Yes** | Phase 1 deliverables and specs `005`/`006` are `Complete`/`Implemented` per `ROADMAP.md`'s Status table. Full baseline-comparison benchmarking stays deferred to spec `009`, the same explicit, already-documented deferral specs `005`/`006` used. |

**Result**: PASS. No violations requiring justification; Complexity Tracking is
intentionally empty.

## Project Structure

### Documentation (this feature)

```text
specs/007-query-execution/
├── plan.md                       # This file
├── research.md                   # Phase 0 output
├── data-model.md                 # Phase 1 output
├── quickstart.md                 # Phase 1 output
├── contracts/
│   └── query-api.md              # Phase 1 output — Rust public API contract
├── checklists/
│   └── requirements.md           # /speckit-specify output
└── tasks.md                      # /speckit-tasks output (not this command)
```

### Source Code (repository root)

```text
crates/
└── core/                         # EXISTING — hl7pet-core (specs 005/006)
    ├── src/
    │   ├── lib.rs                # updated: adds `pub mod query;` + re-exports
    │   ├── scanner.rs            # unchanged (spec 005)
    │   ├── parser.rs             # unchanged (spec 006)
    │   └── query.rs              # NEW — this spec's deliverable (FR-001-FR-013)
    └── tests/
        ├── scanner_regression.rs # unchanged (spec 005)
        ├── scanner_vectors.rs    # unchanged (spec 005)
        ├── parser_vectors.rs     # unchanged (spec 006)
        └── query_vectors.rs      # NEW — integration test over fixtures/vectors/path/valid.json

fixtures/
└── vectors/
    └── path/                     # EXISTING family (spec 001, no new schema needed)
        └── valid.json            # + 6 new entries (FR-014, research.md #6):
                                   #   out-of-range segment index, out-of-range field
                                   #   index, filter matching zero occurrences, filter
                                   #   matching multiple occurrences, non-numeric value
                                   #   compared with an ordering operator, and a
                                   #   segment-only PATH (no field expression)
```

**Structure Decision**: The executor is added to the existing `hl7pet-core` crate
(`crates/core/`) as a new sibling module (`query.rs`) next to `scanner.rs`/`parser.rs`
— not a new crate — for the same reason spec `006` gave: all three are pure,
dependency-free components of one engine with no FFI boundary between them yet.
Conformance vectors extend spec `001`'s existing `path` family in place
(`fixtures/vectors/path/valid.json`) rather than a new family: `invalid.json`'s
entries never reach execution (they fail to parse, spec `006`'s concern), so only
`valid.json` needs new entries, and the existing `conformance-vector.schema.json`
already models `getValue`/`getFirstValue`/`expected` exactly as this spec's output
must shape itself — no schema change required.

## Complexity Tracking

*No Constitution Check violations — this section is intentionally left without entries.*
