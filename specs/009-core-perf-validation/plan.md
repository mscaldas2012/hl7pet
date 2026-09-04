# Implementation Plan: Core Performance Validation

**Branch**: `009-core-perf-validation` | **Date**: 2026-09-03 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/009-core-perf-validation/spec.md`

## Summary

Produce a same-corpus, feature-broad Rust-vs-Scala performance comparison,
completing the obligation specs `005`-`008` each explicitly deferred. Three
prerequisites that don't exist today, verified directly against the real
harness code and its actual output (not assumed): (1) a shared benchmark
corpus both engines read identically — spec `004`'s Scala harness currently
loads its own bundled "interim-v1" corpus, never migrated to `fixtures/`;
(2) hierarchy-mode (`->`) Scala benchmarks — spec `004`'s existing
`ExtractionBenchmarks`/`ParsingBenchmarks` only exercise `HL7StaticParser`
(flat mode); (3) a Rust benchmark harness — none exists in the workspace at
all. This spec builds all three, extends spec `004`'s harness rather than
replacing it, and produces a committed comparison artifact with an explicit
per-metric verdict against the Constitution's non-regression requirement.

## Technical Context

**Language/Version**: Rust stable (workspace, unchanged) for the new Rust
benchmark harness; Java 17+/Maven (spec `004`'s existing toolchain,
unchanged) for the extended Scala harness.

**Primary Dependencies**: No new dependency on either side. Scala: reuses
spec `004`'s existing `gov.cdc:hl7-pet_2.13:1.2.11` (Maven Central) and JMH
(`GCProfiler` already wired) — a new `HierarchyBenchmarks.java` class calls
that same dependency's `HL7ParseUtils`/`HL7HierarchyParser` hierarchy-mode
API, already present in the jar spec `004` declared; no new Maven coordinate.
Rust: a plain `cargo bench` harness (`harness = false` `[[bench]]` targets in
`crates/core/Cargo.toml`) using `std::time::Instant` sampling loops and a
small standalone counting/byte-tracking `#[global_allocator]` defined inside
the bench binary itself — not `criterion` (research.md #2) and not a change
to `hl7pet-core`'s own `[dependencies]`/`[dev-dependencies]` (the bench
binary is a separate compilation unit; it depends on `hl7pet-core` as a
library, nothing new is added to the published crate's dependency graph).

**Storage**: N/A — reads corpus files and profile JSON from disk (`fixtures/`
and a new `fixtures/messages/perf/` subdirectory, research.md #1); writes a
JSON comparison artifact to `specs/009-core-perf-validation/comparison/<run-date>/`,
mirroring spec `004`'s `baseline/<run-date>/` convention.

**Testing**: `cargo bench` (new Rust harness, manually invoked — Cargo
benches are not part of `cargo test`); `mvn compile exec:java` against the
extended `BenchmarkRunner` (spec `004`'s existing invocation, now also
covering hierarchy mode); a new comparison script (Python, matching this
repo's existing tooling-script precedent — `fixtures/scripts/validate_corpus.py`)
that loads both engines' JSON output, matches entries by corpus message
identifier and feature, and emits the verdict report. No unit-test-style
correctness assertions are added by this spec — specs `005`-`008`'s existing
suites already own correctness; this spec only measures.

**Target Platform**: Whatever machine runs the comparison (same constraint
spec `004`'s SC-002 already established: must work from a clean checkout,
Maven-resolved Scala dependency, no local Scala source). Both engines MUST
be benchmarked in the same run, on the same machine, back-to-back — spec
`004`'s own SC-003 (±10% run-to-run reproducibility) is the tolerance this
spec inherits rather than re-deriving (spec.md Edge Cases).

**Project Type**: Benchmarking/tooling addition to an existing library
workspace — not a new deliverable in `hl7pet-core`'s or `hl7pet-cli`'s
public surface (spec.md Assumptions: Constitution Principle IV does not
apply to internal benchmarking infrastructure).

**Performance Goals**: This spec's own subject matter *is* performance
measurement — there is no separate "performance goal" for the harness itself
beyond SC-002/FR-005's per-metric verdict requirement.

**Constraints**: Every benchmarked PATH form and message MUST be identical,
by name, between the two engines (spec.md FR-001) — enforced by both sides
reading the same promoted corpus/manifest (research.md #1). The Rust harness
MUST NOT introduce a new runtime dependency to `hl7pet-core` (Constitution
Principle IV protection carried over from spec `008`'s FR-014 precedent,
even though this spec's own harness isn't itself a public-API concern — the
crate it benchmarks still must stay clean). The Scala harness extension MUST
preserve spec `004`'s FR-002/FR-003 constraints (Maven-only dependency, no
vendored source, buildable from a clean checkout).

**Scale/Scope**: Benchmarks 4 features (parsing/scanning, `getValue`,
`getFirstValue`, hierarchy `->`) across the promoted perf corpus
(research.md #1: spec `004`'s existing 27 messages plus one new large
hierarchy-shaped message/profile this spec adds). Five metric categories per
feature per representative message, per the Constitution's list.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Applies? | Assessment |
|---|---|---|
| I. Path Contract Stability | Not directly | This spec measures existing, already-shipped PATH evaluation behavior (specs `001`/`006`/`007`/`008`); it introduces no new PATH syntax or semantics. |
| II. Zero-Copy & Lazy Evaluation | **Yes** | The entire point of User Story 3: confirm, at corpus scale, that allocation counts stay independent of message size (specs `005`/`007`) and that hierarchy navigation never materializes a full-message tree (spec `008` FR-003) — moving these claims from single-message unit tests to a broader, still-verified confirmation. |
| III. Explicit, Exception-Free Data Absence | **Yes** | FR-009/Edge Cases: an engine failure on a corpus message is recorded as an explicit Engine Failure Record, never silently dropped from the aggregate — the same "documented, not silently mishandled" treatment every prior spec's error-handling has followed. |
| IV. Multi-Language Interoperability | Not applicable | Benchmarking infrastructure is not a user-facing capability requiring binding parity (spec.md Assumptions). |
| V. Conformance Through Declarative Profiles & Documented Limitations | **Yes** | Hierarchy benchmarking reuses the existing declarative `fixtures/profiles/*.json` profiles (plus one new large-scale profile, research.md #1) — no hard-coded per-message-type benchmark logic. A metric category with no natural cross-engine comparison (none identified) would be documented as such, not silently compared against nothing (spec.md Edge Cases). |
| Performance & Portability Standards | **Yes** | This spec exists specifically to discharge this section's "MUST NOT regress... versus the Scala baseline" requirement with a real, executed comparison rather than continued deferral. |

**Result**: PASS. No violations requiring justification; Complexity Tracking
is intentionally empty.

## Project Structure

### Documentation (this feature)

```text
specs/009-core-perf-validation/
├── plan.md                        # This file
├── research.md                    # Phase 0 output
├── data-model.md                  # Phase 1 output
├── quickstart.md                  # Phase 1 output
├── contracts/
│   └── comparison-artifact-schema.md   # Phase 1 output
├── checklists/
│   └── requirements.md            # /speckit-specify output
├── comparison/                    # NEW — this spec's own dated output dir,
│   └── <run-date>/                # mirroring spec 004's baseline/<run-date>/
│       ├── manifest.json
│       ├── scala-results.json     # raw JMH JSON (both thrpt + sample modes)
│       ├── rust-results.json      # raw Rust harness JSON
│       └── comparison-report.json # the verdict artifact (data-model.md)
└── tasks.md                       # /speckit-tasks output (not this command)
```

### Source Code (repository root)

```text
fixtures/
├── messages/
│   └── perf/                      # NEW — promoted from spec 004's interim-v1
│       ├── corpus-manifest.json   # same shape as spec 004's, relocated
│       ├── <27 existing interim-v1 messages, unchanged content>
│       ├── large-hierarchy.hl7    # NEW — many OBR occurrences, each with
│       │                          # many OBX children, for meaningful
│       │                          # hierarchy-mode throughput/scale numbers
│       │                          # (research.md #1 — no existing fixture
│       │                          # is large enough for this)
│       └── ...
└── profiles/
    └── large-hierarchy.json       # NEW — segmentDefinition profile pairing
                                    # with large-hierarchy.hl7

specs/004-scala-baseline-bench/harness/
└── src/main/java/gov/cdc/hl7/bench/
    ├── Corpus.java                 # updated: resource path repointed at
    │                                # fixtures/messages/perf/ (research.md #1)
    ├── HierarchyBenchmarks.java    # NEW — this spec's addition: getValue
    │                                # under hierarchy mode (HL7ParseUtils
    │                                # three-arg constructor, buildHierarchy
    │                                # = true), using the same GCProfiler +
    │                                # sample/thrpt modes as the existing
    │                                # classes
    └── BenchmarkRunner.java        # updated: add HierarchyBenchmarks to the
                                     # JMH `.include(...)` pattern list

crates/core/
├── Cargo.toml                      # updated: new `[[bench]]` entries,
│                                    # `harness = false` — no new
│                                    # [dependencies]/[dev-dependencies]
└── benches/
    ├── common/                     # NEW — shared harness support (module,
    │   ├── timing.rs               # not a bench target itself): Instant-
    │   ├── alloc.rs                # sampling loop + percentile computation
    │   └── corpus.rs               # (timing.rs), a standalone counting +
    │                                # byte-tracking #[global_allocator]
    │                                # (alloc.rs, mirrors src/test_alloc.rs's
    │                                # design but lives outside hl7pet-core's
    │                                # own source), and a loader for
    │                                # fixtures/messages/perf/'s manifest
    │                                # (corpus.rs)
    ├── parsing.rs                   # NEW — scan() benchmarks
    ├── extraction.rs                # NEW — execute() (getValue/getFirstValue-
    │                                # shaped) benchmarks
    └── hierarchy.rs                 # NEW — execute_hierarchy() benchmarks

specs/009-core-perf-validation/
└── scripts/
    └── compare_results.py          # NEW — loads scala-results.json +
                                     # rust-results.json, matches entries by
                                     # corpus message id + feature, emits
                                     # comparison-report.json (data-model.md)
```

**Structure Decision**: The Scala side extends spec `004`'s existing harness
in place (`Corpus.java`, `BenchmarkRunner.java`, a new `HierarchyBenchmarks.java`
sibling to the existing benchmark classes) rather than duplicating it — same
reasoning every Rust-core spec has used for its own sibling-module choices.
The Rust side is new (`crates/core/benches/`), since no Rust benchmark
infrastructure existed before this spec; it lives inside `crates/core`
(the crate under test) using Cargo's standard `benches/` convention, kept
`harness = false` to avoid depending on the nightly-only built-in bench
harness or adding `criterion` (research.md #2) — consistent with every prior
spec's "stable Rust only" Technical Context. The promoted `fixtures/messages/perf/`
subdirectory keeps spec `003`'s existing `fixtures/messages/*.hl7` (used by
specs `001`/`002`/`005`-`008`'s correctness vectors) completely untouched;
performance and correctness fixtures are validated by different scripts and
have different requirements (representative scale and message-type variety
vs. minimal, purpose-built conformance cases), so keeping them in separate,
clearly named locations avoids `fixtures/scripts/validate_corpus.py`
accidentally treating benchmark-only messages as vector-reference candidates
or vice versa.

## Complexity Tracking

*No Constitution Check violations — this section is intentionally left
without entries.*
