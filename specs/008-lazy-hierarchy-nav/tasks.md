---

description: "Task list for Lazy Hierarchy Navigation"
---

# Tasks: Lazy Hierarchy Navigation

**Input**: Design documents from `/specs/008-lazy-hierarchy-nav/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/hierarchy-api.md](contracts/hierarchy-api.md), [quickstart.md](quickstart.md)

**Tests**: Included as core deliverables, not an optional add-on — spec.md's user
stories are each defined by an "Independent Test" that is a `cargo test` invocation
(quickstart.md), and SC-001 through SC-004 are only checkable by running tests
against conformance vectors. Same convention specs `005`-`007`'s tasks.md
established.

**Organization**: Tasks are grouped by user story (US1/US2/US3 from spec.md, in
priority order) so each can be implemented and verified independently. Because US1
and US2 are both served by one `execute_hierarchy` function, the Foundational phase
carries the full bounded-scan algorithm, profile parsing, and corrected child-index
resolution — there is no way to build "a single-hop hierarchy PATH resolves" without
also building what "the full existing conformance suite passes" needs, since both
exercise the same code path. Each story phase then adds the tests/fixture changes
that specifically prove *that story's* acceptance scenarios. US3 (multi-hop
deferral) adds no runtime code at all — its tasks confirm the deferral is enforced
and discoverable, not a fresh capability.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: the hierarchy executor is added to the existing
`hl7pet-core` crate (`crates/core/`) as a new sibling module (`hierarchy.rs`) to
specs `005`-`007`'s `scanner.rs`/`parser.rs`/`query.rs` — no new workspace member.
`serde`/`serde_json` move from `[dev-dependencies]` to `[dependencies]` in
`crates/core/Cargo.toml` (research.md #5). Conformance vectors extend spec `002`'s
existing `fixtures/vectors/hierarchy/{basic,complex}.json` in place — no new schema
or vector family, except two existing entries' `expected` values are corrected
(research.md #4).

---

## Phase 1: Setup

**Purpose**: Stand up the module skeleton, the dependency change, and the test
harness. The Cargo workspace, `hl7pet-core` crate, `scanner.rs`, `parser.rs`, and
`query.rs` already exist (specs `005`-`007`) — this phase is deliberately light.

- [X] T001 In `crates/core/Cargo.toml`, move `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"` from `[dev-dependencies]` to `[dependencies]` (research.md #5) — no version change; both are already resolved at `1.0.151` in `Cargo.lock`.
- [X] T002 [P] Create the empty module `crates/core/src/hierarchy.rs` (module-level doc comment only, mirroring `scanner.rs`/`parser.rs`/`query.rs`'s precedent — state what it implements, spec `008`, and what it explicitly defers, per contracts/hierarchy-api.md's "does NOT provide" section) and add `pub mod hierarchy;` to `crates/core/src/lib.rs` (no re-exports yet — added in T013 once the public surface exists).
- [X] T003 [P] Scaffold `crates/core/tests/hierarchy_vectors.rs`: a `serde`-deserializable `HierarchyVector` struct covering `id`, `path`, `profile_ref: Option<String>`, `message_ref`, `method`, `expected` (as `serde_json::Value`), and `flags: Option<HierarchyFlags>` with `build_hierarchy: Option<bool>` (`#[serde(rename = "buildHierarchy")]`, for `hier-002`); loaders reading `fixtures/vectors/hierarchy/basic.json` and `complex.json` relative to the workspace root (mirroring `query_vectors.rs`'s `fixtures_root()` helper); a message-file loader and a profile-file loader (`fs::read_to_string`); and a dispatch stub that compiles and runs as a no-op (assertions filled in by T014).

**Checkpoint**: `cargo build --workspace` succeeds with the new empty `hierarchy` module and promoted dependencies; `cargo test -p hl7pet-core --test hierarchy_vectors` compiles and passes vacuously.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The full bounded-scan algorithm, profile parsing, and corrected
child-index resolution every user story builds on. No user story's acceptance
scenarios can be verified until this exists.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Define `ProfileError` in `crates/core/src/hierarchy.rs` per [data-model.md](data-model.md): a single variant, `InvalidJson { message: String }`, `Clone`, `Eq`, manual `Display`/`std::error::Error` impls (no error-derive crate, matching `ScanError`/`ParseError`/`QueryError`'s precedent). Exhaustive — no catch-all variant. **Note**: an original draft of this task also specified `DuplicateSegmentType` — removed during T007's implementation once `deep-nested.json` (a real fixture, not a hypothetical) was found to legitimately place `OBX`/`NTE` at more than one tree position; see research.md #2's corrected design.
- [X] T005 Define crate-private `RawSegmentDef`/`RawProfile` `#[derive(serde::Deserialize)]` structs in `crates/core/src/hierarchy.rs` mirroring `segmentDefinition`'s recursive JSON shape (`fixtures/profiles/{basic-two-level,deep-nested}.json`): `RawProfile { #[serde(rename = "segmentDefinition")] segment_definition: HashMap<String, RawSegmentDef> }` and `RawSegmentDef { cardinality: Option<String>, #[serde(default)] children: HashMap<String, RawSegmentDef> }` — `cardinality` is read but never consulted (research.md #2, spec `002` Section A.3). Neither type is `pub` (FR-014) — they never leave this module.
- [X] T006 Implement `HierarchyProfile`'s node arena in `crates/core/src/hierarchy.rs` (data-model.md): a private `struct ProfileNode { children: HashMap<String, usize>, parent: Option<usize> }`, and `pub struct HierarchyProfile { nodes: Vec<ProfileNode> }` with index `0` reserved as the synthetic root plus a private `by_name: HashMap<String, Vec<usize>>` (every position a name occupies, not just one — research.md #2) for `node_for` lookups.
- [X] T007 Implement `HierarchyProfile::from_json(json: &str) -> Result<Self, ProfileError>` in `crates/core/src/hierarchy.rs`: deserialize via T005's `RawProfile`, mapping any `serde_json::Error` to `ProfileError::InvalidJson { message: e.to_string() }` (the `serde_json::Error` type itself never crosses this function's signature, FR-014); recursively walk the raw tree building T006's arena, recording every position a name occupies in `by_name: HashMap<String, Vec<usize>>` — a name recurring is valid data, not an error (research.md #2, corrected here after the `deep-nested.json` fixture falsified the original "reject any repeat" draft). Construction is all-or-nothing — never a partially built `HierarchyProfile`.
- [X] T008 Implement `HierarchyProfile::ancestor_chain(&self, node: usize) -> Vec<usize>` (crate-private) in `crates/core/src/hierarchy.rs`: walks `parent` pointers from `node` up to and including the root, `O(profile depth)` (data-model.md).
- [X] T009 Change `resolve_segment_candidates`, `resolve_field_values`, and `filter_matches` in `crates/core/src/query.rs` from private (`fn`) to `pub(crate) fn` (plan.md Project Structure) — visibility only, no behavior change. Confirm `cargo build --workspace` and the full existing `cargo test -p hl7pet-core` suite still pass unchanged.
- [X] T010 Implement the bounded per-occurrence direct-child scan (crate-private `fn direct_children_of_type`) in `crates/core/src/hierarchy.rs` (research.md #1): given a `&ScanResult`, a `&HierarchyProfile`, one matching parent occurrence's `SegmentSpan`, and the child segment type `cseg: &str`, seed a local `Vec<usize>` stack at `[node_for(parent_type)]` and walk `scan.segments` forward from the line immediately after the parent occurrence. Per line: try matching its segment type as a child of the stack's top node (push on match; if the stack's length just became `2` — a direct child — and the type equals `cseg`, record the span); else if the stack has more than one entry, pop and retry; else (local floor reached) check T008's `ancestor_chain(parent_node)` — a match there means the subtree has been exited (stop scanning entirely), no match anywhere means the line is unrecognized (silently skip it, stack unchanged, continue). Returns `Vec<SegmentSpan>` in document order — already type-filtered (FR-007).
- [X] T011 Implement the corrected per-parent `csegIdx` resolution (crate-private `fn apply_child_index`) in `crates/core/src/hierarchy.rs` per [data-model.md](data-model.md)'s table, operating on T010's already type-filtered output: `None`/`SegIndex::Star` returns every entry; `SegIndex::Numeric(n)` returns the entry at 1-based position `n` within *this parent's own* list, or `vec![]` if out of range; `SegIndex::Last` returns the final entry, or `vec![]` if the list is empty; `SegIndex::Filter(clause)` calls `query::filter_matches` (now `pub(crate)`, T009) against each entry in order.
- [X] T012 Implement the public entry point `pub fn execute_hierarchy<'m>(scan: &ScanResult<'m>, path: &CompiledPath<'_>, profile: Option<&HierarchyProfile>) -> Result<Vec<Vec<&'m str>>, QueryError>` in `crates/core/src/hierarchy.rs` per [contracts/hierarchy-api.md](contracts/hierarchy-api.md): `path.child.is_none()` delegates to `query::execute(scan, path)` unchanged (`profile` ignored); `path.child.is_some()` and `profile.is_none()` returns `Ok(vec![])` (FR-009); otherwise resolves parent occurrences via `query::resolve_segment_candidates` (T009, unchanged), runs T010+T011 per matching parent occurrence, concatenates the selected spans across parents in document order, then applies `query::resolve_field_values` (T009, unchanged) per selected span using `path.child.field`.
- [X] T013 [P] Update `crates/core/src/lib.rs`: add `hierarchy` module re-exports (`pub use hierarchy::{execute_hierarchy, HierarchyProfile, ProfileError};`) matching [contracts/hierarchy-api.md](contracts/hierarchy-api.md)'s public surface exactly.
- [X] T014 [P] Fill in `crates/core/tests/hierarchy_vectors.rs`'s dispatch body (stubbed in T003): for each vector, load its message via `scan()`, parse its `path` via `parse()`, load its `profile_ref` via `HierarchyProfile::from_json` unless `flags.build_hierarchy == Some(false)` (in which case pass `None`, per `hier-002`), call `execute_hierarchy`, and assert the result matches `expected` for the vector's declared `method`. **Note**: also added an `is_multi_hop` filter skipping any vector whose PATH has more than one `" -> "` — not anticipated by this task's original wording; discovered during this task's own implementation that `hier-009`/`hier-010` are unparseable under spec 006's unchanged grammar (research.md #6).

**Checkpoint**: `cargo build --workspace` and `cargo test -p hl7pet-core` both succeed; `execute_hierarchy` is fully implemented for every code path in spec.md FR-001–FR-014 — not yet verified against the full 8-vector suite (that's US2's job; T018/T019's fixture corrections haven't landed yet, so `hier-004`/`hier-008` are still expected to fail at this checkpoint).

---

## Phase 3: User Story 1 - Caller resolves a single-hop hierarchy PATH against a scanned message (Priority: P1) 🎯 MVP

**Goal**: Prove the core bounded-scan algorithm works correctly in isolation and on
the simplest representative case — a single-hop `->` PATH against one matching
parent occurrence.

**Independent Test**: `hier-001` (`OBR[1] -> OBX-3` against `basic-hierarchy.hl7`/
`basic-two-level.json`) executes to its documented expected value via the harness;
the bounded-scan algorithm's individual rules (direct-child detection, boundary
detection, unrecognized-segment drop) each pass a dedicated unit test (spec.md US1
Independent Test).

### Implementation for User Story 1

- [X] T015 [P] [US1] Add unit tests in `crates/core/src/hierarchy.rs` directly asserting T010's `direct_children_of_type`, built against a synthetic `ScanResult`/`HierarchyProfile` constructed in the test (not the fixture corpus): a direct child of the parent is recorded; a grandchild (nested one level deeper) is excluded from the direct-child list but does not end the scan; a sibling top-level segment ends the scan (boundary reached, research.md #1); an unrecognized segment type (not a legal child anywhere in the profile) is silently skipped and scanning continues past it into further real children.
- [X] T016 [US1] Confirm `hier-001` passes via `cargo test -p hl7pet-core --test hierarchy_vectors` (T014's dispatch) — both `OBX` children of the single `OBR` occurrence are returned in document order.
- [X] T017 [US1] Add a unit test in `crates/core/src/hierarchy.rs` confirming FR-009: `execute_hierarchy` called with a hierarchy `CompiledPath` (`child: Some(_)`) and `profile: None` returns `Ok(vec![])`, without evaluating the parent or child sides as independent flat paths.

**Checkpoint**: User Story 1 fully functional and independently verified — the core
algorithm is proven correct on synthetic inputs and on the simplest real fixture
vector.

---

## Phase 4: User Story 2 - Existing hierarchy conformance vectors validate the implementation byte-for-byte (Priority: P1)

**Goal**: Prove the 8 single-hop vectors in the existing 10-vector corpus pass
against this implementation, including the two vectors research.md #4 identified as
needing `expected`-value corrections under FR-007's fix, and that a malformed
profile or an unusually large scoped range never panics or scans unboundedly.

**Independent Test**: The 8 single-hop vectors in
`fixtures/vectors/hierarchy/{basic,complex}.json` (`hier-009`/`hier-010`'s two-hop
PATH is out of scope, discovered during T014, research.md #6) pass via
`cargo test -p hl7pet-core --test hierarchy_vectors`, including the corrected
entries and the static-mode-fallback vector (spec.md US2 Independent Test, SC-001).

### Implementation for User Story 2

- [X] T018 [US2] Update `fixtures/vectors/hierarchy/basic.json`'s `hier-004` entry (research.md #4): change `expected` to `[["OBX-P-CODE^First Observation^LN"]]` and `expected_lines` to `[[5]]`; remove the `known_limitation` field entirely (it is no longer a limitation). **Note**: `semantic_rules` is left as `["A.4-cross-parent-child-indexing"]`, not changed to a new value as this task originally specified — the vector still exercises that documented rule, and the schema's closed `semantic_rules` enum needs no new value as a result (research.md #4).
- [X] T019 [US2] Update `fixtures/vectors/hierarchy/complex.json`'s `hier-008` entry (research.md #4): change `expected` to `[["OBX-A-CODE^Direct Child A^LN"]]` and `expected_lines` to `[[7]]`; remove the `known_limitation` field entirely. Same `semantic_rules` note as T018.
- [X] T020 [US2] Run `cargo test -p hl7pet-core --test hierarchy_vectors` and confirm all 8 single-hop vectors pass, including T018/T019's corrected entries and `hier-002`'s static-mode-fallback case (`flags.buildHierarchy: false` → `execute_hierarchy(..., None)` → `Ok(vec![])`) — spec.md's US2 Independent Test, proving SC-001. (`hier-009`/`hier-010` are excluded by T014's `is_multi_hop` filter, research.md #6 — not part of this "all pass" count.)
- [X] T021 [P] [US2] Add unit tests in `crates/core/src/hierarchy.rs` (SC-004): confirm invalid JSON returns `Err(ProfileError::InvalidJson { .. })` from `HierarchyProfile::from_json` — never a panic; confirm (separately) that a profile with a segment type repeated at two positions in `segmentDefinition` (`deep-nested.json`'s real shape) is accepted, not rejected (research.md #2's corrected design — this task's original wording expected a `DuplicateSegmentType` rejection here, which T007 removed once the real fixture data falsified that design).
- [X] T022 [P] [US2] Add a bounded-scan cost unit test in `crates/core/src/hierarchy.rs` (SC-002): construct a large synthetic message where the matching parent occurrence's real children are a small prefix of the message, and confirm `direct_children_of_type` (T010) never inspects a `SegmentSpan` past the computed boundary line — implemented as a correctness-under-scale test (a 2000-segment tail after the boundary must not appear in or change the result), which falsifies "continues past the boundary" functionally rather than via an allocation/iteration counter.

**Checkpoint**: User Stories 1 and 2 both independently functional; the 8-vector
single-hop conformance suite (8/8) passes against the corrected, non-buggy
implementation.

---

## Phase 5: User Story 3 - Multi-level chained navigation is explicitly deferred, not silently dropped (Priority: P3)

**Goal**: Confirm the multi-hop deferral decision (spec.md Clarifications) is
enforced in code, not just prose, and remains discoverable at the point a future
reader would look for it.

**Independent Test**: A two-hop `->` PATH is still rejected at parse time exactly as
spec `006` already guarantees; the deferral decision and its rationale are
discoverable from the codebase and spec docs without consulting chat history (spec.md
US3 Independent Test).

### Implementation for User Story 3

- [X] T023 [US3] Confirm (via `cargo test -p hl7pet-core --lib parser -- multiple_hierarchy_hops`, or add the test if spec `006` did not already cover this exact input) that `hl7pet_core::parse("ORC[1] -> OBR[1] -> OBX-5")` returns `Err(ParseErrorKind::MultipleHierarchyHops)` — confirming this spec introduces no grammar or parser change (spec.md Clarifications, User Story 3 Acceptance Scenario 1). Already covered by spec `006`'s existing `parser::tests::multiple_hierarchy_hops_is_rejected` — no new test needed.
- [X] T024 [US3] Update `crates/core/src/parser.rs`'s `ChildPath` doc comment (currently cites only `contracts/path-grammar.md` Non-Goals) to also note that spec `008` considered and explicitly deferred implementing multi-hop chaining (spec.md User Story 3), so a future reader of the parser source — not just the spec docs — finds the deferral recorded at the type it constrains.

**Checkpoint**: All three user stories independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Tie the three stories together into one verified, documented
deliverable.

- [X] T025 [P] Run `python3 fixtures/scripts/validate_corpus.py` against the full corpus including T018/T019's corrected `hierarchy` entries; confirm ids remain unique corpus-wide and every `message_ref`/`profile_ref` resolves, with no schema change required.
- [X] T026 [P] Update `crates/core/README.md` to mention the `hierarchy` module, linking to [quickstart.md](quickstart.md) and [contracts/hierarchy-api.md](contracts/hierarchy-api.md) rather than duplicating them.
- [X] T027 Update `crates/cli/src/main.rs` (plan.md Project Structure; not a roadmap requirement per spec.md Assumptions, but the natural follow-up this spec unblocks): add a `--profile <file>` flag, remove the "hierarchy PATH is not evaluated yet (spec 008)" warning, and call `execute_hierarchy` (loading the profile via `HierarchyProfile::from_json` when `--profile` is given, `None` otherwise) instead of `execute` whenever `path.child` is `Some`.
- [X] T028 Run the full [quickstart.md](quickstart.md) validation end-to-end (all 8 steps) and record the outcome.
- [X] T029 Update [ROADMAP.md](../../ROADMAP.md)'s spec `008` status row from "Draft" to "Implemented," noting the new `crates/core/src/hierarchy.rs` module, the `fixtures/vectors/hierarchy/` corrections (T018/T019), the `serde`/`serde_json` promotion to a runtime dependency, the two implementation-time design corrections (research.md #2, #6), and that it's ready for spec `009` (core-perf-validation) to benchmark.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup (T001-T003) — BLOCKS all user stories.
- **User Stories (Phase 3-5)**: All depend on Foundational completion.
  - US1 (T015-T017) has no dependency on US2/US3.
  - US2 (T018-T022) depends only on Foundational (`execute_hierarchy` must exist) —
    independently testable in parallel with US1/US3.
  - US3 (T023-T024) depends only on Foundational (`parse` is unchanged from spec
    `006`, so this story is really a confirmation pass) — independently testable in
    parallel with US1/US2.
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### Within Phase 2 (Foundational)

- T004 has no dependencies — the error type comes first.
- T005 has no dependency on T004; both can start immediately after Setup.
- T006 has no dependency on T004/T005 (the arena's shape doesn't need the raw JSON
  types), but is naturally sequenced after them.
- T007 depends on T004 (error type), T005 (raw deserialization shape), and T006
  (arena to build into).
- T008 depends on T006 (needs the arena's `parent` pointers).
- T009 has no dependency on T004-T008 — it is a pure visibility change to
  `query.rs`, independent of everything else in this phase.
- T010 depends on T006 (`node_for`) and T008 (`ancestor_chain`).
- T011 depends on T009 (`filter_matches` must be `pub(crate)`) and T010 (its input).
- T012 depends on T007 (profile construction), T009 (`resolve_segment_candidates`/
  `resolve_field_values`), T010, and T011 (composes all of them).
- T013 depends on T004, T007, T012 (re-exports need the finished public surface).
- T014 depends on T003 (harness scaffold) and T012 (needs `execute_hierarchy` to
  call).

### Within Phase 4 (US2)

- T018/T019 (fixture corrections) have no code dependency — can be authored as soon
  as research.md #4's derivation is trusted, in parallel with T021/T022.
- T020 depends on T014 (dispatch must exist), T018, and T019 (needs the corrected
  values to pass against).
- T021/T022 depend only on T007/T010 respectively — parallelizable with T018/T019/T020.

### Parallel Opportunities

- Setup: T002 and T003 in parallel (different files); T001 is independent of both.
- Foundational: T004 and T005 in parallel; T009 in parallel with T004-T008 (different
  file); T013 and T014 in parallel once T012 lands.
- User Stories: US1, US2, and US3 can all proceed in parallel once Foundational is
  done.
- Within US1: T015 in parallel with T016/T017 (unit tests vs. fixture-driven test).
- Within US2: T018 in parallel with T019 (different JSON files); T021 in parallel
  with T022 (different concerns, same file — coordinate on merge); T020 depends on
  both fixture updates landing first.
- Polish: T025 and T026 in parallel; T027 is independent of both.

---

## Parallel Example: Foundational Phase

```bash
# T004 and T005 together (different concerns, same file — coordinate on merge):
Task: "Define ProfileError in hierarchy.rs"
Task: "Define RawSegmentDef/RawProfile deserialization shape in hierarchy.rs"

# T013 and T014 together once T012's execute_hierarchy lands (different files):
Task: "Update lib.rs re-exports for the hierarchy module"
Task: "Fill in hierarchy_vectors.rs dispatch body"
```

## Parallel Example: User Stories (post-Foundational)

```bash
# Launch all three stories together once Phase 2 is complete:
Task: "US1: bounded-scan unit tests + hier-001 confirmation"
Task: "US2: fixture corrections (hier-004/hier-008) + full 10-vector suite + panic-safety/bound tests"
Task: "US3: multi-hop rejection confirmation + doc-comment update"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (blocks everything else — implements the full
   bounded-scan algorithm, profile parsing, and corrected indexing, since even the
   simplest single-hop case needs all of it).
3. Complete Phase 3: User Story 1 — proves the core algorithm works on synthetic
   inputs and the simplest real fixture vector (`hier-001`).
4. **STOP and VALIDATE**: run quickstart.md steps 1-2 and 4-5.

### Incremental Delivery

1. Setup + Foundational → `execute_hierarchy` exists and compiles, but isn't yet
   proven against the fixture corpus (`hier-004`/`hier-008` still fail — their
   fixtures haven't been corrected yet).
2. US1 → the core algorithm proven on synthetic inputs plus the simplest real vector
   (MVP).
3. US2 → the full existing conformance suite proven, including the two corrected
   vectors and the malformed-profile/bounded-scan guarantees.
4. US3 → the multi-hop deferral decision confirmed enforced and discoverable in code.
5. Polish → CLI update, docs, corpus validation, `ROADMAP.md` update, readying spec
   `009` to benchmark this module against the Scala baseline.

---

## Notes

- [P] tasks touch different files (or independent regions of the same file with no
  shared state) and have no unmet dependency within their phase.
- [Story] labels (US1/US2/US3) map directly to spec.md's prioritized user stories.
- T018/T019's fixture-value corrections are the single most consequential tasks in
  this feature: they change what "conformance" means for two existing, previously
  Scala-verified vectors, per FR-007's deliberate decision to fix rather than
  reproduce the real engine's child-index bug (research.md #4's hand-derived trace
  against `HL7HierarchyParser.scala`/`HL7ParseUtils.scala`, not a guess) — get these
  two values right before running T020, since a mistake here would silently validate
  the implementation against the wrong target.
- T009's visibility change touches `query.rs`, a file spec `007` already shipped and
  tested — verify the full pre-existing test suite still passes immediately after
  T009, before building anything in `hierarchy.rs` on top of it.

## Implementation-time corrections (not anticipated in planning)

Two real design/scope mistakes surfaced only once T014's integration test ran
against the actual fixture data — both are documented in full in research.md and
left here as a pointer, per this project's established practice of recording
planning-vs-implementation corrections rather than silently absorbing them:

- **research.md #2** (T004/T006/T007/T021): the original design rejected any
  segment type repeated at more than one tree position as malformed
  (`ProfileError::DuplicateSegmentType`). `fixtures/profiles/deep-nested.json` —
  consumed by 5 of `complex.json`'s 6 vectors — legitimately places `OBX` and `NTE`
  at more than one nesting depth; this is normal profile data, not an edge case.
  `hier-005` failed immediately against the original design, which is what surfaced
  the mistake. Corrected: only a `->` expression's *parent*-side type needs
  unambiguous resolution; `ProfileError` lost the `DuplicateSegmentType` variant
  entirely.
- **research.md #6** (T014/T018-T020): `hier-009`/`hier-010` use a two-hop PATH
  (`"OBR[1] -> OBX[3] -> NTE-3"`), which spec `006`'s parser already rejects
  (`MultipleHierarchyHops`) and which tests the multi-hop capability this spec's
  Clarifications deferred. Neither vector can execute under this spec's scope; both
  are excluded from `hierarchy_vectors`'s dispatch loop, documented in the test
  file, rather than silently miscounted as passing. SC-001/FR-010's "10 vectors"
  language was corrected to "8 of 10" across spec.md, plan.md, and quickstart.md.
