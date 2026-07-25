# Implementation Plan: Message Scanner

**Branch**: `005-message-scanner` | **Date**: 2026-07-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/005-message-scanner/spec.md`

## Summary

Implement `hl7pet-core`'s message scanner: a single-pass, zero-copy scan over a raw
HL7 v2 message that produces segment and delimiter *byte offsets* only — no owned
copies of field/component text. The scanner reads the field separator from MSH-1 and
the four encoding characters (component/repetition/escape/subcomponent) from MSH-2 of
the message's own first segment, instead of hardcoding `|`/`^~\&`, fixing the Scala
engine's documented "MSH-1/MSH-2 must be standard" limitation (`SPEC.md` §7) for
non-standard-delimiter messages while leaving standard-delimiter output unchanged.
Malformed MSH segments (missing, truncated, or an unrecognized later segment name)
produce a specific, located `ScanError` rather than a panic or silent mis-scan. This is
the first spec to introduce the Rust workspace (`Cargo.toml`, `crates/core`) described
in `HL7-PET-Rust-Migration-Plan.md`'s Repository Layout.

## Technical Context

**Language/Version**: Rust, stable channel, edition 2021. No MSRV is pinned beyond
"whatever `stable` resolves to at CI/build time" — the constitution only requires
"stable Rust," and no prior spec or tooling in this repo has pinned a specific version
yet (research.md #1).

**Primary Dependencies**: None at runtime — `hl7pet-core` depends only on `std`. `serde`
+ `serde_json` are dev-dependencies only, used by the crate's integration tests to
deserialize `fixtures/vectors/scanner/*.json` conformance vectors (research.md #2).

**Storage**: N/A (no database; input is an in-memory `&str`, output is offset data
structures, both ephemeral).

**Testing**: `cargo test` — unit tests colocated in `scanner.rs` for delimiter
resolution and boundary-condition logic, plus an integration test
(`crates/core/tests/scanner_vectors.rs`) that loads every vector under
`fixtures/vectors/scanner/` and asserts the scanner's actual output matches each
vector's `expected` (offsets) or `expected_error` (structural error) field.

**Target Platform**: Any platform Rust `stable` supports (Linux/macOS/Windows) — the
scanner is pure computation over `&str`/byte slices with no OS-specific behavior.

**Project Type**: Library. First Rust code in the migration: this spec creates the
workspace root `Cargo.toml` and the `hl7pet-core` crate (`crates/core/`), matching the
repository layout `HL7-PET-Rust-Migration-Plan.md` already specifies.

**Performance Goals**: No numeric throughput target in this spec — full comparative
benchmarking against the Scala baseline (spec `004`) is explicitly deferred to spec
`009` (`core-perf-validation`), once the PATH parser and query executor (specs
`006`-`007`) exist to measure an end-to-end operation comparable to the Scala baseline's
measured `getValue`/`getFirstValue` calls (spec.md Assumptions). This spec's own
performance requirement (SC-004) is structural: allocation *count* for a scan is
independent of the message's field/component/repetition count, varying only with
segment count.

**Constraints**: Single pass over the message (FR-001); no heap allocation of
field/component/repetition *text* (FR-001, Constitution Principle II); structural
failures MUST surface as `Result::Err`, never a panic (Constitution Principle III);
must build on stable Rust with no vendored/forked Scala source (constitution
Performance & Portability Standards).

**Scale/Scope**: Operates on one HL7 message at a time (no batch/streaming API — that's
a Migration Plan stretch goal, not in scope here). Validated against the `fixtures/`
corpus plus this spec's own new `fixtures/vectors/scanner/` vector family (a handful of
messages: standard delimiters, non-standard delimiters, and each malformed-MSH case
from FR-006).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Applies? | Assessment |
|---|---|---|
| I. Path Contract Stability | No | The scanner exposes no PATH syntax and evaluates no PATH expressions — it is a lower-level offset index that spec `006`'s PATH parser will be built on top of. Nothing here touches grammar or evaluation semantics. |
| II. Zero-Copy & Lazy Evaluation | **Yes — this spec exists to embody it** | `ScanResult` borrows the input `&str` and stores only byte offsets (`SegmentSpan`, `DelimiterOccurrence`) — no `String`/`Vec<String>` copy of any field, component, or segment text (data-model.md). Segment name lookup is a borrowed slice computed on demand, not stored. |
| III. Explicit, Exception-Free Data Absence | **Yes** | Malformed-MSH and unrecognized-segment cases (User Story 3) are structural-precondition violations per the constitution's own carve-out, so they correctly surface as `Result<ScanResult, ScanError>` — never a panic, and never silently returned as an empty/default `ScanResult`. |
| IV. Multi-Language Interoperability | Not yet applicable | `crates/core` has no FFI boundary yet (Python/Java bindings are Migration Plan Phase 5). No binding exists to lag behind, so there is nothing to keep in parity at this stage. |
| V. Conformance Through Declarative Profiles & Documented Limitations | Partially | The scanner is not profile-driven (no `segmentDefinition` — that's the Validation module, 2000-2999) and Assumptions correctly scope segment-name recognition as a minimal syntactic check, not profile validation. It does actively *fix* a documented Known Limitation (`SPEC.md` §7 MSH-1/MSH-2) rather than silently carrying it forward, which is this principle's spirit. |
| Performance & Portability Standards | **Yes** | Phase 1 deliverables (specs `001`-`004`) are all `Complete` per `ROADMAP.md`'s Status table, satisfying the Development Workflow gate that must hold before Rust core work begins. Full baseline-comparison benchmarking is deferred to spec `009` with an explicit, already-documented reason (spec.md Assumptions) — not an unexamined omission. |

**Result**: PASS. No violations requiring justification; Complexity Tracking is
intentionally empty.

## Project Structure

### Documentation (this feature)

```text
specs/005-message-scanner/
├── plan.md                                   # This file
├── research.md                               # Phase 0 output
├── data-model.md                             # Phase 1 output
├── quickstart.md                             # Phase 1 output
├── contracts/
│   ├── scanner-api.md                        # Phase 1 output — Rust public API contract
│   └── scanner-conformance-vector.schema.json # Phase 1 output — new vector family's schema
├── checklists/
│   └── requirements.md                       # /speckit-specify output
└── tasks.md                                  # /speckit-tasks output (not this command)
```

### Source Code (repository root)

```text
Cargo.toml                              # NEW — workspace manifest (this spec creates it)
crates/
└── core/                               # NEW — hl7pet-core: pure Rust engine, zero FFI deps
    ├── Cargo.toml                      # package manifest; no runtime deps
    ├── src/
    │   ├── lib.rs                      # crate root, re-exports scanner's public types
    │   └── scanner.rs                  # this spec's deliverable (FR-001-FR-009)
    └── tests/
        └── scanner_vectors.rs          # integration test: runs fixtures/vectors/scanner/*.json

fixtures/
├── messages/                           # + new synthetic messages: non-standard delimiters,
│                                        #   each malformed-MSH case (FR-010)
├── vectors/
│   └── scanner/                        # NEW vector family (spec 003 FR-007 extensibility)
│       ├── standard-delimiters.json
│       ├── non-standard-delimiters.json
│       └── malformed-msh.json
└── schemas/
    └── scanner-conformance-vector.schema.json  # copied from contracts/ (Phase 1 output)
```

**Structure Decision**: This spec introduces the Cargo workspace at the repository root
exactly as `HL7-PET-Rust-Migration-Plan.md`'s Repository Layout diagram specifies —
`crates/core` as a standalone, dependency-free library crate, so it can be built, tested,
and (later) benchmarked without any PyO3/JNI involvement, keeping Principle II honest per
that document's own stated rationale. Test data follows spec `003`'s established
convention: new conformance vectors and their messages go directly under the shared
`fixtures/` corpus (not a per-spec copy), using a new `scanner` vector family exactly as
spec `003` FR-007 anticipated. The new vector schema is authored under this spec's own
`contracts/` during design (Phase 1) and copied into `fixtures/schemas/` during
implementation, matching how specs `001`/`002` originated the `path`/`hierarchy` schemas
now living there.

## Complexity Tracking

*No Constitution Check violations — this section is intentionally left without entries.*
