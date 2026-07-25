---

description: "Task list for Query Execution"
---

# Tasks: Query Execution

**Input**: Design documents from `/specs/007-query-execution/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/query-api.md](contracts/query-api.md), [quickstart.md](quickstart.md)

**Tests**: Included as core deliverables, not an optional add-on — spec.md's three
user stories are each defined by an "Independent Test" that is a `cargo test`
invocation (quickstart.md), and SC-001 through SC-004 are only checkable by running
tests against conformance vectors. This is a library with no UI; the test suite *is*
the proof the feature works — same convention specs `005`/`006`'s tasks.md
established.

**Organization**: Tasks are grouped by user story (US1/US2/US3 from spec.md, in
priority order) so each can be implemented and verified independently. Because all
three stories are served by one `execute()` function, the Foundational phase carries
the full resolution algorithm (segment/field index resolution, filter evaluation,
error conditions) — there is no way to build "a plain PATH extracts a value" without
also building most of what "an index selector resolves to the right occurrence" and
"a filter selects the right occurrence" need, since they share the same
`extract_subvalue` navigation (research.md #3). Each story phase then adds the
vectors and tests that specifically prove *that story's* acceptance scenarios.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: the executor is added to the existing `hl7pet-core`
crate (`crates/core/`) as a new sibling module to specs `005`/`006`'s `scanner.rs`/
`parser.rs` — no new workspace member, no new Cargo dependency. New conformance
vectors extend spec `001`'s existing `fixtures/vectors/path/valid.json` in place — no
new schema or vector family (plan.md Structure Decision).

---

## Phase 1: Setup

**Purpose**: Stand up the module skeleton and test harness. The Cargo workspace,
`hl7pet-core` crate, `scanner.rs`, and `parser.rs` already exist (specs `005`/`006`) —
this phase is deliberately light.

- [X] T001 Create the empty module `crates/core/src/query.rs` (module-level doc comment only, per parser.rs/scanner.rs's precedent) and add `pub mod query;` to `crates/core/src/lib.rs` (no re-exports yet — added in T009 once the public surface exists).
- [X] T002 [P] Scaffold `crates/core/tests/query_vectors.rs`: a `serde`-deserializable `PathVector` struct covering `id`, `path`, `message_ref`, `method`, `expected` (as `serde_json::Value`, to accept the 2D-array/string/null shapes `conformance-vector.schema.json` allows), a loader reading `fixtures/vectors/path/valid.json` relative to the workspace root (mirroring `parser_vectors.rs`'s `fixtures_root()` helper), a helper to read a vector's `message_ref` `.hl7` file content, a filter excluding any vector whose `path` contains `" -> "` (out of scope, spec.md Assumptions), and a dispatch stub that compiles and runs as a no-op (assertions filled in by T010).

**Checkpoint**: `cargo build --workspace` succeeds with the new empty `query` module; `cargo test -p hl7pet-core --test query_vectors` compiles and passes vacuously.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared `execute()` happy path and error plumbing every user story
builds on. No user story's acceptance scenarios can be verified until this exists.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T003 Define `QueryError` in `crates/core/src/query.rs` per [data-model.md](data-model.md): a single variant, `NonNumericComparison { operator: FilterOperator }`, `Eq`, `Copy`, manual `Display`/`std::error::Error` impls (no derive-macro crate, matching `ScanError`/`ParseError`'s precedent). No out-of-range variants — verified against the real Scala engine (research.md #2's Verification note) that an out-of-range segment/field index returns no match there, not an error.
- [X] T004 Implement Occurrence Candidate resolution in `crates/core/src/query.rs` (data-model.md): given a `&ScanResult` and a segment name, filter `scan.segments` via `scan.segment_name()` to those matching, in message order, each paired with its 1-based occurrence number counted only among same-named segments (research.md #7) — a standalone internal function reused by both plain index resolution (T005) and filter-clause candidate iteration (T007).
- [X] T005 Implement `SegIndex` resolution in `crates/core/src/query.rs` against T004's candidates: `Numeric(n)` selects the nth candidate, or yields zero candidates (not an error) if `n` exceeds the candidate count (research.md #2, verified: `OBX[5]-5` against a 3-`OBX` message returns no match on the real engine); `Last` selects the final candidate, or zero candidates when there are none; `Star` (and `None`, treated identically per spec `006`'s data-model) selects every candidate, in order. `Filter(clause)` is deferred to T007.
- [X] T006 Implement `extract_subvalue` in `crates/core/src/query.rs` (research.md #3): given one candidate's segment content, the message's `DelimiterSet` (spec `005`), and a field number with optional component/subcomponent, split on the field delimiter to the requested field, then on the component/subcomponent delimiters as needed, returning the empty string `""` (not an error) when the requested level is beyond what that occurrence's content actually contains (spec.md FR-009(e)). Also implement `FieldIndex` resolution here for the repetition dimension: split the targeted field on the repetition delimiter; `Numeric(n)` selects the nth repetition, or yields no repetition (not an error) if `n` exceeds the repetitions present (research.md #2, verified: `OBX-5[5]` against a 2-repetition field returns no match on the real engine); `Last` selects the final repetition; `Star`/`None` selects every repetition in order.
- [X] T007 Implement `FilterClause` evaluation in `crates/core/src/query.rs`, composing T004 and T006 (research.md #3): for each Occurrence Candidate (in message order), call T006's extraction with the filter's target field/component/subcomponent to get its sub-value, then evaluate `operator` against each of `values` in turn (spec.md FR-007/FR-008) — `Eq`/`Ne` do direct string comparison; `Gt`/`Ge`/`Lt`/`Le` parse both sides via `str::parse::<f64>()` (research.md #4) and return `Err(QueryError::NonNumericComparison { operator })` if either side fails to parse (verified: the real Scala engine throws an uncaught `NumberFormatException` here — this is the one case this executor deliberately does not reproduce byte-for-byte, surfacing a typed error instead); a candidate is selected if any one OR'd value comparison succeeds. Zero candidates satisfying the filter is not an error (research.md #2(d)) — it simply contributes no entries to the result.
- [X] T008 Implement the public entry point `pub fn execute<'m>(scan: &ScanResult<'m>, path: &CompiledPath<'_>) -> Result<Vec<Vec<&'m str>>, QueryError>` in `crates/core/src/query.rs`, composing T004-T007: resolve `path.segment` (T005/T007) to a set of targeted candidates; for each, if `path.field` is `None` return the full raw segment content as a single-element inner vec (research.md #5), else use T006 to produce one inner vec per resolved `FieldIndex` position — but if that inner vec is empty (the requested field-repetition index was out of range for this occurrence), drop the occurrence from the result entirely rather than keeping an empty inner vec (verified against the real engine: `OBX-5[5]` beyond the repetitions present collapses to no match entirely, not `[[]]` — a real bug caught by the conformance vector suite during implementation, not anticipated in the original design). Returns `Ok(vec![])`, never `Err`, when zero candidates are targeted for any reason — segment absent, index out of range, or filter no-match (research.md #2). `path.child` MUST be ignored per contracts/query-api.md's precondition (spec `008`'s responsibility, not an error here).
- [X] T009 [P] Update `crates/core/src/lib.rs`: add `pub mod query;`'s re-exports (`pub use query::{execute, QueryError};`) matching contracts/query-api.md's public surface exactly.
- [X] T010 [P] Fill in `crates/core/tests/query_vectors.rs`'s dispatch body (stubbed in T002): for every non-hierarchy entry in `fixtures/vectors/path/valid.json`, scan `message_ref`'s file content, parse `path`, call `execute()`, and assert the result matches `expected` exactly — for `method == "getValue"`, compare the full `Vec<Vec<&str>>` (or `null`/empty per research.md #2) against `expected`'s 2D-array/null shape; for `method == "getFirstValue"`, derive `Option<&str>` per contracts/query-api.md's documented derivation and compare against `expected`'s string/null shape. `expected: "ERROR:NonNumericComparison"` (T017's sentinel convention) asserts `execute()` returns `Err(QueryError::NonNumericComparison { .. })` instead.
- [X] T011 [P] Add a single-pass/allocation-scaling unit test in `crates/core/src/query.rs` (SC-004), reusing `crate::test_alloc::count_allocs` (the existing crate-internal counting-allocator shim specs `005`/`006` already established): construct a message with a segment containing many repetitions/components and confirm total allocations scale with the *output* size (number of matched occurrences × resolved repetitions), not with message size or filter-candidate count — proving `execute()` does not re-scan the full message or re-split a candidate's content redundantly per filter comparison.

**Checkpoint**: `cargo build --workspace` and `cargo test -p hl7pet-core` both succeed; `execute()` is fully implemented for every code path in spec.md FR-001–FR-013, verified against the 13 existing non-hierarchy `fixtures/vectors/path/valid.json` vectors — not yet against this spec's 6 new ones (added in Phases 3-5).

---

## Phase 3: User Story 1 - A single-value PATH returns the correct field/component/subcomponent (Priority: P1) 🎯 MVP

**Goal**: Prove a plain (non-indexed, non-filter) PATH extracts exactly the right
substring — full segment, full field, or a specific component/subcomponent.

**Independent Test**: For every non-hierarchy, non-filter vector in
`fixtures/vectors/path/valid.json`, scan its message, execute its compiled PATH, and
confirm the extracted value matches the vector's Scala-verified expected value
exactly (spec.md US1 Independent Test).

### Implementation for User Story 1

- [X] T012 [P] [US1] Add a new entry to `fixtures/vectors/path/valid.json` (FR-014, research.md #5/#6): `path-segment-only`, a bare segment PATH with no field expression (e.g. `PID` against `messages/baseline.hl7`), `method: "getValue"`, `expected` computed by running the equivalent Scala call against the real library (spec `004`'s Maven Central dependency, same verification policy spec `006`'s T016 used) — a single-element outer vec containing the full raw `PID` segment line, unsplit.
- [X] T013 [US1] Add unit tests in `crates/core/src/query.rs` asserting T006's `extract_subvalue` behavior directly for each level (field only, field+component, field+component+subcomponent) against representative segment content, including the FR-009(e) case: a requested component/subcomponent number beyond what the field's content contains returns `""`, not an error and not a panic.

**Checkpoint**: User Story 1 fully functional and independently verified — `cargo test -p hl7pet-core --test query_vectors` passes for every non-filter, non-indexed, non-hierarchy vector including T012's new one; T013's direct extraction-level tests pass.

---

## Phase 4: User Story 2 - Segment and field index selectors resolve to the right occurrence (Priority: P1)

**Goal**: Prove `Numeric`/`$LAST`/`*`/omitted segment and field index selectors each
resolve to the correct occurrence(s), and that an explicit out-of-range index
produces no match — verified against the real Scala engine to be its actual
behavior, never a panic and never a value from the wrong occurrence.

**Independent Test**: For every vector whose PATH includes a segment or field index
selector, execute it and confirm the result matches the specific occurrence(s) the
vector's expected value identifies (spec.md US2 Independent Test).

### Implementation for User Story 2

- [X] T014 [P] [US2] Add 2 new entries to `fixtures/vectors/path/valid.json` (FR-014, research.md #6), verified against the real Scala library (`gov.cdc:hl7-pet_2.13:1.2.11`, spec `004`'s Maven Central dependency) per spec `006`'s T016 policy:
  - `path-segidx-out-of-range`: `OBX[5]-5` against `messages/multi-obx.hl7` (only 3 `OBX` occurrences present), `expected: null` — confirmed live against `HL7StaticParser.getValue`, which returns `None` here, not an error.
  - `path-fieldidx-out-of-range`: `OBX-5[5]` against `messages/multi-repetition.hl7` (field 5 has only 2 repetitions), `expected: null` — confirmed the same way.
- [X] T015 [US2] Extend `crates/core/tests/query_vectors.rs` (T010's dispatch) to cover `method: "getFirstValue"` vectors whose `expected` is `null` (already handled by T010's general dispatch — this task is the explicit check that T014's 2 new vectors round-trip correctly through the existing `null`-shape comparison, no new dispatch branch needed).
- [X] T016 [US2] Add unit tests in `crates/core/src/query.rs` directly asserting: (a) `SegIndex::Numeric` selects the correct 1-based occurrence among several same-named segments; (b) `SegIndex::Last` selects the final occurrence regardless of count; (c) `SegIndex::Star`/omitted selects every occurrence in message order; (d) `FieldIndex::Numeric`/`Last`/`Star` resolve analogously among a field's `~`-delimited repetitions; (e) both out-of-range conditions from T005/T006 resolve to zero candidates/repetitions (`Ok(vec![])`), never a panic and never `Err`.

**Checkpoint**: User Story 2 fully functional and independently verified — `cargo test -p hl7pet-core --test query_vectors` passes including T014's 2 new vectors; T016's direct resolution-rule tests pass.

---

## Phase 5: User Story 3 - A filter clause selects the matching segment occurrence (Priority: P2)

**Goal**: Prove a filter clause (single or OR'd values, any of the six operators)
selects exactly the occurrence(s) whose targeted sub-value satisfies it, that zero or
multiple matches are each handled correctly, and that a non-numeric operand compared
with an ordering operator is a distinguishable error.

**Independent Test**: For every filter vector in `fixtures/vectors/path/valid.json`,
execute the compiled filter PATH and confirm the executor selects exactly the
occurrence(s) the vector's expected value identifies (spec.md US3 Independent Test).

### Implementation for User Story 3

- [X] T017 [P] [US3] Add 3 new entries to `fixtures/vectors/path/valid.json` (FR-014, research.md #6), verified against the real Scala library (`gov.cdc:hl7-pet_2.13:1.2.11`, spec `004`'s Maven Central dependency) per spec `006`'s T016 policy:
  - `path-filter-no-match`: `OBX[@3.1='NO-SUCH-CODE']-5` against `messages/multi-obx.hl7`, `expected: null` — mirrors the existing `path-zero-values-nonexistent` convention, confirmed live.
  - `path-filter-multi-match`: `OBX[@11='F']-5` against `messages/multi-obx.hl7` (all 3 `OBX` occurrences share observation-result-status `F` in field 11 — no new synthetic message needed), `expected: [["Positive"], ["Negative"], ["Equivocal"]]` in message order — confirmed live against `HL7StaticParser.getValue`.
  - `path-filter-nonnumeric-ordering`: `OBX[@5>'100']-5` against `messages/multi-obx.hl7` (field 5 holds non-numeric text like `"Positive"`) — confirmed live that the real engine throws an uncaught `NumberFormatException` here, i.e. has no graceful byte-for-byte behavior to reproduce (research.md #4). This is the **one** vector in the whole corpus that documents an execution-time error rather than a value/`null`: record `expected: "ERROR:NonNumericComparison"` (a new sentinel, analogous to `invalid.json`'s parse-time `"INVALID"` sentinel but scoped to this single vector, per T010's dispatch handling).
- [X] T018 [US3] Add unit tests in `crates/core/src/query.rs` directly asserting: (a) each of the six `FilterOperator` variants evaluates correctly for a matching and a non-matching candidate; (b) an OR'd multi-value filter (`values.len() > 1`) matches if any one value succeeds; (c) a filter targeting a subcomponent navigates through T006 correctly; (d) an ordering operator against two numeric-parseable sides compares correctly in both directions; (e) an ordering operator against a non-numeric side returns `QueryError::NonNumericComparison`, never silently `false` and never a panic (spec.md FR-008 Acceptance Scenario 4).

**Checkpoint**: All three user stories independently functional — filter-based selection (single match, zero match, multi-match, all six operators, non-numeric rejection) is proven both via the shared vector suite and dedicated unit tests.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Tie the three stories together into one verified, documented deliverable.

- [X] T019 [P] Run `python3 fixtures/scripts/validate_corpus.py` against the full corpus including this spec's 6 new `path` entries; confirm ids remain unique corpus-wide and every `message_ref` resolves.
- [X] T020 [P] Update `crates/core/README.md` to mention the `query` module, linking to [quickstart.md](quickstart.md) and [contracts/query-api.md](contracts/query-api.md) rather than duplicating them.
- [X] T021 Run the full [quickstart.md](quickstart.md) validation end-to-end (all 7 steps) and record the outcome.
- [X] T022 Update [ROADMAP.md](../../ROADMAP.md)'s spec `007` status row from "Spec drafted" to "Implemented," noting the new `crates/core/src/query.rs` module, the `fixtures/vectors/path/` count (27 total: 20 valid — 14 existing + 6 new — plus 7 invalid, unchanged), and that it's ready for spec `008` (lazy hierarchy navigation) to build against `execute()`'s output for non-hierarchy sub-paths.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup (T001's module must exist) — BLOCKS all user stories.
- **User Stories (Phase 3-5)**: All depend on Foundational completion.
  - US1 (T012-T013) has no dependency on US2/US3.
  - US2 (T014-T016) depends only on Foundational (`execute()`, `QueryError` must exist) — independently testable in parallel with US1/US3.
  - US3 (T017-T018) depends only on Foundational — independently testable in parallel with US1/US2.
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### Within Phase 2 (Foundational)

- T003 has no dependencies — the error type comes first.
- T004 depends on T003 only loosely (no direct type dependency, but is sequenced after it); has no dependency on T005-T007.
- T005 depends on T004 (needs Occurrence Candidates to resolve against).
- T006 has no dependency on T003-T005 (its field/repetition resolution never returns `QueryError` — out-of-range yields zero repetitions, not an error).
- T007 depends on T004 (candidate iteration) and T006 (sub-value extraction for filter targets).
- T008 depends on T005, T006, T007 (composes segment resolution, field/repetition resolution, and filter resolution into the public entry point).
- T009 depends on T003, T008 (re-exports need the finished public surface).
- T010 depends on T002 (harness scaffold) and T008 (needs `execute()` to call).
- T011 depends on T008 (needs a complete `execute()` to measure).

### Within Phase 4 (US2) / Phase 5 (US3)

- T014/T017 (new vectors) have no code dependency — can be authored as soon as Foundational's message fixtures are known, in parallel with T015-T016/T018.
- T015 depends on T010 (verifies T014's vectors round-trip through the existing dispatch) and T014 (needs the vectors to exist).
- T016/T018 depend only on T008 (`execute()` must exist) — parallelizable with T014/T015/T017.

### Parallel Opportunities

- Setup: T001 and T002 in parallel (different files).
- Foundational: T009 and T010 in parallel once T008 lands; T011 in parallel with T009/T010 (different concern, same file as T003-T008 — coordinate on merge).
- User Stories: US1, US2, and US3 can all proceed in parallel once Foundational is done — each exercises an independent concern (plain extraction, index resolution, filter resolution) against the same finished `execute()`.
- Within US1: T012 and T013 in parallel (different files).
- Within US2: T014 in parallel with T016; T015 depends on T014.
- Within US3: T017 in parallel with T018.
- Polish: T019 and T020 in parallel.

---

## Parallel Example: Foundational Phase

```bash
# T009 and T010 together once T008's execute() lands (different files):
Task: "Update lib.rs re-exports for the query module"
Task: "Fill in query_vectors.rs dispatch body"
```

## Parallel Example: User Stories (post-Foundational)

```bash
# Launch all three stories together once Phase 2 is complete:
Task: "US1: segment-only vector + extraction-level unit tests"
Task: "US2: out-of-range vectors + index-resolution unit tests"
Task: "US3: filter vectors (no-match/multi-match/non-numeric) + operator unit tests"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (blocks everything else — implements the full
   resolution algorithm, since plain extraction, index resolution, and filter
   resolution all share `extract_subvalue`).
3. Complete Phase 3: User Story 1 — proves the base case every other capability
   builds on (a plain PATH extracts the right value).
4. **STOP and VALIDATE**: run quickstart.md steps 1-3.

### Incremental Delivery

1. Setup + Foundational → `execute()` exists and is verified against the 13 existing
   non-hierarchy vectors, but this spec's own new vectors and error conditions aren't
   yet proven.
2. US1 → plain single-value extraction proven (MVP) — the exact capability spec
   `006`'s compiled PATHs had nothing to exercise before this spec.
3. US2 → index-selector resolution and out-of-range errors proven — the repeated-
   segment/repeated-field correctness bar real HL7 traffic requires.
4. US3 → filter-based selection proven, including the zero/multi-match and
   non-numeric-comparison edge cases.
5. Polish → full quickstart validation + docs + `ROADMAP.md` update, readying spec
   `008` to build hierarchy navigation on top of `execute()`.

---

## Notes

- [P] tasks touch different files (or independent regions of the same file with no
  shared state) and have no unmet dependency within their phase.
- [Story] labels (US1/US2/US3) map directly to spec.md's prioritized user stories.
- US1 and US2 are both P1 in spec.md; they are sequenced here (US1 before US2) only for
  narrative clarity — both depend solely on Foundational, not on each other, and can
  genuinely run in parallel once Phase 2 is complete.
- T012/T014/T017's new vectors MUST be verified against the real Scala library per
  spec `006`'s T016 precedent (research.md #6), not hand-derived — these vectors join
  the shared corpus later specs will also rely on as ground truth. This verification
  is what caught a real planning mistake before any Rust code was written: an earlier
  draft of this spec assumed an out-of-range segment/field index should be a
  distinguishable error (citing Constitution Principle III's "out-of-range field
  index in splitFields" example); running `OBX[5]-5`/`OBX-5[5]` against the real
  engine showed both return `None`, not a thrown exception — so T014's two vectors
  use `expected: null` like any other no-match vector, not an error sentinel.
- T017's `path-filter-nonnumeric-ordering` is the **only** vector in the whole corpus
  using an execution-time error sentinel (`"ERROR:NonNumericComparison"` in
  `expected`) — no existing vector encodes an execution-time error today (the existing
  `invalid.json` sentinel `"INVALID"` is parse-time only, spec `006`'s concern).
  Confirm this doesn't collide with `conformance-vector.schema.json`'s `expected`
  field, which already accepts an arbitrary string (getFirstValue's shape) — no schema
  change needed, but T019's corpus validation run should be treated as the check that
  confirms this, not assumed in advance.
- Commit after each task or logical group; stop at any checkpoint to validate a story
  independently before continuing.
