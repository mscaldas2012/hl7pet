---

description: "Task list for spec 6000: Python Bindings & Core-Sync Tooling"
---

# Tasks: Python Bindings & Core-Sync Tooling

**Input**: Design documents from `/specs/6000-python-bindings-automation/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md (all present)

**Tests**: Not explicitly requested as TDD in the spec, but User Stories 1 and 3 each define an Independent Test that is itself a testing artifact (`parity_check.py`, `test_api.py`, regression/determinism tests) — these are included as they are the mechanism the spec's own Acceptance Scenarios and Success Criteria are verified by, not optional add-ons.

**Organization**: Tasks are grouped by user story (spec.md priorities P1/P2/P3) to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- File paths are exact, relative to repo root

## Path Conventions

Per plan.md's Project Structure: two new Cargo workspace members,
`crates/python/` (PyO3 extension + Python package, maturin mixed layout)
and `crates/xtask/` (pure-Rust maintainer tooling). `crates/core/` and
`crates/cli/` are read-only reference points for this feature — no tasks
modify them.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Register the two new crates with the existing Cargo workspace

- [X] T001 Add `"crates/python"` and `"crates/xtask"` to the `[workspace] members` list in `Cargo.toml` (repo root)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Minimal, independently-compiling scaffolding for both new crates — nothing in Phase 3+ can build without this

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T002 [P] Create `crates/python/Cargo.toml` (`crate-type = ["cdylib"]`, `pyo3` dependency with the `abi3-py39` feature per research.md #1, `hl7pet-core` as a path dependency)
- [X] T003 [P] Create `crates/python/pyproject.toml` (maturin build-backend, project name `hl7pet`, `module-name = "hl7pet._hl7pet"`, `python-source = "python"`, `abi3-py39` target per research.md #1)
- [X] T004 [P] Create `crates/python/src/lib.rs` with a minimal `#[pymodule]` entry point (empty body) so the crate compiles standalone
- [X] T005 [P] Create placeholder `crates/python/python/hl7pet/__init__.py` and `crates/python/python/hl7pet/_hl7pet.pyi` (empty, filled in during US1)
- [X] T006 [P] Create `crates/xtask/Cargo.toml` (`[[bin]] name = "xtask"`; `syn`, `proc-macro2`, `serde`, `serde_json` dependencies per research.md #5 — dev-tool only, never a dependency of `hl7pet-core` itself)
- [X] T007 [P] Create `crates/xtask/src/main.rs` with a minimal `fn main() {}` so the crate compiles standalone
- [X] T008 Run `cargo build --workspace` and confirm both new crates (currently empty) build cleanly alongside `hl7pet-core`/`hl7pet-cli` (depends on T001-T007)

**Checkpoint**: Foundation ready — User Story 1 and User Story 2 can now proceed independently (User Story 3 additionally depends on User Story 1 being complete, see Dependencies below)

---

## Phase 3: User Story 1 - Python developer extracts HL7 values with parity to the Rust core (Priority: P1) 🎯 MVP

**Goal**: A pip-installable `hl7pet` Python package exposing `hl7pet-core`'s full public surface (scanning, PATH query, located extraction, hierarchy navigation, escape decoding) with Scala-API-shaped method names, `None`-on-absence, typed exceptions on structural failure, and a batched entry point.

**Independent Test**: Install the built package into a clean Python environment and, for every vector in every family of `fixtures/vectors/{path,hierarchy,scanner,escapes}/`, call the Python API and confirm the returned value(s) match the vector's documented `expected` output exactly (quickstart.md steps 1-2).

### Implementation for User Story 1

- [X] T009 [P] [US1] Implement the exception hierarchy in `crates/python/src/errors.rs` — `Hl7PetError` root plus `Hl7ScanError`/`Hl7PathError`/`Hl7QueryError`/`Hl7ProfileError` (via `pyo3::create_exception!`), per contracts/python-api.md and data-model.md's Exceptions table
- [X] T010 [P] [US1] Implement the `LocatedValue` PyO3 class in `crates/python/src/located_value.rs` (`#[pyclass]` with `value: String`, `line: usize`, `__repr__`), mirroring `hl7pet_core::LocatedValue`
- [X] T011 [US1] Implement `get_value(message, path)` in `crates/python/src/lib.rs` — scan → parse → `execute`, mapping `ScanError`→`Hl7ScanError`, `ParseError`→`Hl7PathError`, `QueryError`→`Hl7QueryError`, empty result → `None` (depends on T009)
- [X] T012 [US1] Implement `get_first_value(message, path)` in `crates/python/src/lib.rs`, reusing `get_value`'s scan/parse/error-mapping path (depends on T011)
- [X] T013 [US1] Implement `get_value_hierarchy(message, path, profile: dict)` in `crates/python/src/lib.rs` — parses `profile` into `HierarchyProfile`, calls `execute_hierarchy`, raises `Hl7ProfileError` on an invalid profile, raises `Hl7PathError` if `path` has no `->` child (depends on T009)
- [X] T014 [US1] Implement `get_value_located(message, path)` and `get_first_value_located(message, path)` in `crates/python/src/lib.rs` using `execute_located`/`first_located`, returning `LocatedValue` instances; raise `Hl7PathError` for a hierarchy PATH (spec 1000's non-hierarchy-only scope) (depends on T009, T010)
- [X] T015 [US1] Implement `get_values(message, paths: list[str])` batched entry point in `crates/python/src/lib.rs` — one scan, one `execute` per path in order, raising immediately (not per-element) on a bad PATH per research.md #4 (depends on T011)
- [X] T016 [US1] Register every function/class/exception from T009-T015 in the `#[pymodule]` block in `crates/python/src/lib.rs` (depends on T009-T015)
- [X] T017 [P] [US1] Fill in `crates/python/python/hl7pet/__init__.py` to re-export every public name from the compiled `_hl7pet` extension (depends on T016)
- [X] T018 [P] [US1] Fill in `crates/python/python/hl7pet/_hl7pet.pyi` type stubs matching contracts/python-api.md's signatures exactly (depends on T016)
- [X] T019 [US1] Implement `crates/python/tests/parity_check.py` — loads every vector under `fixtures/vectors/{path,hierarchy,scanner,escapes}/*.json`, calls the matching `hl7pet` function per the vector's `method`/family, classifies each as `match`/`mismatch`/`not_implemented` (research.md #8), and writes a report per `contracts/parity-report.schema.json`, exiting non-zero on any non-`match` (depends on T016, T017, T018)
- [X] T020 [P] [US1] Write `crates/python/tests/test_api.py` (pytest) covering contracts/python-api.md's behavioral contract items 1-5: `None` on no match, raises on invalid PATH/malformed message, `get_values` batch ordering and immediate-raise-on-bad-path behavior (depends on T016, T017, T018)
- [X] T021 [P] [US1] Add a runnable Python usage example (`get_value`/`get_first_value`/`get_values`) to `crates/python/README.md`, per FR-013's documentation obligation (depends on T016, T017)
- [X] T022 [US1] Validation checkpoint: run quickstart.md steps 1-2 (`maturin develop`, then `parity_check.py`) end-to-end and confirm `totals.mismatch == 0` and `totals.not_implemented == 0` (depends on T019, T020, T021)

**Checkpoint**: User Story 1 is fully functional and independently testable — the `hl7pet` package can be installed and used standalone.

---

## Phase 4: User Story 2 - Maintainer identifies exactly what a new core spec changed for Python (Priority: P2)

**Goal**: An `xtask surface-diff` command that snapshots `hl7pet-core`'s public API via `syn`, diffs it against a committed baseline, and classifies each change as a Backward-Compatible Addition or a Documented Breaking Change; an `xtask sync-baseline` command that records a new baseline after a verified sync.

**Independent Test**: Starting from a known baseline, add one new `pub fn` to `hl7pet-core` and run `xtask surface-diff`; confirm the report lists exactly that one addition, correctly classified, with no false positives/negatives (quickstart.md steps 3-5). This story is independent of User Story 1 — it operates purely on `crates/core`'s Rust source.

### Implementation for User Story 2

- [X] T023 [P] [US2] Implement `crates/xtask/src/surface.rs` — `syn`-based parser walking `crates/core/src/**/*.rs`, extracting every `pub fn`/`pub struct`/`pub enum`/`pub type` into the surface-snapshot shape from data-model.md, excluding any item marked per FR-010's internal-only convention (depends on T006, T007)
- [X] T024 [US2] Implement `crates/xtask/src/classify.rs` — diffs two surface snapshots into Surface Change Report entries (`added`/`changed`/`removed`), applying data-model.md's classification rules (`added` always `backward_compatible_addition`; `removed` or a non-superset `changed` signature is `documented_breaking_change` with `requires_version_bump`/`requires_migration_note` both `true`) (depends on T023)
- [X] T025 [US2] Wire the `xtask surface-diff` subcommand in `crates/xtask/src/main.rs` — loads `crates/xtask/surface-baseline.json`, parses the current tree (or `--against <commit>` via `git show`) with `surface.rs`, diffs via `classify.rs`, prints the report per `contracts/surface-snapshot.schema.json` (depends on T024)
- [X] T026 [US2] Wire the `xtask sync-baseline` subcommand in `crates/xtask/src/main.rs` — captures the current `git rev-parse HEAD` and current surface snapshot, overwrites `crates/xtask/surface-baseline.json` (depends on T023)
- [X] T027 [P] [US2] Seed the initial `crates/xtask/surface-baseline.json` by running `xtask sync-baseline` once against `hl7pet-core`'s current (spec-`1001`-complete) public surface, establishing the starting baseline for future diffs (depends on T026)
- [X] T028 [P] [US2] Write `cargo test -p xtask` unit tests for `surface.rs`/`classify.rs` against small fixed Rust source snippets, covering added/changed/removed/excluded-internal cases (depends on T023, T024)
- [X] T029 [US2] Validation checkpoint: run quickstart.md steps 3-5 (simulate a new `pub fn` → confirm `backward_compatible_addition`; diff against the pre-spec-`1001` commit → confirm `documented_breaking_change` with both flags `true`; confirm `up_to_date: true`/`changes: []` on a clean tree) (depends on T025, T026, T027)

**Checkpoint**: User Stories 1 AND 2 both work independently — the sync script requires no Python binding to exist.

---

## Phase 5: User Story 3 - Maintainer verifies the Python binding still matches the Rust core after every change (Priority: P3)

**Goal**: A repeatable, reliable, `pytest`-integrated parity check that catches a deliberately introduced regression in the Python binding every time, with deterministic output across repeated runs.

**Independent Test**: Deliberately introduce one incorrect behavior into the Python binding and run the parity check; confirm it fails specifically on the vector(s) exercising that code path and passes on every other vector (quickstart.md step 6). Depends on User Story 1's binding and `parity_check.py` already existing.

### Implementation for User Story 3

- [X] T030 [P] [US3] Write `crates/python/tests/test_parity.py` wiring `parity_check.py`'s run into `pytest` (imports and invokes it, asserts `totals.mismatch == 0` and `totals.not_implemented == 0`), per research.md #7 (depends on US1's T019)
- [X] T031 [US3] Write `crates/python/tests/test_parity_regression.py` — deliberately monkeypatches/breaks one binding code path in a test fixture, runs `parity_check.py`, and asserts exactly the affected vector(s) report `status: "mismatch"` while every other vector still reports `match` (SC-004) (depends on T030)
- [X] T032 [P] [US3] Write `crates/python/tests/test_parity_determinism.py` — runs `parity_check.py` twice against the same build and asserts identical `results` content/ordering aside from the informational `run_at` field (SC-005) (depends on T030)
- [X] T033 [US3] Validation checkpoint: run quickstart.md step 6 manually (introduce a one-line defect in `crates/python/src/lib.rs`, `maturin develop`, run `parity_check.py`, confirm exactly the affected vector(s) mismatch; then revert) (depends on T030, T031, T032)

**Checkpoint**: All three user stories are independently functional — installable binding (US1), maintainer surface-diff tooling (US2), and a proven-reliable parity check (US3).

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Repo-wide consistency and final acceptance validation

- [X] T034 [P] Add a pointer from the repo-root `README.md` to `crates/python/README.md`'s quickstart, so the Python binding is discoverable from the top level
- [X] T035 Update `ROADMAP.md`'s spec `6000` status-table row from "Draft" to "Implemented" with a summary of what shipped, per this project's Spec-Kit roadmap convention (`CLAUDE.md`)
- [X] T036 [P] Run `cargo clippy --workspace --all-targets` and fix any warnings surfaced in `crates/python` or `crates/xtask`
- [X] T037 Run all seven `quickstart.md` scenarios end-to-end in order as the final acceptance pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Setup (T001) — BLOCKS User Story 1 and User Story 2
- **User Story 1 (Phase 3)**: Depends on Foundational only
- **User Story 2 (Phase 4)**: Depends on Foundational only — independent of User Story 1
- **User Story 3 (Phase 5)**: Depends on Foundational AND on User Story 1 being complete (it tests the binding US1 builds; the spec's own "Why this priority" states this explicitly) — does NOT depend on User Story 2
- **Polish (Phase 6)**: Depends on User Stories 1, 2, and 3 all being complete

### User Story Dependencies

- **User Story 1 (P1)**: No dependencies on other stories — the MVP
- **User Story 2 (P2)**: No dependencies on User Story 1 — operates purely on `crates/core`'s Rust source and can be built/tested in parallel with US1
- **User Story 3 (P3)**: Depends on User Story 1 (tests the binding it produces); independent of User Story 2

### Within Each User Story

- Foundational scaffolding before any function implementation
- Error/value types (T009-T010) before the functions that raise/return them (T011-T015)
- Individual functions before `#[pymodule]` registration (T016)
- Registration before the Python-facing package files and tests (T017-T021)
- Implementation before its validation checkpoint

### Parallel Opportunities

- All Foundational tasks marked [P] (T002-T007) can run in parallel once T001 lands
- Once Foundational completes, User Story 1 and User Story 2 can proceed fully in parallel (different developers, different files)
- Within US1: T009 and T010 in parallel; T017/T018/T020/T021 in parallel once T016 lands
- Within US2: T023 can proceed alone; T027 and T028 in parallel once their dependencies land
- Within US3: T030 and T032 can run in parallel once wired; T031 depends on T030
- T034 and T036 in Polish can run in parallel

---

## Parallel Example: User Story 1

```bash
# Once Foundational (T002-T008) is done, launch these together:
Task: "Implement the exception hierarchy in crates/python/src/errors.rs"
Task: "Implement the LocatedValue PyO3 class in crates/python/src/located_value.rs"

# After T016 (pymodule registration) lands, launch these together:
Task: "Fill in crates/python/python/hl7pet/__init__.py re-exports"
Task: "Fill in crates/python/python/hl7pet/_hl7pet.pyi type stubs"
Task: "Write crates/python/tests/test_api.py"
Task: "Add a runnable usage example to crates/python/README.md"
```

## Parallel Example: User Story 1 vs. User Story 2

```bash
# These two stories touch entirely disjoint files and can run at the same
# time by different developers once Foundational is complete:
Track A (US1): T009 -> T010 -> T011 -> ... -> T022   (crates/python/**)
Track B (US2): T023 -> T024 -> T025/T026 -> ... -> T029  (crates/xtask/**)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001)
2. Complete Phase 2: Foundational (T002-T008) — CRITICAL, blocks both P1 and P2 stories
3. Complete Phase 3: User Story 1 (T009-T022)
4. **STOP and VALIDATE**: run quickstart.md steps 1-2, confirm 100% fixtures-corpus parity
5. A pip-installable `hl7pet` package is now a usable deliverable on its own

### Incremental Delivery

1. Setup + Foundational → shared scaffolding ready
2. Add User Story 1 → validate independently → MVP: installable Python package with full parity
3. Add User Story 2 (can start immediately after Foundational, in parallel with US1) → validate independently → maintainer sync tooling ready for the next core spec
4. Add User Story 3 (after US1 lands) → validate independently → parity-check reliability proven
5. Polish (Phase 6) → cross-cutting docs/lint/final acceptance

### Parallel Team Strategy

With two developers:

1. Both complete Setup + Foundational together (T001-T008)
2. Developer A: User Story 1 (T009-T022)
   Developer B: User Story 2 (T023-T029), fully in parallel — no shared files
3. Once User Story 1 lands, either developer picks up User Story 3 (T030-T033)
4. Either developer finishes Polish (T034-T037) once all three stories are done

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- User Story 2 is genuinely independent of User Story 1 (Rust-source-only); User Story 3 is the one story with a real cross-story dependency, and the spec itself states why
- Commit after each task or logical group
- Stop at any checkpoint to validate a story independently
- `crates/core` and `crates/cli` are read-only for this feature — no task modifies them
