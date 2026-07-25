---

description: "Task list for PATH Parser"
---

# Tasks: PATH Parser

**Input**: Design documents from `/specs/006-path-parser/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/path-parser-api.md](contracts/path-parser-api.md), [quickstart.md](quickstart.md)

**Tests**: Included as core deliverables, not an optional add-on — spec.md's three user
stories are each defined by an "Independent Test" that is a `cargo test` invocation
(quickstart.md), and SC-001 through SC-004 are only checkable by running tests against
conformance vectors. This is a library with no UI; the test suite *is* the proof the
feature works — same convention spec `005`'s tasks.md established.

**Organization**: Tasks are grouped by user story (US1/US2/US3 from spec.md, in
priority order) so each can be implemented and verified independently. Because all
three stories are served by one `parse()` function, the Foundational phase carries the
full recursive-descent parser for the entire grammar (there is no way to build "reject
malformed input" without also building "accept well-formed input" — they're the same
code path); each story phase then adds the vectors and tests that specifically prove
*that story's* acceptance scenarios.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: the parser is added to the existing `hl7pet-core`
crate (`crates/core/`) as a new sibling module to spec `005`'s `scanner.rs` — no new
workspace member, no new Cargo dependency. New conformance vectors extend spec `001`'s
existing `fixtures/vectors/path/valid.json`/`invalid.json` in place — no new schema or
vector family (plan.md Structure Decision).

---

## Phase 1: Setup

**Purpose**: Stand up the module skeleton and test harness. The Cargo workspace and
`hl7pet-core` crate already exist (spec `005`) — this phase is deliberately light.

- [X] T001 Create the empty module `crates/core/src/parser.rs` (module-level doc comment only) and add `pub mod parser;` to `crates/core/src/lib.rs` (no re-exports yet — added in T009 once the public surface exists).
- [X] T002 [P] Scaffold `crates/core/tests/parser_vectors.rs`: `serde`-deserializable structs mirroring `fixtures/schemas/conformance-vector.schema.json` (already registered for the `path` family — no schema change needed, plan.md Structure Decision), a loader that reads `fixtures/vectors/path/valid.json` and `fixtures/vectors/path/invalid.json` relative to the workspace root, and a dispatch stub that compiles and runs as a no-op (assertions filled in by T010).

**Checkpoint**: `cargo build --workspace` succeeds with the new empty `parser` module; `cargo test -p hl7pet-core --test parser_vectors` compiles and passes vacuously.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared `parse()` happy path and error plumbing every user story
builds on. No user story's acceptance scenarios can be verified until this exists.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T003 Define the core data types in `crates/core/src/parser.rs` per [data-model.md](data-model.md): `SegIndex<'a>`, `SegmentExpr<'a>`, `FieldIndex`, `FieldExpr`, `FilterOperator` (6 variants), `FilterClause<'a>`, `ChildPath<'a>`, `CompiledPath<'a>` (all `Eq`, `Clone`), and `ParseErrorKind` (9 variants: `InvalidSegmentName`, `InvalidSegIndex`, `InvalidFieldIndex`, `InvalidOperator`, `UnterminatedFilter`, `UnexpectedSeparator`, `MultipleHierarchyHops`, `UnexpectedEnd`, `TrailingInput`) + `ParseError { kind, offset }` with manual `Display`/`std::error::Error` impls (no derive-macro crate, per research.md #1).
- [X] T004 Implement `SEGMENT_EXPR` parsing in `crates/core/src/parser.rs`: the 3-character alpha-first `SEG` check (reusing spec `001`'s tightened rule, `ParseErrorKind::InvalidSegmentName` on violation) followed by an optional `[SEG_IDX]` bracket recognizing `NUMBER`, `$LAST`, `*`, or a filter clause (delegates to T005), returning `ParseErrorKind::InvalidSegIndex` for anything else — a standalone function reusable for both the top-level segment and the `CHILD_PATH` segment (T007).
- [X] T005 Implement `FILTER` clause parsing in `crates/core/src/parser.rs`: `@field_num[.comp_num[.subcomp_num]]`, optional whitespace around the operator (grammar Note #4), the six-token `OPERATOR` set (`ParseErrorKind::InvalidOperator` for anything else, per spec `001` Notes #3), and `'value||value...'` split on `||` into `FilterClause.values` (never empty by construction, research.md #4) — `ParseErrorKind::UnterminatedFilter` if the closing `'` is missing.
- [X] T006 Implement `FIELD_EXPR` parsing in `crates/core/src/parser.rs`: `field_num` followed by an optional `[FIELD_IDX]` bracket (`NUMBER`/`$LAST`/`*`, else `ParseErrorKind::InvalidFieldIndex`) and an optional `.comp_num[.subcomp_num]` suffix, returning `ParseErrorKind::UnexpectedSeparator` when `.`/`-` appear in the wrong position relative to `SEGMENT_EXPR` (e.g. `OBX[1].5`) — a standalone function reusable for both the top-level field expression and `CHILD_PATH`'s field expression.
- [X] T007 Implement top-level `PATH` parsing in `crates/core/src/parser.rs`, composing T004/T006: `SEGMENT_EXPR ["-" FIELD_EXPR]` or `SEGMENT_EXPR " -> " CHILD_PATH` (where `CHILD_PATH` is itself `SEGMENT_EXPR ["-" FIELD_EXPR]`, single-hop only per `contracts/path-grammar.md`'s Non-Goals). Reject a second `" -> "` following a `CHILD_PATH` with `ParseErrorKind::MultipleHierarchyHops`; reject an empty/whitespace-only string or trailing `-`/`->` with `ParseErrorKind::UnexpectedEnd`; reject any unconsumed characters after a fully-matched `PATH` with `ParseErrorKind::TrailingInput`.
- [X] T008 Implement the public entry point `pub fn parse(path: &str) -> Result<CompiledPath<'_>, ParseError>` in `crates/core/src/parser.rs`, composing T007 — the sole public function this spec adds (contracts/path-parser-api.md).
- [X] T009 [P] Update `crates/core/src/lib.rs`: add re-exports (`pub use parser::{parse, CompiledPath, SegmentExpr, SegIndex, FieldExpr, FieldIndex, FilterClause, FilterOperator, ChildPath, ParseError, ParseErrorKind};`) matching contracts/path-parser-api.md's public surface exactly.
- [X] T010 [P] Fill in `crates/core/tests/parser_vectors.rs`'s dispatch body (stubbed in T002): for every entry in `fixtures/vectors/path/valid.json`, assert `parse()` returns `Ok`; for every entry in `invalid.json` (identified by `expected == "INVALID"`), assert `parse()` returns `Err`. This spec checks accept/reject only — asserting the *shape* of a successful `CompiledPath` is US3's job (T017).

**Checkpoint**: `cargo build --workspace` and `cargo test -p hl7pet-core` both succeed; `parse()` is fully implemented for every code path in spec.md FR-001–FR-009, verified against the existing 17 `fixtures/vectors/path/` vectors (11 valid, 6 invalid) — not yet against this spec's 4 new ones (added in Phases 3/5).

---

## Phase 3: User Story 1 - Malformed PATHs are rejected at parse time with a precise reason (Priority: P1) 🎯 MVP

**Goal**: Prove every malformed PATH is rejected with a specific, located
`ParseErrorKind` — never a panic, never a partial result.

**Independent Test**: Feed the parser every entry in `fixtures/vectors/path/invalid.json`
and confirm each is rejected distinctly (spec.md US1 Independent Test).

### Implementation for User Story 1

- [X] T011 [P] [US1] Add a new entry to `fixtures/vectors/path/invalid.json` (FR-012): `invalid-multihop-hierarchy`, path `ORC[1] -> OBR[1] -> OBX-5`, `message_ref` any existing message (unused — the string is rejected before any message would be touched), `expected: "INVALID"`, `grammar_productions: ["PATH", "CHILD_PATH"]`. No Scala verification needed for invalid vectors (research.md #2 applies only to the 3 new *valid* vectors) — rejection is a direct consequence of the current single-hop grammar (`contracts/path-grammar.md` Non-Goals).
- [X] T012 [US1] Add a panic-safety unit test in `crates/core/src/parser.rs` feeding `parse()` a battery of pathological inputs (empty string, whitespace-only string, a lone `-`, a lone `@`, an unterminated filter, a string of only `->`, non-ASCII bytes, a very long garbage string) and asserting every call returns a `Result` — never panics — per contracts/path-parser-api.md's explicit no-panic postcondition (mirrors spec `005`'s T024).
- [X] T013 [US1] Add one unit test per `ParseErrorKind` variant not already exercised by an existing `fixtures/vectors/path/invalid.json` entry — specifically `MultipleHierarchyHops` (T011's case, tested directly rather than only via the vector), `UnexpectedEnd` (empty string), and `TrailingInput` (e.g. `PID-1 extra`) — each asserting the exact variant and a correct `offset`.

**Checkpoint**: User Story 1 fully functional and independently verified — `cargo test -p hl7pet-core --test parser_vectors -- invalid` (all 7 invalid vectors) and T012's panic-safety test both pass; every `ParseErrorKind` variant is reachable and offset-correct.

---

## Phase 4: User Story 2 - A parsed PATH is compiled once and reused across many messages (Priority: P1)

**Goal**: Prove `parse()` is a pure function of the PATH string alone, and that a
`CompiledPath` can be reused across simulated downstream call sites without
re-parsing.

**Independent Test**: Parse a single valid PATH string in isolation from any message,
and confirm the result can be reused across multiple (simulated) evaluation calls
without re-parsing (spec.md US2 Independent Test).

### Implementation for User Story 2

- [X] T014 [US2] Add a `reuse_without_reparse` unit test in `crates/core/src/parser.rs` (quickstart.md step 5): using the same crate-free counting-allocator shim technique spec `005`'s T014 established, parse one PATH once and pass shared (`&CompiledPath`) references to several simulated call sites, asserting total heap allocations equal exactly what a single `parse()` call performs (data-model.md: 0 or 1, depending on whether a filter clause is present) regardless of how many times the compiled result is subsequently passed around — proving SC-004 by construction (research.md #5).
- [X] T015 [P] [US2] Add a `parse_is_pure` unit test in `crates/core/src/parser.rs`: parsing the same PATH string twice (including one filter-bearing and one hierarchy-bearing PATH) produces observably `Eq` `CompiledPath` values each time, and confirm `parse()`'s signature takes only a `&str` with no message/scanner/profile parameter — establishing FR-009's purity claim is testable, not just asserted in prose.

**Checkpoint**: User Story 2 fully functional and independently verified — T014 and T015 both pass; `parse()` requires no HL7 message input and its output is provably reusable and deterministic.

---

## Phase 5: User Story 3 - The compiled PATH exposes structured fields, not just a validated string (Priority: P2)

**Goal**: Prove the compiled representation surfaces segment/field/filter/hierarchy
pieces as distinct, individually-readable data for every grammar shape.

**Independent Test**: Parse a representative PATH from each shape in
`fixtures/vectors/path/valid.json` and confirm the compiled result exposes each piece
as structured data (spec.md US3 Independent Test).

### Implementation for User Story 3

- [X] T016 [P] [US3] Add 3 new entries to `fixtures/vectors/path/valid.json` (FR-012), each with `expected`/`expected_lines` computed by running the expression against the real Scala library per research.md #2 (spec `004`'s Maven Central dependency), not hand-derived:
  - `path-filter-orvalues`: `OBX[@3.1='94500-6||85477-8']-5` against `messages/filter-example.hl7` (OR match across both `OBX` occurrences).
  - `path-filter-subcomponent`: a filter targeting a subcomponent (`@field.comp.subcomp`) — author or reuse a `fixtures/messages/` file containing a real subcomponent value (`&`-separated) so the vector is meaningful, not just syntactically parseable.
  - `path-filter-operator-whitespace`: `OBX[@3.1 = '94500-6']-5` against `messages/filter-example.hl7`, expected identical to the existing `path-filter-single-clause` vector.
- [X] T017 [US3] Extend `crates/core/tests/parser_vectors.rs` (beyond T010's accept/reject check) to assert, for a curated subset covering every grammar shape (bare segment, numeric/`$LAST`/`*` index, field+component+subcomponent, filter with operator/values, hierarchy hop), that specific `CompiledPath` fields match expectations — not just that parsing succeeded (spec.md US3 Acceptance Scenarios 1-3).
- [X] T018 [P] [US3] Add a `compiled_shape` unit test group in `crates/core/src/parser.rs` (quickstart.md step 6) asserting compiled-field shape directly for each PATH form (bare segment, `$LAST`, `*`, numeric index, field+component+subcomponent, filter with subcomponent, OR'd filter values, hierarchy hop) — independent of the fixtures corpus, fast, no file I/O.

**Checkpoint**: All three user stories independently functional — the compiled representation's structured fields are directly asserted for every grammar shape, not just inferred from successful parsing.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Tie the three stories together into one verified, documented deliverable.

- [X] T019 [P] Run `python3 fixtures/scripts/validate_corpus.py` against the full corpus including this spec's 4 new `path` entries; confirm ids remain unique corpus-wide, every `message_ref` resolves, and the grammar-production coverage report reflects any newly-exercised combinations (e.g. `FILTER` + subcomponent together).
- [X] T020 [P] Update `crates/core/README.md` to mention the `parser` module, linking to [quickstart.md](quickstart.md) and [contracts/path-parser-api.md](contracts/path-parser-api.md) rather than duplicating them.
- [X] T021 Run the full [quickstart.md](quickstart.md) validation end-to-end (all 7 steps) and record the outcome.
- [X] T022 Update [ROADMAP.md](../../ROADMAP.md)'s spec `006` status row from "Planned" to "Implemented," noting the new `crates/core/src/parser.rs` module, the `fixtures/vectors/path/` count (21 total: 14 valid, 7 invalid), and that it's ready for spec `007` (query execution) to build against.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup (T001's module must exist) — BLOCKS all user stories.
- **User Stories (Phase 3-5)**: All depend on Foundational completion.
  - US1 (T011-T013) has no dependency on US2/US3.
  - US2 (T014-T015) depends only on Foundational (`parse()`, `CompiledPath` must exist) — independently testable in parallel with US1/US3.
  - US3 (T016-T018) depends only on Foundational — independently testable in parallel with US1/US2, though T016's new vectors are also exercised by T019 in Polish.
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### Within Phase 2 (Foundational)

- T003 has no dependencies — the type definitions come first.
- T004 depends on T003 (needs `SegmentExpr`/`SegIndex`/`ParseErrorKind` to exist) and calls T005 for the filter alternative.
- T005 depends on T003; used by T004.
- T006 depends on T003; has no dependency on T004/T005.
- T007 depends on T004, T006 (composes both for the top-level and `CHILD_PATH` forms).
- T008 depends on T007.
- T009 depends on T003, T008 (re-exports need the finished public surface).
- T010 depends on T002 (harness scaffold) and T008 (needs `parse()` to call).

### Within Phase 3 (US1)

- T011 has no dependencies.
- T012, T013 depend only on T008 (`parse()` must exist) — parallelizable with each other and with T011.

### Parallel Opportunities

- Setup: T001 and T002 in parallel (different files).
- Foundational: T005 and T006 in parallel once T003 lands (independent grammar productions, same file — coordinate on merge); T004 depends on T005; T009 and T010 in parallel once T008 lands.
- User Stories: US1, US2, and US3 can all proceed in parallel once Foundational is done — each exercises an independent concern (rejection, reuse/purity, structured shape) against the same finished `parse()`.
- Within US1: T012 and T013 in parallel; T011 in parallel with both (different file).
- Within US2: T014 and T015 in parallel.
- Within US3: T016 (fixtures) in parallel with T018 (unit tests); T017 depends on T016's new vectors existing.
- Polish: T019 and T020 in parallel.

---

## Parallel Example: Foundational Phase

```bash
# T005 and T006 together once T003's types exist (independent grammar productions, same file):
Task: "Implement FILTER clause parsing in parser.rs"
Task: "Implement FIELD_EXPR parsing in parser.rs"
```

## Parallel Example: User Stories (post-Foundational)

```bash
# Launch all three stories together once Phase 2 is complete:
Task: "US1: panic-safety and ParseErrorKind coverage tests"
Task: "US2: reuse-without-reparse and purity tests"
Task: "US3: new vectors + structured-field assertions"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (blocks everything else — implements the full
   grammar, since accept and reject are the same code path).
3. Complete Phase 3: User Story 1 — proves the exact defect this spec exists to fix
   (malformed PATHs rejected at parse time, never a panic).
4. **STOP and VALIDATE**: run quickstart.md steps 1-4.

### Incremental Delivery

1. Setup + Foundational → `parse()` exists and is verified against the existing 17
   vectors, but this spec's own new vectors and reuse/structure claims aren't yet
   proven.
2. US1 → precise rejection proven (MVP) — the roadmap's core reason for hand-writing
   this parser instead of reusing the Scala regex.
3. US2 → reuse-without-reparse and purity proven — the foundation spec `007` needs.
4. US3 → structured compiled-field shape proven — the contract spec `007`/`008`
   consume.
5. Polish → full quickstart validation + docs + `ROADMAP.md` update.

---

## Notes

- [P] tasks touch different files (or independent regions of the same file with no
  shared state) and have no unmet dependency within their phase.
- [Story] labels (US1/US2/US3) map directly to spec.md's prioritized user stories.
- US1 and US2 are both P1 in spec.md; they are sequenced here (US1 before US2) only for
  narrative clarity — both depend solely on Foundational, not on each other, and can
  genuinely run in parallel once Phase 2 is complete.
- T016's new vectors MUST be verified against the real Scala library per research.md
  #2, not hand-derived — even where the OR/subcomponent semantics look obvious from the
  grammar alone, since these vectors join the shared corpus spec `007` will also rely
  on for its own ground truth.
- Implementation finding during T016 (verified against `gov.cdc:hl7-pet_2.13:1.2.11`
  via spec `004`'s Maven Central dependency, reusing its exact groupId/artifactId/
  version): the real Scala engine returns `None` for `OBX[@3.1 = '94500-6']-5`
  (whitespace around the filter operator) — it does **not** already support this,
  confirming `contracts/path-grammar.md` Note #4 ("newly allowed") is a genuinely new
  grammar capability this parser adds, not a documentation-only fix like Notes #5/#6.
  Consequently `path-filter-operator-whitespace`'s `expected`/`expected_lines` in
  `fixtures/vectors/path/valid.json` are sourced from the equivalent no-whitespace
  form's real-Scala result (`OBX[@3.1='94500-6']-5` → `Some([[Positive]])`, line 4),
  not from running the whitespace variant itself against Scala — research.md #2's
  "verify against the real library" policy is refined by this one exception: it
  applies to what the *grammar defines as equivalent*, not literally re-running a
  string the target library is documented not to support.
- Implementation deviation from T003's original description: `SegmentExpr`,
  `SegIndex`, `FilterClause`, and `ChildPath` derive `PartialEq, Eq, Clone` (not
  `Copy`) since `FilterClause.values: Vec<&str>` isn't `Copy`; `FieldExpr`/`FieldIndex`
  (no borrowed/heap data) derive `Copy` as data-model.md implies. `ParseErrorKind`
  reuses `InvalidSegIndex`/`InvalidFieldIndex` for a missing/invalid `field_num`
  itself (not just bracketed `SEG_IDX`/`FIELD_IDX` content) — a generalization of
  those variants' documented scope discovered while implementing T006/T007, not a new
  variant; not exercised by any vector's exact-kind expectation (the fixture suite
  only checks accept/reject, per T010's design), only by this spec's own T013/T018
  unit tests, which were written to match the actual implementation.
- Commit after each task or logical group; stop at any checkpoint to validate a story
  independently before continuing.
