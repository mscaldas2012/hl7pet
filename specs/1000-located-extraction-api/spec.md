# Feature Specification: Located Extraction API

**Feature Branch**: `1000-located-extraction-api`

**Created**: 2026-09-04

**Status**: Draft

**Input**: User description: "Location-aware extraction API: a new getValueLocated(path) method returning each extracted value paired with its 1-based source line number, additive alongside the existing getValue/getFirstValue methods, built on offset data already tracked by the message scanner and query executor."

## User Scenarios & Testing *(mandatory)*

This is a Parsing & Extraction deliverable (Roadmap module 1000-1999, spec
`1000`) — the first spec in this module, and the first Rust Core capability
with no equivalent at all in the current Scala engine (`HL7ParseUtils`/
`HL7StaticParser` only ever return raw values, never their source location).
Its "users" are callers of `hl7pet-core`'s query API — today's dev CLI, and
any future language binding (Python/Java, module 6000-6999) — who need to
know not just *what* value a PATH resolved to, but *where in the message* it
came from, without giving up any of the existing extraction behavior.

### User Story 1 - Caller extracts a value and its source line together (Priority: P1)

A caller who already has a `ScanResult` (spec `005`) and a `CompiledPath`
(spec `006`) for a non-hierarchy PATH needs the same value(s) `getValue`
already returns, but each one paired with the 1-based line number of the
segment it was read from — e.g. `OBX[2]-5` against a message with several
`OBX` segments should report which line that second `OBX` occurrence
actually sits on.

**Why this priority**: This is the feature's entire reason to exist. Every
other user story is a variation on retrieving this same value+line pairing;
without it, the module has no output at all.

**Independent Test**: Given any vector in `fixtures/vectors/path/valid.json`
that already carries `expected_lines` metadata (spec `001` FR-008), call the
new located extraction entry point with that vector's `path`/`message_ref`
and confirm the returned line numbers match `expected_lines` exactly, and the
returned values match `expected` exactly.

**Acceptance Scenarios**:

1. **Given** a scanned message and a PATH that matches exactly one segment
   occurrence, **When** the located extraction entry point is called,
   **Then** it returns that occurrence's value(s) together with the 1-based
   line number of the segment they came from.
2. **Given** a PATH that addresses a field, component, or subcomponent within
   a segment, **When** the located extraction entry point is called,
   **Then** every value returned for that segment occurrence carries the same
   line number — the segment's own line, not a sub-line position.
3. **Given** the same `ScanResult`/`CompiledPath` pair, **When** the located
   extraction entry point is called twice, **Then** both calls return
   identical values and identical line numbers.

---

### User Story 2 - Caller extracts values from multiple segment occurrences, each with its own line (Priority: P2)

A caller whose PATH matches several segment occurrences (e.g. an unindexed
`OBX-5` against a message repeating `OBX` five times) needs each occurrence's
value paired with *that occurrence's own* line number, not a single line
number for the whole result.

**Why this priority**: Repeating segments are the common case in real HL7
messages (multiple `OBX`, `NTE`, etc. under one message or one parent). A
location API that only worked for single-match PATHs would not cover the
majority of real usage.

**Independent Test**: Given `fixtures/vectors/path/valid.json`'s
`path-obx5-occurrences` vector (or an equivalent multi-occurrence vector),
call the located extraction entry point and confirm the returned line numbers
are `[[4], [5], [6]]`-shaped (one line per matched occurrence, mirroring the
existing outer/inner value shape) rather than a single flattened line number.

**Acceptance Scenarios**:

1. **Given** a message with three occurrences of the same segment type all
   matching a PATH, **When** the located extraction entry point is called,
   **Then** the result contains three value groups, each tagged with the
   line of its own occurrence, in document order.
2. **Given** a filter clause that excludes some occurrences of a repeating
   segment, **When** the located extraction entry point is called, **Then**
   only the occurrences that pass the filter appear in the result, each with
   its own correct line number — excluded occurrences contribute nothing.

---

### User Story 3 - Caller extracts just the first matching value and its line (Priority: P3)

A caller who only needs the first matching value — the location-aware
counterpart to today's `getFirstValue` — needs a single value plus a single
line number, without handling the nested per-occurrence shape User Story 2
introduces.

**Why this priority**: `getFirstValue` exists today as a convenience over
`getValue` for the common "I only need one answer" case. Its location-aware
counterpart is a small, self-contained addition once User Story 1 exists, so
it is lower priority than the core capability but still valuable to ship in
the same spec rather than as a follow-up.

**Independent Test**: Given any vector already exercising `getFirstValue`
with recorded `expected_lines`, call the location-aware first-value entry
point and confirm it returns the single expected value and the single
expected line number.

**Acceptance Scenarios**:

1. **Given** a PATH matching several segment occurrences, **When** the
   location-aware first-value entry point is called, **Then** it returns
   only the first matching value together with that occurrence's line
   number — never the full set.
2. **Given** a PATH that matches nothing, **When** either located extraction
   entry point is called, **Then** it reports no value and no line number,
   exactly mirroring the existing "no match" outcome of `getValue`/
   `getFirstValue`.

---

### Edge Cases

- What happens when a PATH's segment or field index is out of range (already
  a documented "no match" case for `getValue`/`getFirstValue`)? The
  location-aware equivalent MUST also report no match — it must never
  fabricate a line number for a value that was never extracted.
- What happens when a matched segment occurrence's requested field
  repetition doesn't exist? That occurrence contributes no value and no line
  number, mirroring `getValue`'s existing collapse-to-no-match behavior for
  the same case (spec `007`).
- What happens for a hierarchy-mode PATH (`->`, spec `008`)? Out of scope for
  this feature (see Assumptions) — behavior for hierarchy PATHs is
  unspecified by this spec and deferred to a future one.
- What happens when the message itself is malformed (fails to scan at all,
  spec `005`)? Located extraction never runs — the caller already receives
  the scanner's own error before any extraction, located or not, is
  attempted.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a new extraction entry point, additive
  alongside the existing value-extraction method, that returns every matched
  value together with the 1-based line number of the segment occurrence it
  was extracted from.
- **FR-002**: System MUST provide a location-aware counterpart to the
  existing first-match extraction method, returning a single value and a
  single line number for the first matching occurrence only.
- **FR-003**: The value content returned by either location-aware method
  MUST be identical, value-for-value, to what the corresponding existing
  method already returns for the same PATH and message — this feature adds
  location data, it never changes what is returned as the value itself.
- **FR-004**: The line number reported for a value MUST identify the 1-based
  position, among all segments in the source message, of the segment
  occurrence that value was read from.
- **FR-005**: When a PATH addresses a field, component, or subcomponent
  within a segment, every value produced from that same segment occurrence
  MUST carry that occurrence's line number — location is tracked per segment
  occurrence, not per sub-segment position.
- **FR-006**: When a PATH matches multiple segment occurrences, the result
  MUST preserve a per-occurrence pairing between each value group and its own
  line number, mirroring the existing outer/inner value grouping rather than
  collapsing to one line number for the whole result.
- **FR-007**: When a PATH matches nothing, both location-aware methods MUST
  report an empty result, exactly mirroring the existing "no match" shape of
  `getValue`/`getFirstValue` — with no fabricated line number.
- **FR-008**: This feature MUST NOT change the behavior, return shape, or
  output of the existing `getValue`/`getFirstValue` methods in any way; they
  remain callable exactly as today, per the Backward-Compatible-Additions
  convention (`ROADMAP.md`).
- **FR-009**: Conformance vectors for this feature MUST be verified against
  the `expected_lines` metadata already recorded in the shared fixtures
  corpus (spec `001` FR-008), reusing that data rather than deriving new
  expected line numbers independently.

### Key Entities

- **Located Value**: A value already producible by the existing extraction
  API, paired with the 1-based line number of the segment occurrence it came
  from. Carries no other metadata — no column/byte offset, no field/component
  identifier — beyond the value and its line.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every conformance vector in the shared fixtures corpus that
  carries `expected_lines` metadata, the located extraction API's returned
  line numbers match that metadata exactly, with zero discrepancies.
- **SC-002**: For every one of those same vectors, the located extraction
  API's returned values are identical to the existing (non-located)
  extraction API's output for the same vector — confirming this feature adds
  information without altering existing results.
- **SC-003**: Existing callers of `getValue`/`getFirstValue` observe no
  change whatsoever in behavior, output, or performance after this feature
  ships — verified by the full pre-existing regression suite continuing to
  pass unmodified.
- **SC-004**: Determining a value's source line adds no repeated pass over
  the message text — line information is derived from data the scanner
  already collected during the original scan, so cost does not grow with
  message size beyond what plain extraction already costs.

## Assumptions

- Hierarchy-mode PATHs (`->`, spec `008`) are out of scope for this feature.
  The ROADMAP describes this capability as depending on the message scanner
  (spec `005`) and query executor (spec `007`) only, not the hierarchy
  executor (spec `008`); a future spec may extend location-awareness to
  hierarchy navigation if needed.
- "Line number" means the 1-based position of a segment's own occurrence
  among all segments in the source message — the same convention already
  used by the `expected_lines` metadata collected in spec `001` FR-008 and
  the shared fixtures corpus, not a byte offset or a position within a
  segment's own text.
- The two new entry points are named and shaped as close counterparts to the
  existing `getValue`/`getFirstValue` methods (one-to-one, e.g.
  `getValueLocated`/`getFirstValueLocated`), per the Backward-Compatible-
  Additions convention, rather than a single combined API replacing both.
- Conformance vectors reuse the `expected_lines` metadata the shared fixtures
  corpus already carries; no new source-of-truth for line numbers needs to
  be established by this spec.
