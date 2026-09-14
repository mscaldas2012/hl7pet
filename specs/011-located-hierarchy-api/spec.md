# Feature Specification: Located Hierarchy API

**Feature Branch**: `011-located-hierarchy-api`

**Created**: 2026-09-12

**Status**: Draft

**Input**: User description: "Add line-number location tracking to hierarchy-mode PATH extraction (the `->` operator) in hl7pet-core, additive alongside the existing `execute_hierarchy`/`get_value_hierarchy`, surfaced through to Python as a new `get_value_hierarchy_located`."

## User Scenarios & Testing *(mandatory)*

This is a Rust Core / Engine Migration deliverable (Roadmap module 0-999,
spec `011`), with a secondary touch on Language Bindings (module 6000-6999,
next free `6002`) for the Python-facing surface — noted per `ROADMAP.md`'s
cross-module convention, primary module owns the spec. Its "users" are
callers of `hl7pet-core`'s hierarchy query API — today's dev CLI, the Python
binding (module 6000-6999), and the playground webapp (spec `9000`) — who
need to know not just *what* value a hierarchy-navigated (`->`) PATH resolved
to, but *where in the message* it came from, exactly as spec `1000` already
provides for non-hierarchy PATHs.

Non-hierarchy extraction has carried line numbers since spec `1000`
(`execute_located`/`LocatedValue`). Hierarchy-mode extraction
(`execute_hierarchy`, spec `008`) never got the same treatment — it computes
each matched child segment's position while walking the profile tree, then
discards it before returning. Spec `9000` (playground webapp) hit this gap
directly and recorded it as an explicit, deferred scope decision (its own
FR-005a) rather than solving it inline. This spec closes that gap.

### User Story 1 - Caller extracts a hierarchy-matched value and its source line together (Priority: P1)

A caller who already has a `ScanResult` (spec `005`), a `CompiledPath` with a
`->` hop (spec `006`), and a loaded `HierarchyProfile` (spec `008`) needs the
same value(s) `execute_hierarchy`/`get_value_hierarchy` already return, but
each one paired with the 1-based line number of the child segment occurrence
it was read from — e.g. `OBR[1] -> OBX-3` against a message with several
`OBX` children should report which line each matched `OBX` occurrence sits
on, exactly as `execute_located` already does for a plain (non-hierarchy)
`OBX-3`.

**Why this priority**: This is the feature's entire reason to exist. Every
other user story is a variation on retrieving this same value+line pairing
for hierarchy navigation; without it, the module delivers nothing new.

**Independent Test**: Given any vector in
`fixtures/vectors/hierarchy/complex.json` or `basic.json` that already
carries `expected_lines` metadata (e.g. `hier-005`, `hier-006`, `hier-008`),
call the new located hierarchy extraction entry point with that vector's
`path`/`profile_ref`/`message_ref` and confirm the returned line numbers
match `expected_lines` exactly, and the returned values match `expected`
exactly.

**Acceptance Scenarios**:

1. **Given** a scanned message, a loaded hierarchy profile, and a `->` PATH
   that matches exactly one child segment occurrence, **When** the located
   hierarchy extraction entry point is called, **Then** it returns that
   occurrence's value(s) together with the 1-based line number of the child
   segment they came from.
2. **Given** a `->` PATH that addresses a field, component, or subcomponent
   within a matched child segment, **When** the located hierarchy extraction
   entry point is called, **Then** every value returned for that child
   occurrence carries the same line number — the segment's own line, not a
   sub-line position.
3. **Given** the same `ScanResult`/`CompiledPath`/`HierarchyProfile` inputs,
   **When** the located hierarchy extraction entry point is called twice,
   **Then** both calls return identical values and identical line numbers.

---

### User Story 2 - Caller extracts values from multiple matched children across multiple parents, each with its own line (Priority: P2)

A caller whose `->` PATH matches children under several parent occurrences
(e.g. `OBR -> OBX-3` against a message with more than one `OBR`, each with
its own `OBX` children) needs each matched child's value paired with *that
child's own* line number, grouped exactly the way
`execute_hierarchy`/`get_value_hierarchy` already group results today.

**Why this priority**: Repeating parents with repeating children are the
common case for hierarchy-mode PATHs (multiple `OBR` under a message, each
with its own `OBX`/`NTE` children). A location API that only worked for a
single parent occurrence would not cover the majority of real hierarchy
usage.

**Independent Test**: Given `fixtures/vectors/hierarchy/complex.json`'s
`hier-006` vector (`OBR -> OBX-3`, combining children across multiple
matching parents), call the located hierarchy extraction entry point and
confirm the returned line numbers are grouped identically to `expected_lines`
(`[[7], [8]]`) — one line per matched child occurrence, in the same grouping
`execute_hierarchy` already produces for values.

**Acceptance Scenarios**:

1. **Given** a message with more than one occurrence of the parent segment
   type, each with its own matching children, **When** the located hierarchy
   extraction entry point is called, **Then** the result contains one value
   group per matched child, each tagged with that child's own line, in the
   same order and grouping as the existing (non-located) hierarchy result.
2. **Given** a `->` PATH with a child-side numeric index (e.g. `OBX[1]-3`),
   **When** the located hierarchy extraction entry point is called, **Then**
   only the indexed child occurrence(s) appear in the result, each with its
   own correct line number, consistent with spec `008`'s existing
   type-filtered, re-based, 1-based child indexing.

---

### User Story 3 - Caller distinguishes "no match" from a located result, for every existing hierarchy edge case (Priority: P3)

A caller relying on hierarchy navigation's existing documented edge cases —
zero children (`A.2-zero-children`), ambiguous ancestor resolution
(`A.7-ambiguous-parent-resolution`, spec `010`), or an unrecognized parent
type — needs the located entry point to report exactly the same "no match"
outcome as `execute_hierarchy` for those same inputs, never fabricating a
line number for a value that was never extracted.

**Why this priority**: Correctness on the edge cases matters as much as the
happy path for a location-tracking feature, but it is additive verification
of behavior User Stories 1-2 already establish, rather than new surface
area — hence lower priority.

**Independent Test**: Given `fixtures/vectors/hierarchy/complex.json`'s
`hier-007` vector (`OBR[2] -> OBX-3`, zero children — `expected: []`), call
the located hierarchy extraction entry point and confirm it returns an empty
result with no line numbers.

**Acceptance Scenarios**:

1. **Given** a `->` PATH whose parent occurrence has no matching children,
   **When** the located hierarchy extraction entry point is called, **Then**
   it returns an empty result, exactly mirroring `execute_hierarchy`'s
   existing empty-result shape for the same input.
2. **Given** a `->` PATH whose parent-side type is ambiguous in the loaded
   profile (spec `010`), **When** the located hierarchy extraction entry
   point is called, **Then** it resolves the parent and reports line numbers
   exactly as it would for an unambiguous parent — no ambiguity-related
   difference in location behavior.

---

### Edge Cases

- What happens for a multi-hop `->` PATH (chained hierarchy, e.g.
  `OBR -> OBX -> NTE`)? Already rejected by the parser (spec `006`,
  `MultipleHierarchyHops`) or documented as silently empty for the two
  pre-existing corpus vectors that predate that rejection
  (`A.6-chained-arrow-silently-empty`) — this feature does not change that
  behavior; the located entry point inherits whatever `execute_hierarchy`
  already does for such inputs.
- What happens when the hierarchy profile itself fails to load or is
  malformed (spec `008`, `ProfileError`)? Located hierarchy extraction never
  runs — the caller already receives the profile error before any
  extraction, located or not, is attempted, exactly as today.
- What happens when a matched child segment's requested field repetition
  doesn't exist? That occurrence contributes no value and no line number,
  mirroring the existing collapse-to-no-match behavior non-hierarchy
  `execute_located` already has for the same case (spec `1000`).
- What happens for a plain (non-hierarchy) PATH passed to the new entry
  point? Out of scope — the located hierarchy entry point exists specifically
  for `->` PATHs; plain PATHs already have their own located entry point
  (spec `1000`).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a new hierarchy extraction entry point in
  `hl7pet-core`, additive alongside the existing `execute_hierarchy`, that
  returns every matched value together with the 1-based line number of the
  child segment occurrence it was extracted from.
- **FR-002**: The value content returned by the located hierarchy entry point
  MUST be identical, value-for-value and group-for-group, to what
  `execute_hierarchy` already returns for the same PATH, message, and
  profile — this feature adds location data, it never changes what is
  returned as the value itself or how results are grouped by parent
  occurrence.
- **FR-003**: The line number reported for a hierarchy-matched value MUST
  identify the 1-based position, among all segments in the source message,
  of the child segment occurrence that value was read from — the same
  convention spec `1000`'s `LocatedValue`/`execute_located` already use for
  non-hierarchy PATHs.
- **FR-004**: When a `->` PATH addresses a field, component, or subcomponent
  within a matched child segment, every value produced from that same child
  occurrence MUST carry that occurrence's line number — location is tracked
  per child segment occurrence, not per sub-segment position.
- **FR-005**: When a `->` PATH matches children under multiple parent
  occurrences, the result MUST preserve the existing per-matched-child
  grouping `execute_hierarchy` already produces, with each value group
  carrying its own line number rather than collapsing to one line number for
  the whole result.
- **FR-006**: When a `->` PATH matches nothing (no children, unrecognized
  parent, or any other existing "no match" case), the located hierarchy entry
  point MUST report an empty result, exactly mirroring `execute_hierarchy`'s
  existing empty-result shape — with no fabricated line number.
- **FR-007**: This feature MUST NOT change the behavior, signature, or output
  of the existing `execute_hierarchy` (`hl7pet-core`) or `get_value_hierarchy`
  (Python binding) in any way; both remain callable exactly as today, per the
  Backward-Compatible-Additions convention (`ROADMAP.md`).
- **FR-008**: System MUST surface the new capability through the Python
  binding as a new function, additive alongside the existing
  `get_value_hierarchy`, returning each value wrapped in the same
  `LocatedValue` type already used by `get_value_located`/
  `get_first_value_located` (spec `6000`).
- **FR-009**: The Python binding's new located hierarchy function MUST be
  reflected in the package's type stub and public exports, matching the
  existing pattern used for `get_value_hierarchy` and the non-hierarchy
  located functions.
- **FR-010**: Conformance vectors for this feature MUST be verified against
  the `expected_lines` metadata already recorded in
  `fixtures/vectors/hierarchy/` (e.g. `hier-005`, `hier-006`, `hier-008`),
  reusing that existing data rather than deriving new expected line numbers
  or inventing a new fixture family.

### Key Entities

- **Located Hierarchy Value**: A value already producible by the existing
  hierarchy extraction API, paired with the 1-based line number of the child
  segment occurrence it came from. Carries no other metadata — no
  column/byte offset, no field/component identifier, no parent-occurrence
  identifier — beyond the value and its line, matching spec `1000`'s Located
  Value shape exactly.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every conformance vector in
  `fixtures/vectors/hierarchy/` that carries `expected_lines` metadata, the
  located hierarchy extraction API's returned line numbers match that
  metadata exactly, with zero discrepancies.
- **SC-002**: For every one of those same vectors, the located hierarchy
  extraction API's returned values are identical to the existing
  (non-located) hierarchy extraction API's output for the same vector —
  confirming this feature adds information without altering existing
  results.
- **SC-003**: Existing callers of `execute_hierarchy`/`get_value_hierarchy`
  observe no change whatsoever in behavior, output, or performance after this
  feature ships — verified by the full pre-existing regression suite
  continuing to pass unmodified.
- **SC-004**: Determining a hierarchy-matched value's source line adds no
  repeated pass over the message text — line information is derived from the
  child segment's position, already known during the existing bounded
  per-parent-occurrence scan, so cost does not grow with message size beyond
  what plain hierarchy extraction already costs.
- **SC-005**: A Python caller can obtain hierarchy-navigated values with
  source line numbers using one new function call, without needing to
  separately re-derive location by any other means.

## Assumptions

- "Line number" means the 1-based position of a segment's own occurrence
  among all segments in the source message — the identical convention spec
  `1000`'s `LocatedValue` and the shared fixtures' `expected_lines` metadata
  already use, not a byte offset or a position within a segment's own text.
- The new entry point is named and shaped as a close hierarchy counterpart to
  spec `1000`'s `execute_located` (Rust) and `get_value_located` (Python),
  per the Backward-Compatible-Additions convention, rather than a single
  combined API replacing `execute_hierarchy`/`get_value_hierarchy`.
- Conformance vectors reuse the `expected_lines` metadata the shared
  `fixtures/vectors/hierarchy/` corpus already carries (present since spec
  `008`/`010`); no new fixture family or source-of-truth for hierarchy line
  numbers needs to be established by this spec.
- This spec's primary module is Rust Core (`crates/core/src/hierarchy.rs`);
  the Python binding work (`crates/python/src/lib.rs` and the package's
  `__init__.py`/`.pyi`) is secondary, per `ROADMAP.md`'s cross-module
  convention, and falls within the Language Bindings module's number range
  (next free `6002`) for bookkeeping purposes even though it is delivered as
  part of this spec rather than a separate one.
- A first-match counterpart (a located analogue to a hypothetical
  "get first hierarchy value") is not required — no such non-located method
  exists today for hierarchy mode, so none is added here either.
