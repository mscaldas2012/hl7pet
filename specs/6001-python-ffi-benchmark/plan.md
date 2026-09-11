# Implementation Plan: Python FFI Overhead Benchmark

**Branch**: `6001-python-ffi-benchmark` | **Date**: 2026-09-11 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/6001-python-ffi-benchmark/spec.md`

## Summary

Produce a same-corpus, feature-broad Python-vs-Rust performance comparison,
completing the obligation spec `6000`'s own plan.md explicitly deferred.
Reuses spec `009`'s existing `fixtures/messages/perf/` corpus (`corpusId:
perf-v2`) and, message-for-message and PATH-for-PATH, the exact same
representative scope its three `crates/core/benches/*.rs` targets already
established — verified directly against that code, not assumed. Adds a new
Python-side harness (`crates/python/benches/`) matching the Rust harness's
sampling shape exactly (50 warmup / 500 measured, nearest-rank p50/p95), a
small opt-in change to the existing Rust harness's output path so a fresh
same-session re-run can write into this spec's own directory instead of
spec `009`'s, and a new comparison script producing a penalty-ratio report.
No allocation/memory comparison — Python has no equivalent to the Rust
harness's byte-accurate custom global allocator (spec.md FR-008).

## Technical Context

**Language/Version**: Python 3.9+ (the `hl7pet` package's own floor, spec
`6000`) for the new Python-side harness; Rust stable (workspace, unchanged)
for re-running the existing `crates/core/benches/*` targets.

**Primary Dependencies**: No new dependency on either side. Python harness
uses only the standard library (`time.perf_counter`, `json`, `platform`,
`statistics` is not needed — percentile is nearest-rank on a sorted list,
matching `common/timing.rs` exactly rather than using a library's possibly
different definition) plus the already-installed `hl7pet` package itself
(spec `6000`). Rust: the existing `crates/core/benches/*` targets, run
unmodified except for `common/output.rs`'s output-directory resolution
(research.md #1) — no new `[dependencies]`/`[dev-dependencies]` for
`hl7pet-core`.

**Storage**: N/A — reads corpus files and profile JSON from disk
(`fixtures/messages/perf/`, unchanged, research.md's spec `009` precedent);
writes a JSON comparison artifact to
`specs/6001-python-ffi-benchmark/comparison/<run-date>/`, kept separate
from and never overwriting spec `009`'s own `specs/009-core-perf-validation/
comparison/` artifacts (spec.md FR-009).

**Testing**: Manually-invoked harnesses (`cargo bench -p hl7pet-core`,
`python crates/python/benches/run_all.py`), not part of `cargo test`/
`pytest` — matching spec `009`'s own "benchmarks measure, they don't
assert correctness" precedent (specs `005`-`008`/`6000`'s existing suites
already own correctness). A new comparison script
(`specs/6001-python-ffi-benchmark/scripts/compare_penalty.py`, Python,
matching spec `009`'s own `compare_results.py` precedent) loads both
sides' JSON output and computes the penalty ratio per metric.

**Target Platform**: Whatever machine runs the comparison. Both sides MUST
be benchmarked in the same run, on the same machine, back-to-back
(spec.md FR-005/User Story 3) — spec `009`'s own inherited ±10% tolerance
(that spec's Edge Cases, originally spec `004`'s SC-003) applies here too
for operations too fast to measure precisely.

**Project Type**: Benchmarking/tooling addition to an existing library
workspace — not a new deliverable in `hl7pet-core`'s or `hl7pet-python`'s
public surface (spec.md Assumptions: Constitution Principle IV does not
apply to internal benchmarking infrastructure, matching spec `009`'s own
Assumption).

**Performance Goals**: This spec's own subject matter *is* performance
measurement — there is no separate "performance goal" for the harness
itself beyond FR-006/SC-002's per-metric penalty-ratio requirement.

**Constraints**: Every benchmarked message and PATH form MUST be identical,
by name, to what `crates/core/benches/{parsing,extraction,hierarchy}.rs`
already benchmark (spec.md FR-002) — enforced by the Python harness reading
the same `fixtures/messages/perf/corpus-manifest.json` and reusing the same
representative-selection logic those Rust targets already encode (research.md
#2). Neither harness MUST add a new runtime dependency to `hl7pet-core` or
`hl7pet-python` (Constitution Principle IV protection, carried over from
every prior spec's equivalent constraint, even though this spec's own
harness isn't itself a public-API concern).

**Scale/Scope**: Benchmarks 3 features (a parsing/scan proxy, `getValue`/
`getFirstValue` extraction, hierarchy `->` navigation) across the same perf
corpus subset the existing Rust bench targets already use — no new corpus
messages, no new PATH forms beyond what those targets already cover.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Applies? | Assessment |
|---|---|---|
| I. Path Contract Stability | Not directly | This spec measures already-shipped PATH evaluation behavior (specs `001`/`006`/`007`/`008`/`6000`); it introduces no new PATH syntax or semantics. |
| II. Zero-Copy & Lazy Evaluation | **Yes** | User Story 2's whole point: confirm whether the PyO3 boundary's "exactly one conversion per returned value" design (spec `6000` plan.md's own Constraint) holds up as a roughly constant per-call cost at corpus scale, or instead scales with message/result size — which would indicate the binding introduced extra copying the zero-copy Rust core itself doesn't do. |
| III. Explicit, Exception-Free Data Absence | **Yes** | FR-010/Edge Cases: a failure on either side for a corpus message the other handles successfully is recorded as an explicit Engine Failure Record, never silently dropped — the same treatment spec `009`'s own FR-009 already established. |
| IV. Multi-Language Interoperability | Not applicable | Benchmarking infrastructure is not a user-facing capability requiring binding parity (spec.md Assumptions, matching spec `009`'s own). |
| V. Conformance Through Declarative Profiles & Documented Limitations | **Yes** | Hierarchy benchmarking reuses the existing declarative `fixtures/profiles/large-hierarchy.json` profile (spec `009`'s addition) unchanged — no hard-coded per-message-type benchmark logic. The one metric category with no fair cross-side comparison (allocation/memory) is explicitly documented as not-comparable rather than approximated and silently presented as equivalent (FR-008). |
| Performance & Portability Standards | Not directly | This section's "MUST NOT regress... versus the Scala baseline" requirement governs `hl7pet-core` itself and was already discharged by spec `009`; it does not impose a pass/fail bar on how much slower a language binding may be than the core it wraps (spec.md Assumptions — this report is diagnostic, no fixed threshold). |

**Result**: PASS. No violations requiring justification; Complexity
Tracking is intentionally empty.

## Project Structure

### Documentation (this feature)

```text
specs/6001-python-ffi-benchmark/
├── plan.md                        # This file
├── research.md                    # Phase 0 output
├── data-model.md                  # Phase 1 output
├── quickstart.md                  # Phase 1 output
├── contracts/
│   └── comparison-artifact-schema.md   # Phase 1 output
├── checklists/
│   └── requirements.md            # /speckit-specify output (already passing)
├── comparison/                    # NEW — this spec's own dated output dir,
│   └── <run-date>/                # mirroring spec 009's comparison/<run-date>/
│       ├── rust-results-parsing.json      # fresh re-run (research.md #1)
│       ├── rust-results-extraction.json
│       ├── rust-results-hierarchy.json
│       ├── python-results.json            # this spec's new harness output
│       └── penalty-report.json            # the verdict artifact (data-model.md)
└── tasks.md                       # /speckit-tasks output (not this command)
```

### Source Code (repository root)

```text
crates/core/benches/common/
└── output.rs                       # UPDATED — run_dir() honors an optional
                                     # PERF_RUN_OUTPUT_DIR env var (falls
                                     # back to the existing spec-009-hardcoded
                                     # path when unset, so spec 009's own
                                     # workflow is byte-for-byte unchanged);
                                     # research.md #1

crates/python/benches/              # NEW — Python-side harness, mirroring
├── common/                         # crates/core/benches/'s own layout
│   ├── timing.py                   # sample()/measure_operation()-equivalent:
│   │                                # 50 warmup / 500 measured, nearest-rank
│   │                                # p50/p95, matching timing.rs exactly
│   ├── corpus.py                   # loads fixtures/messages/perf/
│   │                                # corpus-manifest.json; reimplements
│   │                                # representative_typical_per_type()/
│   │                                # unique_by_size_category() identically
│   └── output.py                   # writes python-results.json, same
│                                    # ResultRow-shaped rows minus allocation
│                                    # fields (contracts/comparison-artifact-
│                                    # schema.md, FR-008)
├── parsing.py                      # NEW — scan-proxy benchmarks (research.md #3)
├── extraction.py                   # NEW — get_value/get_first_value benchmarks,
│                                    # reusing extraction.rs's exact
│                                    # representative_field() mapping and PATH forms
├── hierarchy.py                    # NEW — get_value_hierarchy benchmarks,
│                                    # reusing hierarchy.rs's exact PATH_FORMS
└── run_all.py                      # runs parsing/extraction/hierarchy in
                                     # sequence, single entry point for
                                     # quickstart.md

specs/6001-python-ffi-benchmark/
└── scripts/
    └── compare_penalty.py          # NEW — loads python-results.json +
                                     # rust-results-*.json from the same run
                                     # directory, computes a penalty ratio
                                     # per (feature, message, metric),
                                     # emits penalty-report.json
                                     # (data-model.md) — a new script, not an
                                     # extension of spec 009's
                                     # compare_results.py (different engine
                                     # pair, different verdict shape: ratio,
                                     # not meets/beats/regresses)
```

**Structure Decision**: The Python harness lives inside `crates/python/`
(the crate/package it benchmarks) using a `benches/` directory named to
match Rust's own convention, even though Python has no built-in equivalent
— consistent with every prior spec's "harness lives next to what it
measures" choice. It is a plain script suite, not wired into `pytest`
(spec `6000`'s `tests/` already owns correctness; benchmarking is a
separate, manually-invoked concern, matching spec `009`'s Rust-side
`cargo bench` — not `cargo test` — precedent exactly). The one Rust-side
change (`common/output.rs`'s output directory becoming overridable) is
additive and backward-compatible: spec `009`'s own comparison workflow
needs zero changes to keep writing to its existing directory, since the
override only activates when this spec's own tooling sets the new env var.
A new `compare_penalty.py` is written rather than extending spec `009`'s
`compare_results.py`, because the two scripts answer genuinely different
questions with different output shapes (a three-way meets/beats/regresses
verdict against a Scala baseline vs. a two-way penalty ratio with no
pass/fail bar) — forcing them into one script would make both harder to
read for no shared benefit beyond the loose "both are comparison scripts"
similarity.

## Complexity Tracking

*No Constitution Check violations — this section is intentionally left
without entries.*
