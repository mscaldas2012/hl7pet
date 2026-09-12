# Feature Specification: HL7-PET Playground Web App

**Feature Branch**: `9000-playground-webapp`

**Created**: 2026-09-11

**Status**: Draft

**Input**: User description: "Build a minimalistic web application that lets a user interactively play with the hl7pet library. The user can: (1) optionally upload/choose a hierarchy profile file (JSON, matching the segmentDefinition format used elsewhere in this repo) — optional, only required if the PATH being tested uses a hierarchy (child-path \"->\") expression; (2) paste a full raw HL7 v2 message into a text area; (3) enter a PATH expression in a text box. When a PATH is entered/submitted, the app runs it against the pasted message (using the profile if one was provided and the PATH needs it) and displays all matching results, each annotated with its 1-based source line number (reusing the located-extraction API's line-number metadata, spec 1000). The UI should be minimal but functional and visually polished — this is a devtool/demo, not a production feature. This belongs to the Cross-cutting/Infra module (tooling), not any core migration phase."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run a PATH against a pasted message (Priority: P1)

A developer or analyst evaluating HL7-PET pastes a raw HL7 v2 message into the app,
types a PATH expression (e.g. `PID-5-1`, `OBX[2]-5`), and immediately sees every value
in the message that the PATH matches, each labeled with the 1-based line number of the
segment it came from.

**Why this priority**: This is the entire reason the playground exists — a fast,
no-install way to check what a PATH actually returns against a real message. Every
other capability (profiles, error states) exists to support this core loop.

**Independent Test**: Paste any well-formed HL7 v2 message, enter a non-hierarchy PATH
that matches one or more fields, and confirm the results panel lists each matched value
with the correct source line number. Delivers standalone value with no profile involved.

**Acceptance Scenarios**:

1. **Given** a pasted, well-formed HL7 message and a PATH matching a single field,
   **When** the user submits the PATH, **Then** the app shows exactly one result: the
   matched value and its 1-based source line number.
2. **Given** a pasted message with a repeating segment (e.g. multiple `OBX`) and a PATH
   that matches more than one occurrence, **When** the user submits the PATH, **Then**
   the app lists every matching value, each with its own line number, in message order.
3. **Given** a pasted message and a PATH with no match in that message, **When** the
   user submits the PATH, **Then** the app clearly indicates "no results" rather than
   showing an empty or ambiguous panel.
4. **Given** the user changes the PATH or the pasted message and resubmits, **When**
   the new PATH is run, **Then** the results panel fully replaces the previous results
   (no stale entries mixed in).

---

### User Story 2 - Run a hierarchy PATH with a profile (Priority: P2)

A user testing a child-path expression (`OBR -> OBX-5`) uploads or selects a hierarchy
profile file (the same `segmentDefinition`-style JSON used elsewhere in this project),
pastes a message, enters the hierarchy PATH, and sees each matched child value,
correctly scoped to its matched parent occurrence. Line numbers are not available for
hierarchy results in this version (see FR-005a) — the app says so explicitly rather
than omitting or guessing them.

**Why this priority**: Hierarchy navigation is a distinct, more advanced capability of
the library that depends on a profile; it's the second most common thing someone
playing with HL7-PET will want to try, but the app is fully useful without it (P1).

**Independent Test**: Load a valid hierarchy profile, paste a message that matches that
profile's segment structure, submit a `->` PATH, and confirm results are scoped per
parent occurrence — independent of whether a profile was ever loaded before.

**Acceptance Scenarios**:

1. **Given** a valid hierarchy profile is loaded and a message with matching parent/child
   segments, **When** the user submits a `->` PATH, **Then** results show only child
   values reachable from a matched parent, with a visible note that line numbers are
   not available for hierarchy results in this version.
2. **Given** no profile is loaded, **When** the user submits a `->` PATH, **Then** the
   app explains that this PATH requires a profile and does not silently return empty or
   incorrect results.
3. **Given** a profile is loaded that does not define the segment types referenced by
   the PATH, **When** the user submits the PATH, **Then** the app reports that the
   PATH's hierarchy could not be resolved against the loaded profile.

---

### User Story 3 - Understand and recover from bad input (Priority: P3)

A user pastes malformed HL7 text, types an invalid PATH, or loads a malformed profile
file, and the app tells them clearly what's wrong instead of crashing or showing a
blank/confusing state.

**Why this priority**: Important for a good devtool experience and for the tool to be
trustworthy as a way to validate PATH behavior, but the two happy-path stories above
deliver the primary value on their own.

**Independent Test**: Individually submit an unparseable PATH, a message with a missing
or truncated MSH segment, and an invalid profile file; confirm each produces a distinct,
readable error message and the app remains usable afterward.

**Acceptance Scenarios**:

1. **Given** a syntactically invalid PATH, **When** the user submits it, **Then** the
   app shows a specific parse error instead of a generic failure or a crash.
2. **Given** a pasted message missing or malformed at the MSH segment, **When** the
   user submits any PATH, **Then** the app reports that the message could not be scanned
   rather than showing misleading or partial results.
3. **Given** an uploaded profile file that is not valid JSON or does not match the
   expected profile schema, **When** the user selects it, **Then** the app rejects it
   with a clear message and the user can still run non-hierarchy PATHs normally.

### Edge Cases

- Message pasted with no trailing newline, or with trailing blank lines — line numbers
  must still correspond exactly to the segment's position among the message's segments.
- PATH submitted before any message is pasted, or message pasted before any PATH is
  entered — the app should not attempt an extraction until both are present.
- Extremely long messages or very large profiles pasted/uploaded — the app should stay
  responsive and, if a practical size limit is reached, say so explicitly rather than
  hanging.
- A message using non-standard field/component/encoding delimiters (declared in its own
  MSH-1/MSH-2) — PATH evaluation and line numbers must still be correct.
- A profile was previously loaded, then the user removes/clears it and submits a
  hierarchy PATH again — the app must treat this the same as "no profile loaded" (Story
  2, Scenario 2), not silently keep using the stale profile.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Users MUST be able to paste a raw HL7 v2 message into a text input area.
- **FR-002**: Users MUST be able to enter a PATH expression into a dedicated input.
- **FR-003**: Users MUST be able to optionally provide a hierarchy profile file; the
  app MUST function fully for non-hierarchy PATHs with no profile provided.
- **FR-004**: When a PATH is submitted with a pasted message present, the system MUST
  evaluate that PATH against that message and display every matching value.
- **FR-005**: Each displayed matching value for a non-hierarchy PATH MUST be annotated
  with the 1-based source line number of the segment occurrence it came from.
- **FR-005a**: Results for a hierarchy (`->`) PATH MUST display matched values without
  line numbers, and the system MUST show an explicit, visible note that line-numbered
  results are not yet available for hierarchy PATHs (a documented limitation, not a
  silent omission — see Constitution Principle V). A future feature may close this gap.
- **FR-006**: Results MUST be shown in message order and MUST fully replace prior
  results on every new submission (no accumulation of stale results).
- **FR-007**: When a PATH matches nothing in the message, the system MUST display an
  explicit "no results" state distinguishable from an error state.
- **FR-008**: When a submitted PATH uses a hierarchy (`->`) expression and no profile is
  loaded, the system MUST explain that a profile is required rather than returning
  empty or misleading results.
- **FR-009**: When a submitted PATH is syntactically invalid, the system MUST display a
  specific, human-readable parse error and MUST NOT crash or leave the UI in an
  inconsistent state.
- **FR-010**: When the pasted message cannot be scanned (e.g. missing/malformed MSH),
  the system MUST report that condition distinctly from "no results" or a PATH error.
- **FR-011**: When an uploaded profile file is not valid JSON or does not match the
  expected profile schema, the system MUST reject it with a clear message and MUST NOT
  block non-hierarchy PATH evaluation.
- **FR-012**: The system MUST correctly evaluate PATHs and report line numbers for
  messages that declare non-standard field/encoding delimiters in their own MSH
  segment, not just the standard `|`/`^~\&` delimiters.
- **FR-013**: The system MUST NOT persist a user's pasted message, profile, or PATH
  beyond their current session — nothing entered is retained or shared across users or
  visits.

### Key Entities

- **HL7 Message (input)**: The raw HL7 v2 text a user pastes; the subject of every PATH
  evaluation. Not stored beyond the current session.
- **Hierarchy Profile (input, optional)**: A `segmentDefinition`-style JSON document
  describing legal parent→child segment structure, supplied only when a hierarchy PATH
  is being tested.
- **PATH Expression (input)**: The query string a user enters, addressing a segment,
  field, component, subcomponent, filter, or hierarchy child-path within the message.
- **Result Set (output)**: The ordered list of matched values produced by evaluating a
  PATH against a message (and, when relevant, a profile). For a non-hierarchy PATH each
  entry pairs a value with its 1-based source line number; for a hierarchy PATH entries
  are values only, with no line number (FR-005a).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user with no prior exposure to the tool can paste a sample message,
  enter a valid PATH, and see correctly line-numbered results within their first
  attempt, with no external documentation.
- **SC-002**: For every matched value the app displays for a non-hierarchy PATH, the
  reported line number exactly matches the segment occurrence the value was extracted
  from. Hierarchy PATH results are excluded from this criterion per FR-005a.
- **SC-003**: A user who submits a hierarchy PATH without a profile understands, from
  the app's response alone, that a profile is needed — without consulting outside
  documentation.
- **SC-004**: Submitting a malformed PATH, malformed message, or malformed profile
  never leaves the app in a blank, frozen, or crashed state; the user can always see
  what went wrong and try again immediately.
- **SC-005**: Results for a typical single-message, single-PATH evaluation appear
  within a couple of seconds of submission.

## Assumptions

- This is an internal devtool/demo for people already familiar with HL7-PET and PATH
  syntax, not an end-user product; usability defaults favor clarity and speed over
  guided onboarding (no tutorial/help wizard required).
- One message and one profile are evaluated at a time; batch/multi-message evaluation
  is out of scope for this feature.
- One PATH is evaluated per submission; running multiple PATHs at once against the same
  message is out of scope for this feature (a user can resubmit with a new PATH).
- "All results" means every matched occurrence for a given PATH (e.g. every repetition
  of a repeating field/segment the PATH resolves to), consistent with the located
  extraction API's (spec `1000`) existing behavior — not merely the first match.
- The profile file format is the existing `segmentDefinition` JSON schema already used
  by this project's fixtures and hierarchy engine (specs `002`/`008`); no new profile
  format is introduced.
- No authentication, multi-user accounts, or saved history are required — this is a
  single-session, stateless tool per the no-persistence requirement (FR-013).
- Since HL7 messages (even synthetic/test ones) can resemble PHI-shaped data, nothing
  entered into the app is logged, stored server-side, or retained past the browser
  session.
- Line numbers for hierarchy (`->`) PATH results are explicitly out of scope for this
  version (FR-005a): no located/line-numbered execution path exists yet anywhere in
  `hl7pet-core` or its bindings for hierarchy navigation (spec `1000`'s located API
  covers non-hierarchy PATHs only). Adding that capability is core-library work
  belonging to the Parsing & Extraction module, not this tooling spec; closing this gap
  is left to a future spec this one does not depend on.
