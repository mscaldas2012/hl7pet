---

description: "Task list for spec 003-regression-suite"
---

# Tasks: Shared Regression Suite (`fixtures/` Corpus)

**Input**: Design documents from `/specs/003-regression-suite/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: No separate unit-test suite is generated. Per `plan.md`'s Technical Context,
`fixtures/scripts/validate_corpus.py`'s own checks ARE the tests for this spec; the
`quickstart.md` scenarios (deliberately-broken vector, unrecognized family, etc.) are
the verification tasks, included below within each story.

**Organization**: Tasks are grouped by user story (from `spec.md`) to enable
independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Per `plan.md`'s Project Structure: the corpus and its tooling live at the repository
root (`fixtures/`, `.github/workflows/`), not inside `specs/003-regression-suite/`.

---

## Phase 1: Setup

**Purpose**: Skeleton directories and dependency manifest shared by every later task

- [X] T001 Create `fixtures/` skeleton directories at the repo root: `fixtures/messages/`, `fixtures/profiles/`, `fixtures/vectors/path/`, `fixtures/vectors/hierarchy/`, `fixtures/schemas/`, `fixtures/scripts/`, per [contracts/fixture-corpus-layout.md](./contracts/fixture-corpus-layout.md)
- [X] T002 [P] Add `fixtures/scripts/requirements.txt` pinning `jsonschema` (Draft 2020-12 support), per [research.md](./research.md) Decision 1

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared inputs both US2 and US3 need before their checks can run

**⚠️ CRITICAL**: Must complete before US2/US3 implementation tasks begin

- [X] T003 [P] Copy `specs/001-path-grammar-spec/contracts/conformance-vector.schema.json` to `fixtures/schemas/conformance-vector.schema.json` verbatim (FR-001)
- [X] T004 [P] Copy `specs/002-hierarchy-semantics/contracts/hierarchy-conformance-vector.schema.json` to `fixtures/schemas/hierarchy-conformance-vector.schema.json` verbatim (FR-001)
- [X] T005 Create `fixtures/scripts/validate_corpus.py` skeleton: argparse CLI accepting `--corpus-root` (default `fixtures/`, auto-detected from script location) and `--json`, no checks implemented yet, per [contracts/validation-script.md](./contracts/validation-script.md) Invocation section

**Checkpoint**: Schemas and script skeleton exist — US1 (data copying) and later US2/US3 (script logic) can now proceed

---

## Phase 3: User Story 1 - Rust core implementer gets one canonical corpus (Priority: P1) 🎯 MVP

**Goal**: Every synthetic message, conformance vector, and profile from specs `001`/`002` is consolidated, unaltered, under `fixtures/`.

**Independent Test**: Point a new, empty test harness at `fixtures/` alone (no access to `specs/001-path-grammar-spec/` or `specs/002-hierarchy-semantics/`) and confirm every message, vector, and profile needed to validate PATH grammar and hierarchy semantics is present and loadable.

### Implementation for User Story 1

- [X] T006 [P] [US1] Copy `specs/001-path-grammar-spec/messages/{baseline,multi-repetition,filter-example,multi-obx}.hl7` to `fixtures/messages/` verbatim (FR-001)
- [X] T007 [P] [US1] Copy `specs/002-hierarchy-semantics/messages/{complex-hierarchy,unrecognized-segment,basic-hierarchy}.hl7` to `fixtures/messages/` verbatim (FR-001)
- [X] T008 [P] [US1] Copy `specs/002-hierarchy-semantics/profiles/{basic-two-level,deep-nested}.json` to `fixtures/profiles/` verbatim (FR-001)
- [X] T009 [P] [US1] Copy `specs/001-path-grammar-spec/vectors/{valid,invalid}.json` to `fixtures/vectors/path/` verbatim (FR-001, FR-002)
- [X] T010 [P] [US1] Copy `specs/002-hierarchy-semantics/vectors/{basic,complex}.json` to `fixtures/vectors/hierarchy/` verbatim (FR-001, FR-002)
- [X] T011 [US1] Verify byte-for-byte (JSON-normalized) content parity between the four origin vector files and their `fixtures/` copies, per [quickstart.md](./quickstart.md) Scenario 1 (FR-002, SC-001)
- [X] T012 [US1] Verify original files under `specs/001-path-grammar-spec/` and `specs/002-hierarchy-semantics/` remain unmodified via `git status`/`git diff` (FR-009)

**Checkpoint**: `fixtures/` is a complete, standalone, loadable corpus — US1's independent test passes

---

## Phase 4: User Story 2 - CI catches corpus drift automatically (Priority: P1)

**Goal**: A CI-runnable check validates schema conformance, reference resolution, and corpus-wide id uniqueness on every change touching `fixtures/`, with specific failure output.

**Independent Test**: Introduce a deliberately broken vector (duplicate id, schema violation, dangling `message_ref`) on a branch and confirm the CI check fails with an error identifying the specific file and problem, before any other test suite runs.

**Depends on**: Phase 3 (US1) — the checks below validate the corpus US1 populates; write the check logic in parallel if desired, but it cannot be exercised end-to-end until `fixtures/vectors/**` has content.

### Implementation for User Story 2

- [X] T013 [US2] Implement schema-conformance check in `fixtures/scripts/validate_corpus.py`: validate every record in `fixtures/vectors/<family>/*.json` against `fixtures/schemas/<family>.schema.json` using `jsonschema` Draft 2020-12, per [contracts/validation-script.md](./contracts/validation-script.md) check 1 (FR-004.1)
- [X] T014 [US2] Implement `message_ref`/`profile_ref` resolution check in `fixtures/scripts/validate_corpus.py`, per check 2 (FR-004.2)
- [X] T015 [US2] Implement corpus-wide vector `id` uniqueness check in `fixtures/scripts/validate_corpus.py`, reporting both colliding file locations on failure, per check 3 (FR-003, FR-004.3)
- [X] T016 [US2] Wire exit codes (`0`/`1`/`2`) and the human-readable output format in `fixtures/scripts/validate_corpus.py`, per [contracts/validation-script.md](./contracts/validation-script.md) Exit Codes and Output sections (exit `2` wired in Phase 5 alongside coverage gaps, per contract)
- [X] T017 [US2] Create `.github/workflows/fixtures-validation.yml`: trigger on `pull_request`/`push` with a `paths:` filter on `fixtures/**`, run `python3 fixtures/scripts/validate_corpus.py` (FR-005, [research.md](./research.md) Decision 5)
- [X] T018 [US2] Verify CI catches a deliberately broken vector locally, per [quickstart.md](./quickstart.md) Scenario 2 (duplicate id → exit code `1` naming both file locations)

**Checkpoint**: Corpus drift is caught automatically without manual cross-checking — US2's independent test passes

---

## Phase 5: User Story 3 - Coverage gaps are visible without manual tallying (Priority: P2)

**Goal**: A coverage report lists, per grammar production / hierarchy rule, which vector covers it, and flags gaps; unrecognized vector families are reported, not rejected.

**Independent Test**: Run the coverage report against the corpus as consolidated from specs `001`/`002` and confirm it lists 100% of `001`'s grammar productions and 100% of `002`'s hierarchy-semantics rules as covered, with zero gaps.

**Depends on**: Phase 4 (US2) — extends the same `fixtures/scripts/validate_corpus.py` file, and needs Phase 3 (US1)'s vector data to compute real coverage.

### Implementation for User Story 3

- [X] T019 [P] [US3] Implement `path`-family coverage counting in `fixtures/scripts/validate_corpus.py`: tally `grammar_productions` enum values from `conformance-vector.schema.json` against vectors in `fixtures/vectors/path/` (FR-006)
- [X] T020 [P] [US3] Implement `hierarchy`-family coverage counting in `fixtures/scripts/validate_corpus.py`: tally `semantic_rules` enum values from `hierarchy-conformance-vector.schema.json` against vectors in `fixtures/vectors/hierarchy/` (FR-006)
- [X] T021 [US3] Implement unrecognized-vector-family handling in `fixtures/scripts/validate_corpus.py`: any `fixtures/vectors/<family>/` other than `path`/`hierarchy` is counted and listed, never fails the run (FR-007)
- [X] T022 [US3] Implement `--json` coverage-report output mode in `fixtures/scripts/validate_corpus.py`, per [contracts/validation-script.md](./contracts/validation-script.md)
- [X] T023 [US3] Set exit code `2` when `path`/`hierarchy` coverage reports any gap, per [contracts/validation-script.md](./contracts/validation-script.md) Exit Codes (SC-004)
- [X] T024 [US3] Verify the coverage report shows zero gaps immediately after consolidation, per [quickstart.md](./quickstart.md) Scenario 3 (12/12 grammar_productions, 10/10 semantic_rules, exit 0)

**Checkpoint**: Coverage gaps are visible without manual tallying — US3's independent test passes

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Whole-corpus validation spanning all three stories

- [X] T025 [P] Run [quickstart.md](./quickstart.md) Scenario 4: confirm the CI workflow triggers only on `fixtures/**` changes and completes in well under a minute (SC-002) — YAML validated, `paths: fixtures/**` confirmed on both `push`/`pull_request` triggers, local script run at ~0.1s
- [X] T026 [P] Run [quickstart.md](./quickstart.md) Scenario 5: confirm an unrecognized vector family is accepted and reported, not rejected (FR-007)
- [X] T027 Update the `003` row in [ROADMAP.md](../../ROADMAP.md)'s Status table from Draft to Complete once T001-T026 are all verified, per this project's `CLAUDE.md` convention

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup. Blocks US2/US3 (both extend `validate_corpus.py` and need `fixtures/schemas/`); does not block US1.
- **User Story 1 (Phase 3)**: Depends on Foundational (needs `fixtures/` skeleton). Independent of US2/US3's code.
- **User Story 2 (Phase 4)**: Depends on Foundational (script skeleton). Its code (T013-T017) can be written in parallel with US1, but its independent test (T018) needs US1's vector data in `fixtures/vectors/` to have something to validate — so it is *practically* sequenced after Phase 3 even though the files it touches don't overlap with US1's.
- **User Story 3 (Phase 5)**: Depends on US2 (extends the same `validate_corpus.py` file — T019-T023 build directly on T013-T016's structure) and on US1 (needs real vector data for a meaningful coverage report).
- **Polish (Phase 6)**: Depends on all three stories being complete.

### Within Each User Story

- US1: message/profile/vector copies (T006-T010, all `[P]`, disjoint files) before the verification tasks (T011-T012, which read what was just copied).
- US2: checks are added to the same file in the order they're documented in `contracts/validation-script.md` (T013 → T014 → T015 → T016), then the CI workflow (T017) that runs them, then verification (T018).
- US3: the two coverage-counting tasks (T019, T020) are `[P]` (independent dimensions, same file but non-overlapping logic) before the shared unrecognized-family/output/exit-code tasks (T021-T023), then verification (T024).

### Parallel Opportunities

- T001-T002 (Setup) in parallel.
- T003-T004 (Foundational schema copies) in parallel; T005 (script skeleton) is independent of both and can run alongside them.
- T006-T010 (US1 data copies) all in parallel — five disjoint source→destination copies.
- T019-T020 (US3 coverage counting) in parallel — distinct dimensions.
- T025-T026 (Polish verification) in parallel.

---

## Parallel Example: User Story 1

```bash
# Launch all five data-copy tasks for User Story 1 together:
Task: "Copy specs/001-path-grammar-spec/messages/*.hl7 to fixtures/messages/"
Task: "Copy specs/002-hierarchy-semantics/messages/*.hl7 to fixtures/messages/"
Task: "Copy specs/002-hierarchy-semantics/profiles/*.json to fixtures/profiles/"
Task: "Copy specs/001-path-grammar-spec/vectors/{valid,invalid}.json to fixtures/vectors/path/"
Task: "Copy specs/002-hierarchy-semantics/vectors/{basic,complex}.json to fixtures/vectors/hierarchy/"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: run `quickstart.md` Scenario 1 — `fixtures/` is a complete, standalone corpus even with no validation tooling yet
5. This alone already satisfies the Migration Plan's "single shared corpus" repository-layout requirement for any Rust/Python/Java test suite to start reading from

### Incremental Delivery

1. Setup + Foundational → schemas and skeleton in place
2. Add US1 → corpus is consolidated and verifiably unaltered (MVP)
3. Add US2 → drift is now caught automatically in CI
4. Add US3 → coverage gaps are now visible, not just implicitly assumed complete
5. Polish → whole-corpus checks, ROADMAP status update

### Notes

- [P] tasks touch different files (or, for T019/T020, different logical sections of the
  same new file) and have no completed-task dependency between them.
- Every task names an exact file path so it's directly actionable without further
  context lookup.
- Commit after each task or logical group (e.g. after all of T006-T010, or after each
  of T013-T016).
