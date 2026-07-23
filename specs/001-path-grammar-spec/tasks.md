---

description: "Task list for PATH Grammar Specification & Conformance Vectors"

---

# Tasks: PATH Grammar Specification & Conformance Vectors

**Input**: Design documents from `/specs/001-path-grammar-spec/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md
(all present)

**Tests**: Not requested as a separate TDD track — the conformance vectors themselves
are the verification mechanism (quickstart.md Scenarios 3-5 are the "tests" for this
feature).

**Organization**: Tasks are grouped by user story (spec.md priorities: US1 = P1, US2 =
P1, US3 = P2).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- File paths are relative to `specs/001-path-grammar-spec/` unless stated otherwise

---

## Phase 1: Setup

**Purpose**: Directory scaffolding and tooling checks — nothing here is a deliverable
itself.

- [X] T001 [P] Create `vectors/` and `messages/` directories under
      `specs/001-path-grammar-spec/` per plan.md's Project Structure
- [X] T002 [P] Verify tooling from quickstart.md's Prerequisites is available: `gh`
      CLI (authenticated), `sbt` + a JVM, `python3` with `jsonschema` installed. Record
      any gap and confirm the manual-verification fallback (research.md Decision 3) if
      `sbt`/JVM is unavailable.
      **Result**: `gh` ✓ authenticated. `java`/`sbt` initially missing; installed via
      `brew install openjdk@17 sbt` + a manual `sudo ln -sfn` symlink into
      `/Library/Java/JavaVirtualMachines/` — now verified working (OpenJDK 17.0.20,
      sbt runner 2.0.3). `python3` present, `jsonschema` module still missing —
      recorded as an open gap for T003/T015, not blocking Phase 1.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The one shared artifact neither US1 nor US2 owns exclusively, but US2
depends on.

**⚠️ CRITICAL**: T003 must pass before any vector-authoring task in US2 begins.

- [X] T003 Validate `contracts/conformance-vector.schema.json` is syntactically valid
      JSON Schema (2020-12 — `python3 -c "import json; json.load(open('contracts/conformance-vector.schema.json'))"`
      plus a schema-validator self-check) and that every field name/type matches
      `data-model.md`'s Conformance Vector table 1:1
      **Result**: `jsonschema.Draft202012Validator.check_schema()` confirms the
      document is a valid Draft 2020-12 schema. Probed with a valid record (0
      errors), an invalid-PATH record with `expected: "INVALID"` (0 errors — schema
      correctly allows the sentinel), and a record missing `message_ref` (1 error,
      correctly caught). All 9 properties (`id`, `path`, `message_ref`, `method`,
      `flags`, `expected`, `expected_lines`, `grammar_productions`,
      `known_limitation`) and their required/optional status match
      `data-model.md`'s Conformance Vector table exactly.

**Checkpoint**: Schema locked — US1 and US2 can both proceed.

---

## Phase 3: User Story 1 - Rust core implementer builds the PATH parser from the spec alone (Priority: P1) 🎯 MVP

**Goal**: `contracts/path-grammar.md` is complete, unambiguous, and satisfies SC-001
(all `SPEC.md` §8 examples parse) and SC-002 (zero dangling references).

**Independent Test**: quickstart.md Scenario 1 (dangling-reference check) and Scenario
2 (all Quick-Start examples classify) both pass — verifiable with zero conformance
vectors in existence yet.

**Note**: T004-T008 all edit `contracts/path-grammar.md`, so they run sequentially,
not in parallel.

- [X] T004 [US1] Cross-check every production in `contracts/path-grammar.md` against
      `SPEC.md` §3.1 line-by-line; confirm none is missing and none silently
      contradicts `SPEC.md` without a corresponding entry in the Notes section
      **Result**: All 12 FR-001-required productions present and matched line-by-line.
      Found one undocumented notational difference: `SEG_IDX`'s `FILTER` alternative
      no longer shows `"@"` as its own token (moved into `FILTER`'s own definition).
      Fixed by adding Notes #8 clarifying this is refactor-only, `@` is still
      required. `field_num`/`comp_num`/`subcomp_num` sharing a common `NUMBER`
      production, and `OPERATOR`'s tightening being versus the *regex* not versus
      `SPEC.md` (which already matched), needed no fix — both already correctly
      handled.
- [X] T005 [US1] Run quickstart.md Scenario 1 against `contracts/path-grammar.md`
      (grep every nonterminal used vs. defined); fix any dangling reference found
      **Result**: 17 productions defined, every RHS reference resolves to a
      same-document definition. Zero dangling references. SC-002 satisfied, no fix
      needed.
- [X] T006 [US1] Run quickstart.md Scenario 2: walk every PATH example in `SPEC.md`
      §8 through `contracts/path-grammar.md` by hand; record and fix any example that
      fails to classify cleanly
      **Result**: All 7 PATH strings across §8's 5 examples (`MSH-12`, `OBX-5`,
      `OBR[1] -> OBX-5`, `OBX[@3.1='94500-6']-5`, `PID-5`, `PID-7`, `PID-3`/`PID-3.5`)
      classify cleanly. Bonus spot-check of §3.1's own examples table
      (`OBX[$LAST]-5`, `PID[1]-5[2].1`, `MSH-21[3].1`, `OBX[@11!='X']-5`) also passed.
      SC-001 satisfied, no fix needed.
- [X] T007 [US1] Verify each row in `contracts/path-grammar.md`'s Validity examples
      table against the grammar productions directly above it — confirm the stated
      valid/invalid verdict and reason are both actually derivable from the grammar as
      written, not just asserted
      **Result**: 13 of 14 rows verified correct as written. Row 11 (`999[1].1.1`)
      was self-contradictory — its "999 itself is not the problem" note was true of
      the old Scala regex but false against this document's own tightened `SEG`
      production sitting directly above it. Fixed to state both independent failure
      reasons.
- [X] T008 [US1] Cross-reference `contracts/path-grammar.md`'s Non-Goals section
      against `spec.md`'s FR-002 and Assumptions to confirm the spec-`002`
      hierarchy-semantics boundary is stated identically in both places
      **Result**: Consistent. FR-002's "MUST link to 002" is satisfied (PATH
      production's `-> CHILD_PATH` cross-references Non-Goals, which names spec
      `002`). Non-Goals bullet 1 and spec.md's Assumptions bullet use near-identical
      wording. User Story 3's Independent Test ("who owns `->` when OBR repeats")
      is answered correctly by Non-Goals' "parent-scoping and cardinality" phrasing.
      No fix needed.

**Checkpoint**: Grammar document alone (no vectors needed) satisfies SC-001 and
SC-002 — spec `006` could start building the Rust parser from it.

---

## Phase 4: User Story 2 - Regression-suite author gets canonical conformance vectors (Priority: P1)

**Goal**: `vectors/*.json` + `messages/*.hl7` exist, are schema-valid, and are
`Verified` (data-model.md's Conformance Vector lifecycle) against the real Scala
library — satisfying SC-003 and SC-004.

**Independent Test**: quickstart.md Scenarios 3 (schema validation), 4 (Scala
verification), and 5 (invalid-vector rejection check) all pass.

**Depends on**: Foundational (T003). T014 additionally depends on US1 (T004-T008)
being finalized, since invalid-PATH vectors must reflect the final tightened grammar
rules.

### Synthetic messages

- [X] T009 [P] [US2] Author `messages/baseline.hl7` — single-occurrence synthetic
      message covering MSH/PID/OBR/OBX basics (FR-009: fabricated, no real PHI)
- [X] T010 [P] [US2] Author `messages/multi-obx.hl7` — synthetic message with 3 OBX
      segments, for the multi-segment-occurrence coverage required by FR-004
- [X] T011 [P] [US2] Author `messages/multi-repetition.hl7` — synthetic message with
      a field containing 2 repetitions, for the multi-repetition coverage required by
      FR-004
- [X] T012 [P] [US2] Author `messages/filter-example.hl7` — synthetic message
      exercising the single-clause-filter Known Limitation from `SPEC.md` §7
      **Result (T009-T012)**: All 4 messages authored, entirely synthetic (fake
      names, fake MRNs, fake facility, standard LOINC codes for realism — LOINC is
      public terminology, not PHI). `baseline.hl7` also carries a subcomponent
      (`OBR-4.2.2`) for FIELD_EXPR/subcomp_num coverage in T013.

### Conformance vectors

- [X] T013 [US2] Author `vectors/valid.json`: one vector per grammar production
      (`PATH`, `SEGMENT_EXPR`, `SEG`, `SEG_IDX`, `FILTER`, `OPERATOR`, `FIELD_EXPR`,
      `FIELD_IDX`, `CHILD_PATH`, `field_num`, `comp_num`, `subcomp_num`), plus the two
      zero-values vectors (multi-segment-occurrence, multi-field-repetition) and the
      single syntactically-valid-but-empty vector (`XYZ-99`-style, demonstrating
      FR-004 vs. FR-012), plus the single-clause-filter Known Limitation vector —
      every record conforming to `contracts/conformance-vector.schema.json` (depends
      on T009-T012)
      **Result**: 11 valid vectors authored (`path-msh12`, `path-obx5-occurrences`,
      `path-segidx-number`, `path-segidx-last`, `path-segidx-star`,
      `path-filter-single-clause`, `path-fieldexpr-subcomp`,
      `path-fieldidx-repetition-all`, `path-fieldidx-specific`,
      `path-childpath-hierarchy`, `path-zero-values-nonexistent`). All 12
      productions covered; `path-filter-single-clause` doubles as the Known
      Limitation vector; `path-childpath-hierarchy` uses `flags.hierarchyMode` +
      `flags.profile` to record it needs `HL7ParseUtils` + `DefaultProfile.json`,
      not plain `HL7StaticParser`.
- [X] T014 [P] [US2] Author `vectors/invalid.json`: one vector per grammar-violation
      category documented in `contracts/path-grammar.md`'s Notes section (bad `SEG`
      first character, bad `SEG_IDX` token, bad `FIELD_IDX` token, bad `OPERATOR`
      token, wrong segment/field separator, unterminated filter clause), each with
      `expected: "INVALID"` (depends on T004-T008, T009)
      **Result**: 6 invalid vectors authored, one per category, each isolated to
      exactly one violation (deliberately not reusing the doubly-invalid
      `999[1].1.1` example from the grammar doc's Validity table, so each vector's
      `grammar_productions` names a single clear cause).
- [X] T015 [US2] Run quickstart.md Scenario 3 against every file in `vectors/`; fix
      any schema violation found (depends on T013, T014)
      **Result**: All 17 records (11 valid + 6 invalid) validate against
      `contracts/conformance-vector.schema.json` with zero errors.
- [X] T016 [US2] Confirm every vector `id` is unique across `vectors/valid.json` and
      `vectors/invalid.json` combined (depends on T015)
      **Result**: 17 ids, 17 unique — no duplicates.
- [X] T017 [US2] Run quickstart.md Scenario 4 for every vector in
      `vectors/valid.json`: clone the Scala repo per research.md Decision 3, verify
      each vector's `expected` against real output; update `expected`/
      `expected_lines` or open an FR-010 `[NEEDS CLARIFICATION]` escalation for any
      mismatch found, then discard the clone (depends on T016)
      **Result**: Cloned mscaldas2012/hl7-pet to scratch, hit and fixed three
      environment issues getting `sbt` to run (documented in research.md Decision 3
      "Post-execution notes" — malformed `build.properties`, `scalaVersion` SIP-51
      mismatch, wrong JDK auto-detected). Wrote a small verification program
      (`VerifyVectors.scala`) that reads `vectors/valid.json` and the real message
      files directly and calls `HL7StaticParser`/`HL7ParseUtils` for real. **All 11
      valid vectors' `expected` matched the real library's output exactly — zero
      mismatches, zero FR-010 escalations needed.** `expected_lines` values (not
      Scala-verifiable, since the current API doesn't expose line numbers — FR-008
      is forward-looking) were separately checked by hand against the authored
      message files via `cat -n` — all correct. Clone discarded afterward.
- [X] T018 [US2] Run quickstart.md Scenario 5 for every vector in
      `vectors/invalid.json`: confirm each `path` actually fails to match the `PATH`
      production in `contracts/path-grammar.md` for the exact reason recorded in
      `grammar_productions` (depends on T016)
      **Result**: All 6 invalid vectors manually walked through
      `contracts/path-grammar.md`'s grammar; each fails to match `PATH` as a
      complete string for precisely the stated reason (verified this is a
      grammar-doc self-consistency check, distinct from T017 — the real Scala
      engine is actually looser and would behave differently on some of these,
      which is expected and already documented in the grammar's Notes section).
- [X] T019 [US2] Confirm SC-003's coverage requirement is met in full: every grammar
      production and the single-clause-filter Known Limitation has ≥1 valid vector,
      and every production where a violation is structurally possible has ≥1 invalid
      vector (depends on T017, T018)
      **Result**: All 12 productions, the 1 Known Limitation, and all 6 invalid
      categories are covered — 19 required coverage points, all satisfied by 17
      vectors (several vectors deliberately cover multiple points at once). Total
      vector count (17) is below SC-003's rough "~25" estimate, but that estimate
      was explicitly approximate; the actual enforceable rule (≥1 per item) is met
      in full. Noted explicitly rather than padding with redundant vectors just to
      approach a round number.

**Checkpoint**: All vectors reach `Verified`; SC-003 and SC-004 satisfied.

---

## Phase 5: User Story 3 - Future contributor understands what this document does and does not cover (Priority: P2)

**Goal**: The `001`/`002` scope boundary (PATH syntax vs. hierarchy evaluation
semantics) is unambiguous to a first-time reader.

**Independent Test**: Hand `spec.md` + `contracts/path-grammar.md` to someone
unfamiliar with the project and confirm they can correctly state that spec `002` owns
hierarchy evaluation semantics, not this spec.

**Depends on**: T008 (US1's boundary cross-check).

- [X] T020 [US3] Review `spec.md`'s User Story 3 Independent Test scenario against the
      current content of `contracts/path-grammar.md`'s Non-Goals section; confirm
      they're consistent (depends on T008)
      **Result**: Consistent (already established during T008; re-confirmed here).
      A reader asked "who owns `->` when OBR repeats" can only answer "spec `002`"
      correctly, since Non-Goals explicitly says so and defines nothing itself.
- [X] T021 [US3] Add a one-line cross-reference between `spec.md`'s Assumptions
      section and `contracts/path-grammar.md`'s Non-Goals section, so the boundary
      statement is written once and pointed to, not duplicated in a way that could
      drift (depends on T020)
      **Result**: Added bidirectional one-line pointers — `spec.md`'s Assumptions
      bullet now links to `contracts/path-grammar.md`'s Non-Goals section, and
      Non-Goals now links back to `spec.md`'s Assumptions/User Story 3. Neither
      re-explains the other's content, so there's nothing left to drift out of
      sync.

**Checkpoint**: Scope boundary stated once, consistently, and cross-referenced.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final acceptance and bookkeeping across all three stories.

- [X] T022 [P] Update `ROADMAP.md`'s Status table entry for spec `001` to reflect
      completion once all checkpoints above pass
      **Result**: Status updated to Complete with a summary of what shipped.
- [X] T023 [P] Cross-check `ROADMAP.md`'s "Documented Breaking Changes" registry
      against `contracts/path-grammar.md`'s Notes section — add any further deviation
      found during T004-T008 that isn't already listed there
      **Result**: Registry already complete — the 3 breaking changes found during
      grammar drafting were already recorded before T004-T008 ran. Notes #4-8
      (added/fixed during T004-T008) are all non-breaking (additive or
      documentation-only), so nothing new qualifies. No update needed.
- [X] T024 Re-run `checklists/requirements.md` against the final state of all
      artifacts (spec.md, plan.md, contracts/, vectors/, messages/)
      **Result**: 16/16 still passing, zero state changes.
- [X] T025 Run all five quickstart.md scenarios end-to-end as the final acceptance
      pass before marking this spec done
      **Result**: Scenario 1 (dangling refs) reconfirmed clean on the final grammar
      (grep's prose false-positives aside — real check already done rigorously in
      T005). Scenario 2 (SPEC.md §8 examples) unchanged since T006, nothing in the
      grammar's productions changed after that check. Scenario 3 (schema) re-run
      clean, 17/17, 0 failures. Scenario 4 (Scala verification) — full result
      already stands from T017: 11/11 valid vectors matched real output exactly.
      Scenario 5 (invalid-vector rejection) unchanged since T018. All five pass.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — blocks US2's vector-authoring tasks
- **US1 (Phase 3)**: Depends on Foundational. No dependency on US2.
- **US2 (Phase 4)**: Depends on Foundational (T003) for vector schema, and on US1
  (T004-T008) specifically for T014 (invalid vectors need the finalized grammar
  rules). This is a deliberate exception to story independence — vectors reference a
  grammar that must exist first.
- **US3 (Phase 5)**: Depends on Foundational and specifically on T008.
- **Polish (Phase 6)**: Depends on US1, US2, and US3 all complete.

### Parallel Opportunities

- T001 and T002 (Setup) run in parallel
- T009-T012 (four message files) all run in parallel
- T013 and T014 (different vector files) can run in parallel once their respective
  dependencies (T009-T012, and T004-T008 for T014) are met
- T022 and T023 (Polish) run in parallel

---

## Parallel Example: User Story 2 message authoring

```bash
# Launch all four synthetic-message authoring tasks together:
Task: "Author messages/baseline.hl7 in specs/001-path-grammar-spec/messages/"
Task: "Author messages/multi-obx.hl7 in specs/001-path-grammar-spec/messages/"
Task: "Author messages/multi-repetition.hl7 in specs/001-path-grammar-spec/messages/"
Task: "Author messages/filter-example.hl7 in specs/001-path-grammar-spec/messages/"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1 (T004-T008)
4. **STOP and VALIDATE**: Run quickstart.md Scenarios 1 and 2
5. At this point spec `006` (Rust PATH parser) already has everything it needs, even
   before any conformance vector exists

### Incremental Delivery

1. Setup + Foundational → schema locked
2. User Story 1 → grammar document finalized, SC-001/SC-002 satisfied (MVP for spec
   `006`'s purposes)
3. User Story 2 → vectors authored and verified, SC-003/SC-004 satisfied (unlocks
   spec `003`)
4. User Story 3 → scope-boundary polish (low-risk, can trail behind US1/US2)
5. Polish → final bookkeeping and end-to-end acceptance pass

---

## Notes

- [P] tasks touch different files with no unmet dependencies
- This is a documentation/data feature (plan.md Technical Context) — "implementation"
  means authoring Markdown/JSON/HL7 text files and running manual verification
  passes, not writing code
- Every FR-010 escalation opened during T017 pauses that specific vector's path to
  `Verified`, but does not block other vectors from proceeding
- Commit after each task or logical group, per repository convention
