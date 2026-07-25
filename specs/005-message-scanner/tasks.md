---

description: "Task list for Message Scanner"
---

# Tasks: Message Scanner

**Input**: Design documents from `/specs/005-message-scanner/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/scanner-api.md](contracts/scanner-api.md), [contracts/scanner-conformance-vector.schema.json](contracts/scanner-conformance-vector.schema.json), [quickstart.md](quickstart.md)

**Tests**: Included as core deliverables, not an optional add-on — spec.md's three user
stories are each defined by an "Independent Test" that is a `cargo test` invocation
(quickstart.md), and SC-001 through SC-004 are only checkable by running tests against
conformance vectors. This is a library with no UI; the test suite *is* the proof the
feature works.

**Organization**: Tasks are grouped by user story (US1/US2/US3 from spec.md, in
priority order) so each can be implemented and verified independently. Because all
three stories are served by one `scan()` function, the Foundational phase carries the
shared happy-path scan loop (which necessarily already reads MSH-1/MSH-2 dynamically —
that's not deferrable to US2 alone); each story phase then adds the vectors, tests, and
edge-case handling that specifically prove *that story's* acceptance scenarios.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Maps the task to spec.md's US1/US2/US3
- File paths are exact and relative to the repository root

## Path Conventions

Per plan.md's Project Structure: the new Cargo workspace lives at the repository root
(`Cargo.toml`, `crates/core/`), and new fixtures live under the shared `fixtures/`
corpus (`fixtures/vectors/scanner/`, `fixtures/messages/`, `fixtures/schemas/`) per
spec `003`'s established convention — nothing scanner-specific lives under
`specs/005-message-scanner/` itself except design docs.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Stand up the Cargo workspace so anything can be built at all — this is
the first Rust code in the repository's history.

- [X] T001 Create the workspace manifest `Cargo.toml` at the repository root (`[workspace]` with `members = ["crates/core"]`, `resolver = "2"`) and the `hl7pet-core` package manifest `crates/core/Cargo.toml` (edition 2021, no `[dependencies]` — zero runtime deps per research.md #2 — with `[dev-dependencies]` `serde` (`derive` feature) and `serde_json`). Create the directory skeleton: `crates/core/src/`, `crates/core/tests/`.
- [X] T002 [P] Copy [contracts/scanner-conformance-vector.schema.json](contracts/scanner-conformance-vector.schema.json) to `fixtures/schemas/scanner-conformance-vector.schema.json` (research.md #8) — the canonical, implementation-consumed copy; the `contracts/` copy remains the spec's own design-time record, matching how specs `001`/`002` originated their schemas.
- [X] T003 [P] Register the `scanner` vector family in `fixtures/scripts/validate_corpus.py`'s family → schema mapping, pointing at `fixtures/schemas/scanner-conformance-vector.schema.json` (spec `003` FR-007's extensibility mechanism — confirm the script's existing "unrecognized family" fallback path is replaced with an explicit mapping entry, not left to fall through).

**Checkpoint**: `cargo build --workspace` succeeds against an empty `lib.rs` stub; `python3 fixtures/scripts/validate_corpus.py` runs cleanly with zero `scanner` vectors present.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared `scan()` happy path and error plumbing every user story
builds on. No user story's acceptance scenarios can be verified until this exists.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Define the core data types in `crates/core/src/scanner.rs` per [data-model.md](data-model.md): `DelimiterSet` (`Copy, Eq`), `SegmentSpan` (`Copy, Eq`), `DelimiterKind` (`Copy, Eq`, exhaustive 5-variant enum: `Field, Component, Repetition, Escape, Subcomponent`), `DelimiterOccurrence` (`Copy, Eq`), `ScanResult<'a>` (borrows `message: &'a str`), and `ScanError` (3 variants: `MissingMsh { offset }`, `TruncatedMsh { offset }`, `UnrecognizedSegment { offset, segment_index }`) with manual `Display` + `std::error::Error` impls (no derive-macro crate, per research.md #2).
- [X] T005 Implement MSH-1/MSH-2 delimiter resolution in `crates/core/src/scanner.rs`: confirm the message begins with the 3-byte segment name `MSH` (else `ScanError::MissingMsh { offset: 0 }`), read the single field-separator byte immediately after (MSH-1), then read the next 4 bytes as `component`/`repetition`/`escape`/`subcomponent` in that fixed order (MSH-2) — if the segment ends before all 4 are present, return `ScanError::TruncatedMsh { offset }` at the point of truncation (FR-002, FR-003, FR-006 conditions 1-2).
- [X] T006 Implement the segment-name recognition check in `crates/core/src/scanner.rs`: exactly 3 bytes, alphabetic-led first character, reusing spec `001`'s tightened `SEG` grammar rule (research.md #7) — a standalone function usable both for MSH (T005) and for later segments (T007).
- [X] T007 Implement the two-pass segment/delimiter scan loop in `crates/core/src/scanner.rs` (research.md #6): first pass counts segments (splitting on `\r`, `\n`, or `\r\n`, FR-008) and total delimiter occurrences using the `DelimiterSet` from T005; second pass fills two pre-sized `Vec`s (`Vec<SegmentSpan>`, `Vec<DelimiterOccurrence>`) in one final allocation each. Apply T006's check to every segment after the first, returning `ScanError::UnrecognizedSegment { offset, segment_index }` on the first violation found (FR-004, FR-006 condition 3).
- [X] T008 Implement the public entry point `pub fn scan(message: &str) -> Result<ScanResult<'_>, ScanError>` in `crates/core/src/scanner.rs`, composing T005 → T007, and the `ScanResult::segment_name(&self, segment: &SegmentSpan) -> &'a str` helper (contracts/scanner-api.md) that slices `message[span.start..span.start + 3]`.
- [X] T009 [P] Implement `crates/core/src/lib.rs`: `pub mod scanner;` plus re-exports (`pub use scanner::{scan, ScanResult, ScanError, DelimiterSet, SegmentSpan, DelimiterKind, DelimiterOccurrence};`) matching contracts/scanner-api.md's public surface exactly.
- [X] T010 [P] Scaffold `crates/core/tests/scanner_vectors.rs`: `serde`-deserializable structs mirroring `fixtures/schemas/scanner-conformance-vector.schema.json` (T002), a loader that globs `fixtures/vectors/scanner/*.json` relative to the workspace root, and a dispatch function that runs `scan()` against each vector's `message_ref` file, branching on whether the vector has `expected_error` (compare `ScanError` variant + fields) or `expected_delimiters`/`expected_segments`/optional `expected_delimiter_occurrences` (compare `Ok(ScanResult)` fields). No vector files exist yet (Phases 3-5 add them) — this task only needs to compile and run as a no-op.

**Checkpoint**: `cargo build --workspace` and `cargo test -p hl7pet-core` both succeed (the vector test passes vacuously with zero vectors). `scan()` is fully implemented for every code path in spec.md FR-002 through FR-008, just not yet vector-verified.

---

## Phase 3: User Story 1 - Downstream Rust components get segment/field offsets without allocating (Priority: P1) 🎯 MVP

**Goal**: Prove `scan()`'s output is a complete, zero-copy offset map — every segment
and every one of the five delimiter kinds located — with allocation count independent
of field/component/repetition count.

**Independent Test**: Run `scan()` alone against a `fixtures/messages/` file and
confirm a complete offset map with allocation count varying only by segment count
(spec.md US1 Independent Test).

### Implementation for User Story 1

- [X] T011 [P] [US1] Author `fixtures/messages/scanner-multi-segment.hl7` — a synthetic, standard-delimiter message with multiple segment types and at least one repeating field/component/subcomponent, chosen to exercise all five `DelimiterKind` variants at least once.
- [X] T012 [P] [US1] Author `fixtures/vectors/scanner/standard-delimiters.json` with vectors `scan-001` (full `expected_segments` + `expected_delimiters` for T011's message) and `scan-002` (adds `expected_delimiter_occurrences` covering at least one occurrence of each `DelimiterKind`, per contracts/scanner-conformance-vector.schema.json).
- [X] T013 [US1] Fill in `crates/core/tests/scanner_vectors.rs`'s success-path assertion body (stubbed in T010) using T012's vectors: compare `ScanResult.segments`, `.delimiters`, and (when present) `.delimiter_occurrences` field-for-field against the vector's `expected_*` values.
- [X] T014 [US1] Add a unit test in `crates/core/src/scanner.rs` using a global-allocator wrapper that counts `alloc` calls (a small `#[global_allocator]` counting shim, crate-free per research.md #2) asserting `scan()` performs exactly 2 heap allocations for both a small message and a large synthetic one with many more fields/components/repetitions — proving allocation count is independent of field count (SC-004).
- [X] T015 [US1] Add unit tests in `crates/core/src/scanner.rs` for `ScanResult::segment_name` against small in-memory `&str` messages (not fixtures), confirming it returns the correct borrowed 3-byte slice for the first and a later segment.

**Checkpoint**: User Story 1 fully functional and independently verified — `cargo test -p hl7pet-core --test scanner_vectors -- standard_delimiters` and the T014 allocation test both pass.

---

## Phase 4: User Story 2 - Non-standard delimiters are read from MSH-1/MSH-2 instead of hardcoded (Priority: P1)

**Goal**: Prove delimiters are resolved from each message's own MSH-1/MSH-2 — correct
for non-standard delimiters, and byte-identical to today's implicit hardcoded behavior
for standard ones.

**Independent Test**: Feed a message with non-standard MSH-1/MSH-2 and confirm correct
offsets using its own declared characters; feed a standard-delimiter message and
confirm output is unchanged from what hardcoding would have produced (spec.md US2
Independent Test, Acceptance Scenario 1).

### Implementation for User Story 2

- [X] T016 [P] [US2] Author `fixtures/messages/scanner-non-standard-delimiters.hl7` — a synthetic message whose MSH-1 is a non-`|` character (e.g. `#`) and whose MSH-2 declares non-default component/repetition/escape/subcomponent characters, used consistently by every subsequent segment.
- [X] T017 [P] [US2] Author `fixtures/messages/scanner-msh1-collision.hl7` — a synthetic message where MSH-1's character also appears as ordinary data within MSH-2's own encoding-characters field, to exercise spec.md US2 Acceptance Scenario 3's disambiguation requirement.
- [X] T018 [US2] Author `fixtures/vectors/scanner/non-standard-delimiters.json` with vectors `scan-003` (full expected output for T016's message, proving SC-002) and `scan-004` (T017's collision message, proving MSH-1/MSH-2 are read positionally, not by character search).
- [X] T019 [US2] Extend `crates/core/tests/scanner_vectors.rs` (no new code needed beyond T013's generic dispatch — confirm the new vector files are picked up by the existing glob loader and pass).
- [X] T020 [US2] Add a regression test in `crates/core/tests/scanner_vectors.rs` (or a dedicated `crates/core/tests/scanner_regression.rs`) that runs `scan()` against every `fixtures/messages/*.hl7` file already used by specs `001`-`003` (all standard-delimiter) and asserts the resolved `DelimiterSet` equals the literal `DelimiterSet { field: b'|', component: b'^', repetition: b'~', escape: b'\\', subcomponent: b'&' }` — proving FR-005/SC-001's no-regression requirement against the full existing corpus, not just new scanner-specific fixtures.

**Checkpoint**: User Stories 1 and 2 both independently functional — `cargo test -p hl7pet-core --test scanner_vectors -- non_standard_delimiters` passes and the T020 regression test shows zero deviation across the existing corpus.

---

## Phase 5: User Story 3 - Malformed MSH segments produce a clear structural error (Priority: P2)

**Goal**: Every malformed-MSH condition from FR-006 produces a specific, located
`ScanError` — never a panic, never a silent mis-scan.

**Independent Test**: Feed each malformed-MSH case and confirm a distinct, correctly
located structural error (spec.md US3 Independent Test).

### Implementation for User Story 3

- [X] T021 [P] [US3] Author malformed-MSH fixture messages under `fixtures/messages/`: `scanner-empty.hl7` (empty file), `scanner-no-msh.hl7` (does not start with `MSH`), `scanner-truncated-msh2.hl7` (MSH segment ends before a complete MSH-2), `scanner-bad-segment-name.hl7` (well-formed MSH followed by a segment with an unrecognizable name) — one file per FR-006 condition plus the empty-message Edge Case.
- [X] T022 [P] [US3] Author `fixtures/vectors/scanner/malformed-msh.json` with one vector per T021 fixture (`scan-005` through `scan-008`), each with `expected_error` giving the exact `ScanError` variant, `offset`, and (for `UnrecognizedSegment`) `segment_index` per data-model.md's `ScanError` table.
- [X] T023 [US3] Fill in `crates/core/tests/scanner_vectors.rs`'s error-path assertion body (stubbed in T010) using T022's vectors: assert `scan()` returns `Err` with the exact variant/fields the vector declares, and that no `Ok(ScanResult)` is ever produced alongside (FR-006's "produces no offset map").
- [X] T024 [US3] Add a panic-safety unit test in `crates/core/src/scanner.rs` feeding `scan()` a range of pathological inputs (empty string, single byte, a string containing only segment terminators, a very short truncated string at every possible truncation point of a valid MSH) and asserting every call returns a `Result` — never panics — per contracts/scanner-api.md's explicit no-panic postcondition.

**Checkpoint**: All three user stories independently functional — `cargo test -p hl7pet-core --test scanner_vectors -- malformed_msh` and the T024 panic-safety test both pass.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Tie the three stories together into one verified, documented deliverable.

- [X] T025 [P] Run `python3 fixtures/scripts/validate_corpus.py` against the full corpus including the new `scanner` family; confirm `scan-*` ids are unique corpus-wide, every `message_ref` resolves, and the coverage report lists `scanner` as a recognized (not merely tolerated) family per research.md #8.
- [X] T026 [P] Add `crates/core/README.md` summarizing how to build and test the crate, linking to [quickstart.md](quickstart.md) and [contracts/scanner-api.md](contracts/scanner-api.md) rather than duplicating them.
- [X] T027 Run the full [quickstart.md](quickstart.md) validation end-to-end (all 8 steps) and record the outcome.
- [X] T028 Update [ROADMAP.md](../../ROADMAP.md)'s spec `005` status row from "Draft" to "Implemented," noting the new `crates/core` scanner module and the `fixtures/vectors/scanner` family now exist, and that it's ready for spec `006` (PATH parser) to build against.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup (T001's workspace must exist) — BLOCKS all user stories.
- **User Stories (Phase 3-5)**: All depend on Foundational completion.
  - US1 (T011-T015) has no dependency on US2/US3.
  - US2 (T016-T020) depends on Foundational's delimiter-resolution logic (T005), not on US1's tasks — independently testable once Foundational lands, though T019 reuses T013's dispatch code as shared test-harness infrastructure.
  - US3 (T021-T024) depends only on Foundational (T007's error branches must already exist) — independently testable in parallel with US1/US2.
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### Within Phase 2 (Foundational)

- T004 has no dependencies — the type definitions come first.
- T005 depends on T004 (needs `DelimiterSet`/`ScanError` to exist).
- T006 has no dependency on T005 (pure byte-pattern check) but is used by T007.
- T007 depends on T005, T006.
- T008 depends on T007.
- T009 depends on T004, T008 (re-exports need the finished public surface).
- T010 depends on T002 (schema shape) and T008 (needs `scan()` to call).

### Within Phase 3 (US1)

- T011, T012 have no dependencies on each other but T012 references T011's message file.
- T013 depends on T010 (harness scaffold) and T012 (vectors to assert against).
- T014, T015 have no dependency on T011-T013 — parallelizable with them.

### Parallel Opportunities

- Setup: T002 and T003 in parallel (different files); T001 first since both later tasks assume the workspace layout exists.
- Foundational: T009 and T010 in parallel once T004-T008 land; T004 must come first, then T005/T006 in parallel, then T007, T008.
- User Stories: US2 and US3 can proceed in parallel with US1 once Foundational is done — all three exercise independent code paths (offset completeness, delimiter resolution, error branches) that already exist after Phase 2.
- Within US1: T011+T012 as one parallel pair, T014+T015 as another, independent of the first.
- Within US2: T016 and T017 in parallel (different fixture files).
- Within US3: T021 and T022 in parallel (fixtures vs. vectors), though T022 references T021's filenames.
- Polish: T025 and T026 in parallel.

---

## Parallel Example: Foundational Phase

```bash
# T005 and T006 together once T004's types exist (independent logic, same file — coordinate on merge):
Task: "Implement MSH-1/MSH-2 delimiter resolution in scanner.rs"
Task: "Implement segment-name recognition check in scanner.rs"
```

## Parallel Example: User Story 1

```bash
# Launch T011 and T012 together, then T014/T015 independently of T013:
Task: "Author fixtures/messages/scanner-multi-segment.hl7"
Task: "Author fixtures/vectors/scanner/standard-delimiters.json"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (blocks everything else — and already implements
   MSH-1/MSH-2 dynamic reading, since US1 and US2 share one code path).
3. Complete Phase 3: User Story 1 — this alone proves the offset map is complete and
   allocation-count-independent (SC-004), the spec's foundational zero-copy claim.
4. **STOP and VALIDATE**: run quickstart.md steps 1-4, 7.

### Incremental Delivery

1. Setup + Foundational → the `scan()` function exists and already reads delimiters
   dynamically, but is unverified against vectors.
2. US1 → offset completeness and allocation-count independence proven (MVP).
3. US2 → the specific "MSH-1/MSH-2 must be standard" limitation fix is proven against
   both non-standard and the full existing standard-delimiter corpus (zero regressions).
4. US3 → malformed-MSH structural errors proven distinct and panic-free.
5. Polish → full quickstart validation + docs + ROADMAP update.

---

## Notes

- [P] tasks touch different files and have no unmet dependency within their phase.
- [Story] labels (US1/US2/US3) map directly to spec.md's prioritized user stories.
- US1 and US2 are both P1 in spec.md; they are sequenced here (US1 before US2) only for
  narrative clarity — both depend solely on Foundational, not on each other, and can
  genuinely run in parallel once Phase 2 is complete.
- Every fixture message (T011, T016, T017, T021) MUST be synthetic/fabricated — never
  real or de-identified patient data, matching specs `001`-`003`'s established
  convention (spec `001` FR-009, carried forward by spec `003` FR-010).
- Implementation deviation from T010's original description: `crates/core/tests/scanner_vectors.rs` ended up as three `#[test]` functions (`standard_delimiters`, `non_standard_delimiters`, `malformed_msh`), each loading one specific vector file by name, rather than one test globbing the whole `fixtures/vectors/scanner/` directory. Discovered during T027's quickstart run — steps 4-6 use `cargo test -- <name>` to target one story's vectors, which requires separate named tests to filter on. The shared assertion logic (`assert_vectors`) is still a single function, so there's no duplicated comparison code.
- Commit after each task or logical group; stop at any checkpoint to validate a story
  independently before continuing.
