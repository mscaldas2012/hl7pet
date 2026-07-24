---

description: "Task list for Hierarchy-Mode Semantics Specification"

---

# Tasks: Hierarchy-Mode Semantics Specification

**Input**: Design documents from `/specs/002-hierarchy-semantics/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md
(all present)

**Tests**: Not requested as a separate TDD track — the conformance vectors themselves
are the verification mechanism (quickstart.md Scenarios 3-6 are the "tests" for this
feature), matching spec `001`'s approach.

**Organization**: Tasks are grouped by user story (spec.md priorities: US1 = P1, US2 =
P1, US3 = P2, US4 = P3).

**Scope note**: Findings specific to the Validation module (`StructureValidator`'s
cardinality/usage-predicate parsing, the unused `conformance`/`xtraValidation` profile
fields) are explicitly deferred to whenever the Validation module (Roadmap 2000-2999)
is implemented — not part of this task list.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- File paths are relative to `specs/002-hierarchy-semantics/` unless stated otherwise

---

## Phase 1: Setup

**Purpose**: Directory scaffolding and tooling checks — nothing here is a deliverable
itself.

- [X] T001 [P] Create `vectors/`, `messages/`, and `profiles/` directories under
      `specs/002-hierarchy-semantics/` per plan.md's Project Structure
      **Result**: All three directories created.
- [X] T002 [P] Verify tooling from quickstart.md's Prerequisites: confirm the local
      Scala checkout at `/Users/m/projects/personal/HL7-PET` is still present and
      buildable (`sbt compile`), `python3` with `jsonschema` installed. If the
      checkout is gone, fall back to spec `001`'s scratch-clone approach
      (`specs/001-path-grammar-spec/research.md` Decision 3), including its three
      documented local-only `sbt`/JDK fixes.
      **Result**: Checkout present. `python3`/`jsonschema` 4.26.0 present. Hit the
      same three environment issues spec `001`'s research documented, but this time
      in the persistent checkout rather than a scratch clone: (1)
      `project/build.properties` had a live, **committed** unresolved git merge
      conflict (`<<<<<<< HEAD` markers) — fixed locally to `sbt.version=1.10.5`; (2)
      SIP-51 `scalaVersion` mismatch — bumped to `2.13.15` in `build.sbt`; (3) sbt's
      launcher auto-selected Homebrew Java 26 despite JDK 17 being the only
      registered JVM — fixed by exporting `JAVA_HOME` explicitly before invoking
      `sbt`. `sbt compile` now succeeds. These are local-only edits in the external
      checkout, not committed/pushed — flagged separately to the user, since #1 is a
      real unresolved conflict in that repo's own history, not an artifact of this
      session.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Lock the schema every vector must conform to before any vector is authored.

**⚠️ CRITICAL**: T003 must pass before any vector-authoring task in US2 begins.

- [X] T003 Validate `contracts/hierarchy-conformance-vector.schema.json` is
      syntactically valid JSON Schema (2020-12) and that every field name/type
      matches `data-model.md`'s Hierarchy Conformance Vector table 1:1
      (`python3 -c "import json,jsonschema; s=json.load(open('specs/002-hierarchy-semantics/contracts/hierarchy-conformance-vector.schema.json')); jsonschema.Draft202012Validator.check_schema(s)"`)
      **Result**: `check_schema()` confirms valid Draft 2020-12. All 10 fields
      (`id`, `path`, `profile_ref`, `message_ref`, `method`, `flags`, `expected`,
      `expected_lines`, `semantic_rules`, `known_limitation`) and the 7
      required/3 optional split match `data-model.md`'s table exactly. Probed with a
      valid record (0 errors), a record missing `profile_ref` (1 error, correctly
      caught), a valid `getFirstValue` scalar record (0 errors), and a
      `getFirstValue` record with a 2D-array `expected` (1 error, correctly
      rejected).

**Checkpoint**: Schema locked — US1 and US2 can both proceed.

---

## Phase 3: User Story 1 - Rust core implementer builds lazy hierarchy navigation from the spec alone (Priority: P1) 🎯 MVP

**Goal**: `contracts/hierarchy-semantics.md` Section A is complete, accurate against
the real Scala source, and satisfies SC-001.

**Independent Test**: quickstart.md Scenario 1 (SPEC.md §3.3/§4.1 coverage check) passes.

**Note**: T004-T006 all review/edit `contracts/hierarchy-semantics.md`, so they run
sequentially, not in parallel.

- [X] T004 [US1] Cross-check `contracts/hierarchy-semantics.md` Section A (A.1-A.6)
      against `SPEC.md` §3.3 and §4.1 line-by-line per quickstart.md Scenario 1;
      confirm every hierarchy-related sentence there is traceable to a subsection here,
      and fix any gap found
      **Result**: Every sentence in §3.3 and the constructor comments in §4.1 traces
      to Section A. Found and fixed one real gap: §3.3's "requires a profile" and
      §7's "profile is supplied" both conflate "hierarchy mode" with "caller supplies
      a profile" — re-reading `HL7ParseUtils.scala:14-26` shows the 1-arg constructor
      still loads a default `PhinGuideProfile.json` (unused, since `buildHierarchy`
      gates everything) while a 3-arg call with an explicit `null` profile and
      `buildHierarchy=true` falls back to that same default *and enters hierarchy
      mode*. Added a paragraph to Section A.5 documenting this: `buildHierarchy`
      alone gates hierarchy behavior, not "a profile was passed."
- [X] T005 [US1] Re-verify each factual claim in Section A against the cited source
      lines in `/Users/m/projects/personal/HL7-PET/src/main/scala/gov/cdc/hl7/HL7HierarchyParser.scala`
      and `HL7ParseUtils.scala` (re-open the actual file at the cited line ranges for
      A.1, A.2, A.4, A.5, A.6 and confirm the prose still matches); fix any drift
      **Result**: Re-opened and re-checked every cited range
      (`HL7HierarchyParser.scala:47-93`, `HL7ParseUtils.scala:74-90`, `:104`,
      `:106-111`, `HL7StaticParser.scala:138-154`) against `research.md`'s Decisions
      1 and 3. All citations accurate, no drift found beyond the A.5 gap already
      fixed in T004.
- [X] T006 [US1] Confirm Section A contains no unresolved ambiguity per FR-010 — every
      corner found underspecified in `SPEC.md` during T004/T005 has an explicit
      resolved-decision writeup, not a silent guess
      **Result**: `grep -n "NEEDS CLARIFICATION\|TBD\|TODO"` across
      `contracts/hierarchy-semantics.md` and `research.md` finds only one hit, inside
      Decision 5's "alternatives considered" prose explaining why an escalation was
      *not* needed — not an actual open marker. Zero unresolved items.

**Checkpoint**: `contracts/hierarchy-semantics.md` Section A alone satisfies SC-001 —
spec `008` has an accurate compatibility-floor description even before any conformance
vector exists.

---

## Phase 4: User Story 2 - Regression-suite author gets canonical hierarchy conformance vectors (Priority: P1)

**Goal**: `profiles/*.json`, `messages/*.hl7`, and `vectors/*.json` exist, are
schema-valid, and are `Verified` (data-model.md's Conformance Vector lifecycle)
against the real Scala library — satisfying SC-003 and SC-004.

**Independent Test**: quickstart.md Scenarios 3 (schema validation), 4 (rule
coverage), 5 (Scala verification), and 6 (A.4 reproducibility) all pass.

**Depends on**: Foundational (T003). Vector authoring (T012, T013) additionally
depends on US1 (T004-T006) being finalized, since vectors must reflect the final
documented rules, especially Section A.4's precise mechanism.

### Synthetic profiles

- [X] T007 [P] [US2] Author `profiles/basic-two-level.json` — synthetic
      `segmentDefinition`: `MSH -> {PID, ORC, OBR -> {OBX}}`, cardinalities matching
      the shape (but not the literal content) of the real bundled `BasicProfile.json`
      **Result**: Authored.
- [X] T008 [P] [US2] Author `profiles/deep-nested.json` — synthetic
      `segmentDefinition` four levels deep, mirroring the real `COVID_ORC.json`'s
      actual nesting shape: `MSH -> {ORC, OBR -> {NTE, OBX -> {NTE}, SPM -> {OBX}}}`
      (needed so `OBX` legitimately appears at two different nesting positions under
      the same `OBR`, exercising Section A.1's nearest-enclosing-ancestor rule for
      real, not just in the abstract)
      **Result**: Authored, with `OBX` under `OBR` deliberately marked `[1..*]`
      (required) rather than `[0..*]`, so the second `OBR` occurrence (zero children)
      also demonstrates A.3-cardinality-not-enforced-by-navigation, not just
      A.2-zero-children.

### Synthetic messages

- [X] T009 [P] [US2] Author `messages/basic-hierarchy.hl7` using
      `profiles/basic-two-level.json`'s shape: one `MSH`/`PID`/`ORC`, one `OBR` with
      two `OBX` children — covers A.2-single-hop-basic and A.5-static-mode-fallback
      (FR-012: fabricated, no real PHI)
      **Result**: Authored, 6 lines, fully synthetic.
- [X] T010 [P] [US2] Author `messages/complex-hierarchy.hl7` using
      `profiles/deep-nested.json`'s shape, designed to exercise as many Section A/B
      rules as possible in one message: two `OBR` occurrences — the first with
      interleaved `OBX`/`NTE` children plus a nested `SPM` with its own `OBX` child
      (for A.1-nearest-enclosing-ancestor, A.2-multi-parent-combined-children,
      A.4-cross-parent-child-indexing's type-mixing and off-by-one behaviors, and a
      2-hop `OBR -> OBX -> NTE` chain for B.2-multi-level-navigation), the second
      `OBR` with zero `OBX` children (for A.2-zero-children)
      **Result**: Authored, 12 lines. Construction order was deliberately worked out
      segment-by-segment against Section A.1's nearest-enclosing-ancestor rule (two
      `NTE`s placed immediately after `OBR` so they attach as `OBR`'s *direct*
      children rather than nesting under a preceding `OBX`, matching how the real
      algorithm's "try current position first" rule actually behaves) so the tree
      shape used by the vectors below is exactly as designed, not accidental.
- [X] T011 [P] [US2] Author `messages/unrecognized-segment.hl7` — reuse
      `basic-hierarchy.hl7`'s shape plus one trailing segment type absent from
      `profiles/basic-two-level.json` entirely (e.g. an `IN1` segment) — covers
      A.1-unrecognized-segment-dropped (confirm it's dropped from the tree but still
      visible to a flat, non-`->` query on the same message)
      **Result**: Authored, 7 lines (basic-hierarchy.hl7 + trailing `IN1`).

### Conformance vectors

- [X] T012 [US2] Author `vectors/basic.json`: vectors for
      A.2-single-hop-basic, A.5-static-mode-fallback (same `path`, once evaluated with
      `profiles/basic-two-level.json` and once with no profile passed — record both as
      separate vectors sharing `message_ref`), A.3-cardinality-not-enforced-by-navigation
      (a parent occurrence whose profile cardinality is violated but navigation still
      returns whatever children exist, no error), and
      A.1-unrecognized-segment-dropped (using `messages/unrecognized-segment.hl7`) —
      every record conforming to `contracts/hierarchy-conformance-vector.schema.json`
      (depends on T007, T009, T011)
      **Result**: Authored `hier-001` through `hier-004` (single-hop-basic,
      static-mode-fallback, unrecognized-segment-dropped, and the off-by-one
      A.4 case — the A.3 cardinality case ended up co-located with the
      zero-children vector in `complex.json` instead, since it needed
      `deep-nested.json`'s required-`OBX` cardinality). Also discovered and
      documented, while computing `expected` precisely, that hierarchy `->` results
      use a different empty-result `Option` convention than flat paths (`None` only
      when the parent side fails to match `SEGMENT_EXPR` at all; `Some(emptyArray)`
      when the parent matches but finds zero children) — added to
      `contracts/hierarchy-semantics.md` Section A.2 with exact source citations
      before finalizing `expected` values.
- [X] T013 [US2] Author `vectors/complex.json`: vectors for
      A.1-nearest-enclosing-ancestor (distinguishing the `OBR->OBX` branch from the
      `OBR->SPM->OBX` branch), A.2-multi-parent-combined-children,
      A.2-zero-children (the second `OBR` occurrence), two vectors for
      A.4-cross-parent-child-indexing (one demonstrating the type-mixing empty-result
      case, one demonstrating the off-by-one/no-rebasing case per
      `contracts/hierarchy-semantics.md` Section A.4 points 2 and 3),
      A.6-chained-arrow-silently-empty (a 2-arrow path against today's engine,
      `expected` = empty), and B.2-multi-level-navigation (the same 2-hop path,
      `expected` = the real chained result, explicitly labeled per quickstart.md
      Scenario 5 as "designed future behavior, not verifiable against current Scala"
      since no chaining implementation exists yet) — every record conforming to the
      schema (depends on T008, T010)
      **Result**: Authored `hier-005` through `hier-010` (6 vectors). `hier-007`
      and `hier-008` use `expected: []` (`Some(emptyArray)`), not `null`, per the
      Option-convention finding from T012. `hier-010` (B.2) is the only vector in
      the whole suite excluded from SC-004 verification (T017) — its `expected` is
      the designed future 2-hop result, not current behavior.
- [X] T014 Run quickstart.md Scenario 3 against every file in `vectors/`; fix any
      schema violation found (depends on T012, T013)
      **Result**: All 10 records across both files validate against
      `contracts/hierarchy-conformance-vector.schema.json` with zero errors.
- [X] T015 Confirm every vector `id` is unique across `vectors/basic.json` and
      `vectors/complex.json` combined, and does not collide with spec `001`'s
      `path-*` ids (depends on T014)
      **Result**: 10 ids (`hier-001` through `hier-010`), all unique, zero collisions
      with spec `001`'s `path-*` namespace.
- [X] T016 Run quickstart.md Scenario 4 (SC-003 rule-coverage check): confirm every
      `semantic_rules` enum value in `contracts/hierarchy-conformance-vector.schema.json`
      is exercised by at least one vector (depends on T015)
      **Result**: All 10 enum values covered by at least one vector; several vectors
      deliberately cover more than one rule at once (e.g. `hier-005` covers both A.1
      and A.2; `hier-007` covers both A.2-zero-children and A.3).
- [X] T017 Run quickstart.md Scenario 5 for every vector in `vectors/basic.json` and
      `vectors/complex.json` *except* the B.2-multi-level-navigation vector: using the
      local Scala checkout, verify each vector's `expected` against real output;
      update `expected`/`expected_lines` or open an FR-011 `[NEEDS CLARIFICATION]`
      escalation for any mismatch found (depends on T016)
      **Result**: Wrote a throwaway `VerifyHierarchyVectors.scala` in the local
      checkout (`/Users/m/projects/personal/HL7-PET/src/test/scala/`, removed after
      use), constructing the real `HL7ParseUtils` against each vector's profile and
      message and calling `getValue` directly. **All 9 verifiable vectors
      (`hier-001` through `hier-009`) matched their documented `expected` value
      exactly — zero discrepancies, zero FR-011 escalations needed**, including the
      exact `None`/`Some(empty)` distinction and the precise off-by-one
      (`hier-004`) and type-mixing (`hier-008`) mechanics predicted from source
      reading alone. Getting the real Scala checkout runnable required the same
      three environment fixes as T002 (JAVA_HOME export, `scalaVersion` bump,
      `sbt.version` fix for the pre-existing unresolved merge conflict) — already
      applied and left in place from T002, not repeated here.
- [X] T018 Run quickstart.md Scenario 6 specifically: construct the two manual checks
      described there (type-mixing, off-by-one) against the real Scala library using
      `messages/complex-hierarchy.hl7` and `profiles/deep-nested.json`; confirm
      Section A.4's two sub-claims are exactly reproduced, or correct Section A.4 and
      the corresponding vectors if the real behavior differs from what was traced
      from source reading (depends on T010, T013)
      **Result**: Superseded by T017's automated verification, which covers exactly
      these two checks as `hier-004` (off-by-one, on `basic-hierarchy.hl7`) and
      `hier-008` (type-mixing, on `complex-hierarchy.hl7`) — both matched
      Section A.4's predictions exactly. No further manual check needed.
- [X] T019 Confirm SC-003's coverage requirement is met in full and SC-004's
      verification requirement is met for every vector except the explicitly-labeled
      forward-looking B.2 vector (depends on T017, T018)
      **Result**: SC-003 met (T016: 10/10 rules covered, 10 vectors total, above the
      spec's ~8 minimum). SC-004 met (T017: 9/9 verifiable vectors matched exactly);
      `hier-010` remains explicitly labeled forward-looking, correctly excluded from
      the SC-004 claim.

**Checkpoint**: All verifiable vectors reach `Verified`; SC-003 and SC-004 satisfied.

---

## Phase 5: User Story 3 - Project decides whether Rust hierarchy navigation requires a profile (Priority: P2)

**Goal**: Confirm `contracts/hierarchy-semantics.md` Section B.1 (already drafted
during `/speckit-plan`'s research phase) is a definite, well-supported decision, not
an open question.

**Independent Test**: quickstart.md Scenario 2 passes for Section B.1 specifically.

**Depends on**: Foundational. Independent of US1/US2.

- [X] T020 [US3] Re-read `research.md` Decision 2 and `contracts/hierarchy-semantics.md`
      Section B.1 together; confirm the decision statement, rationale, and rejected
      alternatives are mutually consistent and that Section B.1 alone (without needing
      to open `research.md`) gives a reader a definite answer
      **Result**: Consistent. Section B.1 opens with an unambiguous **Decision:**
      statement ("still requires an explicit, declarative profile... MUST NOT
      eagerly materialize a full tree") that stands alone without requiring
      `research.md` — `research.md` Decision 2 supplies the same conclusion plus
      rejected alternatives, nothing contradicts Section B.1's summary.
- [X] T021 [US3] Cross-check Section B.1's claim ("profile required, full eager tree
      not required") against Constitution Principles II and V in
      `.specify/memory/constitution.md`; confirm no unstated tension remains
      **Result**: No tension. Principle II's own text ("defer hierarchy construction
      until it is actually requested (`buildHierarchy`)") matches Section B.1's
      "computed lazily, on demand" almost word-for-word. Principle V's text
      ("declarative, versionable JSON profiles... rather than hard-coded
      per-message-type logic") matches Section B.1's "requires an explicit,
      declarative profile... without hard-coding per-message-type logic" almost
      verbatim. The two principles answer different sub-questions of FR-004
      (whether a profile is required vs. when/how the tree gets built) and don't
      compete.

**Checkpoint**: Profile-requirement question closed — spec `008` can be planned
against a definite answer.

---

## Phase 6: User Story 4 - Project decides whether multi-level (chained) navigation is worth adding (Priority: P3)

**Goal**: Confirm `contracts/hierarchy-semantics.md` Section B.2 (already drafted
during `/speckit-plan`'s research phase) is a definite decision with a falsifiable
performance claim (SC-005), and that it's correctly framed as a Backward-Compatible
Addition against spec `001`.

**Independent Test**: quickstart.md Scenario 2 passes for Section B.2 specifically;
SC-005 is satisfied.

**Depends on**: Foundational. Benefits from US2's B.2 vector (T013) existing, since it
demonstrates the decision concretely, but does not require full SC-004 verification of
that vector (it's forward-looking, per T013's note).

- [X] T022 [US4] Re-read `research.md` Decision 4 and `contracts/hierarchy-semantics.md`
      Section B.2 together; confirm SC-005's falsifiable performance claim
      ("O(message size) total across all hops, hops partition rather than repeat the
      scan") is stated precisely enough that spec `009`'s eventual benchmark could
      actually falsify it, not just assert it
      **Result**: Precise enough. Section B.2 names a concrete comparison baseline
      ("an N-hop chained query's cost against N independently-run single-hop queries
      over disjoint ranges of the same message") and a concrete failure condition
      ("if chaining shows super-linear cost relative to that baseline, this
      recommendation should be revisited") — a real benchmark could be designed
      directly from this text, not just an unquantified "should be fine" assertion.
- [X] T023 [US4] Cross-reference spec `001`'s `contracts/path-grammar.md` `CHILD_PATH`
      production and Non-Goals section against this spec's proposed grammar addition
      (Section B.2's recursive `CHILD_PATH` alternative); add a one-line forward
      pointer in spec `001`'s Non-Goals section noting spec `002` has since proposed a
      specific additive grammar change here, so the two documents don't drift out of
      sync the way spec `001`'s Non-Goals section already cross-references this spec
      for evaluation semantics (mirrors spec `001` T020-T021's cross-referencing
      pattern)
      **Result**: Added an "Update (spec `002`)" note to
      `specs/001-path-grammar-spec/contracts/path-grammar.md`'s Non-Goals section,
      describing the proposed recursive `CHILD_PATH` addition and explicitly
      deferring the grammar file's own update to whichever spec (likely `008`)
      actually implements it — spec `001`'s own SC-004-verified deliverable stays
      unchanged, only a forward pointer was added.

**Checkpoint**: Multi-level navigation question closed, with a benchmarkable claim and
a grammar cross-reference back into spec `001`.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Final acceptance and bookkeeping across all four stories.

- [X] T024 [P] Update `ROADMAP.md`'s Status table entry for spec `002` to reflect
      completion once all checkpoints above pass
      **Result**: Status updated to Complete with a summary of what shipped,
      including the source-verified semantics, the two resolved decisions, and the
      newly-discovered child-indexing limitation.
- [X] T025 [P] Re-run `checklists/requirements.md` against the final state of all
      artifacts (spec.md, plan.md, contracts/, vectors/, messages/, profiles/)
      **Result**: 16/16 still passing. Updated the Notes section to record that
      FR-004/FR-005 are now resolved (were open at spec time by design), not just
      that they were deliberately deferred.
- [X] T026 Run all six quickstart.md scenarios end-to-end as the final acceptance pass
      before marking this spec done (depends on T019, T021, T023)
      **Result**: Scenario 1 (SPEC.md coverage) — confirmed clean in T004, one real
      gap found and fixed there (buildHierarchy vs. profile-presence conflation).
      Scenario 2 (decisions settled) — re-confirmed: both `### B.1` and `### B.2`
      headers present with definite `**Decision**:` statements, re-verified in
      T020-T023. Scenario 3 (schema validation) — re-run clean, 10/10 vectors, 0
      errors. Scenario 4 (rule coverage) — re-run clean, 10/10 semantic rules
      covered. Scenario 5 (Scala verification) — stands from T017: 9/9 verifiable
      vectors matched exactly. Scenario 6 (A.4 reproducibility) — satisfied by
      `hier-004`/`hier-008` within the same T017 run. All six pass.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — blocks US2's vector-authoring tasks
- **US1 (Phase 3)**: Depends on Foundational. No dependency on US2/US3/US4.
- **US2 (Phase 4)**: Depends on Foundational (T003) for vector schema, and on US1
  (T004-T006) specifically for T012/T013 (vectors must reflect the finalized
  semantics document, especially Section A.4).
- **US3 (Phase 5)**: Depends on Foundational only. Independent of US1/US2/US4.
- **US4 (Phase 6)**: Depends on Foundational only; T023 benefits from but does not
  strictly require T013 (US2's B.2 vector) to exist first.
- **Polish (Phase 7)**: Depends on US1, US2, US3, and US4 all complete.

### Parallel Opportunities

- T001 and T002 (Setup) run in parallel
- T007 and T008 (profiles) run in parallel
- T009, T010, T011 (messages) run in parallel once their respective profiles exist
- US3 (Phase 5) and US4 (Phase 6) can run fully in parallel with each other and with
  US1/US2, since all four only depend on Foundational
- T024 and T025 (Polish) run in parallel

---

## Parallel Example: User Story 2 fixture authoring

```bash
# Launch both profile-authoring tasks together:
Task: "Author profiles/basic-two-level.json in specs/002-hierarchy-semantics/profiles/"
Task: "Author profiles/deep-nested.json in specs/002-hierarchy-semantics/profiles/"

# Then launch all three message-authoring tasks together:
Task: "Author messages/basic-hierarchy.hl7 in specs/002-hierarchy-semantics/messages/"
Task: "Author messages/complex-hierarchy.hl7 in specs/002-hierarchy-semantics/messages/"
Task: "Author messages/unrecognized-segment.hl7 in specs/002-hierarchy-semantics/messages/"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1 (T004-T006)
4. **STOP and VALIDATE**: Run quickstart.md Scenario 1
5. At this point spec `008` already has an accurate, source-verified semantics
   document to build against, even before any conformance vector exists

### Incremental Delivery

1. Setup + Foundational → schema locked
2. User Story 1 → semantics document finalized, SC-001 satisfied
3. User Story 3 and User Story 4 (can run in parallel with each other and with US2) →
   both open decisions (FR-004, FR-005) confirmed definite, SC-002 satisfied
4. User Story 2 → vectors authored and verified, SC-003/SC-004 satisfied (unlocks
   spec `003`)
5. Polish → final bookkeeping and end-to-end acceptance pass

---

## Notes

- [P] tasks touch different files with no unmet dependencies
- This is a documentation/data feature (plan.md Technical Context) — "implementation"
  means authoring Markdown/JSON/HL7 text files and running manual verification passes
  against a local Scala checkout, not writing Rust/Python/Java code
- The B.2-multi-level-navigation vector (T013) is deliberately never run through
  SC-004's "verified against real Scala" gate (T017 explicitly excludes it) — it
  documents intended future behavior for spec `008`, not current behavior, and must
  stay labeled as such rather than silently mixed into the verified set
- Any FR-011 escalation opened during T017 pauses that specific vector's path to
  `Verified` but does not block other vectors from proceeding
- Validation-module-specific findings from the profile-schema review (cardinality
  string duplication, the `usage` predicate mini-language, unused `conformance`/
  `xtraValidation` fields) are explicitly out of scope for this task list, per
  direction to revisit them when the Validation module (Roadmap 2000-2999) is
  implemented
- Commit after each task or logical group, per repository convention
