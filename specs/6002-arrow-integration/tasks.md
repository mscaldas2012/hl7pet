---

description: "Task list for Arrow Integration for PySpark & PyArrow"
---

# Tasks: Arrow Integration for PySpark & PyArrow

**Input**: Design documents from `/specs/6002-arrow-integration/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/arrow-api.md](contracts/arrow-api.md), [quickstart.md](quickstart.md)

**Tests**: Included as core deliverables, not an optional add-on — spec.md's
Independent Test criteria for every story are fixtures-corpus parity checks or
scan-count/allocation-style proofs (quickstart.md steps 2-4), and SC-002/SC-003/
SC-004 are only checkable by running them. Same convention specs `005`-`6001`'s
tasks.md files established.

**Organization**: Tasks are grouped by user story (US1-US4 from spec.md, in
priority order — US1 and US2 are co-equal P1 stories per spec.md). US1
(`extract_value`) and US2 (`extract_values`) share one per-row extraction
helper (T011) split deliberately into "scan the message" and "execute one PATH
against an already-scanned message" so US2's one-scan-per-message guarantee
(SC-002) holds by construction, not by convention — so the Foundational phase
carries the Arrow/error/conversion plumbing every story needs, and T011 (the
one piece of real per-row extraction logic) is introduced at the start of US1
since US1 is the simplest consumer of it, with US2 reusing it unchanged.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3/US4
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: one new Cargo workspace member, `crates/arrow`
(`hl7pet-arrow`), packaged as its own `maturin` root (`hl7pet_arrow`), plus a
new top-level `notebooks/` directory. No existing crate is modified except the
root `Cargo.toml`'s `[workspace] members` list.

---

## Phase 1: Setup

**Purpose**: Stand up the new crate and Python package skeleton, and resolve
research.md #2's flagged `pyo3-arrow`/`abi3-py39` compatibility spike before
any real code depends on the answer.

- [X] T001 Create `crates/arrow/Cargo.toml`: `hl7pet-core` (path dep, unchanged), `arrow = "59"`, `pyo3 = { version = "0.29", features = ["abi3-py39"] }`, `pyo3-arrow = "0.19"` (research.md #1-#2); `[lib] name = "_hl7pet_arrow"`, `crate-type = ["cdylib"]`. Add `"crates/arrow"` to the root `Cargo.toml`'s `[workspace] members` list (currently `["crates/core", "crates/cli", "crates/python", "crates/xtask"]`). With an empty `src/lib.rs` (just `#![allow(dead_code)]` as a placeholder), run `cargo build -p hl7pet-arrow` to confirm `pyo3-arrow` actually compiles under `abi3-py39` — **this resolves research.md #2's flagged spike**. If it fails, remove `pyo3-arrow` from `Cargo.toml`, add a `# pyo3-arrow dropped: abi3-py39 incompatible as of 0.19 (research.md #2) — hand-rolled arrow::ffi used instead` comment, and flag T008/T012/T018 below to implement PyO3↔Arrow conversion via `arrow::ffi::{FFI_ArrowArray, FFI_ArrowSchema}` directly instead of `pyo3_arrow::PyArray`.
- [X] T002 [P] Create `crates/arrow/pyproject.toml`: `[build-system] requires = ["maturin>=1.7,<2.0"]`, `build-backend = "maturin"`; `[project] name = "hl7pet_arrow"`, `requires-python = ">=3.9"`, `dependencies = ["hl7pet"]` (contracts/arrow-api.md's exception-reuse dependency — `hl7pet_arrow` imports `hl7pet.Hl7PathError`/`Hl7ProfileError` at runtime); `[tool.maturin] module-name = "hl7pet_arrow._hl7pet_arrow"`, `python-source = "python"`, `features = ["pyo3/extension-module"]` — mirrors `crates/python/pyproject.toml` exactly except the package/module names.
- [X] T003 [P] Create `crates/arrow/python/hl7pet_arrow/__init__.py` (module docstring only; `from ._hl7pet_arrow import ...` and `__all__` filled in by T013/T019) and `crates/arrow/python/hl7pet_arrow/spark.py` (module docstring only; filled in by T025-T026).
- [X] T004 [P] Create the `notebooks/` top-level directory with a placeholder `notebooks/arrow_pyspark_demo.ipynb` (one markdown cell: "TODO: spec 6002 demo notebook — filled in by T029") — this repo currently has no notebook directory (plan.md Structure Decision).

**Checkpoint**: `cargo build -p hl7pet-arrow` succeeds with `pyo3-arrow` (or its confirmed fallback) actually compiling under `abi3-py39`; the Python package skeleton and notebook placeholder exist.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The Arrow/error/conversion plumbing both `extract_value` (US1)
and `extract_values` (US2) build on. No user story's acceptance scenarios can
be verified until this exists.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T005 Implement `crates/arrow/src/errors.rs`: `fn hl7pet_path_error(py: Python<'_>, msg: impl Into<String>) -> PyErr` and `fn hl7pet_profile_error(py: Python<'_>, msg: impl Into<String>) -> PyErr`, each doing `py.import("hl7pet")?.getattr("Hl7PathError"/"Hl7ProfileError")?` and constructing a `PyErr` from that type object (contracts/arrow-api.md's `py.import("hl7pet")` design — the same pattern `crates/python/src/lib.rs`'s `get_value_hierarchy` already uses for the stdlib `json` module), falling back to a plain `PyErr::new::<pyo3::exceptions::PyImportError, _>` if the `hl7pet` import itself fails (should never happen given T002's `pyproject.toml` dependency, but must not panic). Also `fn empty_paths_error() -> PyErr` returning `pyo3::exceptions::PyValueError::new_err("paths must not be empty")` (contracts/arrow-api.md's `ValueError` mapping for FR-008).
- [X] T006 [P] Implement `crates/arrow/src/result_schema.rs`: `enum RowStatus { Ok, NoMatch, Error }` and `fn build_result_struct_array<'m>(outcomes: Vec<(Option<Vec<Vec<std::borrow::Cow<'m, str>>>>, RowStatus)>) -> arrow::array::StructArray` producing the `{value: List<List<Utf8>>, status: Utf8}` Arrow struct array per data-model.md's Result Struct (`status` serialized as `"ok"`/`"no_match"`/`"error"`). Confirm the exact `arrow` 59.x builder types (`GenericListBuilder`/`GenericStringBuilder`/`StructBuilder` or equivalent) against the crate's actual API at implementation time — not pinned further here since research.md didn't drill into builder-level API shape, only the crate version.
- [X] T007 Implement `crates/arrow/src/convert.rs`: `fn row_outcome<'m>(execute_result: Result<Vec<Vec<std::borrow::Cow<'m, str>>>, hl7pet_core::QueryError>) -> (Option<Vec<Vec<std::borrow::Cow<'m, str>>>>, result_schema::RowStatus)` — `Err(_)` (a non-numeric filter comparison) maps to `(None, RowStatus::Error)`; `Ok(rows)` with `rows.is_empty()` maps to `(None, RowStatus::NoMatch)`; `Ok(rows)` non-empty maps to `(Some(rows), RowStatus::Ok)` — data-model.md's single merged `"error"` status, not split by cause.
- [X] T008 Implement the messages-array input helper in `crates/arrow/src/convert.rs`: `fn messages_iter<'a>(messages: &'a pyo3_arrow::PyArray) -> PyResult<impl Iterator<Item = Option<&'a str>>>` (or the `arrow::ffi`-based equivalent if T001's spike required the fallback) — downcasts the incoming Arrow array to `Utf8Array`/`LargeUtf8Array`, yielding `None` for a null row and `Some(&str)` otherwise, per data-model.md's Message Column entity and FR-007.
- [X] T009 Implement the `#[pymodule]` skeleton in `crates/arrow/src/lib.rs`: `mod convert; mod errors; mod result_schema;`, an empty `fn _hl7pet_arrow(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> { Ok(()) }` registered via `#[pymodule]`, mirroring `crates/python/src/lib.rs`'s existing module-doc-comment style. `cargo build -p hl7pet-arrow` succeeds with no `#[pyfunction]`s registered yet.
- [X] T010 Implement the shared call-level validation gate in `crates/arrow/src/lib.rs`: `fn validate_path<'p>(py: Python<'_>, path: &'p str, profile: Option<&Bound<'_, PyAny>>) -> PyResult<hl7pet_core::CompiledPath<'p>>` — calls `hl7pet_core::parse(path)`, mapping a `ParseError` through T005's `errors::hl7pet_path_error`; if the compiled path is a hierarchy PATH (`compiled.child.is_some()`) and `profile.is_none()`, raises T005's `errors::hl7pet_profile_error` immediately, per contracts/arrow-api.md's "hierarchy PATH with `profile=None` raises immediately, no partial row processing" rule. Both `extract_value` (T012) and `extract_values` (T018, once per requested path) call this before any row is touched.
- [X] T011 Implement the two shared per-row extraction helpers in `crates/arrow/src/lib.rs`, split deliberately so US2's one-scan-per-message guarantee is structural, not conventional: `fn scan_message<'m>(message: Option<&'m str>) -> Option<Result<hl7pet_core::ScanResult<'m>, hl7pet_core::ScanError>>` (`None` in → `None` out; a null message row is never scanned) and `fn extract_one<'m>(scanned: Option<&Result<hl7pet_core::ScanResult<'m>, hl7pet_core::ScanError>>, compiled: &hl7pet_core::CompiledPath<'_>, profile: Option<&hl7pet_core::HierarchyProfile>) -> (Option<Vec<Vec<std::borrow::Cow<'m, str>>>>, result_schema::RowStatus)` — `None`/`Some(Err(_))` map to `(None, NoMatch)`/`(None, Error)` respectively (contracts/arrow-api.md: null message → `"no_match"`, scan failure → `"error"`); `Some(Ok(scan))` dispatches to `hl7pet_core::execute`/`execute_hierarchy` per `compiled.child.is_some()`, feeding the result into T007's `convert::row_outcome`.

**Checkpoint**: `cargo build -p hl7pet-arrow` succeeds; every shared helper (`errors`, `result_schema`, `convert`, `validate_path`, `scan_message`/`extract_one`) exists and compiles; no `#[pyfunction]` is registered yet — that's US1/US2's job.

---

## Phase 3: User Story 1 - Extract one field across a column of HL7 messages (Priority: P1) 🎯 MVP

**Goal**: Prove single-PATH extraction is correct and usable standalone
against a PyArrow array, for both non-hierarchy and hierarchy-mode PATHs.

**Independent Test**: Build a PyArrow Table from `fixtures/messages/`, run
`extract_value` with a PATH that has a `fixtures/vectors/path/` vector, and
confirm the output row matches that vector's `expected` value (spec.md's
Independent Test for US1).

- [X] T012 [US1] Implement `#[pyfunction] fn extract_value(py: Python<'_>, messages: pyo3_arrow::PyArray, path: &str, profile: Option<&Bound<'_, PyAny>>) -> PyResult<pyo3_arrow::PyArray>` in `crates/arrow/src/lib.rs` per [contracts/arrow-api.md](contracts/arrow-api.md): calls T010's `validate_path` once; if the PATH is hierarchy-mode, builds `hl7pet_core::HierarchyProfile` once (reusing `crates/python/src/lib.rs`'s existing JSON-re-serialization pattern — `py.import("json")?.call_method1("dumps", (profile,))?` then `HierarchyProfile::from_json`, mapped through T005's `errors::hl7pet_profile_error` on failure); iterates T008's `messages_iter`, calling T011's `scan_message` then `extract_one` once per row; builds the return value via T006's `build_result_struct_array`.
- [X] T013 [US1] Register `extract_value` in `crates/arrow/src/lib.rs`'s `#[pymodule]` block (`m.add_function(wrap_pyfunction!(extract_value, m)?)?;`) and re-export it from `crates/arrow/python/hl7pet_arrow/__init__.py` (`from ._hl7pet_arrow import extract_value`, added to `__all__`).
- [X] T014 [P] [US1] Build locally: `cd crates/arrow && maturin develop`; confirm `import pyarrow as pa, hl7pet_arrow as ha; ha.extract_value(pa.array([open("../../fixtures/messages/baseline.hl7").read()]), "MSH-12")[0]` prints `{'value': [['2.5.1']], 'status': 'ok'}`, matching `fixtures/vectors/path/valid.json`'s `path-msh12` vector (quickstart.md step 1).
- [X] T015 [P] [US1] Write `crates/arrow/tests/test_parity.py` (fixtures-corpus parity, single-PATH only for now — extended by T020): mirrors `crates/python/tests/parity_check.py`'s vector-loading/family-skip conventions, iterating `fixtures/vectors/{path,hierarchy}/*.json`, calling `hl7pet_arrow.extract_value` on a one-row `pa.array([message])` per vector, and asserting the row's `status`/`value` matches what `hl7pet.get_value`/`hl7pet.get_value_hierarchy` return for that same vector (an `Hl7ScanError`/`Hl7QueryError` from the plain binding maps to `status == "error"`, per quickstart.md step 2 — Story 1 Acceptance Scenarios 1-3).
- [X] T016 [US1] Add a Rust unit test in `crates/arrow/src/lib.rs`'s `#[cfg(test)]` module covering Story 1 Acceptance Scenario 3 (a PATH matching nothing → `(None, RowStatus::NoMatch)`, never an `Err`) and FR-007 (a `None` message row → `(None, RowStatus::NoMatch)`), calling T011's `scan_message`/`extract_one` directly — no PyO3/Arrow boundary needed for this assertion.
- [X] T017 [US1] Run `pytest crates/arrow/tests/test_parity.py` (T015) and fix any mismatch until it's zero-mismatch clean (SC-003 for the single-PATH mechanism).

**Checkpoint**: US1's acceptance scenarios pass independently — `extract_value` is usable end-to-end for both non-hierarchy and hierarchy-mode PATHs, standalone against a PyArrow array. A working MVP even before US2/US3/US4 land.

---

## Phase 4: User Story 2 - Extract multiple fields across a column of HL7 messages in one pass (Priority: P1)

**Goal**: Prove multi-PATH extraction returns the same per-path results as
US1's `extract_value`, computed from exactly one scan per message regardless
of how many PATHs are requested.

**Independent Test**: Using the same fixtures-derived Table, run
`extract_values` with a list of PATHs that each have their own
`fixtures/vectors/path/` vector, and confirm every PATH's output matches its
own vector's `expected` value, row-for-row, in one call (spec.md's
Independent Test for US2).

- [X] T018 [US2] Implement `#[pyfunction] fn extract_values(py: Python<'_>, messages: pyo3_arrow::PyArray, paths: Vec<String>, profile: Option<&Bound<'_, PyAny>>) -> PyResult<pyo3_arrow::PyArray>` in `crates/arrow/src/lib.rs` per [contracts/arrow-api.md](contracts/arrow-api.md): raises T005's `errors::empty_paths_error()` immediately if `paths.is_empty()` (FR-008); calls T010's `validate_path` once per (possibly duplicate) requested path, building the shared `HierarchyProfile` **at most once** and reusing it across every hierarchy-mode path in `paths`; iterates T008's `messages_iter` **once**, and for each row calls T011's `scan_message` **once**, then `extract_one` once per requested `path` in `paths` (reusing the same scanned `Result` reference across all of them — the one-scan-per-message guarantee (SC-002) follows directly from T011's scan/execute split, not from anything `extract_values` itself has to get right). Builds the struct-of-structs return value via T006: one outer field per requested `path`, by position (duplicates allowed per contracts/arrow-api.md), each an inner Result Struct.
- [X] T019 [US2] Register `extract_values` in `crates/arrow/src/lib.rs`'s `#[pymodule]` block and re-export it from `crates/arrow/python/hl7pet_arrow/__init__.py`, alongside T013's `extract_value`.
- [X] T020 [US2] Extend `crates/arrow/tests/test_parity.py` (T015) to also call `hl7pet_arrow.extract_values` with 2+ paths per vector's message (grouping vectors that share a `message_ref`) and assert each outer field matches what `extract_value` alone returns for that same path — proving multi-PATH and single-PATH agree, not just that multi-PATH runs (Story 2 Acceptance Scenario 1).
- [X] T021 [P] [US2] Write `crates/arrow/tests/test_scan_count.py` (SC-002): an instrumented build or call-counting wrapper around `hl7pet_core::scan`/T011's `scan_message` (mirroring specs `009`/`1000`/`011`'s counting-allocator pattern, adapted to "count of `scan_message` calls" instead of allocation count) proving `extract_values` with N paths against the same message column performs exactly the same number of scans as `extract_value` with 1 path — fails if requesting more PATHs ever re-scans a message (Story 2 Acceptance Scenario 2). **Delivered as Rust unit tests in `crates/arrow/src/lib.rs`'s test module instead of a `.py` file**: a `thread_local!` counter (matching `crates/core/src/test_alloc.rs`'s exact counting-allocator pattern, not a shared `static`, so it's safe under parallel test threads) instruments `scan_message`, and the row/path loop itself was factored out of `extract_values` into a standalone `extract_rows_for_paths` (no PyO3 types) so the test exercises the *real* production code path directly, with no live Python interpreter needed in the test binary. More direct than an external black-box Python proxy could be, since Python has no visibility into internal scan counts.
- [X] T022 [US2] Add a Rust unit test in `crates/arrow/src/lib.rs`'s test module for the empty-`paths` rejection (FR-008: `extract_values` with `paths=[]` raises immediately, no row processed — Story 2 Acceptance Scenario 3) and the duplicate-PATH case (spec Edge Cases: the same PATH string twice in `paths` produces two independent outer fields, both computed). **Delivered as two Python-level tests in `test_parity.py` instead**: both behaviors are only observable through the PyO3 boundary (a raised `ValueError`, a `pyarrow.StructArray`'s actual duplicate-field-name behavior), so a real end-to-end call against the compiled extension is more direct than reconstructing PyO3/GIL plumbing inside a Rust unit test.
- [X] T023 [US2] Run `pytest crates/arrow/tests/test_parity.py crates/arrow/tests/test_scan_count.py` (T020/T021) to green.

**Checkpoint**: US1 and US2 both pass independently — `extract_values` computes every requested PATH from one scan per message (SC-002), matching `extract_value`'s own per-path results exactly.

---

## Phase 5: User Story 3 - Use the extraction mechanisms directly from PySpark (Priority: P2)

**Goal**: Prove both mechanisms are usable as ordinary PySpark DataFrame
column operations, with results identical to the standalone PyArrow path.

**Independent Test**: In a local PySpark session, build a DataFrame from the
fixtures corpus, apply single-PATH and multi-PATH extraction via the
DataFrame API, `.collect()` the result, and confirm it matches the non-Spark
PyArrow-only result for the same input (spec.md's Independent Test for US3).

- [X] T024 [US3] Spike (research.md #4): in a local PySpark 4.2.0 session, confirm whether `@pyspark.sql.functions.arrow_udf("struct<...>")` can return a `pa.StructArray` (needed for T026's multi-field output). Record the outcome as a one-line comment at the top of `crates/arrow/python/hl7pet_arrow/spark.py`: either "arrow_udf struct-output confirmed working" (proceed with T026 as designed) or "arrow_udf struct-output unsupported as of PySpark 4.2.0 — extract_values_udf falls back to DataFrame.mapInArrow internally" (contracts/arrow-api.md's documented fallback — a change to T026's implementation only, not to `extract_values_udf`'s public signature). **Confirmed working**, verified live against a real local PySpark 4.2.0 session (a struct-of-structs return type, `` struct<`p`: struct<value:..., status:...>> ``, round-tripped correctly) — no `mapInArrow` fallback needed. One real limitation found and documented (not silently mishandled): a Spark struct type string requires distinct field names, so `extract_values_udf` — unlike the standalone `extract_values` — does not support duplicate PATHs (module docstring, `crates/arrow/python/hl7pet_arrow/spark.py`).
- [X] T025 [US3] Implement `def extract_value_udf(path: str, profile: dict | None = None) -> pyspark.sql.column.Column` in `crates/arrow/python/hl7pet_arrow/spark.py` per [contracts/arrow-api.md](contracts/arrow-api.md): a factory returning an `@arrow_udf`-decorated closure over `hl7pet_arrow.extract_value` (T012), parameterized by `path`/`profile` at definition time, applied to a message column at call time (`extract_value_udf("PID-5.1")(df["message"])`) — Story 1 Acceptance Scenario 4.
- [X] T026 [US3] Implement `def extract_values_udf(paths: list[str], profile: dict | None = None) -> pyspark.sql.column.Column` in `crates/arrow/python/hl7pet_arrow/spark.py`, closing over `hl7pet_arrow.extract_values` (T018) the same way T025 does — using `@arrow_udf` with a struct return type if T024's spike confirmed support, or `DataFrame.mapInArrow` internally otherwise (T024's fallback) — Story 2 usable from Spark.
- [X] T027 [P] [US3] Write `crates/arrow/tests/test_spark.py`: a local (`local[1]`) PySpark session smoke test building a small `DataFrame` from `fixtures/messages/`, applying `extract_value_udf`/`extract_values_udf` (T025/T026) via `.withColumn`/`.select`, `.collect()`-ing the result, and asserting it matches T015/T020's non-Spark results for the same messages/PATHs (Story 3 Acceptance Scenarios 1-2).
- [X] T028 [US3] Run `pytest crates/arrow/tests/test_spark.py` (T027) to green.

**Checkpoint**: All three of US1/US2/US3 pass independently; both extraction mechanisms are usable from a PySpark DataFrame pipeline with results identical to standalone PyArrow usage (SC-004).

---

## Phase 6: User Story 4 - Evaluate hl7pet's Arrow support via a runnable demo (Priority: P3)

**Goal**: A new adopter can run one notebook top-to-bottom and see every
mechanism from US1-US3 working on real sample data.

**Independent Test**: Run the notebook top-to-bottom in a clean environment
with the project's declared dependencies installed and confirm every cell
executes without error and produces the output it narrates (spec.md's
Independent Test for US4).

- [ ] T029 [US4] Author `notebooks/arrow_pyspark_demo.ipynb` (replacing T004's placeholder): cells demonstrating (a) loading `fixtures/messages/` into a standalone `pyarrow.Table` and a local PySpark `DataFrame`, (b) `hl7pet_arrow.extract_value` on the PyArrow Table, (c) `hl7pet_arrow.extract_values` on the same Table, (d) hierarchy-mode PATHs via both mechanisms using a `fixtures/profiles/` profile, (e) `extract_value_udf`/`extract_values_udf` (T025/T026) applied to the PySpark DataFrame, and (f) a final comparison cell printing the row-by-row plain `hl7pet.get_value`/`get_values` result alongside the columnar `hl7pet_arrow` result for the same sample messages, asserting they agree (FR-011, SC-005 Acceptance Scenario 2).
- [ ] T030 [US4] Validate: `jupyter nbconvert --to notebook --execute notebooks/arrow_pyspark_demo.ipynb --output /tmp/arrow_pyspark_demo.out.ipynb` exits `0` with every cell executing without error (quickstart.md step 5, SC-005 Acceptance Scenario 1).

**Checkpoint**: All four user stories pass independently; a new adopter can run the notebook top-to-bottom and see every mechanism working on real data.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Confirm the feature's non-functional claims, document the
package, and run the full regression suite alongside this feature's new
tests.

- [ ] T031 [P] Write `crates/arrow/README.md` (mirrors `crates/python/README.md`'s Install/Quickstart/Testing structure): install via `maturin develop`, the `extract_value`/`extract_values`/`hl7pet_arrow.spark` quickstart snippets from `contracts/arrow-api.md`, and a link to `specs/6002-arrow-integration/contracts/arrow-api.md` for the full contract.
- [ ] T032 Run `cargo clippy --workspace --all-targets` and confirm clean, including the new `hl7pet-arrow` crate; run `cargo test --workspace` and confirm the full pre-existing suite (specs `005`-`1001`, `6000`-`6001`) passes unmodified alongside this feature's new tests (spec.md FR-004/SC-003).
- [ ] T033 Execute every step of [quickstart.md](quickstart.md) manually end-to-end (build, parity, scan-count, Spark, notebook) and confirm each documented "Expected outcome" holds.
- [ ] T034 Update `ROADMAP.md`'s spec `6002` Status row from "Draft" to "Implemented" with a summary of what shipped (mirroring specs `6000`/`6001`/`011`'s entries' level of detail), noting which of T001/T024's two flagged spikes needed their fallback (if either did) and updating the Language Bindings module's "Next free" if applicable.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately. T001's spike result (pyo3-arrow vs. hand-rolled `arrow::ffi`) affects T008/T012/T018's exact implementation but not their signatures/contracts.
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories. T007 depends on T006 (uses `RowStatus`); T011 depends on T006/T007; T010 is independent of T006/T007/T008 but still gates T012/T018.
- **User Stories (Phase 3-6)**: All depend on Foundational (Phase 2) completion.
  - US1 (Phase 3) and US2 (Phase 4) both call T011's `scan_message`/`extract_one` directly — US2 has no code dependency on US1's `extract_value` itself, only on the same Foundational helpers, so the two could be built in either order or in parallel by different people; this file orders US1 first only because it's the simpler consumer to verify T011 against.
  - US3 (Phase 5) depends on US1 (T012) and US2 (T018) existing — it wraps both as PySpark UDFs, no new extraction logic of its own.
  - US4 (Phase 6) depends on US1-US3 all existing — the notebook exercises every mechanism.
- **Polish (Phase 7)**: Depends on Phases 2-6 all being complete.

### Within Each User Story

- T012 (extract_value) before T013 (registration) before T014/T015/T016/T017 (build/tests) — Story 1.
- T018 (extract_values) before T019 (registration) before T020/T021/T022/T023 (tests) — Story 2. T020 extends T015's file, so it depends on T015 having landed first (not parallel with it).
- T024's spike before T026 (its outcome determines T026's implementation) — Story 3. T025 has no dependency on T024's outcome.
- Story complete before moving to the next priority, though US1/US2 have no code dependency on each other beyond Phase 2.

### Parallel Opportunities

- T002/T003/T004 (Setup) can all run in parallel once T001 lands (different files).
- T006 (Foundational) can be drafted in parallel with T005 (different files, no shared type); T007 depends on T006's `RowStatus` existing, and both must land before T011.
- T014/T015 (US1 build check + parity test) touch different files/activities and can run in parallel once T013 lands.
- T021 (US2 scan-count test) is a new file, parallel with T020 (extending T015's existing file) once T018/T019 land.
- T027 (US3 Spark test) is independent of T024-T026's own sequencing beyond needing T025/T026 to exist.
- T031 (README) can run in parallel with T032-T034 (all different files/activities).

---

## Parallel Example: User Story 1

```bash
# Launch both new checks for User Story 1 together, once T013 lands:
Task: "Local maturin develop + manual extract_value sanity check (quickstart.md step 1)"
Task: "Write crates/arrow/tests/test_parity.py fixtures-corpus parity check for extract_value"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (including T001's `pyo3-arrow`/`abi3` spike)
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: `pytest crates/arrow/tests/test_parity.py` passes for every single-PATH vector
5. `extract_value` is usable end-to-end standalone against PyArrow even before US2/US3/US4 land

### Incremental Delivery

1. Setup + Foundational → the Arrow/error/conversion plumbing exists and compiles
2. Add US1 → single-PATH extraction proven correct against the full fixtures corpus (MVP!)
3. Add US2 → multi-PATH extraction proven correct and provably one-scan-per-message
4. Add US3 → both mechanisms usable from a live PySpark session, matching standalone results
5. Add US4 → a runnable notebook ties all three together for a new adopter
6. Polish → README, full regression sweep, ROADMAP update

### Parallel Team Strategy

With multiple developers, once Foundational (Phase 2) is done: one developer
can take US1+US2 (they share T011 and are sequenced only by file, not by
logic), while another starts drafting US3's `spark.py` skeleton against
US1/US2's *contracts* (contracts/arrow-api.md) ahead of their landing, and a
third drafts US4's notebook structure/narrative ahead of having real output
to paste in.

---

## Notes

- [P] tasks = different files or independent test functions, no dependency on an incomplete task
- [Story] label maps task to specific user story for traceability
- Two implementation-time spikes are carried from research.md rather than
  pre-decided: T001 (`pyo3-arrow` under `abi3-py39`) and T024 (`arrow_udf`
  struct-output support). Both have designed fallbacks that don't change any
  function's public contract (contracts/arrow-api.md) — record which path was
  taken in T034's ROADMAP update.
- This feature has no Scala or prior-Python baseline to regress against
  (plan.md Performance Goals) — the shared fixtures corpus plus T015/T020's
  parity assertions against the existing plain `hl7pet` binding are the sole
  source of correctness truth, and T021's scan-count test is the sole hard
  performance claim (SC-002).
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
