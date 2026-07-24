# Implementation Plan: Scala Baseline Benchmark Harness

**Branch**: `004-scala-baseline-bench` | **Date**: 2026-07-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/004-scala-baseline-bench/spec.md`

## Summary

Build a small, self-contained Maven/Java project that benchmarks the current Scala
HL7-PET engine — parsing throughput, extraction throughput, memory usage, allocations,
and latency (p50/p95) — by declaring the engine as a normal Maven dependency
(`gov.cdc:hl7-pet_2.13:1.2.11`, confirmed published on Maven Central, no auth required)
and driving it with JMH (Java Microbenchmark Harness). Results are captured as a
versioned, machine-readable artifact (a per-run manifest + JMH's native JSON output)
committed to this repository, so Roadmap spec `009` (`core-perf-validation`) can compare
the Rust core against it later without needing a JVM, Maven, or the Scala engine at all.

## Technical Context

**Language/Version**: Java 11 (matches the constitution's stated "Current engine runtime
target: JVM 11+"). No Scala/sbt toolchain is required to *build or run* the harness —
only to consume the already-published Scala jar as a Maven dependency.

**Primary Dependencies**: JMH (`org.openjdk.jmh:jmh-core` / `jmh-generator-annprocess`)
for benchmarking; `gov.cdc:hl7-pet_2.13:1.2.11` (the benchmark target itself) resolved
from Maven Central, which transitively pulls `org.scala-lang:scala-library:2.13.13` —
confirmed via the artifact's published POM, no manual Scala runtime wiring needed.

**Storage**: N/A (no database). Benchmark results are committed as flat JSON files
(JMH's native output format) plus a small hand-written run manifest — see
[contracts/baseline-artifact-schema.md](contracts/baseline-artifact-schema.md).

**Testing**: A small JUnit 5 smoke check (harness wiring only: corpus loads, the
`hl7-pet` dependency resolves and its static API is callable, benchmark classes are
well-formed) — not exhaustive engine-correctness coverage, which is Roadmap spec `003`'s
(regression-suite) responsibility, not this spec's.

**Target Platform**: Any machine with JDK 11+ and Maven 3.9+, with no local `hl7-pet`
(Scala) checkout — Linux/macOS dev machines and CI runners alike (FR-003).

**Project Type**: Standalone tooling module (benchmark harness), not a library exposed
to other code and not part of the eventual Rust `crates/` workspace described in
`HL7-PET-Rust-Migration-Plan.md`.

**Performance Goals**: N/A for the harness's own performance — its job is to *measure*
the Scala engine accurately, not to be fast itself. Reproducibility instead of raw speed
is the goal: repeated same-machine runs within ±10% of each other (spec SC-003).

**Constraints**: No vendored Scala source anywhere in this repository (FR-002); no
local-filesystem-path dependency reference (FR-002); harness build must succeed on a
machine that has never cloned the Scala `hl7-pet` repository (FR-003); every benchmark
input message must be synthetic/fabricated, never real patient data (FR-009).

**Scale/Scope**: An interim, hand-authored corpus of roughly 20-30 synthetic HL7 v2
messages spanning a handful of common message types and at least one large/high-repetition
and one minimal message (research.md decision 4), pending migration to Roadmap spec
`003`'s shared corpus once it exists (FR-004).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Applies? | Assessment |
|---|---|---|
| I. Path Contract Stability | No | This spec adds no PATH grammar or evaluation semantics; it calls the existing Scala engine's public static API (`HL7StaticParser.getValue`/`getFirstValue`, confirmed via `javap` against the real published jar) as an unmodified black box. |
| II. Zero-Copy & Lazy Evaluation | Indirectly | The harness does not implement engine behavior, so this principle doesn't constrain its own code — but it is the principle this spec exists to *measure a baseline for*, so later Rust-core work (spec `009`) has a concrete number to hold itself to. |
| III. Explicit, Exception-Free Data Absence | No | No new data-extraction API is introduced. |
| IV. Multi-Language Interoperability | No | This is internal JVM benchmarking tooling, not a shipped capability requiring Python/Java binding parity. |
| V. Conformance Through Declarative Profiles | No | The harness runs extraction paths against fixed synthetic messages; it introduces no new validation/conformance rule logic. |
| Performance & Portability Standards | **Yes — this spec fulfills it** | The constitution explicitly requires "a Scala baseline benchmark" as a named Phase 1 deliverable, required before Rust core implementation begins. This plan directly satisfies that gate. |

**Result**: PASS. No violations; Complexity Tracking table below is intentionally empty.

## Project Structure

### Documentation (this feature)

```text
specs/004-scala-baseline-bench/
├── plan.md                          # This file
├── research.md                      # Phase 0 output
├── data-model.md                    # Phase 1 output
├── quickstart.md                    # Phase 1 output
├── contracts/
│   └── baseline-artifact-schema.md  # Phase 1 output — the results artifact's schema
├── checklists/
│   └── requirements.md
└── tasks.md                         # /speckit-tasks output (not this command)
```

### Source Code (repository root)

```text
specs/004-scala-baseline-bench/
├── harness/                                   # Maven project — the benchmark harness itself
│   ├── pom.xml                                # declares gov.cdc:hl7-pet_2.13:1.2.11 + JMH
│   ├── src/main/java/gov/cdc/hl7/bench/
│   │   ├── ParsingBenchmarks.java             # splitFields/retrieveSegment JMH benchmarks
│   │   ├── ExtractionBenchmarks.java          # getValue/getFirstValue JMH benchmarks
│   │   ├── Corpus.java                        # loads the synthetic message corpus
│   │   └── ManifestWriter.java                # writes the run manifest (engine version, host, corpus id)
│   ├── src/main/resources/corpus/             # interim synthetic HL7 message corpus (FR-004, FR-009)
│   └── src/test/java/gov/cdc/hl7/bench/       # JUnit 5 smoke check (Testing, above)
└── baseline/                                  # committed output — one dated subdirectory per run
    └── <run-date>/
        ├── manifest.json                      # engine version, corpus id, host/env metadata
        └── jmh-results.json                   # JMH's native structured results (FR-005, FR-008)
```

**Structure Decision**: Everything for this feature — harness code and committed
baseline data alike — lives self-contained under `specs/004-scala-baseline-bench/`,
matching the convention specs `001` and `002` already established (their own
`contracts/`, `vectors/`, `messages/`, `profiles/` directories live under their own
spec folder rather than a shared top-level location). This repository has no Rust
`crates/` workspace yet and no top-level `fixtures/` directory (that's introduced by
spec `003`), so there is no existing shared location this harness would otherwise slot
into; migrating the corpus to spec `003`'s shared `fixtures/` once it exists is called
out explicitly in Edge Cases / FR-004 as expected follow-up, not part of this spec.

## Complexity Tracking

*No Constitution Check violations — this section is intentionally left without entries.*
