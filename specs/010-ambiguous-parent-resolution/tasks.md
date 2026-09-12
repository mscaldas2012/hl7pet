# Tasks: Ambiguous-Position Parent Resolution for Hierarchy PATHs

**Input**: Design documents from `/specs/010-ambiguous-parent-resolution/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md,
contracts/ambiguous-parent-resolution.md, quickstart.md

**Tests**: Included — plan.md's Testing section and this repo's established
Rust-Core convention (every prior spec in this module shipped new unit tests plus
new `fixtures/vectors/` entries) treat these as deliverables, not optional TDD.

**Organization**: Tasks are grouped by user story (spec.md priorities P1/P2/P3) to
enable independent implementation and testing of each story.

## Path Conventions

Single-crate fix, no new project: `crates/core/src/hierarchy.rs` (the change),
`fixtures/vectors/hierarchy/` (new conformance vectors, reusing existing
`fixtures/messages/complex-hierarchy.hl7` + `fixtures/profiles/deep-nested.json`),
`specs/008-lazy-hierarchy-nav/contracts/hierarchy-api.md` (cross-spec doc update).

---

## Phase 1: Setup

**Purpose**: Capture a pre-fix baseline to compare against later

- [X] T001 Run `cargo test -p hl7pet-core` and `cargo bench -p hl7pet-core` (spec `009`'s harness, hierarchy-mode benchmarks) on the current code and record the results, so User Story 2's non-regression check (SC-004) has a concrete "before" to compare against

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared resolution mechanism every user story depends on — the new
occurrence classifier, the ambiguity check that decides when to use it, and its
wiring into `execute_hierarchy`'s parent-resolution step. All in
`crates/core/src/hierarchy.rs`, so these tasks are sequential (same file).

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T002 Add an ambiguity-check accessor to `HierarchyProfile` in `crates/core/src/hierarchy.rs` (e.g. `fn is_ambiguous(&self, name: &str) -> bool`), a thin wrapper over the existing private `by_name` map — no change to `by_name`'s shape or to `node_for`'s existing behavior/signature (data-model.md)
- [X] T003 Implement the new top-of-message occurrence classifier in `crates/core/src/hierarchy.rs` (e.g. `fn resolve_occurrence_node(scan: &ScanResult, profile: &HierarchyProfile, target: SegmentSpan) -> Option<usize>`): replays the nearest-enclosing-ancestor matching rule from the top of `scan.segments` (starting at the synthetic root, mirroring where `HL7HierarchyParser.scala`'s walk begins) through `target`'s index, with full backup/restore of popped levels on an unrecognized-anywhere segment (research.md #6) — do NOT reuse `direct_children_of_type`'s existing pop-without-restore inner loop as-is, since that simplification is only valid for its own narrower depth-2-from-one-known-parent purpose
- [X] T004 Wire T002/T003 into `execute_hierarchy`'s parent-resolution step in `crates/core/src/hierarchy.rs`: for each parent candidate span, if `path.segment.name` is ambiguous (T002), resolve its seed node via T003's classifier — if it returns `None`, drop that candidate (no children, FR-004) — otherwise keep today's `node_for` O(1) lookup completely unchanged; pass whichever resolved node results into `direct_children_of_type` (its signature changes to accept an already-resolved node index rather than deriving one internally via `node_for(parent_type)`)
- [X] T005 Add unit tests for the classifier (T003) in isolation, in `crates/core/src/hierarchy.rs`'s existing `#[cfg(test)] mod tests`: both `deep-nested.json` ambiguous positions (`OBX` directly under `OBR`, and under `OBR`'s `SPM` child) resolving different occurrences in the same message to their correct, different nodes; and the backup/restore case from research.md #6 (an unrecognized segment sandwiched between two differently-nested legitimate segments still resolves correctly)

**Checkpoint**: The mechanism is complete, wired into `execute_hierarchy`, and unit-tested in isolation — ambiguous-type PATHs already return correct results at this point. Each story phase below adds conformance-vector-level proof and story-specific coverage.

---

## Phase 3: User Story 1 - Correct results for ambiguous parent types (Priority: P1) 🎯 MVP

**Goal**: A `->` PATH whose parent type is legal at multiple profile positions
returns the children of whichever position each real occurrence actually occupies,
matching the real Scala engine — not an unconditional empty result.

**Independent Test**: Run `OBX -> NTE-3`, `OBX[2] -> NTE-3`, and `SPM -> OBX-3`
against `fixtures/messages/complex-hierarchy.hl7` + `fixtures/profiles/deep-nested.json`
and confirm each returns the correct, non-empty result (quickstart.md Story 1).

### Tests for User Story 1

- [X] T006 [US1] Add new vectors to `fixtures/vectors/hierarchy/complex.json`: `OBX -> NTE-3` and `OBX[2] -> NTE-3` (both expect `["Note attached to OBX-C, not to OBR directly"]`, line 9) and `SPM -> OBX-3` (expects `["OBX-UNDER-SPM-CODE^Nested Under SPM^LN"]`, line 11) — reusing the existing message/profile pair, per research.md #5

### Implementation for User Story 1

- [X] T007 [US1] Run `crates/core/tests/hierarchy_vectors.rs` (spec `008`'s existing data-driven integration test) and confirm the new vectors (T006) pass with no changes needed to that test file itself
- [X] T008 [US1] Run `fixtures/scripts/validate_corpus.py` and confirm full corpus validation (schema conformance, coverage reporting) still passes with the new vectors added
- [X] T009 [US1] Manually cross-check all three new results against the live Scala-engine output already captured in research.md #1, confirming agreement

**Checkpoint**: Run `specs/010-ambiguous-parent-resolution/quickstart.md`'s Story 1 steps. `OBX`/`SPM`-as-parent PATHs now return correct results, independently of Stories 2/3.

---

## Phase 4: User Story 2 - No regression for unambiguous profiles (Priority: P2)

**Goal**: Every hierarchy PATH that already worked continues to produce
byte-for-byte identical results and unaffected performance.

**Independent Test**: Re-run every pre-existing hierarchy vector and benchmark and
confirm no change (quickstart.md Story 2).

### Tests for User Story 2

- [X] T010 [P] [US2] Add a regression-guard unit test in `crates/core/src/hierarchy.rs` asserting an unambiguous type still resolves via `node_for`'s existing O(1) path, not T003's classifier (e.g. an allocation-count or call-count assertion, matching spec `008`/`009`'s existing style for proving a fast path is actually taken)
- [X] T011 [P] [US2] Re-run every pre-existing vector in `fixtures/vectors/hierarchy/basic.json` and the pre-existing entries of `complex.json` (`hier-005` through `hier-010`) and confirm byte-for-byte identical `expected`/`expected_lines` output to the T001 baseline

### Implementation for User Story 2

- [X] T012 [P] [US2] Re-run `cargo bench -p hl7pet-core`'s hierarchy-mode benchmarks and compare against the T001 baseline and spec `009`'s own committed baseline; confirm no measurable regression (SC-004), recording a `comparison/<date>/` artifact under this spec's directory only if a deviation needs explaining (matching specs `009`/`6001`'s precedent)

**Checkpoint**: Run `specs/010-ambiguous-parent-resolution/quickstart.md`'s Story 2 steps. All pre-existing hierarchy behavior and benchmark figures are confirmed unaffected.

---

## Phase 5: User Story 3 - Explicit absence for an unresolvable ambiguous occurrence (Priority: P3)

**Goal**: An ambiguous-type occurrence that doesn't correspond to any legal position
given the message's real structure yields absence, never a panic or an incorrect
guess.

**Independent Test**: Select such an occurrence as a `->` PATH's parent and confirm
an explicit no-match result, never a crash (quickstart.md Story 3).

### Tests for User Story 3

- [X] T013 [US3] Add a new fixture message (or extend an existing one) and hierarchy vector under `fixtures/vectors/hierarchy/` where an `OBX` occurrence appears before any segment that could establish it as a legal position (e.g. before any `OBR`), with a `->` PATH selecting it as parent, `expected: []` per the existing convention (`fixtures/vectors/hierarchy/complex.json`'s `hier-007` style)
- [X] T014 [US3] Add a unit test in `crates/core/src/hierarchy.rs` directly exercising T003's classifier returning `None` for such an occurrence, and confirming `execute_hierarchy` never panics for it (Constitution Principle III, mirroring spec `008`'s existing panic-safety test style)

**Checkpoint**: Run `specs/010-ambiguous-parent-resolution/quickstart.md`'s Story 3 steps. An unresolvable ambiguous occurrence yields explicit absence, never a panic, independently of Stories 1/2.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Close out the documentation this fix touches and do a final full validation

- [X] T015 [P] Update `specs/008-lazy-hierarchy-nav/contracts/hierarchy-api.md`: retire/strike the "Resolving an ambiguous parent-side type ... via history-dependent disambiguation" bullet under "What this contract explicitly does NOT provide," cross-referencing `specs/010-ambiguous-parent-resolution/contracts/ambiguous-parent-resolution.md` (plan.md's Constitution Check requirement, Principle V)
- [X] T016 [P] Update `ROADMAP.md`'s spec `010` status-table row from "Spec drafted" to "Implemented," summarizing the fix and its live-Scala verification, matching this repo's existing Status-table convention
- [X] T017 Run `cargo clippy --workspace --all-targets` and confirm clean
- [X] T018 Run the full `specs/010-ambiguous-parent-resolution/quickstart.md` end-to-end (all three stories plus its Automated checks section) as the final validation pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories; all four tasks touch `crates/core/src/hierarchy.rs` and are sequential (same file)
- **User Story 1 (Phase 3)**: Depends on Phase 2 only
- **User Story 2 (Phase 4)**: Depends on Phase 2 only — independently testable, though it's inherently a *verification* story (no new mechanism, only proof the existing one is unaffected)
- **User Story 3 (Phase 5)**: Depends on Phase 2 only
- **Polish (Phase 6)**: Depends on Phases 3-5 being complete

### Within Each User Story

- Tests before the implementation/verification task that depends on them
- Story complete (its own quickstart.md section passes) before moving to the next priority

### Parallel Opportunities

- T010, T011, T012 (User Story 2) — different files/targets (source, fixtures, benchmarks), no shared-file conflicts
- T013, T014 (User Story 3) — different files (fixtures vs. source)
- T015, T016 (Polish) — different files
- Foundational (T002-T005) and User Story 1's single test task (T006) are **not**
  parallel with each other — same file (`hierarchy.rs`) or direct sequential
  dependency

---

## Parallel Example: User Story 2

```bash
Task: "Regression-guard unit test in crates/core/src/hierarchy.rs"              # T010
Task: "Re-run pre-existing hierarchy vectors in fixtures/vectors/hierarchy/"     # T011
Task: "Re-run cargo bench -p hl7pet-core hierarchy-mode benchmarks"              # T012
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (baseline capture)
2. Complete Phase 2: Foundational (the fix itself)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: run quickstart.md's Story 1 steps
5. This alone delivers the actual bug fix — ambiguous-type hierarchy PATHs work correctly

### Incremental Delivery

1. Setup + Foundational → the mechanism exists and is wired in
2. Add User Story 1 → validate → the fix is proven correct against real fixtures and the live Scala baseline
3. Add User Story 2 → validate → confirm zero regression, byte-for-byte and performance-wise
4. Add User Story 3 → validate → confirm the unresolvable-occurrence edge case is safe
5. Polish → retire the stale limitation doc, update ROADMAP, final full validation

---

## Notes

- [P] tasks touch different files with no dependency on an incomplete task
- [Story] labels map tasks to spec.md's user stories for traceability
- This fix touches no Python binding code (spec `6000`'s `get_value_hierarchy` benefits automatically, unchanged) — confirmed by plan.md's Constitution Check (Principle IV)
- Commit after each task or logical group
