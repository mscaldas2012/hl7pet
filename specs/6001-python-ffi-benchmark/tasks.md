---

description: "Task list for Python FFI Overhead Benchmark"
---

# Tasks: Python FFI Overhead Benchmark

**Input**: Design documents from `/specs/6001-python-ffi-benchmark/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/comparison-artifact-schema.md](contracts/comparison-artifact-schema.md), [quickstart.md](quickstart.md)

**Tests**: This spec's own deliverable *is* a measurement pipeline, not a
user-facing feature with its own correctness test suite — spec `6000`'s
existing `parity_check.py`/`pytest` suite already owns the Python binding's
correctness, and specs `005`-`009` already own the Rust core's. Each user
story's "test" is therefore a `quickstart.md` verification step run
against a real `penalty-report.json`, per that document — not a
`cargo test`/`pytest` task, matching spec `009`'s own precedent exactly.

**Organization**: Tasks are grouped by user story (US1/US2/US3 from
spec.md, in priority order). Because all three stories examine the *same*
`penalty-report.json`, differing only in which property of it each
story's acceptance scenarios check, the Foundational phase builds the
entire pipeline (Rust output-path override, Python harness, comparison
script) — there is no way to check "same-corpus, per-feature penalty"
(US1), "constant vs. scaling" (US2), or "measured fairly" (US3) without a
working, real `penalty-report.json` to inspect. Each story phase then
runs and verifies its own specific quickstart steps against that one
pipeline's real output.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: one small update to the existing Rust
harness (`crates/core/benches/common/output.rs`); a new Python harness
(`crates/python/benches/`, mirroring `crates/core/benches/`'s own layout);
a new comparison script
(`specs/6001-python-ffi-benchmark/scripts/compare_penalty.py`). No changes
to `hl7pet-core`'s or `hl7pet-python`'s `[dependencies]`.

---

## Phase 1: Setup

**Purpose**: Stand up the new directories this spec's harness and script live in — nothing else can be created without them.

- [X] T001 Create `crates/python/benches/common/` and `specs/6001-python-ffi-benchmark/scripts/` directories

**Checkpoint**: Directory structure ready; no code yet.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Build the full measurement pipeline — Rust output-path
override, Python harness, comparison script — so all three user stories
have a real `penalty-report.json` to check.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 [P] Update `crates/core/benches/common/output.rs`'s `run_dir()` to honor an optional `PERF_RUN_OUTPUT_DIR` environment variable — when set, use it directly as the run directory; when unset, behavior is byte-for-byte unchanged (still resolves under `specs/009-core-perf-validation/comparison/`) (research.md #1)
- [X] T003 [P] Implement `crates/python/benches/common/timing.py`: `sample()`/`measure_operation()`-equivalent functions — 50 warmup calls (discarded) then 500 measured calls timed individually via `time.perf_counter()`, sorted, nearest-rank percentile for p50/p95, throughput as `n / total_microseconds` — matching `crates/core/benches/common/timing.rs` exactly (research.md #4)
- [X] T004 [P] Implement `crates/python/benches/common/corpus.py`: loads `fixtures/messages/perf/corpus-manifest.json` and reimplements `representative_typical_per_type()`/`unique_by_size_category()`/hierarchy-eligible-entry selection identically to `crates/core/benches/common/corpus.rs` (research.md #2)
- [X] T005 [P] Implement `crates/python/benches/common/output.py`: writes `python-results.json` — `corpusId`, `pythonEngineVersion`, `hostEnvironment` (`platform.system()`/`platform.machine()`), `results` (rows with `feature`/`messageId`/`pathExpression`/`throughput`/`latencyP50`/`latencyP95` only — no allocation fields), `engineFailures` (contracts/comparison-artifact-schema.md, research.md #5)
- [X] T006 [US-independent] Implement `crates/python/benches/parsing.py`: for every representative message (T004), time `hl7pet.get_first_value(message, "MSH-1")` via T003's sampling loop as the scan-proxy measurement, recording a `feature: "parsing"` row via T005 (research.md #3) — depends on T003-T005
- [X] T007 Implement `crates/python/benches/extraction.py`: reuses `crates/core/benches/extraction.rs`'s exact `representative_field()` mapping (`PV1-3.1` for `ADT^A01`/`ADT^A08`, `OBX-5` for `ORU^R01`, `RXA-5.2` for `VXU^V04`, `OBR-4.2` for `ORM^O01`), the plain/indexed/filter-clause PATH variants, the universal `getFirstValue` path `"PID-5.1"`, and the large/minimal message cases — calling `hl7pet.get_value`/`hl7pet.get_first_value` for each, recording `feature: "getValue"`/`"getFirstValue"` rows — depends on T003-T005
- [X] T008 Implement `crates/python/benches/hierarchy.py`: reuses `crates/core/benches/hierarchy.rs`'s exact four-form `PATH_FORMS` constant against `large_hierarchy_028`, calling `hl7pet.get_value_hierarchy(message, path, profile_dict)` (profile loaded via `json.loads`), recording `feature: "hierarchy"` rows — depends on T003-T005
- [X] T009 Implement `crates/python/benches/run_all.py`: runs T006-T008 in sequence against a single `--out <dir>` argument, the harness's single entry point for `quickstart.md` step 2 — depends on T006-T008
- [X] T010 Implement `specs/6001-python-ffi-benchmark/scripts/compare_penalty.py`: loads `python-results.json` (T009's output) and the three `rust-results-*.json` from the same run directory; asserts `corpusId` matches across all four, aborting non-zero with an explanatory error otherwise (contracts/comparison-artifact-schema.md's hard precondition); for every `(feature, messageId, pathExpression)` triple both sides cover, emits 3 Comparison Result rows (`throughput`/`latencyP50`/`latencyP95`) with a computed `penaltyRatio` (data-model.md's rule: `rustValue/pythonValue` for throughput, `pythonValue/rustValue` for latency); for every triple covered by only one side, emits an Engine Failure Record instead (FR-010); writes `penalty-report.json` with `runDate`/`corpusId`/`pythonEngineVersion`/`rustEngineVersion`/`pythonHostEnvironment`/`rustHostEnvironment`/`notComparableMetrics` (always `["allocationBytesPerOp", "allocationCallCount", "memoryAllocRateBytesPerSec"]`) — depends on T002, T009

**Checkpoint**: `cargo bench -p hl7pet-core` (with `PERF_RUN_OUTPUT_DIR` set) → `run_all.py` → `compare_penalty.py` runs end-to-end and produces a real `penalty-report.json` — not yet checked against any specific user story's acceptance criteria, and not yet carrying the `scalingCheck` field (US2's own addition).

---

## Phase 3: User Story 1 - Maintainer runs one comparison and sees Python vs. Rust, feature by feature (Priority: P1) 🎯 MVP

**Goal**: Prove the comparison is genuinely same-corpus, broken out by
feature with an explicit penalty ratio, and never silently drops a
mismatch.

**Independent Test**: Run [quickstart.md](quickstart.md) steps 1-4 and 6,
and inspect the resulting `penalty-report.json` directly (spec.md US1
Independent Test).

### Implementation for User Story 1

- [X] T011 [US1] Run quickstart.md steps 1-4 for real; confirm every Comparison Result row's `messageId` resolves to an entry both `python-results.json` and the matching `rust-results-*.json` actually used, and that `parsing`/`getValue`/`getFirstValue`/`hierarchy` appear as distinct `feature` values on separate rows with their own `penaltyRatio` (spec.md US1 Acceptance Scenarios 1-2)
- [X] T012 [US1] Run quickstart.md step 6; confirm any Python-vs-Rust mismatch (a corpus message/PATH one side has no result for) produces an explicit Engine Failure Record, never a silently-dropped row (spec.md US1 Acceptance Scenario 3)
- [X] T013 [US1] Deliberately test T010's corpus-mismatch precondition: run `compare_penalty.py` against real inputs with one `rust-results-*.json` doctored to a different `corpusId`; confirm it aborts non-zero with an explanatory error rather than silently comparing (contracts/comparison-artifact-schema.md's hard precondition)

**Checkpoint**: User Story 1 fully functional and independently verified — the report is provably same-corpus, feature-broken-out, and honest about gaps.

---

## Phase 4: User Story 2 - Maintainer can tell whether the penalty is a fixed per-call cost or grows with data size (Priority: P1)

**Goal**: Prove the report distinguishes "constant per-call FFI tax" from "scales with message/result size" using real measured figures.

**Independent Test**: Run [quickstart.md](quickstart.md) step 5 against a real `penalty-report.json` (spec.md US2 Independent Test).

### Implementation for User Story 2

- [X] T014 [US2] Implement the `scalingCheck` derivation in `compare_penalty.py` (data-model.md's Scaling Comparison): pick the `"getValue"`/`"OBX-5"` rows for the representative-typical `ORU^R01` message and the `large-high-repetition` message from `results` already computed by T010, and add a `scalingCheck` object to `penalty-report.json` with both messages' `penaltyRatio` values side by side (research.md #6) — depends on T010
- [X] T015 [US2] Run quickstart.md step 5; record the actual `typicalPenaltyRatio`/`largePenaltyRatio` for `getValue`/`OBX-5`, and state explicitly (in this task's own completion note or a short results summary) whether the penalty looks roughly constant or grew substantially — not merely running the command and discarding its output (spec.md US2 Acceptance Scenario 1)

**Checkpoint**: User Stories 1 and 2 both independently functional — the report is same-corpus, feature-broken-out, and answers the "constant or scaling" question with real numbers.

---

## Phase 5: User Story 3 - Maintainer trusts the numbers because they were measured fairly (Priority: P2)

**Goal**: Confirm both sides were measured in the same session, and that allocation/memory metrics are never presented as directly comparable.

**Independent Test**: Inspect `penalty-report.json`'s metadata, and run [quickstart.md](quickstart.md) step 7 (spec.md US3 Independent Test).

### Implementation for User Story 3

- [X] T016 [US3] Confirm `penalty-report.json`'s `pythonHostEnvironment`/`rustHostEnvironment` fields and shared `runDate`/run directory tie both sides to the same benchmarking session — no cross-run/cross-host figures used for the penalty ratio (spec.md US3 Acceptance Scenario 1)
- [X] T017 [US3] Run quickstart.md step 7; confirm `notComparableMetrics` lists the three allocation/memory metric names exactly once, at the run level, and none of `allocationBytesPerOp`/`allocationCallCount`/`memoryAllocRateBytesPerSec` appears inside any individual Comparison Result row (spec.md US3 Acceptance Scenario 2, FR-008)

**Checkpoint**: All three user stories independently functional and confirmed against one real, executed comparison run.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Tie the pipeline together into one verified, documented, committed deliverable.

- [X] T018 [P] Run `cargo build --workspace`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`, and `pytest crates/python/tests/` — confirm T002's `output.rs` change and the new `crates/python/benches/` directory don't affect any existing suite
- [X] T019 [P] Update `crates/python/README.md` to mention the new `benches/` harness, linking to [quickstart.md](quickstart.md) and [contracts/comparison-artifact-schema.md](contracts/comparison-artifact-schema.md) rather than duplicating them
- [X] T020 Commit the actual comparison run's output (`specs/6001-python-ffi-benchmark/comparison/<run-date>/{rust-results-parsing.json,rust-results-extraction.json,rust-results-hierarchy.json,python-results.json,penalty-report.json}`) as this spec's own Comparison Artifact (spec.md FR-009) — a real, dated, retained run, not a placeholder
- [X] T021 Run the full [quickstart.md](quickstart.md) validation end-to-end (all 7 steps) and record the outcome
- [X] T022 Update [ROADMAP.md](../../ROADMAP.md)'s spec `6001` status row from "Draft" to "Implemented," summarizing the actual penalty numbers found (per-feature ratios, the constant-vs-scaling finding) and confirming spec `009`'s own comparison directory was left untouched

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup (T001, the directories must exist) — BLOCKS all user stories.
- **User Stories (Phase 3-5)**: All depend on Foundational completion (a real `penalty-report.json` must exist).
  - US1 (T011-T013) and US3 (T016-T017) each only *read* the pipeline's output — independently checkable in parallel once Foundational is done.
  - US2 (T014-T015) adds one new field (`scalingCheck`) to `compare_penalty.py` before it can be verified — not purely read-only like US1/US3, so it has one small implementation task first.
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### Within Phase 2 (Foundational)

- T002, T003, T004, T005 have no dependencies on each other — independent scaffolding, parallelizable.
- T006 depends on T003-T005 (needs the sampling loop, corpus loader, and output writer to exist).
- T007 depends on T003-T005, independent of T006.
- T008 depends on T003-T005, independent of T006/T007.
- T009 depends on T006-T008 (runs all three).
- T010 depends on T002 (the Rust side must be able to write into this spec's directory) and T009 (needs real `python-results.json` to parse).

### Parallel Opportunities

- Setup: single task, nothing to parallelize.
- Foundational: T002, T003, T004, T005 in parallel; T006, T007, T008 in parallel with each other once T003-T005 land.
- User Stories: US1 (T011-T013) and US3 (T016-T017) can run in parallel once Foundational's `penalty-report.json` exists (both only read it, though US3's checks are more meaningful once `scalingCheck` is present too, so doing US2 first is the more natural order even though not a hard dependency).
- Polish: T018 and T019 in parallel.

---

## Parallel Example: Foundational Phase

```bash
# Independent scaffolding, no shared files:
Task: "Update crates/core/benches/common/output.rs (PERF_RUN_OUTPUT_DIR override)"
Task: "Implement crates/python/benches/common/timing.py"
Task: "Implement crates/python/benches/common/corpus.py"
Task: "Implement crates/python/benches/common/output.py"

# Once T003-T005 land, all three benchmark targets in parallel:
Task: "Implement crates/python/benches/parsing.py"
Task: "Implement crates/python/benches/extraction.py"
Task: "Implement crates/python/benches/hierarchy.py"
```

## Parallel Example: User Stories (post-Foundational)

```bash
# US1 and US3 both just read the same penalty-report.json:
Task: "US1: verify same-corpus traceability, feature breakout, and honest failure reporting"
Task: "US3: verify same-session host metadata and allocation-metric exclusion"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational — the entire pipeline; there's no smaller working slice, since even the simplest "same corpus, per-feature penalty" claim needs both harnesses and the comparison script to exist and run.
3. Complete Phase 3: User Story 1 — proves the comparison is genuinely same-corpus, feature-broken-out, and honest about gaps.
4. **STOP and VALIDATE**: run quickstart.md steps 1-4 and 6.

### Incremental Delivery

1. Setup + Foundational → a real `penalty-report.json` exists, but hasn't been checked against any of this spec's own acceptance criteria yet, and has no `scalingCheck` field.
2. US1 → same-corpus, feature-broken-out, honest-about-gaps reporting proven (MVP).
3. US2 → the constant-vs-scaling question answered with real numbers, discharging the obligation spec `6000` deferred to this spec.
4. US3 → same-session measurement and allocation-metric honesty confirmed.
5. Polish → commit the real run as this spec's Comparison Artifact, docs, `ROADMAP.md` update summarizing the actual penalty found.

---

## Notes

- [P] tasks touch different files and have no unmet dependency within their phase.
- [Story] labels (US1/US2/US3) map directly to spec.md's prioritized user stories.
- T015/T022 are the tasks that actually answer this spec's reason for existing — "how much does the Python binding cost versus raw Rust, and does that cost scale." Whatever the real run shows must be recorded plainly (Constitution Principle V), not smoothed over or rounded to a vague "acceptable."
- T002's `PERF_RUN_OUTPUT_DIR` change is the one edit to existing, already-committed infrastructure this spec makes — confirm spec `009`'s own workflow (the env var unset) still writes to its original directory before relying on the override (T018 covers this).
