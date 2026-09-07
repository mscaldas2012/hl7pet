---

description: "Task list for Escape-Sequence Decoding"
---

# Tasks: Escape-Sequence Decoding

**Input**: Design documents from `/specs/1001-escape-sequence-decoding/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/escape-decoding-api.md](contracts/escape-decoding-api.md), [quickstart.md](quickstart.md)

**Tests**: Included as core deliverables, not an optional add-on — spec.md's US1
is defined by conformance vectors proving each escape-sequence type decodes
correctly, and SC-001/SC-002 are only checkable by running tests. Same
convention specs `005`-`1000`'s tasks.md established.

**Organization**: Unlike specs `1000`/`008`, this feature has no independent
"cardinality" split between user stories — US1 (decode, unconditionally) *is*
the entire behavioral change, applied uniformly everywhere a value is
returned. Because nothing in `hl7pet-core` can compile once `LocatedValue`'s
field type changes, the Foundational phase necessarily carries the *entire*
type-signature ripple (the actual ship of this feature) before US1's own
phase can add anything beyond it — US1's phase is then almost entirely new
tests proving the already-landed behavior is correct, plus the new fixture
data those tests run against. US2 (migration guide) is a genuinely separate,
documentation-only addition layered on top.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: no new crate or module. `query.rs` (spec
`007`'s home, already extended by spec `1000`) and `hierarchy.rs` (spec
`008`'s home) are modified in place. A new fixture vector family,
`fixtures/{messages,vectors}/escapes/`, is added — kept separate from the
existing `path`/`hierarchy`/`scanner` families (spec.md Assumptions). A new
`crates/core/MIGRATION.md` is this feature's actual migration-guide
deliverable.

---

## Phase 1: Setup

**Purpose**: Stand up the new fixture family's schema, corpus registration,
and the new integration test's skeleton — everything the Foundational phase's
own tests (and US1's later ones) will need to run against.

- [X] T001 Create `fixtures/schemas/escape-conformance-vector.schema.json`, mirroring `conformance-vector.schema.json`'s structure (`$schema`, `$id`, `title`, `description` referencing this spec): `required: ["id", "path", "message_ref", "method", "expected"]`, `id`/`path`/`message_ref` typed exactly as the existing schema does, `method` enum `["getValue", "getFirstValue"]`, `expected` using the same `oneOf` shape (array-of-arrays / string / null), plus a new required `escape_types` field: `{"type": "array", "minItems": 1, "items": {"type": "string", "enum": ["F", "S", "T", "R", "E", "H_N", "X", "Z"]}}` — this family's coverage dimension (research.md #7).
- [X] T002 [P] Register the new family in `fixtures/scripts/validate_corpus.py`: add `"escapes": "escape-conformance-vector.schema.json"` to `KNOWN_FAMILIES` and `"escapes": "escape_types"` to `COVERAGE_FIELD`.
- [X] T003 [P] Create `fixtures/messages/escapes/basic-escapes.hl7`: a synthetic message (no real patient data, per spec `003`'s FR-009 convention) with at least: one field containing `\F\`, `\S\`, `\T\`, `\R\`, and `\E\` each decoding to that message's own standard delimiter characters; one field containing `\H\...\N\` highlighting; one field containing a `\Xdddd\` hex sequence decoding to a short ASCII word; one field containing a `\Zxxx\` custom sequence; and one field containing a trailing unterminated escape character (for the malformed case). Document each field's exact raw content and PATH in a comment or the vector file itself (T007) so the fixture and its vectors stay in sync.
- [X] T004 Scaffold `crates/core/tests/escape_vectors.rs`: a `serde`-deserializable `EscapeVector` struct (`id`, `path`, `message_ref`, `method`, `expected: serde_json::Value`, `escape_types: Vec<String>`), a `fixtures_root()`/loader pair mirroring `query_vectors.rs`'s, and a dispatch stub that compiles and runs as a no-op (assertions filled in by T023).

**Checkpoint**: `cargo build --workspace` succeeds (this feature hasn't touched any Rust source yet); `fixtures/scripts/validate_corpus.py` recognizes the (still-empty) `escapes` family without erroring; `cargo test -p hl7pet-core --test escape_vectors` compiles and passes vacuously.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The actual decode implementation and every type-signature change
it forces — nothing in `hl7pet-core` compiles again until this entire phase
is complete, since `LocatedValue`'s field type changes.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T005 Implement `pub(crate) fn decode_escapes<'m>(raw: &'m str, delimiters: &DelimiterSet) -> Cow<'m, str>` in `crates/core/src/query.rs` per [data-model.md](data-model.md) and research.md #2: first, a single presence check for `delimiters.escape` anywhere in `raw` — if absent, return `Cow::Borrowed(raw)` immediately (the required zero-allocation fast path). Otherwise, scan left to right building an owned `String`: `\F\`/`\S\`/`\T\`/`\R\`/`\E\` (type-char immediately followed by the closing escape byte) push `delimiters.field`/`component`/`subcomponent`/`repetition`/`escape` respectively; `\H\`/`\N\` push nothing; `\Xdddd..\` (one or more hex-digit pairs up to the next escape byte) decode the pairs to bytes and push them only if the byte run is valid UTF-8, else fall through to the malformed case; `\Zxxx\` (any bytes up to the next escape byte) push the inner content unchanged; anything else (unterminated, unrecognized type-char, odd hex-digit count, invalid-UTF-8 hex result) pushes the escape byte and continues rescanning from the character right after it, unmodified. Never panics, never returns `Err` — this function has no `Result` in its signature at all.
- [X] T006 Change `LocatedValue<'m>` in `crates/core/src/query.rs` per [data-model.md](data-model.md): `value: &'m str` -> `value: Cow<'m, str>`; drop `Copy` from its `#[derive(...)]` (retain `Debug, Clone, PartialEq, Eq`). Add `use std::borrow::Cow;` to the module's imports.
- [X] T007 Change `resolve_field_values` in `crates/core/src/query.rs`: return type `Vec<&'m str>` -> `Vec<Cow<'m, str>>`; wrap its existing `None => vec![segment_content]` arm as `vec![decode_escapes(segment_content, delimiters)]` and its `Some(fe) => ...` arm's final `.map(|rep| extract_component(...))` to additionally decode: `.map(|rep| decode_escapes(extract_component(rep, delimiters, fe.component, fe.subcomponent), delimiters))`.
- [X] T008 Change `resolve_field_values_located` in `crates/core/src/query.rs` identically to T007, wrapping each produced value in `LocatedValue { value: decode_escapes(...), line }` instead of the raw `&str`.
- [X] T009 Update `execute`'s signature in `crates/core/src/query.rs`: `Result<Vec<Vec<&'m str>>, QueryError>` -> `Result<Vec<Vec<Cow<'m, str>>>, QueryError>`. No body change needed beyond what T007 already provides — confirm it compiles.
- [X] T010 Update `execute_located`'s and `first_located`'s doc comments in `crates/core/src/query.rs` to state they now return decoded values (contracts/escape-decoding-api.md) — their signatures' visible shape (`Vec<Vec<LocatedValue<'m>>>` / `Option<LocatedValue<'m>>`) doesn't change textually, only `LocatedValue`'s own field type (T006) does; confirm both compile against T008's updated `resolve_field_values_located`.
- [X] T011 Update `execute_hierarchy`'s signature in `crates/core/src/hierarchy.rs`: `Result<Vec<Vec<&'m str>>, QueryError>` -> `Result<Vec<Vec<Cow<'m, str>>>, QueryError>`; add `use std::borrow::Cow;` if not already imported. The `path.child.is_none()` branch's `return query::execute(scan, path);` needs no change beyond the return type now matching; the hierarchy branch's own `query::resolve_field_values(...)` call now returns decoded `Cow` values automatically via T007.
- [X] T012 Fix compilation in `crates/core/src/query.rs`'s existing test module (spec `1000`'s tests): every `LocatedValue { value: "literal", line: N }` struct literal becomes `LocatedValue { value: Cow::Borrowed("literal"), line: N }` (research.md #6). Fix `execute_located_matches_execute_for_every_value_when_multiple_repetitions_present`'s `let stripped: Vec<Vec<&str>> = located.iter().map(|group| group.iter().map(|lv| lv.value.as_ref())...)` — change the inner map to `.map(|lv| lv.value.as_ref())` so it compares against `plain: Vec<Vec<Cow<str>>>`'s content correctly (or compare `Cow`-to-`Cow` directly without the intermediate `&str` Vec).
- [X] T013 [P] Fix compilation in `crates/core/tests/query_vectors.rs`: `assert_get_value`'s parameter type `values: &[Vec<&str>]` must accept `Cow`-typed values now that `execute()` returns them — change to `values: &[Vec<Cow<'_, str>>]` (or a generic `impl AsRef<str>`-bounded parameter) and adjust its body's comparisons accordingly; `assert_get_first_value`'s `.copied()` call on the first value likely needs to become `.cloned()` or `.map(|v| v.as_ref())` since `Cow` isn't `Copy`.
- [X] T014 [P] Fix compilation in `crates/core/tests/hierarchy_vectors.rs` and `crates/core/tests/located_vectors.rs` for the same `Cow` ripple, if `cargo build --workspace` surfaces any (per research.md #6, most direct `assert_eq!(value, "literal")` comparisons should keep compiling unchanged via `Cow`'s `PartialEq<&str>` impl — only type-annotated intermediate variables need fixing).
- [X] T015 Run `cargo build --workspace` and `cargo test --workspace`; confirm the full pre-existing suite (specs `005`-`1000`) passes **unmodified** — this is SC-002's direct proof, not a coincidental side effect, since no message in that corpus contains an escape sequence (research.md, spec.md Assumptions).

**Checkpoint**: The entire feature's runtime behavior now exists and compiles; the full pre-existing suite passes with zero expected-value changes anywhere. Not yet verified against new escape-specific fixture data — that's US1's job.

---

## Phase 3: User Story 1 - Caller receives correctly decoded values, unconditionally (Priority: P1) 🎯 MVP

**Goal**: Prove `decode_escapes`'s algorithm is correct for every documented
escape-sequence type, both via direct unit tests and via the new conformance
vector family, and confirm the zero-copy fast path is real.

**Independent Test**: Run `cargo test -p hl7pet-core --test escape_vectors`
and confirm every vector in `fixtures/vectors/escapes/valid.json` passes,
with the corpus coverage report showing 8/8 `escape_types` covered.

### Tests for User Story 1

- [X] T016 [P] [US1] Unit tests in `crates/core/src/query.rs`'s test module: `decode_escapes` correctly decodes `\F\`, `\S\`, `\T\`, `\R\`, `\E\` to a `DelimiterSet`'s corresponding field, and — using a **non-standard** delimiter set (mirroring `scanner.rs`'s own non-standard-delimiter test precedent) — confirms the decoded character matches that message's own delimiters, not hardcoded standard ones (spec.md FR-002).
- [X] T017 [P] [US1] Unit test in `crates/core/src/query.rs`'s test module: `decode_escapes` on `"before\\H\\highlighted\\N\\after"` (with a standard `\` escape char) returns `"beforehighlightedafter"` — both markers removed, enclosed and surrounding text preserved exactly.
- [X] T018 [P] [US1] Unit tests in `crates/core/src/query.rs`'s test module: `decode_escapes` on a `\X48656C6C6F\` sequence returns `"Hello"`; a second case with hex pairs forming a multi-byte UTF-8 character split across consecutive pairs (e.g. `\XC3A9\` -> `"é"`) decodes correctly.
- [X] T019 [P] [US1] Unit test in `crates/core/src/query.rs`'s test module: `decode_escapes` on `\ZCUSTOM123\` returns `"CUSTOM123"` (delimiters stripped, content unchanged).
- [X] T020 [P] [US1] Unit tests in `crates/core/src/query.rs`'s test module covering FR-006's malformed cases, each left completely unmodified: a trailing unterminated escape character; an unrecognized type-char (e.g. `\Q\`); an odd number of hex digits in a `\X\` sequence; a `\X\` sequence whose decoded bytes are not valid UTF-8.
- [X] T021 [P] [US1] Counting-allocator unit test in `crates/core/src/query.rs`'s test module (reusing the pattern from specs `005`/`1000`): `decode_escapes` on a value containing no escape-delimiter byte performs zero allocations and returns `Cow::Borrowed`, confirmed via `matches!(result, Cow::Borrowed(_))` plus the allocation count.
- [X] T022 [US1] Populate `fixtures/vectors/escapes/valid.json` with conformance vectors against `basic-escapes.hl7` (T003), one or more per `escape_types` value (`F`, `S`, `T`, `R`, `E`, `H_N`, `X`, `Z`), each with a PATH, `method`, computed `expected` decoded value, and its `escape_types` array — run against the actual implementation (T005-T011) to derive each vector's correct `expected` value rather than hand-deriving it, mirroring spec `001`'s "verify against the real thing before writing the vector" discipline.
- [X] T023 [US1] Fill in `crates/core/tests/escape_vectors.rs`'s dispatch body (stubbed in T004): for each vector, scan the message, parse the path, call `execute()`, and assert the decoded value matches `expected` for the vector's declared `method` (mirroring `query_vectors.rs`'s `assert_get_value`/`assert_get_first_value` pattern, adapted for `Cow`-typed values per T013).
- [X] T024 [US1] Run `python3 fixtures/scripts/validate_corpus.py` and confirm the coverage report shows 8/8 `escape_types` covered for the `escapes` family, with zero regressions reported for `path`/`hierarchy`/`scanner`.

**Checkpoint**: US1's acceptance scenarios (spec.md) pass independently — every documented escape-sequence type decodes correctly, the fast path is proven zero-copy, and the full pre-existing suite remains untouched.

---

## Phase 4: User Story 2 - Caller upgrading consults the migration guide (Priority: P2)

**Goal**: Ship the mandatory migration-guide deliverable (Constitution
Principle I / spec.md FR-008/FR-009) as a real, discoverable crate file.

**Independent Test**: Read `crates/core/MIGRATION.md` and confirm it names
every escape-sequence type this feature decodes and states plainly that no
opt-out exists.

### Implementation for User Story 2

- [X] T025 [US2] Write `crates/core/MIGRATION.md` per research.md #8: state plainly that escape-sequence decoding (`\F\`, `\S\`, `\T\`, `\R\`, `\E\`, `\H\`/`\N\`, `\Xdddd\`, `\Zxxx\`) is now unconditional across `execute`, `execute_located`, `first_located`, and `execute_hierarchy`; state there is no per-call or global opt-out; and note that a consumer needing pre-decode raw text has no built-in way to get it from this crate (pin the prior version, or handle it externally).
- [X] T026 [P] [US2] Add a short "Breaking change: escape-sequence decoding" pointer section to `crates/core/README.md` linking to `MIGRATION.md`, mirroring spec `009`'s precedent of adding a README section for a notable change.

**Checkpoint**: All user stories complete; the full public contract
(contracts/escape-decoding-api.md) is implemented, tested, and documented.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Confirm the Performance Goal (no regression against spec `009`'s
baseline), run final full validation, and update `ROADMAP.md`.

- [X] T027 [P] Re-run spec `009`'s existing `cargo bench -p hl7pet-core --bench extraction` and compare against its last recorded baseline (`specs/009-core-perf-validation/comparison/2026-09-04/`) — confirm no measurable regression. **Note**: the first run surfaced a real regression (avg throughput -17.4%, worst -67.9%) — the zero-allocation fast path alone didn't prevent it, since `resolve_field_values`'s outer `&str -> Cow<str>` map/collect couldn't reuse `select_by_field_index`'s buffer (different element size, research.md #1). Fixed by introducing `select_and_map` (collects directly from the repetitions slice, never through an intermediate `Vec<&str>`), replacing `select_by_field_index` entirely. Re-run after the fix: avg +1.7%, worst -8.4% — within spec `004`'s own documented microbenchmark noise band.
- [X] T028 Run `cargo test --workspace` and `cargo clippy --workspace --all-targets`; confirm everything passes/is clean.
- [X] T029 Execute every step of [quickstart.md](quickstart.md) manually and confirm each documented "Expected outcome" holds.
- [X] T030 Update `ROADMAP.md`'s spec `1001` Status row from "Draft" to "Implemented" with a summary of what shipped (mirroring specs `1000`/`008`'s entries' level of detail) — confirm the module's "Next free" value and the Documented Breaking Changes table row (already updated during `/speckit-specify`/`/speckit-plan`) still read correctly against the final implementation.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies on Rust source — can start immediately, in parallel with nothing since T001-T004 are the first things needed.
- **Foundational (Phase 2)**: Depends on Setup only loosely (T003's fixture message isn't required to compile T005-T015, only to populate vectors later) — BLOCKS all user stories. Internally sequential: T005 before T006-T008 (they call it); T006 before T007/T008 (they construct/return the changed type); T009-T011 depend on T007/T008; T012-T014 depend on T006-T011 (fixing callers of the now-changed types); T015 depends on all of the above.
- **User Story 1 (Phase 3)**: Depends on Foundational (Phase 2) completion — the implementation must exist and compile before it can be tested against real fixture data. T022 depends on T005-T011 being implemented (to derive correct `expected` values) and T003 (the fixture message existing).
- **User Story 2 (Phase 4)**: Depends only on Foundational (Phase 2) — the migration guide describes the shipped behavior, not test coverage of it; could be written in parallel with Phase 3 by a different contributor, though listed after it here since it has no test-coverage stake in getting it right first.
- **Polish (Phase 5)**: Depends on Phases 2-4 all being complete.

### Within Each Phase

- Tests before the story's own dispatch/data tasks that make them meaningful (T016-T021 before T022; T022 before T023).
- Setup's schema/registration (T001-T002) before anything reads or validates against them (T024).

### Parallel Opportunities

- T002 and T003 are independent files, safe in parallel with each other and with T001/T004.
- T013 and T014 touch different test files, safe in parallel with each other once T006-T012 land.
- T016-T021 are independent unit test functions in the same file — safe to write in parallel, sequenced only by whoever merges last into `query.rs`.
- T025 and T026 touch different files (`MIGRATION.md` vs `README.md`) and are independent of Phase 3 entirely.
- T027 is independent of T028-T030.

---

## Parallel Example: Foundational Phase's test fixes

```bash
# Once T006-T011 land, these two can run in parallel (different files):
Task: "Fix Cow ripple in crates/core/tests/query_vectors.rs"
Task: "Fix Cow ripple in crates/core/tests/hierarchy_vectors.rs and located_vectors.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — this phase *is* the feature's runtime behavior; nothing compiles until it's done)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: `cargo test -p hl7pet-core --test escape_vectors` passes, 8/8 escape-type coverage
5. The feature is fully functional at this point — US2 is documentation only

### Incremental Delivery

1. Setup + Foundational -> decoding works, full pre-existing suite passes unmodified (SC-002 proven)
2. Add US1 -> every escape type proven correct against real fixture data (SC-001 proven)
3. Add US2 -> migration guide shipped (SC-003 proven)
4. Polish -> performance sanity check, full regression, ROADMAP update

---

## Notes

- [P] tasks = different files or independent test functions, no dependency on an incomplete task
- [Story] label maps task to specific user story for traceability
- This feature has no Scala equivalent to verify against (`SPEC.md` §7 documents
  the limitation, not a working decoder) — the new `escapes` fixture family is
  the sole source of truth for correctness, derived from running the actual
  implementation (T022), not hand-computed independently of it
- Commit after each task or logical group
- Stop at any checkpoint to validate independently
- `filter_matches` (query.rs) is not touched by any task in this file — verify
  this stays true through T015's checkpoint (research.md #3)
