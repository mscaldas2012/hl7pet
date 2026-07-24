---

description: "Task list for Scala Baseline Benchmark Harness"
---

# Tasks: Scala Baseline Benchmark Harness

**Input**: Design documents from `/specs/004-scala-baseline-bench/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/baseline-artifact-schema.md](contracts/baseline-artifact-schema.md), [quickstart.md](quickstart.md)

**Tests**: One lightweight JUnit 5 smoke check is included (T008), per plan.md's Technical Context — harness wiring only (corpus loads, the Scala dependency resolves and is callable, `ManifestWriter` produces valid JSON), not exhaustive engine-correctness coverage. No further test tasks are included; the spec does not request TDD, and engine correctness itself is Roadmap spec `003`'s responsibility.

**Organization**: Tasks are grouped by user story (US1/US2/US3 from spec.md, in priority order) so each can be implemented and verified independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3
- File paths are exact and relative to the repository root

## Path Conventions

All paths are under `specs/004-scala-baseline-bench/`, per plan.md's Project Structure
(this feature is self-contained tooling, not part of a shared `src/`/`tests/` tree —
see plan.md's Structure Decision for why).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Stand up the harness's Maven project so it can be built at all.

- [X] T001 Create the harness Maven project at `specs/004-scala-baseline-bench/harness/pom.xml` (groupId `gov.cdc.hl7.bench`, artifactId `scala-baseline-harness`, Java 11 source/target) declaring: `gov.cdc:hl7-pet_2.13:1.2.11` (the benchmark target, resolved from Maven Central per research.md #2), `org.scala-lang:scala-library:2.13.13` pinned explicitly via `<dependencyManagement>` (research.md #2's diamond-dependency note), `org.openjdk.jmh:jmh-core` + `org.openjdk.jmh:jmh-generator-annprocess` (research.md #1), and `org.junit.jupiter:junit-jupiter` (test scope). Create the directory skeleton: `src/main/java/gov/cdc/hl7/bench/`, `src/main/resources/corpus/`, `src/test/java/gov/cdc/hl7/bench/`.
- [X] T002 [P] Create `specs/004-scala-baseline-bench/baseline/README.md` documenting the committed-artifact directory layout — one dated `baseline/<run-date>/` subdirectory per run, `manifest.json` + `jmh-results.json` inside, never overwritten in place (FR-010) — linking to [contracts/baseline-artifact-schema.md](contracts/baseline-artifact-schema.md) for the field-level schema.

**Checkpoint**: `mvn -f specs/004-scala-baseline-bench/harness/pom.xml validate` succeeds and the Scala dependency resolves.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared harness infrastructure every user story's benchmarks and artifacts depend on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T003 [P] Author the interim synthetic HL7 v2 message corpus (research.md #4) under `specs/004-scala-baseline-bench/harness/src/main/resources/corpus/`: ~20-30 messages spanning a handful of common types (e.g. `ADT^A01`, `ORU^R01`, `VXU^V04`), including at least one large/high-repetition message and one minimal message, all synthetic/fabricated (FR-009, no real or de-identified patient data). Include `corpus-manifest.json` listing `{messageId, messageType, sizeCategory, filePath}` per message, with `corpusId: "interim-v1"` (data-model.md Benchmark Message Corpus).
- [X] T004 [P] Implement `ExclusionLog.java` in `specs/004-scala-baseline-bench/harness/src/main/java/gov/cdc/hl7/bench/ExclusionLog.java` — a thread-safe collector of `{messageId, reason}` entries that benchmark code reports to when a corpus message fails to parse under the Scala engine, instead of aborting the run (FR-007, Edge Cases).
- [X] T005 [P] Implement `HostEnvironment.java` in `specs/004-scala-baseline-bench/harness/src/main/java/gov/cdc/hl7/bench/HostEnvironment.java` that collects CPU model, OS name/version, JDK vendor/version, and JMH version at run time (data-model.md Benchmark Run `hostEnvironment` field; Edge Cases' "different hardware isn't directly comparable" rule).
- [X] T006 Implement `Corpus.java` in `specs/004-scala-baseline-bench/harness/src/main/java/gov/cdc/hl7/bench/Corpus.java` that loads `corpus-manifest.json` and the referenced message files (T003), exposing them to JMH `@State` benchmark classes grouped by `messageType`/`sizeCategory`.
- [X] T007 Implement `ManifestWriter.java` in `specs/004-scala-baseline-bench/harness/src/main/java/gov/cdc/hl7/bench/ManifestWriter.java` that writes `manifest.json` to a new `baseline/<run-date>/` directory — `runDate`, `engineCoordinate` (`gov.cdc:hl7-pet_2.13:1.2.11`), `corpusId` (from T006), `hostEnvironment` (from T005), `excludedMessages` (from T004), `resultsFile: "jmh-results.json"` — per [contracts/baseline-artifact-schema.md](contracts/baseline-artifact-schema.md)'s `manifest.json` shape.
- [X] T008 Add a JUnit 5 smoke check in `specs/004-scala-baseline-bench/harness/src/test/java/gov/cdc/hl7/bench/HarnessWiringTest.java` verifying: the corpus (T006) loads with the expected message count and types, `gov.cdc.hl7.HL7StaticParser` is on the classpath and its static methods are callable, and `ManifestWriter` (T007) produces valid, schema-conformant JSON.

**Checkpoint**: `mvn test` passes. Foundation ready — user story work can begin.

---

## Phase 3: User Story 1 - Migration engineer captures the Scala baseline (Priority: P1) 🎯 MVP

**Goal**: Produce a single committed results artifact reporting all five FR-001 metric
categories (parsing throughput, extraction throughput, memory, allocations, latency
p50/p95), broken out by operation, plus engine version/corpus id/run date/host metadata.

**Independent Test**: Run the harness against the Scala engine once; confirm the
produced artifact has no metric category silently missing (spec.md US1 Independent Test).

### Implementation for User Story 1

- [X] T009 [P] [US1] Implement `ParsingBenchmarks.java` in `specs/004-scala-baseline-bench/harness/src/main/java/gov/cdc/hl7/bench/ParsingBenchmarks.java` — JMH `@Benchmark` methods, one per corpus `messageType`, calling `HL7StaticParser.splitFields`/`retrieveSegment`/`retrieveFirstSegmentOf` (research.md #3's "parsing" operation). Annotate `@BenchmarkMode({Mode.Throughput, Mode.SampleTime})` so each method yields both a throughput score and p50/p95 sample percentiles. Report `HL7ParseError` failures to `ExclusionLog` (T004) instead of failing the run.
- [X] T010 [P] [US1] Implement `ExtractionBenchmarks.java` in `specs/004-scala-baseline-bench/harness/src/main/java/gov/cdc/hl7/bench/ExtractionBenchmarks.java` — JMH `@Benchmark` methods, one per corpus `messageType`, calling `HL7StaticParser.getValue`/`getFirstValue` with representative PATH strings per message type (research.md #3's "extraction" operation, labeled as end-to-end parse+extract). Same `@BenchmarkMode` and `ExclusionLog` wiring as T009.
- [X] T011 [US1] Implement `BenchmarkRunner.java` (main class) in `specs/004-scala-baseline-bench/harness/src/main/java/gov/cdc/hl7/bench/BenchmarkRunner.java` that runs JMH programmatically (`OptionsBuilder`) across `ParsingBenchmarks` + `ExtractionBenchmarks` with `-prof gc` (allocation/memory metrics, per research.md #1 and contracts/baseline-artifact-schema.md) and `-rf json` output written to that run's `baseline/<run-date>/jmh-results.json`, then invokes `ManifestWriter` (T007) to write the accompanying `manifest.json`. Depends on T009, T010.
- [X] T012 [US1] Wire `BenchmarkRunner` as a runnable Maven target in `specs/004-scala-baseline-bench/harness/pom.xml` (`exec-maven-plugin`, run as `mvn compile exec:exec` — `exec:java` was tried first but fails JMH's forked-VM mode with `ClassNotFoundException: org.openjdk.jmh.runner.ForkedMain`, since it never sets `java.class.path` for the child JVM; `exec:exec` launches a real `java` subprocess that does), then run it once to produce the first committed baseline: `specs/004-scala-baseline-bench/baseline/<today's-date>/manifest.json` and `jmh-results.json`. Depends on T011.
- [X] T013 [US1] Verify the committed baseline (T012) against spec.md US1's Acceptance Scenarios: confirm all five FR-001 metric categories are present, broken out by operation (parsing vs. extraction), and that `manifest.json` records `engineCoordinate`, `corpusId`, `runDate`, and `hostEnvironment`.

**Checkpoint**: User Story 1 fully functional — a real, committed baseline exists (spec.md SC-001).

---

## Phase 4: User Story 2 - Later Rust benchmarks compare without re-running Scala (Priority: P1)

**Goal**: Prove the committed baseline artifact is self-describing and programmatically
consumable with no JVM, Maven, or Scala engine present.

**Independent Test**: Ignore the harness's build tooling entirely; confirm the committed
baseline file can still be loaded and used on its own (spec.md US2 Independent Test).

### Implementation for User Story 2

- [X] T014 [P] [US2] Write a standalone, dependency-free example reader at `specs/004-scala-baseline-bench/baseline/read-baseline-example.py` (stdlib-only, no build step) that loads a given run's `manifest.json` + `jmh-results.json` and prints, per operation and message type, the throughput/latencyP50/latencyP95/memory/allocation values — a concrete reference implementation for Roadmap spec `009`'s comparison logic, per the field mapping in [contracts/baseline-artifact-schema.md](contracts/baseline-artifact-schema.md).
- [X] T015 [US2] Run the example reader (T014) against the baseline committed in T012; confirm it resolves every FR-001 metric category correctly with zero references to any harness Java source file (spec.md US2 Acceptance Scenarios 1 and 2).

**Checkpoint**: User Stories 1 and 2 both work independently — the baseline is proven consumable standalone (spec.md SC-004).

---

## Phase 5: User Story 3 - New contributor rebuilds on a clean machine via Maven (Priority: P2)

**Goal**: Prove the harness builds and resolves the Scala engine using only its declared
Maven dependency, with no vendored source or local-path reference anywhere in the repo.

**Independent Test**: On a machine with no prior `hl7-pet` checkout, build the harness
using only its Maven dependency and confirm it resolves cleanly (spec.md US3 Independent Test).

### Implementation for User Story 3

- [X] T016 [P] [US3] Add `specs/004-scala-baseline-bench/harness/verify-no-vendored-source.sh` — a repo-wide check confirming zero `.scala` files containing `package gov.cdc.hl7` and zero build-file references to a local filesystem path for the Scala engine (SC-005; matches quickstart.md step 5).
- [X] T017 [US3] Validate clean-dependency resolution: run `mvn dependency:resolve` from `specs/004-scala-baseline-bench/harness/` in an environment with no cached `gov.cdc`/`org.scala-lang` artifacts and no local `hl7-pet` checkout; confirm `gov.cdc:hl7-pet_2.13:1.2.11` and its transitive `scala-library` dependency resolve directly from Maven Central with no credential prompts (spec.md US3 Acceptance Scenario 1; quickstart.md step 1).
- [X] T018 [US3] Run `verify-no-vendored-source.sh` (T016) against the full repository; confirm a clean pass (spec.md US3 Acceptance Scenario 2, SC-005).

**Checkpoint**: All three user stories independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Tie the three stories together into one verified, documented deliverable.

- [ ] T019 [P] Run the full [quickstart.md](quickstart.md) validation script end-to-end (all 5 steps, including the ±10% reproducibility check in step 4) and record the outcome.
- [X] T020 [P] Add `specs/004-scala-baseline-bench/harness/README.md` summarizing how to build and run the harness, linking to [quickstart.md](quickstart.md) and [contracts/baseline-artifact-schema.md](contracts/baseline-artifact-schema.md) rather than duplicating them.
- [X] T021 Update [ROADMAP.md](../../ROADMAP.md)'s spec `004` status row from "Planned" to "Implemented" once T019 passes, noting the committed baseline's run date.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup (T001's `pom.xml` must exist) — BLOCKS all user stories.
- **User Stories (Phase 3-5)**: All depend on Foundational completion.
  - US1 (T009-T013) has no dependency on US2/US3.
  - US2 (T014-T015) depends on US1's committed baseline existing (T012) as *data*, not on US1's code — still independently verifiable once that data exists.
  - US3 (T016-T018) depends only on Foundational (T001's `pom.xml`); it can in fact run in parallel with US1/US2 since it verifies the build/dependency story, not the benchmark results.
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### Within Phase 2 (Foundational)

- T003, T004, T005 have no dependencies on each other — parallelizable.
- T006 depends on T003 (needs the corpus manifest format).
- T007 depends on T004, T005, T006.
- T008 depends on T006, T007.

### Within Phase 3 (US1)

- T009, T010 have no dependencies on each other — parallelizable.
- T011 depends on T009, T010.
- T012 depends on T011.
- T013 depends on T012.

### Parallel Opportunities

- Setup: T002 is parallelizable with T001 (different files).
- Foundational: T003, T004, T005 in parallel; then T006; then T007; then T008.
- User Stories: US1 and US3 can proceed in parallel once Foundational is done (different concerns — benchmark results vs. build/dependency verification). US2 can start as soon as US1's T012 lands.
- Within US1: T009 and T010 in parallel.
- Polish: T019 and T020 in parallel.

---

## Parallel Example: Foundational Phase

```bash
# Launch T003, T004, T005 together (independent files):
Task: "Author interim synthetic HL7 corpus in harness/src/main/resources/corpus/"
Task: "Implement ExclusionLog.java in harness/src/main/java/gov/cdc/hl7/bench/"
Task: "Implement HostEnvironment.java in harness/src/main/java/gov/cdc/hl7/bench/"
```

## Parallel Example: User Story 1

```bash
# Launch T009 and T010 together (independent benchmark classes):
Task: "Implement ParsingBenchmarks.java calling splitFields/retrieveSegment"
Task: "Implement ExtractionBenchmarks.java calling getValue/getFirstValue"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (blocks everything else).
3. Complete Phase 3: User Story 1 — this alone satisfies spec.md SC-001 (a committed
   baseline exists) and is the whole point of the spec per its own "Why this priority."
4. **STOP and VALIDATE**: run quickstart.md steps 1-3 against the US1 output.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. US1 → committed baseline exists (MVP).
3. US2 → prove the baseline is standalone-consumable (needs US1's data, not its code).
4. US3 → prove the build itself is clean-machine-portable (independent of US1/US2, can
   actually be done in parallel with either).
5. Polish → full quickstart validation + docs + ROADMAP update.

---

## Notes

- [P] tasks touch different files and have no unmet dependency within their phase.
- [Story] labels (US1/US2/US3) map directly to spec.md's prioritized user stories.
- US1 and US2 are both P1 in spec.md; they are sequenced here (US1 before US2) because
  US2's independent test requires a committed baseline to already exist as *data* —
  this is a data dependency, not a code dependency, so US2 remains independently
  testable once that data is present, per spec.md's own framing.
- Every corpus message (T003) MUST be synthetic/fabricated — never real or de-identified
  patient data (FR-009), matching specs `001` and `002`'s established convention.
- Commit after each task or logical group; stop at any checkpoint to validate a story
  independently before continuing.
