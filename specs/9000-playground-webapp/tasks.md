# Tasks: HL7-PET Playground Web App

**Input**: Design documents from `/specs/9000-playground-webapp/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/playground-api.md, quickstart.md

**Tests**: Included — plan.md's Project Structure explicitly designates
`playground/tests/test_extraction.py` and `playground/tests/test_routes.py` as
deliverables (pytest + Flask test client), so contract/unit tests are part of
this feature, not an optional add-on.

**Organization**: Tasks are grouped by user story (spec.md priorities P1/P2/P3)
to enable independent implementation and testing of each story.

## Path Conventions

Single small web app at repo root: `playground/` (backend package
`playground/hl7_playground/`, frontend `playground/static/` +
`playground/templates/`, tests `playground/tests/`) — per plan.md's Project
Structure. No `backend/`/`frontend/` split (no frontend build step exists to
justify one).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Scaffold the new `playground/` project

- [X] T001 Create the `playground/` directory skeleton: `playground/hl7_playground/__init__.py` (empty), `playground/static/`, `playground/templates/`, `playground/tests/` — per plan.md's Project Structure
- [X] T002 [P] Write `playground/requirements.txt` pinning `Flask` and `pytest` (research.md #1)
- [X] T003 [P] Write `playground/README.md` with the install/run steps from `specs/9000-playground-webapp/quickstart.md`'s Prerequisites and Run sections (venv, `maturin develop` from `crates/python`, `pip install -r playground/requirements.txt`, `flask --app playground.app run`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The one shared endpoint, its dispatch engine, and the page
skeleton every user story renders into. No user story is testable until this
phase is done.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Create the Flask app factory in `playground/app.py`: set `MAX_CONTENT_LENGTH = 5 * 1024 * 1024` (research.md #6), configure the `templates/`/`static/` folders, and register the routes from `hl7_playground.routes` so `flask --app playground.app run` works
- [X] T005 [P] Implement the dispatch skeleton in `playground/hl7_playground/extraction.py`: a single `extract(message: str, path: str, profile_file) -> dict` function stubbing the full `status` taxonomy from `contracts/playground-api.md` (`results`, `no_results`, `profile_required`, `path_error`, `scan_error`, `profile_error`) and the `"->"` substring check from research.md #3 that will select between `hl7pet.get_value_located` and `hl7pet.get_value_hierarchy` — branch bodies filled in by later story phases
- [X] T006 Implement the `POST /api/extract` route in `playground/hl7_playground/routes.py`, reading `request.form['message']`, `request.form['path']`, and `request.files.get('profile')`, calling `hl7_playground.extraction.extract()`, and returning its dict as JSON (depends on T004, T005)
- [X] T007 [P] Create the static page skeleton in `playground/templates/index.html`: a message `<textarea>`, a PATH `<input>`, an optional profile `<input type="file">`, a submit control, and an empty results container — structure only, no styling (styling is Phase 6)
- [X] T008 [P] Create the fetch-and-render skeleton in `playground/static/app.js`: a submit handler that `POST`s the form as `multipart/form-data` to `/api/extract`, and a `render(responseJson)` function with one empty branch per `status` value from `contracts/playground-api.md` (each story phase below fills in its own branches)

**Checkpoint**: Foundation ready — `/api/extract` responds with a shaped (if empty) `status` for every case; user story implementation can now begin.

---

## Phase 3: User Story 1 - Run a PATH against a pasted message (Priority: P1) 🎯 MVP

**Goal**: Paste a message, run a non-hierarchy PATH, see every matching value with its 1-based source line number; see an explicit no-match state when nothing matches.

**Independent Test**: Paste `fixtures/messages/multi-obx.hl7`, submit PATH `OBX-5`, confirm every matched value shows the correct line number; submit a PATH with no match and confirm an explicit "no results" state (quickstart.md Story 1).

### Tests for User Story 1

- [X] T009 [US1] In `playground/tests/test_routes.py`, add: (a) a test that a non-hierarchy PATH matching multiple `OBX` occurrences in `fixtures/messages/multi-obx.hl7` returns `{"status": "results", "hierarchy": false, "results": [...]}` with correct `value`/`line` pairs in message order; (b) a test that a PATH with no match returns `{"status": "no_results"}`
- [X] T010 [P] [US1] In `playground/tests/test_extraction.py`, add a unit test that `extract()` calls `hl7pet.get_value_located` (not `get_value_hierarchy`) when `path` contains no `"->"`

### Implementation for User Story 1

- [X] T011 [US1] In `playground/hl7_playground/extraction.py`, implement the non-hierarchy branch: call `hl7pet.get_value_located(message, path)`, shape a `None` result as `{"status": "no_results"}` (FR-007) and a real result as `{"status": "results", "hierarchy": false, "results": [{"value": ..., "line": ...}, ...]}` per data-model.md (makes T009/T010 pass)
- [X] T012 [P] [US1] In `playground/static/app.js`'s `render()`, add the `results` (non-hierarchy) branch — a list of value/line rows — and the `no_results` branch — an explicit empty-state message — replacing any prior results wholesale on each call (FR-006)
- [X] T013 [P] [US1] In `playground/templates/index.html`, ensure the message textarea, PATH input, and submit control are wired end-to-end (no profile UI needed for this story — that's Phase 4)

**Checkpoint**: Run quickstart.md's Story 1 steps in a browser. User Story 1 is fully functional and testable independently of profiles/hierarchy.

---

## Phase 4: User Story 2 - Run a hierarchy PATH with a profile (Priority: P2)

**Goal**: Upload a hierarchy profile, run a `->` PATH, see matched child values (no line numbers, per FR-005a); without a profile, see an explicit "profile required" message instead of empty/wrong results.

**Independent Test**: Submit `OBR -> OBX-5` against `fixtures/messages/basic-hierarchy.hl7` with no profile loaded (expect `profile_required`), then again with `fixtures/profiles/basic-two-level.json` loaded (expect scoped child values, no line numbers) — quickstart.md Story 2.

### Tests for User Story 2

- [X] T014 [US2] In `playground/tests/test_routes.py`, add: (a) a test that a `"->"` PATH with no profile file returns `{"status": "profile_required", ...}` and never calls into `hl7pet`; (b) a test that a `"->"` PATH with `fixtures/profiles/basic-two-level.json` uploaded against `fixtures/messages/basic-hierarchy.hl7` returns `{"status": "results", "hierarchy": true, "results": [<bare strings>, ...]}` with no `line` keys present
- [X] T015 [P] [US2] In `playground/tests/test_extraction.py`, add a unit test that `extract()` calls `hl7pet.get_value_hierarchy` (not `get_value_located`) when `path` contains `"->"` and a profile is supplied

### Implementation for User Story 2

- [X] T016 [US2] In `playground/hl7_playground/extraction.py`, implement the hierarchy branch: short-circuit to `{"status": "profile_required", "message": "..."}` (research.md #3) when `"->"` is in `path` and no profile file was supplied; otherwise `json.loads()` the profile file and call `hl7pet.get_value_hierarchy(message, path, profile)`, shaping the result as `{"status": "results", "hierarchy": true, "results": [...]}` per data-model.md (makes T014/T015 pass)
- [X] T017 [P] [US2] Add the optional profile file `<input type="file">` to `playground/templates/index.html`
- [X] T018 [P] [US2] In `playground/static/app.js`, include the profile file (if present) in the submitted `FormData`, and add the `profile_required` branch and the hierarchy-`results` branch to `render()` — hierarchy results render as a bare value list with a fixed, visible note that line numbers aren't available for hierarchy PATHs yet (FR-005a)

**Checkpoint**: Run quickstart.md's Story 2 steps in a browser. User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - Understand and recover from bad input (Priority: P3)

**Goal**: Invalid PATH syntax, an unscannable message, or a malformed profile file each produce a specific, readable error — never a crash, blank state, or misleading result — and the app stays usable afterward.

**Independent Test**: Submit an invalid PATH (expect `path_error`), a message with no MSH (expect `scan_error`), and a non-JSON file as the profile (expect `profile_error`, and confirm a follow-up non-hierarchy PATH still works) — quickstart.md Story 3.

### Tests for User Story 3

- [X] T019 [US3] In `playground/tests/test_routes.py`, add: (a) a test that PATH `OBX[[1]-5` returns `{"status": "path_error", ...}`; (b) a test that a message with no `MSH` segment returns `{"status": "scan_error", ...}` for any PATH; (c) a test that a non-JSON file uploaded as the profile with a `"->"` PATH returns `{"status": "profile_error", ...}`, followed in the same test by a non-hierarchy PATH request that still succeeds normally; (d) a test that a request exceeding `MAX_CONTENT_LENGTH` returns the `too_large` status (research.md #6)
- [X] T020 [P] [US3] In `playground/tests/test_extraction.py`, add unit tests that `extract()` maps each of `hl7pet.Hl7ScanError`, `Hl7PathError`, `Hl7QueryError`, and `Hl7ProfileError` (and a `json.JSONDecodeError` on the profile file) to its documented `status` from `contracts/playground-api.md`

### Implementation for User Story 3

- [X] T021 [US3] In `playground/hl7_playground/extraction.py`, wrap the `hl7pet` calls with `except` blocks for `Hl7ScanError` → `scan_error`, `Hl7PathError`/`Hl7QueryError` → `path_error`, `Hl7ProfileError` → `profile_error`, and `json.JSONDecodeError` on the profile file → `profile_error`, each carrying the underlying exception's message (research.md #4/#5) (makes T019/T020 pass)
- [X] T022 [P] [US3] Register a `413` error handler in `playground/app.py` returning `{"status": "too_large", "message": "..."}` per `contracts/playground-api.md` (research.md #6)
- [X] T023 [P] [US3] In `playground/static/app.js`, add the `path_error`/`scan_error`/`profile_error`/`too_large` branches to `render()` as a visible inline error banner, distinct from the `no_results` empty-state, and confirm the form stays submittable afterward (SC-004)

**Checkpoint**: Run quickstart.md's Story 3 steps in a browser. All three user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Visual polish and edge-case coverage that spans all three stories

- [X] T024 [P] Visual polish pass in `playground/static/style.css`: minimal, clean layout (typography, spacing, a single accent color, a card-style results list) per plan.md's "minimal but functional and visually polished" goal
- [X] T025 [P] Add a test in `playground/tests/test_routes.py` for the non-standard-delimiter Edge Case: a message with custom `MSH-1`/`MSH-2` delimiters, confirming both the extracted value and its line number are correct (FR-012)
- [X] T026 Reconcile `playground/README.md` against `specs/9000-playground-webapp/quickstart.md` so the two don't drift
- [X] T027 Run the full `specs/9000-playground-webapp/quickstart.md` validation end-to-end in a browser (all three stories + the non-standard-delimiter check)
- [X] T028 Run `cd playground && pytest` and confirm the full suite passes

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Phase 2 only
- **User Story 2 (Phase 4)**: Depends on Phase 2 only — independently testable even though it shares `extraction.py`/`app.js` files with US1 (different functions/branches within them)
- **User Story 3 (Phase 5)**: Depends on Phase 2 only — same file-sharing note as US2
- **Polish (Phase 6)**: Depends on Phases 3-5 being complete

### Within Each User Story

- Tests before the implementation task that makes them pass
- `extraction.py` branch logic before the corresponding `app.js` render branch (the route must return the right shape before the frontend can render it) — though the frontend task can be *written* in parallel against the documented contract, it can't be *verified* until the backend task lands
- Story complete (backend + frontend + its own quickstart section passes) before moving to the next priority

### Parallel Opportunities

- T002, T003 (Setup) — different files
- T005, T007, T008 (Foundational) — different files, no cross-dependency
- Within each story, the unit-test task (`test_extraction.py`) and the two implementation tasks touching `app.js`/`templates`/`app.py` are marked [P] — different files from each other and from that story's `test_routes.py` task
- T024, T025 (Polish) — different files

---

## Parallel Example: User Story 1

```bash
# After T009 (test_routes.py additions) and T011 (extraction.py implementation) land sequentially:
Task: "Unit test extract() dispatch in playground/tests/test_extraction.py"          # T010
Task: "Add results/no_results render branches in playground/static/app.js"           # T012
Task: "Wire message/PATH/submit controls in playground/templates/index.html"         # T013
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: run quickstart.md's Story 1 section in a browser
5. This alone is a usable playground for every non-hierarchy PATH — the majority of day-to-day PATH testing

### Incremental Delivery

1. Setup + Foundational → shaped `/api/extract` endpoint exists, empty page renders
2. Add User Story 1 → validate → usable MVP for flat PATHs
3. Add User Story 2 → validate → hierarchy PATHs supported
4. Add User Story 3 → validate → robust against bad input
5. Polish → visual pass + full quickstart + full test suite

---

## Notes

- [P] tasks touch different files with no dependency on an incomplete task
- [Story] labels map tasks to spec.md's user stories for traceability
- No task in this feature touches `crates/` — this is a pure consumer of the existing `hl7pet` package (plan.md Constitution Check)
- Commit after each task or logical group
