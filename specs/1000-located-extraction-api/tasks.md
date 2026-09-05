---

description: "Task list for Located Extraction API"
---

# Tasks: Located Extraction API

**Input**: Design documents from `/specs/1000-located-extraction-api/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/located-extraction-api.md](contracts/located-extraction-api.md), [quickstart.md](quickstart.md)

**Tests**: Included as core deliverables, not an optional add-on — spec.md's user
stories are each defined by an "Independent Test" that is a `cargo test`
invocation (quickstart.md), and SC-001/SC-002 are only checkable by running tests
against the `expected_lines` fixture metadata. Same convention specs `005`-`009`'s
tasks.md established.

**Organization**: Tasks are grouped by user story (US1/US2/US3 from spec.md, in
priority order). US1 and US2 are both served by the same `execute_located`
function — a single-match PATH and a multi-occurrence PATH are the same code path
at different cardinalities — so the Foundational phase carries the full
`execute_located` implementation (including `LocatedValue` and the
`resolve_segment_candidates_indexed` helper research.md #1 settled on), and each
story phase then adds the tests that specifically prove *that story's* acceptance
scenarios. US3 (`first_located`) is a small, genuinely separate addition on top
(research.md #2) and gets its own implementation task, not just tests.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: no new crate or module. Everything lands in the
existing `hl7pet-core` crate (`crates/core/`), extending `query.rs` (spec `007`'s
home) in place, plus one new integration test file and one new CLI flag.
`crates/core/src/hierarchy.rs` (spec `008`) is not touched by any task below —
verified achievable by research.md #1's refinement (`resolve_segment_candidates`
keeps its existing signature and behavior unchanged).

---

## Phase 1: Setup

**Purpose**: Stand up the new integration test's skeleton. Everything else this
feature needs (`hl7pet-core` crate, `query.rs`, the shared fixtures corpus) already
exists (specs `005`-`007`) — this phase is deliberately light, matching spec
`008`'s precedent.

- [X] T001 Scaffold `crates/core/tests/located_vectors.rs`: a loader reusing `query_vectors.rs`'s existing `fixtures_root()`/vector-deserialization helpers (extend the vector struct, or add a second one, to also capture `expected_lines: Option<serde_json::Value>` alongside the fields `query_vectors.rs` already reads) that iterates `fixtures/vectors/path/valid.json`, skips any vector with no `expected_lines`, and has a dispatch stub that compiles and runs as a no-op (assertions filled in by T009).

**Checkpoint**: `cargo build --workspace` succeeds; `cargo test -p hl7pet-core --test located_vectors` compiles and passes vacuously.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The `LocatedValue` type and the `execute_located` function every user
story builds on. No user story's acceptance scenarios can be verified until this
exists.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 Define `pub struct LocatedValue<'m> { pub value: &'m str, pub line: usize }` in `crates/core/src/query.rs` per [data-model.md](data-model.md): `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`, doc comment stating the invariant that `line` is 1-based and shared by every value from the same segment occurrence (spec.md FR-004/FR-005).
- [X] T003 Implement `pub(crate) fn resolve_segment_candidates_indexed<'m>(scan: &ScanResult<'m>, segment_name: &str, index: Option<&SegIndex<'_>>) -> Result<Vec<(usize, SegmentSpan)>, QueryError>` in `crates/core/src/query.rs` per research.md #1's refinement: the same filtering logic `resolve_segment_candidates` already implements, changed to `.enumerate()` over `scan.segments` before filtering by name, so each returned `SegmentSpan` is paired with its 1-based line (`index + 1`) in one pass.
- [X] T004 Change the existing `resolve_segment_candidates` in `crates/core/src/query.rs` to delegate to T003: `resolve_segment_candidates_indexed(scan, segment_name, index)?.into_iter().map(|(_, span)| span).collect()`. Confirm its signature, return type, and behavior are byte-for-byte unchanged — `cargo build --workspace` and the full existing `cargo test -p hl7pet-core` suite (including `hierarchy_vectors` and `resolve_segment_candidates`'s own five existing unit tests) pass with zero changes required at any other call site (spec.md FR-008).
- [X] T005 Implement `pub fn execute_located<'m>(scan: &ScanResult<'m>, path: &CompiledPath<'_>) -> Result<Vec<Vec<LocatedValue<'m>>>, QueryError>` in `crates/core/src/query.rs` per [contracts/located-extraction-api.md](contracts/located-extraction-api.md): mirrors `execute()`'s body exactly, calling T003's `resolve_segment_candidates_indexed` instead of `resolve_segment_candidates`, then for each `(line, span)` calls the existing (unchanged) `resolve_field_values` and wraps each resulting `&'m str` as `LocatedValue { value, line }`. A candidate whose requested field index has no matching repetition contributes no entry at all, exactly as `execute()` already does for the same case.
- [X] T006 [P] Update `crates/core/src/lib.rs`: add `execute_located` and `LocatedValue` to the existing `pub use query::{execute, QueryError};` re-export line, matching [contracts/located-extraction-api.md](contracts/located-extraction-api.md)'s public surface (`first_located` added separately in T012, once it exists).

**Checkpoint**: `cargo build --workspace` succeeds; `execute_located` is fully
implemented and exported; `resolve_segment_candidates`'s existing callers
(`execute()`, `hierarchy.rs`) are provably unaffected (T004's checkpoint). Not yet
verified against the fixtures corpus — that's US1/US2's job.

---

## Phase 3: User Story 1 - Caller extracts a value and its source line together (Priority: P1) 🎯 MVP

**Goal**: Prove `execute_located` is correct for the simplest representative case —
a PATH matching exactly one segment occurrence, including sub-segment addressing
(field/component/subcomponent) within it.

**Independent Test**: Run `cargo test -p hl7pet-core --test located_vectors` for a
single-occurrence vector (e.g. `path-msh12`) and confirm the returned value and
line match `expected`/`expected_lines` exactly.

### Tests for User Story 1

- [X] T007 [P] [US1] Unit test in `crates/core/src/query.rs`'s test module: `execute_located` against a single-occurrence PATH (e.g. `MSH-12`) returns exactly one `LocatedValue` whose `value` matches `execute()`'s own output for the same input and whose `line` is `1` (MSH is always the first segment).
- [X] T008 [P] [US1] Unit test in `crates/core/src/query.rs`'s test module: for a PATH addressing a component/subcomponent within one segment (e.g. `PID-5.1`), every `LocatedValue` produced from that occurrence carries the same `line` (spec.md FR-005) — construct a case with a field expression that yields more than one repetition/value from a single occurrence and assert all share one `line`.
- [X] T009 [US1] Fill in `crates/core/tests/located_vectors.rs`'s dispatch body (stubbed in T001): for each single-occurrence vector with `expected_lines`, load the message, parse the path, call `execute_located`, and assert both `value` and `line` match `expected`/`expected_lines` (contracts/located-extraction-api.md's `execute_located`-to-`execute` equivalence, spec.md SC-001/SC-002).

**Checkpoint**: US1's acceptance scenarios (spec.md) pass independently — a caller
can extract a single value and its correct line.

---

## Phase 4: User Story 2 - Caller extracts values from multiple segment occurrences, each with its own line (Priority: P2)

**Goal**: Prove `execute_located` assigns each matched occurrence *its own* line,
not one line for the whole result, including when a filter clause excludes some
occurrences.

**Independent Test**: Run `cargo test -p hl7pet-core --test located_vectors` for
`path-obx5-occurrences` (three `OBX` occurrences) and confirm the result is
`[[{"Positive", 4}]], [[{"Negative", 5}]], [[{"Equivocal", 6}]]`-shaped — three
groups, each with its own line — not a single flattened line.

### Tests for User Story 2

- [X] T010 [P] [US2] Unit test in `crates/core/src/query.rs`'s test module: `execute_located` against an unindexed multi-occurrence PATH (e.g. `OBX-5` against a message with several `OBX` segments) returns one `LocatedValue` group per occurrence, each carrying that occurrence's own distinct, ascending line number, in document order (spec.md FR-006, Acceptance Scenario US2 #1).
- [X] T011 [P] [US2] Unit test in `crates/core/src/query.rs`'s test module: `execute_located` against a PATH with a filter clause that excludes some occurrences of a repeating segment returns only the passing occurrences, each with its own correct line — mirroring `filter_matches`'s existing behavior in `execute()` (spec.md Acceptance Scenario US2 #2).
- [X] T012 [US2] Extend `crates/core/tests/located_vectors.rs` (from T009) to also run every multi-occurrence and filtered vector with `expected_lines` in `fixtures/vectors/path/valid.json` (e.g. `path-obx5-occurrences`, `path-filter-multi-match`), asserting the full per-occurrence value/line pairing against `expected`/`expected_lines`.

**Checkpoint**: US1 and US2 both pass independently — `execute_located` is fully
verified against every `expected_lines`-bearing vector in the shared fixtures
corpus (spec.md SC-001).

---

## Phase 5: User Story 3 - Caller extracts just the first matching value and its line (Priority: P3)

**Goal**: Add the location-aware counterpart to `getFirstValue` as a thin
convenience over `execute_located` (research.md #2) — no independent traversal.

**Independent Test**: Call `first_located` for a PATH matching several
occurrences and confirm it returns only the first value/line pair; call it for a
PATH matching nothing and confirm it returns `None`.

### Tests for User Story 3

- [X] T013 [P] [US3] Unit test in `crates/core/src/query.rs`'s test module: `first_located` against a multi-occurrence PATH returns `Some(lv)` where `lv` equals `execute_located(...)?[0][0]` for the same input (contracts/located-extraction-api.md).
- [X] T014 [P] [US3] Unit test in `crates/core/src/query.rs`'s test module: `first_located` against a PATH that matches nothing (absent segment type, out-of-range index, or an unmatched filter) returns `Ok(None)` — never a fabricated value/line, never an error for this case (spec.md Acceptance Scenario US3 #2).

### Implementation for User Story 3

- [X] T015 [US3] Implement `pub fn first_located<'m>(scan: &ScanResult<'m>, path: &CompiledPath<'_>) -> Result<Option<LocatedValue<'m>>, QueryError>` in `crates/core/src/query.rs` per [contracts/located-extraction-api.md](contracts/located-extraction-api.md): calls `execute_located`, returns `Ok(groups.into_iter().flatten().next())` — no separate traversal (research.md #2).
- [X] T016 [US3] Update `crates/core/src/lib.rs`'s re-export line (T006) to also include `first_located`.

**Checkpoint**: All three user stories pass independently; the full public surface
`contracts/located-extraction-api.md` defines now exists and is exported.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Confirm the feature's non-functional claims (SC-003, SC-004), extend
the dev CLI for manual exercise (quickstart.md step 5), and run the full
regression suite.

- [X] T017 [P] Counting-allocator unit test in `crates/core/src/query.rs`'s test module (reusing the pattern from `scanner.rs`/`hierarchy.rs`'s existing allocation tests, per research.md #4): `execute_located`'s allocation count is independent of unrelated message/segment-count size, confirming SC-004's "no extra pass over the message" claim directly. **Note**: an original draft of this task expected allocation-count parity with `execute()` — corrected during implementation once a diagnostic test found `execute_located` costs one small, constant allocation more per matched occurrence (a `&str -> LocatedValue` map/collect cannot reuse its input buffer the way `execute()`'s same-size `&str -> &str` map/collect does); see research.md #5.
- [X] T018 [P] Add a `--located` flag to `crates/cli/src/main.rs` (mirroring the existing `--first`/`--profile` flag-parsing pattern): when set, calls `execute_located` instead of `execute` and prints each value prefixed with `line {n}: ` (quickstart.md step 5). Update the CLI's usage/help text to document the new flag.
- [X] T019 Run `cargo test --workspace` and `cargo clippy --workspace --all-targets`; confirm the full pre-existing suite (specs `005`-`009`) passes unmodified alongside this feature's new tests (spec.md FR-008/SC-003), and clippy is clean.
- [X] T020 Execute every step of [quickstart.md](quickstart.md) manually and confirm each documented "Expected outcome" holds, including the CLI comparison in step 5 (located vs. non-located output showing identical values).
- [X] T021 Update `ROADMAP.md`'s spec `1000` Status row from "Draft" to "Implemented" with a summary of what shipped (mirroring specs `005`-`009`'s entries' level of detail), and update the module's "Next free" if applicable.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories. T004 specifically depends on T003; T005 depends on T002 and T004 (or T003, since T004 is a pure delegation); T006 depends on T005.
- **User Stories (Phase 3-5)**: All depend on Foundational (Phase 2) completion.
  - US1 (Phase 3) and US2 (Phase 4) both exercise `execute_located` directly — independent test data, no code dependency between them, but both require T005.
  - US3 (Phase 5) depends on `execute_located` (T005) existing, since `first_located` (T015) is implemented in terms of it — not on US1/US2's *tests* being written first, but their shared foundation.
- **Polish (Phase 6)**: Depends on all three user stories being complete (T017 and T019 exercise `first_located`/`execute_located` together; T020's quickstart walks all three).

### Within Each User Story

- Tests before the story's own dispatch/implementation tasks that make them meaningful (T007/T008 before T009; T010/T011 before T012; T013/T014 before T015).
- Story complete before moving to the next priority, though US1/US2 have no code dependency on each other and could be reordered.

### Parallel Opportunities

- T006 (lib.rs re-export) can run in parallel with later Foundational work once T005 lands, since it only touches `lib.rs`.
- All tests marked [P] within a story (T007/T008, T010/T011, T013/T014) touch the same test module but different, independent test functions — safe to write in parallel, sequenced only by whoever merges last into `query.rs`.
- T017 and T018 are independent files (`query.rs` test module vs. `crates/cli/src/main.rs`) and can run in parallel.

---

## Parallel Example: User Story 1

```bash
# Launch both new unit tests for User Story 1 together:
Task: "Unit test: execute_located single-occurrence value+line in crates/core/src/query.rs"
Task: "Unit test: execute_located shared line across sub-segment values in crates/core/src/query.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: `cargo test -p hl7pet-core --test located_vectors` passes for single-occurrence vectors
5. `execute_located` is usable end-to-end for the single-match case even before US2/US3 land

### Incremental Delivery

1. Setup + Foundational → `execute_located` exists and compiles
2. Add US1 → single-occurrence correctness proven → could ship as-is for simple PATHs
3. Add US2 → multi-occurrence/filtered correctness proven → `execute_located` fully verified against the shared fixtures corpus
4. Add US3 → `first_located` convenience added
5. Polish → CLI flag, allocation-count proof, full regression, ROADMAP update

---

## Notes

- [P] tasks = different files or independent test functions, no dependency on an incomplete task
- [Story] label maps task to specific user story for traceability
- This feature has no Scala baseline to verify against (research.md #4) — the
  shared fixtures corpus's `expected_lines` metadata is the sole source of truth
  for correctness
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- `crates/core/src/hierarchy.rs` is not touched by any task in this file — verify
  this stays true through T004's checkpoint
