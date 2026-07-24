# Feature Specification: Hierarchy-Mode Semantics Specification

**Feature Branch**: `002-hierarchy-semantics`

**Created**: 2026-07-24

**Status**: Draft

**Input**: User description: "Document hierarchy-mode semantics: segmentDefinition-driven parent-to-child navigation, cardinality, and the -> operator. Research whether to use profiles. Consider support for more than one level of hierarchy if it can perform well; performance is the top priority."

## User Scenarios & Testing *(mandatory)*

This is a documentation/specification deliverable (Migration Plan Phase 1,
Roadmap module 0-999 "Rust Core", spec `002`) rather than a runtime feature,
following the same pattern as spec `001` (path-grammar-spec). Its "users" are
the people and processes that will build against or verify against it —
principally the Rust core implementer for hierarchy navigation (Roadmap spec
`008`, `lazy-hierarchy-nav`).

### User Story 1 - Rust core implementer builds lazy hierarchy navigation from the spec alone (Priority: P1)

An engineer implementing the Rust core's contextual hierarchy navigation
(Roadmap spec `008`) needs an unambiguous definition of what `->` means at
evaluation time — how a child segment occurrence gets scoped to a specific
parent occurrence, how cardinality from `segmentDefinition` is enforced or
reported, and how multiple parent/child occurrences interact — so they can
build the feature without reverse-engineering the Scala source.

**Why this priority**: This is the entire point of the spec. Spec `001`
explicitly delegates hierarchy evaluation semantics here (its FR-002); without
this document, "parent-scoping" has no single source of truth and Constitution
Principle I (Path Contract Stability) becomes unenforceable for anything past
plain field addressing.

**Independent Test**: Give the semantics document (with no access to Scala
source) to someone unfamiliar with HL7-PET and ask them to predict the result
of `OBR[2] -> OBX-5` against a message with three `OBR` segments, each with a
different number of child `OBX` segments. Success = prediction matches the
documented/verified behavior with no guessing.

**Acceptance Scenarios**:

1. **Given** the semantics document, **When** a reader looks up how a child
   segment is matched to "its" parent occurrence, **Then** they find a
   precise rule (e.g. positional containment between one parent occurrence
   and the next) with no ambiguity about ties or gaps.
2. **Given** the semantics document, **When** a reader looks up what happens
   when a required child (per `segmentDefinition` cardinality) is absent from
   a specific parent occurrence, **Then** they find a documented answer
   (e.g. "zero results for that occurrence," not an exception) consistent
   with Constitution Principle III.

---

### User Story 2 - Regression-suite author gets canonical hierarchy conformance vectors (Priority: P1)

The author of Roadmap spec `003` (regression suite) needs canonical hierarchy
conformance vectors — profile, source message, PATH string using `->`,
expected result — so the same vectors validate the Scala baseline, the Rust
core, and later bindings for parity, exactly as spec `001` did for plain PATH
syntax.

**Why this priority**: Without executable vectors, hierarchy semantics are
just prose nobody can check mechanically in CI.

**Independent Test**: Feed each conformance vector's profile, message, and
PATH string into the current Scala library
([mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet)) in hierarchy
mode and confirm the actual output matches the vector's documented expected
result.

**Acceptance Scenarios**:

1. **Given** a conformance vector for `OBR[1] -> OBX-5` against a message with
   two `OBR` occurrences, **When** it is run against the Scala library,
   **Then** the actual output matches the vector's documented expected result
   exactly, including which `OBX` occurrences are scoped to which `OBR`.
2. **Given** the full vector set, **When** each vector's documented rule is
   tallied, **Then** every semantic rule in this document (parent-scoping,
   cardinality reporting, multi-occurrence parents) is exercised by at least
   one vector.

---

### User Story 3 - Project decides whether Rust hierarchy navigation requires a profile (Priority: P2)

Before Roadmap spec `008` is planned, the project needs a researched,
documented answer to whether the Rust core's `->` navigation requires an
explicit declarative profile (`segmentDefinition`, as the current Scala engine
mandates) or can operate structurally from the message's own segment nesting,
trading off Constitution Principle II (zero-copy/lazy, "contextual navigation
without building a full tree" per the Migration Plan's Phase 3 description)
against Principle V (declarative, profile-driven conformance).

**Why this priority**: This determines the shape of spec `008`'s API surface
(profile-required vs. profile-optional) and must be settled before that spec
is planned, but is explicitly **not** a decision to make by guessing now —
per the feature request, it is deferred to research conducted as part of this
spec's own planning phase.

**Independent Test**: The planning phase for this spec produces a written
recommendation (with rationale and rejected alternative) on the profile
question, and that recommendation is incorporated into this document before
spec `002` is marked complete.

**Acceptance Scenarios**:

1. **Given** this document after its planning phase completes, **When** a
   reader looks for whether hierarchy navigation requires a profile,
   **Then** they find a definite answer with rationale, not an open question.

---

### User Story 4 - Project decides whether multi-level (chained) navigation is worth adding (Priority: P3)

Before Roadmap spec `008` is planned, the project needs a researched,
documented answer to whether `->` should be extended (as a Backward-Compatible
Addition, per `ROADMAP.md`'s convention) to support chaining beyond one hop
(e.g. `ORC[1] -> OBR[1] -> OBX-5`), gated strictly on whether doing so can be
shown not to cost meaningfully more than single-hop navigation.

**Why this priority**: Lower priority than the P1/P2 items because the
current Scala engine and PATH grammar (spec `001`) only support one hop; this
is a candidate future capability, not a compatibility requirement. Per the
feature request, performance is the deciding factor, not desirability alone.

**Independent Test**: The planning phase produces a written recommendation:
either "include multi-level chaining, with a stated performance argument for
why it costs no more than single-hop navigation" or "single-hop only for now,
revisit once spec `008`'s implementation exists to benchmark" — never a
default inclusion without that argument.

**Acceptance Scenarios**:

1. **Given** this document after its planning phase completes, **When** a
   reader looks for how many hops `->` supports, **Then** they find a definite
   answer (one hop, or N hops) with the performance rationale behind it.
2. **Given** a recommendation to include multi-level chaining, **When** a
   reader checks its grammar impact, **Then** it is documented as an addition
   to `CHILD_PATH` (spec `001`) that existing single-hop paths remain valid
   under, not a breaking change.

---

### Edge Cases

- What happens when a PATH uses `->` but hierarchy mode was not enabled (no
  profile / static mode)? Already documented by `SPEC.md` §7 as silent
  fallback to flat extraction; this spec must state whether that is the
  behavior the Rust core should preserve for compatibility, or whether it
  should differ (e.g. explicit error), as it is a callable-behavior question
  hierarchy semantics owns.
- What happens when a parent segment occurrence has zero matching children
  (child cardinality allows zero, e.g. `[0..*]`)? Must resolve to zero
  results for that occurrence (Constitution Principle III), not an exception.
- What happens when a required child (cardinality `[1..1]` or `[1..*]`) is
  missing under a specific parent occurrence? This spec must state whether
  that is reported as a structural error (aligning with `StructureValidator`'s
  responsibility in the 2000-2999 Validation module) or silently yields zero
  results for that occurrence within `->` navigation itself — these are
  different concerns (extraction vs. validation) and the boundary must be
  explicit.
- What happens when the same child segment type appears at more than one
  nesting depth in `segmentDefinition` (e.g. `OBX` under both `OBR` and a
  hypothetical direct child of `MSH`)? The scoping rule must disambiguate by
  the specific parent occurrence used in the `->` expression, never globally.
- What happens when a profile's `segmentDefinition` itself is malformed
  (invalid cardinality string, cyclical children)? Per `SPEC.md` §6,
  `HL7ParseError` is the existing behavior for invalid cardinality strings;
  this spec inherits that as a structural-precondition violation
  (Constitution Principle III) rather than redefining it, and must state
  whether cyclical `segmentDefinition` children are rejected the same way.
- What happens to a chained multi-level example if the planning-phase
  decision (User Story 4) is "single-hop only"? All conformance vectors and
  document prose in that case describe exactly one level, and the
  possibility of more levels is recorded only as a documented, explicitly
  rejected-for-now option with its rationale (not silently omitted).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The semantics document MUST formally define what a segment
  occurrence's "parent" is under `segmentDefinition`-driven hierarchy: how
  child segment lines are bounded between one parent occurrence and the
  start of the next occurrence of the same parent segment type (or end of
  message), matching the current Scala engine's hierarchy-mode behavior as
  the compatibility floor (Constitution Principle I).
- **FR-002**: The semantics document MUST define how `segmentDefinition`
  cardinality (`[m..n]` strings, per `SPEC.md` §3.2) applies during `->`
  navigation itself, distinct from `StructureValidator`'s separate,
  Validation-module (2000-2999) responsibility for reporting cardinality
  *violations* as validation errors. This document owns "how navigation
  behaves given a cardinality-conformant or -non-conformant message,"
  not "how conformance is reported to a caller running full validation."
- **FR-003**: The semantics document MUST define the current, single-hop
  `->` operator's full evaluation semantics: `SEGMENT_EXPR -> CHILD_PATH`
  scopes the `CHILD_PATH` segment lookup to only the lines belonging to the
  selected parent occurrence(s) (per FR-001), before applying the child
  path's own field/component/index/filter logic (spec `001`'s `FIELD_EXPR`,
  `SEG_IDX`, `FILTER` productions apply unchanged within the scoped lines).
- **FR-004**: The semantics document MUST explicitly state, with rationale,
  whether Rust core hierarchy navigation requires an explicit declarative
  profile (as the current Scala engine mandates) or can operate without one.
  This decision MUST be produced during this spec's planning-phase research,
  weighing Constitution Principle II (zero-copy/lazy, "contextual navigation
  without building a full tree") against Principle V (declarative,
  versionable profiles) — it MUST NOT be guessed at spec-writing time.
- **FR-005**: The semantics document MUST explicitly state, with rationale,
  whether `->` should be extended to support more than one level of chained
  parent→child navigation (e.g. grandparent→parent→child in a single PATH).
  This decision MUST be produced during this spec's planning-phase research
  and MUST be gated on a stated performance argument — inclusion is not the
  default; "single-hop only for now" is an acceptable and expected outcome
  if a performance cost cannot be ruled out before spec `008`'s
  implementation exists to benchmark against. If included, it MUST be
  documented as a Backward-Compatible Addition to spec `001`'s `CHILD_PATH`
  production (existing single-hop paths remain valid), per
  `ROADMAP.md`'s Backward-Compatible Additions convention.
- **FR-006**: A conformance vector suite MUST be produced covering: basic
  single-parent/single-child scoping, multiple parent occurrences with
  different child counts (including zero), a required child missing from one
  parent occurrence, and — if FR-005's decision is to include multi-level
  chaining — at least one multi-hop vector; if the decision is single-hop
  only, the suite documents that scope boundary instead.
- **FR-007**: Each conformance vector MUST specify, in a structured
  (machine-readable) format compatible with spec `001`'s conformance-vector
  schema (FR-011 of that spec): the profile used, the PATH string (using
  `->`), a source HL7 message (inline or by reference), and the exact
  expected result, following the same two-dimensional (occurrences ×
  repetitions) result shape spec `001` defined, extended per-parent-occurrence
  for hierarchy vectors.
- **FR-008**: The semantics document MUST state whether a `->` expression
  evaluated without hierarchy mode enabled (no profile / static mode)
  preserves the current Scala behavior (silent fallback to flat extraction,
  per `SPEC.md` §7) for the Rust core, or documents an intentional deviation;
  a deviation would be a Documented Breaking Change per `ROADMAP.md`'s
  convention and requires the same MAJOR version bump and migration guide
  Constitution Principle I mandates.
- **FR-009**: The semantics document MUST reuse spec `001`'s terminology and
  grammar productions (`SEGMENT_EXPR`, `CHILD_PATH`, `SEG_IDX`, `FIELD_EXPR`)
  without redefining them, and MUST link to spec `001` at every point where a
  syntax-level (rather than semantic) question arises, mirroring how spec
  `001` links here for semantic questions.
- **FR-010**: Any corner of hierarchy behavior found to be ambiguous or
  underspecified in `SPEC.md` during extraction MUST be flagged explicitly in
  this document as a resolved decision with rationale, not silently guessed.
- **FR-011**: When verifying a conformance vector against the real Scala
  library reveals a discrepancy with what this document (or `SPEC.md`)
  states, neither source MUST be silently trusted — the discrepancy MUST be
  escalated as a `[NEEDS CLARIFICATION]` item for a case-by-case human
  decision, matching spec `001`'s FR-010 precedent.
- **FR-012**: Every source HL7 message and profile fixture used in a
  conformance vector MUST be synthetic/fabricated test data; real patient
  data, including de-identified real messages, MUST NOT be used, matching
  spec `001`'s FR-009.
- **FR-013**: Deliverables from this spec (semantics document, conformance
  vectors, profile fixtures) live under `specs/002-hierarchy-semantics/`.
  Promoting the conformance vectors into the shared `fixtures/` corpus
  described in `HL7-PET-Rust-Migration-Plan.md` is in scope for spec `003`
  (regression-suite), not this spec.

### Key Entities

- **Segment Hierarchy Tree**: The parent→child structure built from a
  profile's `segmentDefinition` at hierarchy-construction time (today) or
  navigated contextually (candidate future Rust behavior per FR-004).
- **Parent Occurrence**: One specific occurrence of a parent segment type
  (e.g. the 2nd `OBR` in a message) that a set of child segment lines is
  scoped to.
- **SegmentConfig / Cardinality**: The `[m..n]` occurrence-count contract
  attached to a segment within `segmentDefinition`, and the distinction
  between navigation-time behavior (this spec) and validation-time reporting
  (Validation module, 2000-2999).
- **Hierarchy Conformance Vector**: A tuple of (profile, source message,
  hierarchy PATH string, expected result) used to verify an implementation's
  `->` evaluation behavior, extending spec `001`'s conformance-vector concept.
- **Profile Requirement Decision**: The researched, documented answer (FR-004)
  to whether Rust hierarchy navigation mandates an explicit profile.
- **Multi-Level Navigation Decision**: The researched, documented answer
  (FR-005) to whether `->` supports more than one hop, and the performance
  rationale behind it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the hierarchy-mode behavior described in `SPEC.md`
  §3.3 and §4.1 (constructors, `->` operator, profile requirement) is
  captured in this document with no undefined terms or forward references
  outside spec `001`.
- **SC-002**: The profile-requirement question (FR-004) and the multi-level
  navigation question (FR-005) both have a stated decision with written
  rationale before this spec is marked complete — neither is left open.
- **SC-003**: At least one conformance vector exists per documented semantic
  rule (parent-scoping, zero-children case, missing-required-child case, and
  any multi-level case per FR-005's outcome) — minimum ~8 vectors given the
  current rule count.
- **SC-004**: 100% of conformance vectors, when manually run against the
  external Scala library, either match the documented expected result, or
  have an FR-011 discrepancy escalation opened and resolved before this spec
  is considered complete.
- **SC-005**: If the multi-level navigation decision (FR-005) is "include,"
  the documented rationale includes a specific, falsifiable performance
  claim (e.g. "no additional full-message pass beyond single-hop scoping")
  rather than an unquantified assertion of "should be fine."

## Assumptions

- No Rust, Python, or Java code exists yet for this spec — it is a
  documentation/data deliverable consumed by later specs (`003`, `008`),
  exactly as spec `001` was for PATH syntax.
- `SPEC.md` §3.2–§3.3 and §4.1 (this repo) is the primary source of truth for
  today's Scala hierarchy behavior; the external Scala repo
  ([mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet)) is
  consulted only to resolve ambiguities `SPEC.md` doesn't settle.
- The profile-requirement question (FR-004) and multi-level navigation
  question (FR-005) are deliberately left undecided in this initial draft per
  the feature request ("leave that decision after research" /
  "if it can perform well") — both are resolved during this spec's own
  planning phase (`/speckit-plan`) rather than guessed here, and the plan's
  research output is folded back into this document's final form.
- Performance is the overriding constraint for the multi-level navigation
  decision (FR-005): the feature request states it explicitly ("performance
  is always top priority"), so absent a concrete performance argument for
  supporting multiple levels, the default outcome is single-hop only.
- Cardinality *violation reporting* (as opposed to navigation-time behavior)
  remains owned by the Validation module (2000-2999, `StructureValidator`);
  this spec does not redefine or duplicate that module's responsibilities.
- Turning these vectors into an executable, CI-wired regression suite under
  `fixtures/` is out of scope here and owned by Roadmap spec `003`.
