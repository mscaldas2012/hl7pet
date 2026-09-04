---

description: "Task list for Core Performance Validation"
---

# Tasks: Core Performance Validation

**Input**: Design documents from `/specs/009-core-perf-validation/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/comparison-artifact-schema.md](contracts/comparison-artifact-schema.md), [quickstart.md](quickstart.md)

**Tests**: This spec's own deliverable *is* a measurement pipeline, not a
user-facing feature with its own correctness test suite — specs `005`-`008`'s
existing test suites already own correctness of the engines being measured. Each
user story's "test" is therefore a `quickstart.md` verification step run against
a real comparison output, per that document — not a `cargo test`/unit-test task.

**Organization**: Tasks are grouped by user story (US1/US2/US3 from spec.md, in
priority order). Because all three stories examine the *same*
`comparison-report.json`, differing only in which property of it each story's
acceptance scenarios check, the Foundational phase builds the entire pipeline
(promoted corpus, extended Scala harness, new Rust harness, comparison script) —
there is no way to check "same corpus" (US1), "explicit verdict" (US2), or
"allocation independence at scale" (US3) without a working, real
`comparison-report.json` to inspect. Each story phase then runs and verifies its
own specific quickstart steps against that one pipeline's real output.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: the Scala side extends spec `004`'s existing
harness in place (`specs/004-scala-baseline-bench/harness/`); the Rust side is new
(`crates/core/benches/`, `harness = false` `[[bench]]` targets, no new
`[dependencies]`/`[dev-dependencies]`); the corpus is promoted to
`fixtures/messages/perf/` (research.md #1); the comparison script is new
(`specs/009-core-perf-validation/scripts/compare_results.py`).

---

## Phase 1: Setup (Corpus Promotion)

**Purpose**: Stand up the one shared corpus both engines will read — nothing else
in this spec can produce a same-corpus comparison without it existing first.

- [X] T001 Create `fixtures/messages/perf/` and copy spec `004`'s existing 27 interim-v1 corpus messages (currently at `specs/004-scala-baseline-bench/harness/src/main/resources/corpus/*.hl7`) into it unchanged, byte-for-byte (research.md #1) — this is a relocation of existing synthetic data, not new content.
- [X] T002 Copy `specs/004-scala-baseline-bench/harness/src/main/resources/corpus/corpus-manifest.json` to `fixtures/messages/perf/corpus-manifest.json`, changing only `corpusId` from `"interim-v1"` to `"perf-v2"` (data-model.md) — entries unchanged otherwise.
- [X] T003 [P] Author `fixtures/messages/perf/large-hierarchy.hl7`: a synthetic (FR-010) message with several `OBR` occurrences, each with multiple `OBX` children (large enough — at minimum dozens of segments — that a bounded-scan implementation and an accidentally-unbounded one would produce visibly different allocation/timing behavior, research.md #1).
- [X] T004 [P] Author `fixtures/profiles/large-hierarchy.json`: a `segmentDefinition` profile matching T003's message shape (same JSON shape as `fixtures/profiles/{basic-two-level,deep-nested}.json`).
- [X] T005 Add a new entry to `fixtures/messages/perf/corpus-manifest.json` for `large-hierarchy.hl7`: `sizeCategory: "large-high-repetition"`, a new hierarchy-specific `messageType`, referencing T003's file (and, in a manifest field this task adds if not already present in the existing shape, a `profileRef` pointing at T004's profile — needed only for hierarchy-eligible entries).

**Checkpoint**: `fixtures/messages/perf/` exists as a complete, self-contained corpus (28 messages + manifest); nothing yet reads it.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Build the full measurement pipeline — extended Scala harness, new
Rust harness, comparison script — so all three user stories have a real
`comparison-report.json` to check.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T006 Update `specs/004-scala-baseline-bench/harness/pom.xml` to add `fixtures/messages/perf/` (relative path from the harness module) as an additional Maven resource directory, so its contents are available on the classpath the same way the module's own `src/main/resources/corpus/` currently is — without copying/duplicating the files (research.md #1's "relocate, not duplicate").
- [X] T007 Update `Corpus.java` (`specs/004-scala-baseline-bench/harness/src/main/java/gov/cdc/hl7/bench/Corpus.java`) to load `corpus-manifest.json` from its new classpath location (T006) instead of the module's own bundled copy; delete the now-unused `src/main/resources/corpus/` directory once T001/T002 have relocated its contents.
- [X] T008 [P] Create `HierarchyBenchmarks.java` (`specs/004-scala-baseline-bench/harness/src/main/java/gov/cdc/hl7/bench/HierarchyBenchmarks.java`), matching `ExtractionBenchmarks.java`'s existing shape: `@State` classes loading `large-hierarchy.hl7` + its profile (T003-T005) via `HL7ParseUtils`'s three-argument hierarchy-mode constructor (`buildHierarchy = true`, per spec `002`'s traced `HL7HierarchyParser.scala` source), and `@Benchmark` methods for `getValue` against at least: a plain single-hop `->` PATH, and one indexed/filtered hierarchy PATH (spec.md FR-004) — `@BenchmarkMode({Mode.Throughput, Mode.SampleTime})`, matching the existing classes.
- [X] T009 Update `BenchmarkRunner.java`'s `OptionsBuilder.include(...)` list to add `"gov\\.cdc\\.hl7\\.bench\\.HierarchyBenchmarks\\..*"` alongside the existing two patterns (research.md #4).
- [X] T010 [P] Create `crates/core/benches/common/alloc.rs`: a standalone `#[global_allocator]` (not touching `src/test_alloc.rs`, which stays `cfg(test)`-only and unchanged) tracking, per measured operation: allocation call count, and total bytes allocated (`Layout::size()` summed at `alloc`/`realloc`) — research.md #3.
- [X] T011 [P] Create `crates/core/benches/common/timing.rs`: a warmup-then-measure sampling loop — discard a warmup phase, then run N iterations each timed via `std::time::Instant`, collecting `Vec<Duration>`; expose throughput (`N / sum(durations)`) and percentile helpers (sort samples, index for p50/p95) — research.md #2.
- [X] T012 [P] Create `crates/core/benches/common/corpus.rs`: a loader reading `fixtures/messages/perf/corpus-manifest.json` (workspace-root-relative, mirroring `query_vectors.rs`'s `fixtures_root()` pattern) plus each message's `.hl7` content and, for hierarchy-eligible entries, the paired profile JSON.
- [X] T013 Update `crates/core/Cargo.toml`: add three `[[bench]]` entries (`parsing`, `extraction`, `hierarchy`), each `harness = false` — no new `[dependencies]`/`[dev-dependencies]` (plan.md Constraints).
- [X] T014 Implement `crates/core/benches/parsing.rs`: for every corpus message (T012), run `hl7pet_core::scan` through `timing.rs`'s sampling loop and `alloc.rs`'s allocator, recording throughput/p50/p95/bytes-per-op/alloc-rate/call-count.
- [X] T015 Implement `crates/core/benches/extraction.rs`: for every corpus message, run `hl7pet_core::execute` (via `scan` + `parse`) for representative `getValue`/`getFirstValue`-shaped PATH expressions covering, per spec.md FR-004: a plain field, an indexed segment/field selector, and a filter clause.
- [X] T016 Implement `crates/core/benches/hierarchy.rs`: for every hierarchy-eligible corpus message (T005's `large-hierarchy` entry, plus the existing `basic-hierarchy.hl7`/`complex-hierarchy.hl7` fixtures reused directly since they're already synthetic and profile-paired), run `hl7pet_core::execute_hierarchy` for representative hierarchy PATH expressions.
- [X] T017 Wire T014-T016's three bench targets to each append their results into one shared `rust-results.json` under `specs/009-core-perf-validation/comparison/<run-date>/` (contracts/comparison-artifact-schema.md's shape) — the run-date directory is created if absent; concurrent bench-target writes are coordinated (e.g. each target writes its own temp file, a `cargo bench` post-step or the final target invoked merges them) so no result is lost to a race.
- [X] T018 Implement `specs/009-core-perf-validation/scripts/compare_results.py` (research.md #5): load `scala-results.json` (JMH native format) and `rust-results.json` (T017); assert `corpusId` matches between them, aborting non-zero with an explanatory error otherwise (contracts/comparison-artifact-schema.md's hard precondition); for every `(feature, messageId)` pair both cover, emit 6 Comparison Result rows (throughput, latencyP50, latencyP95, allocationBytesPerOp, memoryAllocRateBytesPerSec, allocationCallCount) with a computed verdict (data-model.md's rule: beats/meets/regresses using spec `004`'s existing ±10% tolerance, or `not-comparable` for `allocationCallCount`); for every pair covered by only one input, emit an Engine Failure Record instead (FR-009); write `comparison-report.json`.

**Checkpoint**: `mvn compile exec:java` (extended `BenchmarkRunner`) → `cargo bench -p hl7pet-core` → `compare_results.py` runs end-to-end and produces a real `comparison-report.json` — not yet checked against any specific user story's acceptance criteria.

---

## Phase 3: User Story 1 - Maintainer runs one comparison and sees Rust vs. Scala, feature by feature (Priority: P1) 🎯 MVP

**Goal**: Prove the comparison is genuinely same-corpus and broken out by
feature, not two independently-sized runs collapsed into one aggregate.

**Independent Test**: Run [quickstart.md](quickstart.md) steps 1-3 and inspect
the resulting `comparison-report.json` directly (spec.md US1 Independent Test).

### Implementation for User Story 1

- [X] T019 [US1] Run quickstart.md steps 1-3 for real; confirm every Comparison Result row's `messageId` resolves to an entry both `scala-results.json` and `rust-results.json` actually used (spec.md US1 Acceptance Scenario 1).
- [X] T020 [US1] Deliberately test T018's corpus-mismatch precondition: run `compare_results.py` against one real results file and one doctored to a different `corpusId`; confirm it aborts non-zero with an explanatory error rather than silently comparing (spec.md US1 Acceptance Scenario 2, contracts/comparison-artifact-schema.md).
- [X] T021 [US1] Confirm `comparison-report.json` labels `getValue`, `getFirstValue`, and `hierarchy` as distinct `feature` values on separate rows, not merged into one "extraction" figure (spec.md US1 Acceptance Scenario 3).

**Checkpoint**: User Story 1 fully functional and independently verified — the
report is provably same-corpus and feature-broken-out.

---

## Phase 4: User Story 2 - Confirm the Rust core does not regress performance versus Scala (Priority: P1)

**Goal**: Prove the report yields an explicit, actionable pass/regression
verdict against the Constitution's non-regression requirement.

**Independent Test**: Run [quickstart.md](quickstart.md) steps 4 and 6 against a
real `comparison-report.json` (spec.md US2 Independent Test).

### Implementation for User Story 2

- [X] T022 [US2] Run quickstart.md step 4; confirm zero Comparison Result rows are missing a `verdict` field (spec.md SC-002, US2 Acceptance Scenario 1).
- [X] T023 [US2] Run quickstart.md step 6; record the actual regression verdict this run produces — zero regressions, or an explicit list of which `feature`/`messageId`/`metric` combinations regress and by how much (spec.md US2 Acceptance Scenario 2, SC-005). Document the outcome (e.g. in this task's own completion note or a short results summary) rather than only running the command and discarding its output.
- [X] T024 [US2] If T023 finds any regression: confirm it is reported distinctly (its own row, `verdict: "regresses"`, actual figures shown) and not averaged away or omitted from the report (spec.md US2 Acceptance Scenario 2, Constitution Principle V). If zero regressions are found, confirm that outcome is itself clearly stated, not merely the absence of a regression row.

**Checkpoint**: User Stories 1 and 2 both independently functional — the report
is same-corpus, feature-broken-out, and every metric carries an actionable
verdict.

---

## Phase 5: User Story 3 - Confirm the zero-copy/lazy design claims hold at realistic scale (Priority: P2)

**Goal**: Confirm specs `005`/`007`/`008`'s allocation-independence claims hold
across the full corpus, not only each spec's one original unit-test message.

**Independent Test**: Run [quickstart.md](quickstart.md) step 7 against a real
`comparison-report.json` (spec.md US3 Independent Test).

### Implementation for User Story 3

- [X] T025 [US3] Run quickstart.md step 7; confirm, for each of `parsing`/`getValue`/`getFirstValue`/`hierarchy`, the distinct Rust `allocationCallCount` values observed across the full corpus stay small and bounded — not trending upward with message size (spec.md SC-004).
- [X] T026 [US3] Specifically for `hierarchy`: confirm `large-hierarchy.hl7` (T003, deliberately built large) does not show an allocation count or bytes-per-op consistent with a full-message tree having been built, versus the smaller existing hierarchy fixtures — direct corpus-scale evidence for spec `008` FR-003's claim, not just the single-message unit test that already exists (spec.md User Story 3 Acceptance Scenario 1).

**Checkpoint**: All three user stories independently functional and confirmed
against one real, executed comparison run.

## Real comparison run: 2026-09-04 (US1-US3 results)

Executed via `mvn compile exec:exec` (real `BenchmarkRunner`, `forks(3)`, full
default warmup/measurement — not a reduced smoke-test config) and
`cargo bench -p hl7pet-core` (all three targets), both against `corpusId:
perf-v2`. Full artifacts at `specs/009-core-perf-validation/comparison/2026-09-04/`.

- **US1**: 138 Comparison Result rows, all 8 messageIds actually used trace to
  real `fixtures/messages/perf/corpus-manifest.json` entries (T019). Both
  corpus-mismatch preconditions (rust-vs-rust, rust-vs-manifest) correctly
  hard-abort with a non-zero exit and explanatory error (T020). All 4 features
  (`parsing`: 42 rows, `getFirstValue`: 36, `getValue`: 36, `hierarchy`: 24)
  appear as distinct rows, never merged (T021).
- **US2**: 0 of 138 results missing a verdict (T022). **The real verdict: 0
  regressions.** 92 of 138 comparable rows are `"beats"` (never merely
  `"meets"` — every comparable metric beat Scala by more than the ±10%
  tolerance); the remaining 46 are `"not-comparable"` by design
  (`allocationCallCount` — no Scala equivalent; `memoryAllocRateBytesPerSec`
  — a rate metric confounded by throughput, see below) (T023/T024). By
  metric, the factor Rust is better by (min/median/max across the 23
  comparable message×PATH combinations):

  | Metric | min | median | max |
  |---|---|---|---|
  | throughput | 2.0x | 20.5x | 2622.8x |
  | latencyP50 | 2.0x | 21.5x | 3086.7x |
  | latencyP95 | 2.0x | 19.3x | 2700.6x |
  | allocationBytesPerOp | 1.2x | 9.0x | 449.2x |

  The extreme end (~2000-3000x) is entirely the `hierarchy` feature: Scala's
  `HL7ParseUtils` hierarchy-mode constructor rebuilds a full parent/child tree
  from scratch on every call (spec 002 Section A.1/B.1's documented eager-tree
  design), while Rust's bounded scan only ever touches the specific parent
  occurrence(s) a query actually asks for — this run is the first real,
  at-scale, executed confirmation of the exact architectural difference specs
  002/008 were designed around, not just a unit-test-scale assertion. Flat
  `parsing`/`getValue`/`getFirstValue` still beat consistently at a more
  modest ~2-20x.
- **A real methodology bug found and fixed against this run**:
  `memoryAllocRateBytesPerSec` (`bytesPerOp * throughput`) initially reported
  as "21 regressions" — but inspecting them showed every single one was Rust
  moving *more total bytes per second* purely because it completes so many
  more operations per second, while the very same rows' `throughput` and
  `allocationBytesPerOp` both showed Rust winning 10-20x. A rate metric
  compared directly between two engines at very different throughputs is
  confounded, not a real regression signal — confirmed concretely (e.g.
  `adt_a01_001`/`getValue`: 18x higher throughput, 9x fewer bytes/op, yet
  "regressed" on the rate). Fixed: this metric's verdict is now
  `"not-comparable"` (values still reported for reference);
  `allocationBytesPerOp` is the metric that actually answers "which engine
  uses less memory per call," and it never showed this artifact. See
  research.md/data-model.md for the full writeup.
- **US3**: `allocationCallCount` stays small and constant per feature
  regardless of message size — `parsing`: always 2 (matching spec 005's own
  claim exactly, now confirmed at corpus scale); `getFirstValue`: always 9;
  `getValue`: 7-17 for single-occurrence queries, 507 specifically for
  `oru_r01_large_026`'s unindexed `"OBX-5"` (100 matched repetitions) — scaling
  with *result count*, not message size, which is the correct, expected
  behavior, not a violation (T025). For `hierarchy` on `large_hierarchy_028`
  specifically (T026): `OBR[1] -> OBX[3]-5` (one parent, one result) → 18;
  `OBR[1] -> OBX-5` (one parent, 5 results) → 37; `OBR -> OBX-5` (all 20
  parents, 100 results) → 611 — roughly 20x the single-parent figure for
  roughly 20x the parents actually processed. This is precisely what spec 008
  FR-003's bounded-per-parent-scan design predicts (cost scales with how many
  parents a query actually selects) and precisely what an accidentally-eager
  full-message-tree implementation would *not* show (which would cost the
  same regardless of how many parents were requested) — direct, at-scale
  falsification-that-didn't-happen for the "no full-message tree" claim.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Tie the pipeline together into one verified, documented, committed
deliverable.

- [X] T027 [P] Run `python3 fixtures/scripts/validate_corpus.py` and confirm it still passes unmodified with `fixtures/messages/perf/` and `fixtures/profiles/large-hierarchy.json` present — these aren't referenced by any conformance vector, so this confirms the validator only checks vectors' own references resolve, not that every corpus file is vector-referenced (plan.md Structure Decision).
- [X] T028 [P] Update `crates/core/README.md` to mention the new `benches/` harness, linking to [quickstart.md](quickstart.md) and [contracts/comparison-artifact-schema.md](contracts/comparison-artifact-schema.md) rather than duplicating them.
- [X] T029 Commit the actual comparison run's output (`specs/009-core-perf-validation/comparison/<run-date>/{scala-results.json,rust-results.json,comparison-report.json}`) as this spec's own Comparison Artifact (spec.md FR-008) — a real, dated, retained run, not a placeholder.
- [X] T030 Run the full [quickstart.md](quickstart.md) validation end-to-end (all 7 steps) and record the outcome.
- [X] T031 Update [ROADMAP.md](../../ROADMAP.md)'s spec `009` status row from "Draft" to "Implemented," summarizing the actual verdict (zero regressions, or which metrics regress), noting the corpus promotion (`fixtures/messages/perf/`, completing spec `004`'s deferred migration) and the new hierarchy-mode Scala benchmarks.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup (T001-T005, the corpus must exist
  before either harness can read it) — BLOCKS all user stories.
- **User Stories (Phase 3-5)**: All depend on Foundational completion (a real
  `comparison-report.json` must exist).
  - US1 (T019-T021), US2 (T022-T024), and US3 (T025-T026) each only *read* the
    same comparison output — independently checkable in parallel once
    Foundational is done, though T023's recorded verdict is useful context for
    T025/T026's own reading of the same report.
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### Within Phase 1 (Setup)

- T001 and T002 are sequenced (the manifest references the message files) but
  have no dependency on T003-T005.
- T003 and T004 in parallel (different files); T005 depends on both (references
  their filenames).

### Within Phase 2 (Foundational)

- T006 has no dependency on T001-T005's *content*, only that the target
  directory (T001) exists to point at.
- T007 depends on T006 (the new classpath location must exist first).
- T008 depends on T003-T005 (needs the large-hierarchy message/profile to load)
  and is otherwise independent of T006/T007.
- T009 depends on T008 (the class must exist to be included).
- T010, T011, T012 have no dependencies on each other or on T006-T009 — pure
  Rust-side scaffolding, parallelizable.
- T013 depends on T010-T012 (the bench targets it declares need the shared
  modules to compile against).
- T014, T015, T016 each depend on T013 (their own `[[bench]]` entry must exist)
  and T010-T012 (the shared modules they use); independent of each other.
- T017 depends on T014-T016 (needs all three targets' output to merge).
- T018 depends on T007/T009 (a real `scala-results.json` shape to parse) and
  T017 (a real `rust-results.json` to parse).

### Parallel Opportunities

- Setup: T003 and T004 in parallel.
- Foundational: T008 in parallel with T006/T007 (different toolchains,
  different files); T010, T011, T012 in parallel with each other and with the
  Scala-side tasks; T014, T015, T016 in parallel with each other once T013 lands.
- User Stories: US1, US2, and US3's tasks can all be run in parallel once
  Foundational's `comparison-report.json` exists — they only read it.
- Polish: T027 and T028 in parallel.

---

## Parallel Example: Foundational Phase

```bash
# Rust-side scaffolding, independent of the Scala-side work and of each other:
Task: "Create benches/common/alloc.rs (allocator instrumentation)"
Task: "Create benches/common/timing.rs (sampling loop + percentiles)"
Task: "Create benches/common/corpus.rs (fixtures/messages/perf/ loader)"

# Once T013 lands, all three bench targets in parallel:
Task: "Implement benches/parsing.rs"
Task: "Implement benches/extraction.rs"
Task: "Implement benches/hierarchy.rs"
```

## Parallel Example: User Stories (post-Foundational)

```bash
# All three stories just read the one comparison-report.json T018 produced:
Task: "US1: verify same-corpus traceability and feature breakout"
Task: "US2: verify explicit verdicts and record the regression outcome"
Task: "US3: verify allocation-count independence across the corpus"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (promote/extend the corpus).
2. Complete Phase 2: Foundational (the entire pipeline — there's no smaller
   working slice, since even the simplest "same corpus" claim needs both
   harnesses and the comparison script to exist and run).
3. Complete Phase 3: User Story 1 — proves the comparison is genuinely
   same-corpus and feature-broken-out.
4. **STOP and VALIDATE**: run quickstart.md steps 1-3.

### Incremental Delivery

1. Setup + Foundational → a real `comparison-report.json` exists, but hasn't
   been checked against any of this spec's own acceptance criteria yet.
2. US1 → same-corpus, feature-broken-out reporting proven (MVP).
3. US2 → the actual regression verdict recorded and confirmed explicit,
   discharging the obligation specs `005`-`008` each deferred to this spec.
4. US3 → allocation-independence claims confirmed at corpus scale, not just
   each prior spec's single hand-picked message.
5. Polish → commit the real run as this spec's Comparison Artifact, docs,
   `ROADMAP.md` update summarizing the actual verdict.

---

## Notes

- [P] tasks touch different files (or independent regions of the same file with
  no shared state) and have no unmet dependency within their phase.
- [Story] labels (US1/US2/US3) map directly to spec.md's prioritized user
  stories.
- T023/T031 are the tasks that actually answer this spec's reason for
  existing — "does the Rust core regress versus Scala." Do not let them become
  a formality: whatever the real run shows (zero regressions, or specific ones)
  must be recorded plainly, per Constitution Principle V, not smoothed over.
- T007's deletion of the old `src/main/resources/corpus/` directory is the one
  step in this task list that removes existing files — confirm T001/T002's
  relocation is complete and `fixtures/messages/perf/` is correct before
  deleting the originals, not after.

## Implementation-time discoveries (Setup + Foundational, T001-T018)

Several real issues surfaced only once the pipeline was actually built and run
end-to-end — each verified against real output, not assumed, and fixed before
moving on rather than left for a later phase to trip over:

- **`ProfileFactory.apply` is broken in the real Scala library**: a smoke test
  (`HierarchyBenchmarksSmokeTest`, kept as a permanent regression check) NPE'd
  immediately — the real source reads the JSON key `"catdinality"` (a typo for
  `"cardinality"`) and unconditionally dereferences a leaf segment's absent
  `"children"` key. Switched to the Jackson/`DefaultScalaModule` path
  `HL7HierarchyParser.parseMessageHierarchyFromJson` already uses (spec 002's
  originally traced source) — confirmed working live against
  `large-hierarchy.json`.
- **Manifest `sizeCategory` collision**: `large-hierarchy.hl7`'s manifest entry
  initially reused `"large-high-repetition"`, the same category spec 004's
  pre-existing `LargeMessageState` filters on with no `messageType` check —
  correct only by manifest array ordering, not by design. Renamed to
  `"large-hierarchy"` and added a permanent regression test
  (`LargeMessageDisambiguationTest`) confirming `"large-high-repetition"`
  resolves to exactly one message.
- **Coverage-granularity mismatch**: several message types have 5 `"typical"`
  manifest entries (inherited from spec 004's original corpus), but Scala's
  own `TypicalMessageState` only ever benchmarks the first one
  (`.findFirst()`). Running the pipeline end-to-end against real JMH output
  surfaced ~100 spurious "no scala result" entries before this was caught.
  Fixed by adding `Corpus::representative_typical_per_type`/
  `unique_by_size_category` to the Rust harness, mirroring Scala's own
  selection exactly, rather than benchmarking every corpus message.
- **Missing `getFirstValue` coverage**: `extraction.rs`'s first draft only
  benchmarked the `getValue` shape; Scala's `getFirstValuePatientLastName`/
  `getFirstValueMinimal` are a distinct feature this spec's FR-003 requires.
  Added as its own derivation over `execute()` (`.first().and_then(|r|
  r.first())`, per spec 007's own documented derivation — no separate Rust
  call path exists).
- **`rust-results.json` is three files, not one**: T017's original "coordinate
  a shared write across three binaries" sketch was replaced with one file per
  bench target (`rust-results-{parsing,extraction,hierarchy}.json`), avoiding
  the write race entirely — `compare_results.py` merges them at read time.
- **Six expected, not erroneous, cross-engine mismatches**: Rust's FR-004
  indexed/filter PATH forms have no per-form Scala benchmark method
  equivalent. `compare_results.py`'s Engine Failure Record gained a `reason`
  field (`"path-form-mismatch"` vs. `"missing-result"`, data-model.md) so this
  expected, spec-anticipated asymmetry (spec.md Edge Cases) is distinguishable
  from an actual gap, rather than one undifferentiated "failure" bucket
  hiding the difference.

Validated end-to-end with a quick, reduced-fidelity JMH run (1 fork, 200ms
warmup/measurement — not the production configuration) purely to prove the
pipeline works; that test output was deleted afterward, not committed, since
it isn't a real benchmark result. The actual, full-fidelity comparison run
(proper `forks(3)`, T020/T023/T029) has not been executed yet.
