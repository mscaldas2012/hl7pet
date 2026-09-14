---

description: "Task list for Located Hierarchy API"
---

# Tasks: Located Hierarchy API

**Input**: Design documents from `/specs/011-located-hierarchy-api/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/located-hierarchy-api.md](contracts/located-hierarchy-api.md), [quickstart.md](quickstart.md)

**Tests**: Included as core deliverables, not an optional add-on — spec.md's user
stories are each defined by an "Independent Test" that is a `cargo test`
invocation (quickstart.md), and SC-001/SC-002 are only checkable by running tests
against the `expected_lines` fixture metadata already present in
`fixtures/vectors/hierarchy/`. Same convention specs `005`-`1000`'s tasks.md
established.

**Organization**: Tasks are grouped by user story (US1/US2/US3 from spec.md, in
priority order). All three stories are served by the same `execute_hierarchy_located`
function — a single-match `->` PATH, a multi-parent PATH, and every existing
"no match" edge case are the same code path at different inputs — so the
Foundational phase carries the full `execute_hierarchy_located` implementation
(including the `_indexed` helper refactor research.md #1 settled on), and each
story phase then adds the tests that specifically prove *that story's*
acceptance scenarios. The Python binding (FR-008/FR-009, secondary module:
Language Bindings `6000`-range) is not tied to one story — a Python caller needs
all three working — so it gets its own phase after all three Rust stories are
verified, ahead of Polish.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: no new crate or module. Everything lands in the
existing `hl7pet-core` crate (`crates/core/`), extending `hierarchy.rs` (spec
`008`/`010`'s home) in place, plus one new integration test file; and the
existing `hl7pet-python` crate (`crates/python/`), extending `lib.rs` (spec
`6000`'s home) in place. `crates/core/src/query.rs` is not touched by any task
below — verified achievable by research.md #2 (`resolve_field_values_located`
is already `pub(crate)` and needs no visibility or signature change).

---

## Phase 1: Setup

**Purpose**: Stand up the new integration test's skeleton. Everything else this
feature needs (`hl7pet-core` crate, `hierarchy.rs`, `query::LocatedValue`, the
shared fixtures corpus) already exists (specs `008`/`010`/`1000`) — this phase
is deliberately light, matching specs `008`/`1000`'s precedent.

- [X] T001 Scaffold `crates/core/tests/located_hierarchy_vectors.rs`: a loader mirroring `hierarchy_vectors.rs`'s existing `fixtures_root()`/`load_message()`/`load_profile()` helpers and `HierarchyVector` struct, extended with `expected_lines: Option<serde_json::Value>` alongside the fields `hierarchy_vectors.rs` already reads (`id`, `path`, `profile_ref`, `message_ref`, `method`, `flags`, `expected`). Iterates `fixtures/vectors/hierarchy/basic.json` and `complex.json`, reuses `hierarchy_vectors.rs`'s own `is_multi_hop` exclusion (a `" -> "` count > 1), skips any vector with no `expected_lines`, and has a dispatch stub that compiles and runs as a no-op (assertions filled in by T010).

**Checkpoint**: `cargo build --workspace` succeeds; `cargo test -p hl7pet-core --test located_hierarchy_vectors` compiles and passes vacuously.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The `execute_hierarchy_located` function every user story builds
on, per data-model.md's `_indexed`-delegation design (research.md #1). No user
story's acceptance scenarios can be verified until this exists.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 Implement `fn direct_children_of_type_indexed<'m>(scan: &ScanResult<'m>, profile: &HierarchyProfile, parent_span: SegmentSpan, parent_node: usize, cseg: &str) -> Vec<(usize, SegmentSpan)>` in `crates/core/src/hierarchy.rs` per data-model.md: identical body to the existing `direct_children_of_type` (line 275), changing the scan loop from `for span in &scan.segments[parent_line + 1..]` to `.iter().enumerate()` and computing each matched child's 1-based line as `parent_line + 2 + local_i` (matching `query::resolve_segment_candidates_indexed`'s `i + 1` convention), pushing `(line, *span)` instead of `*span` into `result: Vec<(usize, SegmentSpan)>`.
- [X] T003 Change the existing `direct_children_of_type` in `crates/core/src/hierarchy.rs` to delegate to T002: `direct_children_of_type_indexed(scan, profile, parent_span, parent_node, cseg).into_iter().map(|(_, span)| span).collect()`. Confirm its signature, return type, and behavior are byte-for-byte unchanged — `cargo test -p hl7pet-core --lib hierarchy` passes with zero changes required to any of its 5 existing dedicated unit tests (`direct_children_of_type_records_direct_child` through `direct_children_of_type_ignores_lines_past_the_boundary_regardless_of_tail_size`, lines 426-507).
- [X] T004 Implement `fn apply_child_index_indexed<'m>(scan: &ScanResult<'m>, candidates: Vec<(usize, SegmentSpan)>, index: Option<&SegIndex<'_>>) -> Result<Vec<(usize, SegmentSpan)>, QueryError>` in `crates/core/src/hierarchy.rs` per data-model.md: identical `Numeric`/`Last`/`Star`/`Filter` selection logic to the existing `apply_child_index` (line 331), operating on `(usize, SegmentSpan)` pairs so the line survives selection — `Filter` still calls `query::filter_matches(scan, &span, clause)` against only the span half of each pair.
- [X] T005 Change the existing `apply_child_index` in `crates/core/src/hierarchy.rs` to delegate to T004: tag each input span with a placeholder line (e.g. `0`, unused by the selection logic itself), call `apply_child_index_indexed`, then `.into_iter().map(|(_, span)| span).collect()`. Confirm signature, return type, and behavior are byte-for-byte unchanged — `execute_hierarchy`'s own existing behavior (verified via `cargo test -p hl7pet-core --lib hierarchy`) is unaffected.
- [X] T006 Implement `pub fn execute_hierarchy_located<'m>(scan: &ScanResult<'m>, path: &CompiledPath<'_>, profile: Option<&HierarchyProfile>) -> Result<Vec<Vec<query::LocatedValue<'m>>>, QueryError>` in `crates/core/src/hierarchy.rs` per [contracts/located-hierarchy-api.md](contracts/located-hierarchy-api.md): mirrors `execute_hierarchy`'s body exactly (lines 366-417) — `path.child.is_none()` delegates to `query::execute_located(scan, path)`; `profile.is_none()` (with a child) returns `Ok(vec![])`; otherwise resolves parent candidates and, per matching parent, calls T002's `direct_children_of_type_indexed` and T004's `apply_child_index_indexed` instead of their non-indexed counterparts, accumulating `(usize, SegmentSpan)` pairs into `selected_children`. The final loop calls `query::resolve_field_values_located(line, child.field.as_ref(), segment_content, segment_name, &scan.delimiters)` instead of `query::resolve_field_values`. Update the module's `use crate::query::{self, QueryError};` import to `use crate::query::{self, LocatedValue, QueryError};`.
- [X] T007 [P] Update `crates/core/src/lib.rs`: add `execute_hierarchy_located` to the existing `pub use hierarchy::{execute_hierarchy, HierarchyProfile, ProfileError};` re-export line, matching [contracts/located-hierarchy-api.md](contracts/located-hierarchy-api.md)'s public surface.

**Checkpoint**: `cargo build --workspace` succeeds; `execute_hierarchy_located` is
fully implemented and exported; `direct_children_of_type`/`apply_child_index`'s
existing callers and all of `hierarchy.rs`'s pre-existing unit tests are
provably unaffected (T003/T005's checkpoints). Not yet verified against the
fixtures corpus — that's US1/US2/US3's job.

---

## Phase 3: User Story 1 - Caller extracts a hierarchy-matched value and its source line together (Priority: P1) 🎯 MVP

**Goal**: Prove `execute_hierarchy_located` is correct for the simplest
representative case — a `->` PATH matching exactly one child segment
occurrence, including sub-segment addressing within it.

**Independent Test**: Run `cargo test -p hl7pet-core --test located_hierarchy_vectors`
for a single-match vector (e.g. `hier-004`, `OBR[1] -> OBX[1]-3`) and confirm
the returned value and line match `expected`/`expected_lines` exactly.

### Tests for User Story 1

- [X] T008 [P] [US1] Unit test in `crates/core/src/hierarchy.rs`'s test module: `execute_hierarchy_located` against a `->` PATH matching exactly one child (e.g. `"SPM -> OBX-3"` against `COMPLEX_HIERARCHY_MESSAGE`/`DEEP_NESTED_PROFILE`, the same fixtures spec `010`'s existing tests already define at lines 549-570) returns exactly one `LocatedValue` group whose `value` matches `execute_hierarchy`'s own output for the same input and whose `line` is `11` (the SPM-nested `OBX`'s 1-based position — `scan_result.segments[10]`).
- [X] T009 [P] [US1] Unit test in `crates/core/src/hierarchy.rs`'s test module: for a `->` PATH addressing a field expression that yields more than one value from a single matched child occurrence, every `LocatedValue` produced from that occurrence carries the same `line` (spec.md FR-004) — mirrors spec `1000`'s analogous `T008` for non-hierarchy extraction.
- [X] T010 [US1] Fill in `crates/core/tests/located_hierarchy_vectors.rs`'s dispatch body (stubbed in T001): for each single-match vector with `expected_lines` (`hier-004`, `hier-008`), load the message and profile, parse the path, call `execute_hierarchy_located`, and assert both `value` and `line` match `expected`/`expected_lines` (contracts/located-hierarchy-api.md's `execute_hierarchy_located`-to-`execute_hierarchy` equivalence, spec.md SC-001/SC-002).

**Checkpoint**: US1's acceptance scenarios (spec.md) pass independently — a
caller can extract a single hierarchy-matched value and its correct line.

---

## Phase 4: User Story 2 - Caller extracts values from multiple matched children across multiple parents, each with its own line (Priority: P2)

**Goal**: Prove `execute_hierarchy_located` assigns each matched child *its
own* line, flattened across matching parent occurrences exactly as
`execute_hierarchy` already groups its output, including when a child-side
`SEG_IDX` is applied.

**Independent Test**: Run `cargo test -p hl7pet-core --test located_hierarchy_vectors`
for `hier-006` (`OBR -> OBX-3`, children across two matching `OBR`
occurrences) and confirm the result is `[[{"...A", 7}]], [[{"...C", 8}]]`-shaped
— two groups, each with its own line, flattened rather than nested by parent.

### Tests for User Story 2

- [X] T011 [P] [US2] Unit test in `crates/core/src/hierarchy.rs`'s test module: `execute_hierarchy_located` against a `->` PATH matching children under more than one parent occurrence (e.g. `"OBR -> OBX-3"` against `COMPLEX_HIERARCHY_MESSAGE`/`DEEP_NESTED_PROFILE`) returns one `LocatedValue` group per matched child, each carrying that child's own distinct, ascending line number, flattened across parents in document order — not grouped or nested by parent (spec.md FR-005, research.md #3, Acceptance Scenario US2 #1).
- [X] T012 [P] [US2] Unit test in `crates/core/src/hierarchy.rs`'s test module: `execute_hierarchy_located` against a `->` PATH with a child-side numeric index (e.g. `"OBR[1] -> OBX[1]-3"`) returns only the indexed child occurrence(s), each with its own correct line, consistent with spec `008`'s existing type-filtered, re-based, 1-based child indexing (spec.md Acceptance Scenario US2 #2).
- [X] T013 [US2] Extend `crates/core/tests/located_hierarchy_vectors.rs` (from T010) to also run every remaining single-hop vector with `expected_lines` in `fixtures/vectors/hierarchy/` (`hier-001`, `hier-003`, `hier-005`, `hier-006`, `hier-011`, `hier-014`, `hier-015`, `hier-016` — `hier-010` stays excluded as multi-hop, per T001's inherited `is_multi_hop` guard), asserting the full per-child value/line pairing against `expected`/`expected_lines`.

**Checkpoint**: US1 and US2 both pass independently — `execute_hierarchy_located`
is fully verified against every `expected_lines`-bearing single-hop vector in
the shared fixtures corpus (spec.md SC-001).

---

## Phase 5: User Story 3 - Caller distinguishes "no match" from a located result, for every existing hierarchy edge case (Priority: P3)

**Goal**: Prove `execute_hierarchy_located` reports the exact same "no match"
outcome as `execute_hierarchy` for every documented edge case (zero children,
no profile, ambiguous-parent resolution), never fabricating a line number.

**Independent Test**: Call `execute_hierarchy_located` for `hier-007`'s shape
(`"OBR[2] -> OBX-3"`, zero children) and confirm it returns an empty result
with no line numbers.

### Tests for User Story 3

- [X] T014 [P] [US3] Unit test in `crates/core/src/hierarchy.rs`'s test module: `execute_hierarchy_located` against a `->` PATH whose parent occurrence has no matching children (e.g. `"OBR[2] -> OBX-3"` against `COMPLEX_HIERARCHY_MESSAGE`/`DEEP_NESTED_PROFILE`, mirroring `hier-007`) returns an empty result, exactly mirroring `execute_hierarchy`'s existing empty-result shape for the same input (spec.md Acceptance Scenario US3 #1).
- [X] T015 [P] [US3] Unit test in `crates/core/src/hierarchy.rs`'s test module: `execute_hierarchy_located` against a `->` PATH whose parent-side type is ambiguous in the loaded profile (spec `010`, e.g. `"OBX -> NTE-3"` against `COMPLEX_HIERARCHY_MESSAGE`/`DEEP_NESTED_PROFILE`, mirroring the existing `execute_hierarchy_resolves_ambiguous_obx_parent_correctly` test at lines 629-651) resolves the parent and reports line `9` — identical resolution and line reporting to the unambiguous case (spec.md Acceptance Scenario US3 #2).
- [X] T016 [P] [US3] Unit test in `crates/core/src/hierarchy.rs`'s test module: `execute_hierarchy_located` with `profile: None` for a `->` PATH returns `Ok(vec![])`, mirroring the existing `execute_hierarchy_without_profile_is_empty` test (line 512) — no fabricated `LocatedValue` for this case either.

**Checkpoint**: All three user stories pass independently; the full public
surface `contracts/located-hierarchy-api.md` defines for `hl7pet-core` now
exists, is exported, and is verified against every documented hierarchy edge
case.

---

## Phase 6: Python Binding Parity (Constitution Principle IV, secondary module: Language Bindings `6000`-range)

**Purpose**: Close the exact gap spec `9000` (playground webapp) recorded as
deferred (FR-005a) — surface `execute_hierarchy_located` to Python, so no
capability lands in the Rust core without a tracked path to binding parity
(spec.md FR-008/FR-009, SC-005).

- [X] T017 Implement `#[pyfunction]` `get_value_hierarchy_located` in `crates/python/src/lib.rs` per [contracts/located-hierarchy-api.md](contracts/located-hierarchy-api.md): same signature and profile-handling as the existing `get_value_hierarchy` (lines 71-95: `#[pyo3(signature = (message, path, profile, build_hierarchy=true))]`, JSON re-serialization via the stdlib `json` module, `build_hierarchy` toggle), calling `hl7pet_core::execute_hierarchy_located` instead of `hl7pet_core::execute_hierarchy`, then collapsing to `None`/wrapping each value in `LocatedValue::from` exactly as the existing `get_value_located` does (lines 110-125).
- [X] T018 [P] Register the new function in `crates/python/src/lib.rs`'s `#[pymodule]` block: add `m.add_function(wrap_pyfunction!(get_value_hierarchy_located, m)?)?;` alongside the existing `get_value_hierarchy`/`get_value_located` registrations.
- [X] T019 [P] Add `get_value_hierarchy_located` to `crates/python/python/hl7pet/_hl7pet.pyi`: `def get_value_hierarchy_located(message: str, path: str, profile: dict[str, Any], build_hierarchy: bool = True) -> list[list[LocatedValue]] | None: ...`, placed alongside the existing `get_value_hierarchy`/`get_value_located` stubs.
- [X] T020 [P] Update `crates/python/python/hl7pet/__init__.py`: add `get_value_hierarchy_located` to both the `from ._hl7pet import (...)` block and `__all__`, alongside the existing `get_value_hierarchy`/`get_value_located` entries.
- [X] T021 [P] Add pytest coverage in `crates/python/tests/test_api.py`, mirroring the existing `get_value_hierarchy`/`get_value_located` test patterns (lines 64-104): `test_get_value_hierarchy_located_pairs_value_with_source_line` (using `MULTI_OBX` and the inline profile `{"segmentDefinition": {"OBR": {"children": {"OBX": {}}}}}}` already used by `test_get_value_hierarchy_with_build_hierarchy_false_returns_none`, asserting each returned `LocatedValue`'s `.value`/`.line` matches the non-located `get_value_hierarchy`'s output plus the expected line) and `test_get_value_hierarchy_located_raises_hl7_profile_error_on_invalid_profile_json` (mirroring `test_get_value_hierarchy_raises_hl7_profile_error_on_invalid_profile_json`, line 102).

**Checkpoint**: `maturin develop --release && pytest crates/python/tests/`
passes, including the new `get_value_hierarchy_located` coverage; the existing
`get_value_hierarchy`/`get_value_located` tests remain unaffected.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Confirm the feature's non-functional claims (SC-003, SC-004), and
run the full regression suite across both the Rust core and Python binding.

- [X] T022 [P] Counting-allocator unit test in `crates/core/src/hierarchy.rs`'s test module (reusing the pattern from spec `010`'s existing `unambiguous_parent_resolution_allocation_count_is_unaffected_by_unrelated_ambiguity` test, lines 701-730, and spec `1000`'s analogous test for `execute_located`): `execute_hierarchy_located`'s allocation count is independent of unrelated message/segment-count size, confirming SC-004's "no extra pass" claim directly.
- [X] T023 Run `cargo test --workspace` and `cargo clippy --workspace --all-targets`; confirm the full pre-existing suite (specs `005`-`010`, `1000`-`1001`, `6000`-`6001`) passes unmodified alongside this feature's new tests (spec.md FR-007/SC-003), and clippy is clean.
- [X] T024 Run `pytest crates/python/tests/` (full suite, not just T021's new tests) and confirm zero regressions to the existing `get_value_hierarchy`/`get_value_located`/other coverage.
- [X] T025 Execute every step of [quickstart.md](quickstart.md) manually and confirm each documented "Expected outcome" holds, including the Python binding comparison in step 4 (located vs. non-located hierarchy output showing identical values).
- [X] T026 Update `ROADMAP.md`'s spec `011` Status row from "Draft" to "Implemented" with a summary of what shipped (mirroring specs `008`/`010`/`1000`'s entries' level of detail), and update the Rust Core module's "Next free" if applicable.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories. T003 depends on T002; T005 depends on T004; T006 depends on T002-T005; T007 depends on T006.
- **User Stories (Phase 3-5)**: All depend on Foundational (Phase 2) completion.
  - US1 (Phase 3), US2 (Phase 4), and US3 (Phase 5) all exercise `execute_hierarchy_located` directly — independent test data/edge cases, no code dependency between them, but all require T006.
- **Python Binding Parity (Phase 6)**: Depends on Foundational (Phase 2) — specifically T006/T007 (`execute_hierarchy_located` exported from the crate root) — but not on US1/US2/US3's own tests being written first, only on the function they all exercise existing and being correct in its own right (which Phase 2's checkpoint already establishes; Phase 3-5 add confidence, not a hard dependency).
- **Polish (Phase 7)**: Depends on Phases 2-6 all being complete (T022 exercises `execute_hierarchy_located` directly; T023/T024 run the full regression suite across both languages; T025's quickstart walks Rust and Python steps).

### Within Each User Story

- Tests before the story's own dispatch-filling task that makes them meaningful (T008/T009 before T010; T011/T012 before T013; T014/T015/T016 have no follow-on dispatch task since US3 is pure edge-case verification of behavior Phase 2 already implements).
- Story complete before moving to the next priority, though US1/US2/US3 have no code dependency on each other and could be reordered.

### Parallel Opportunities

- T007 (lib.rs re-export) can run in parallel with later work once T006 lands, since it only touches `lib.rs`.
- All tests marked [P] within a story (T008/T009, T011/T012, T014/T015/T016) touch the same test module but different, independent test functions — safe to write in parallel, sequenced only by whoever merges last into `hierarchy.rs`.
- T018/T019/T020/T021 in Phase 6 touch four different files and can all run in parallel once T017 lands.
- T022 (`hierarchy.rs` test module) and T024 (Python pytest) are independent files/languages and can run in parallel.

---

## Parallel Example: User Story 1

```bash
# Launch both new unit tests for User Story 1 together:
Task: "Unit test: execute_hierarchy_located single-match value+line in crates/core/src/hierarchy.rs"
Task: "Unit test: execute_hierarchy_located shared line across sub-segment values in crates/core/src/hierarchy.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: `cargo test -p hl7pet-core --test located_hierarchy_vectors` passes for single-match vectors
5. `execute_hierarchy_located` is usable end-to-end for the single-match case even before US2/US3/Python parity land

### Incremental Delivery

1. Setup + Foundational → `execute_hierarchy_located` exists and compiles
2. Add US1 → single-match correctness proven → could ship as-is for simple `->` PATHs
3. Add US2 → multi-parent/indexed correctness proven → fully verified against the shared fixtures corpus
4. Add US3 → every existing hierarchy edge case proven identical to `execute_hierarchy`'s
5. Add Python Binding Parity → closes spec `9000`'s recorded gap end-to-end
6. Polish → allocation-count proof, full regression (both languages), ROADMAP update

---

## Notes

- [P] tasks = different files or independent test functions, no dependency on an incomplete task
- [Story] label maps task to specific user story for traceability
- This feature has no Scala baseline to verify against (research.md #5) — the
  shared fixtures corpus's `expected_lines` metadata (already present but
  previously unused in `fixtures/vectors/hierarchy/`) is the sole source of
  truth for correctness
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- `crates/core/src/query.rs` is not touched by any task in this file — verify
  this stays true through T006's checkpoint (research.md #2)
