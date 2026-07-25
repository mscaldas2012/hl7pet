# Feature Specification: Message Scanner

**Feature Branch**: `005-message-scanner`

**Created**: 2026-07-24

**Status**: Draft

**Input**: User description: "Single-pass segment/delimiter scanner, offsets only, no field/component allocations. MUST read the field separator from MSH-1 and the encoding characters (component/repetition/escape/subcomponent) from MSH-2 rather than hardcoding `|` and `^~\&` -- fixes the Scala engine's 'MSH-1/MSH-2 must be standard' limitation (SPEC.md §7). For messages using standard delimiters (the common case), output is unchanged; this only changes behavior for non-standard-delimiter messages, which previously mis-parsed or errored. Needs its own conformance vectors: at least one message with non-default delimiters, plus malformed-MSH error cases. (Roadmap module 0-999 Rust Core, Migration Plan Phase 2)"

## User Scenarios & Testing *(mandatory)*

This is the first spec in the migration to produce runtime engine code
(Migration Plan Phase 2, Roadmap module 0-999 "Rust Core", spec `005`) rather
than a design document, fixture corpus, or external benchmark harness. Its
"users" are the downstream Rust core components that consume scanner offsets
(the PATH parser and query executor, specs `006`-`007`) and, transitively,
every caller of the engine who currently hits the Scala library's fixed-MSH
limitation.

### User Story 1 - Downstream Rust components get segment/field offsets without allocating (Priority: P1)

The PATH parser and query executor (specs `006`-`007`) need, for any given
message, the byte offsets of every segment and every delimiter occurrence
within it, without the scanner allocating a `String`/`Vec` per field or
component. This is the foundational capability the rest of the Rust core
Phase 2-3 work is built on.

**Why this priority**: Constitution Principle II (Zero-Copy & Lazy
Evaluation) requires that extraction never materialize a full object model.
The scanner is the only component that touches every byte of the raw
message; if it allocates per-field here, no downstream component can undo
that cost later.

**Independent Test**: Run the scanner alone (no PATH parser, no query
executor) against a `fixtures/messages/` file and confirm it returns a
complete offset map for every segment and delimiter occurrence, with the
scanner's own peak allocation count independent of the number of fields,
components, or repetitions in the message (only proportional to segment
count).

**Acceptance Scenarios**:

1. **Given** a well-formed HL7 message with a standard MSH segment, **When**
   the scanner runs, **Then** it returns the start/end byte offset of every
   segment in the message, in order.
2. **Given** a scanned message, **When** a downstream caller asks for the
   offsets of all field-separator, component-separator, repetition-separator,
   escape-character, and subcomponent-separator occurrences within a given
   segment, **Then** the scanner returns them without allocating a copy of
   the segment's field/component text.

---

### User Story 2 - Non-standard delimiters are read from MSH-1/MSH-2 instead of hardcoded (Priority: P1)

An HL7 message that legitimately uses a non-default field separator or
encoding-characters string (e.g. a different component separator) needs to
be scanned correctly using the delimiters declared in *that message's own*
MSH-1/MSH-2, rather than being mis-parsed or rejected the way the current
Scala engine does (SPEC.md §7: "MSH-1 and MSH-2 must be standard").

**Why this priority**: This is the specific, named engine limitation this
spec exists to fix (per `ROADMAP.md`'s scope for spec `005`) and the reason
the scanner is being rebuilt now rather than ported as-is. It is P1 alongside
User Story 1 because a scanner that still hardcodes `|` and `^~\&` would not
actually satisfy this spec's purpose, even if it met User Story 1's
zero-allocation goal.

**Independent Test**: Feed the scanner a message whose MSH-1 is a character
other than `|` and whose MSH-2 declares different component/repetition/
escape/subcomponent characters, with every subsequent segment in that
message using those declared characters consistently, and confirm the
scanner produces the same shape of offset map as User Story 1 (correct
segment and delimiter offsets), using the message's own declared characters
rather than the standard ones.

**Acceptance Scenarios**:

1. **Given** a message using the standard delimiters (`|` field separator,
   `^~\&` encoding characters), **When** the scanner runs, **Then** its
   output is identical to what it would have produced had the delimiters
   been hardcoded — this spec MUST NOT change behavior for the common case.
2. **Given** a message declaring a non-standard field separator in MSH-1 and
   non-standard encoding characters in MSH-2, **When** the scanner runs,
   **Then** it uses the declared characters (not `|`/`^~\&`) to locate every
   segment, field, component, repetition, and subcomponent boundary in the
   message.
3. **Given** a message whose MSH-1 character also happens to appear as
   ordinary data later in MSH-2's own encoding-characters field, **When** the
   scanner parses MSH-1 and MSH-2, **Then** it reads MSH-1 as exactly the one
   character immediately following the `MSH` segment name and MSH-2 as
   exactly the four (or fewer, per FR-006) characters immediately following
   that, without ambiguity.

---

### User Story 3 - Malformed MSH segments produce a clear structural error (Priority: P2)

A caller who scans a message with a missing, truncated, or otherwise
malformed MSH segment (e.g. no MSH-1 character present, MSH-2 shorter than
required) needs a clear, specific structural error rather than a silent
mis-scan, a panic, or a confusing downstream failure once the PATH parser or
query executor tries to use bogus offsets.

**Why this priority**: Constitution Principle III (Explicit, Exception-Free
Data Absence) reserves exceptions/errors specifically for violated
structural preconditions — a malformed MSH is exactly that case, distinct
from "field not present." This is P2 rather than P1 because it depends on
User Story 2's delimiter-reading logic existing first (there is nothing to
validate a malformed MSH against otherwise).

**Independent Test**: Feed the scanner a set of deliberately malformed MSH
messages (empty message, message not starting with `MSH`, MSH segment
shorter than the minimum needed to contain MSH-1 and MSH-2) and confirm each
produces a distinct, specific structural error rather than a generic failure
or incorrect offsets.

**Acceptance Scenarios**:

1. **Given** a message that does not begin with the 3-character segment name
   `MSH`, **When** the scanner runs, **Then** it reports a structural error
   identifying that the message does not start with a valid MSH segment,
   and produces no offset map.
2. **Given** a message whose first segment is `MSH` but is too short to
   contain both MSH-1 and a complete MSH-2, **When** the scanner runs,
   **Then** it reports a structural error identifying the MSH segment as
   truncated, and produces no offset map.
3. **Given** a message with a well-formed MSH but a malformed later segment
   (e.g. a segment with no recognizable name), **When** the scanner runs,
   **Then** the error identifies which segment and byte offset triggered the
   failure, rather than only reporting failure in general terms.

---

### Edge Cases

- What happens when MSH-2 declares fewer than four encoding characters (e.g.
  only component and repetition separators, omitting escape or
  subcomponent)? MSH-2 MUST be exactly four characters
  (component/repetition/escape/subcomponent, in that fixed order) per HL7
  MSH-2 semantics; a shorter or longer MSH-2 is a malformed-MSH structural
  error per User Story 3, not a partial/defaulted parse.
- What happens when a message uses `\r`, `\n`, or `\r\n` as its segment
  terminator? The scanner MUST accept any of the three as a segment
  boundary, since real-world HL7 messages are inconsistent about this and
  the current Scala engine already tolerates it; segment terminator handling
  is unaffected by this spec's MSH-1/MSH-2 delimiter fix.
- What happens when the message is empty or contains only whitespace? This
  is a malformed-MSH structural error (User Story 3, Acceptance Scenario 1)
  — there is no MSH segment to read delimiters from.
- What happens when a segment other than MSH uses a character that
  coincides with the message's declared field separator inside what would
  otherwise be a literal value? Escaping within field values is out of
  scope for this spec — the scanner records delimiter *offsets*, and does
  not interpret or decode escape sequences (that is spec `1001`,
  escape-sequence decoding, per `ROADMAP.md`).
- What happens when two different segments in the same message try to
  declare different delimiters (only MSH-1/MSH-2 are meaningful per the HL7
  standard)? The scanner MUST use only the first MSH segment's MSH-1/MSH-2
  for the entire message; delimiter characters have no meaning if
  encountered as segment/field content elsewhere.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The scanner MUST perform a single pass over the raw message
  bytes/characters, producing byte offsets for every segment boundary and
  every field/component/repetition/subcomponent delimiter occurrence, without
  allocating a `String`/`Vec`-equivalent copy per field or component.
- **FR-002**: The scanner MUST read the field separator from the single
  character immediately following the `MSH` segment name (MSH-1) in the
  message's first segment, rather than assuming `|`.
- **FR-003**: The scanner MUST read the four encoding characters — component
  separator, repetition separator, escape character, subcomponent separator,
  in that fixed order — from the field immediately following MSH-1 (MSH-2),
  rather than assuming `^~\&`.
- **FR-004**: The scanner MUST use the delimiters read per FR-002/FR-003 to
  locate every segment and delimiter boundary for the entire remainder of
  the message; per-segment or per-field delimiter overrides do not exist in
  HL7 and MUST NOT be supported.
- **FR-005**: For a message using the standard delimiters (`|` and `^~\&`),
  the scanner's output MUST be identical (same offsets, same segment
  boundaries) to what a hardcoded-delimiter scanner would produce — this
  spec MUST NOT alter behavior for the common case.
- **FR-006**: The scanner MUST treat a message as malformed, and return a
  structural error instead of an offset map, when any of the following hold:
  the message does not begin with the 3-character segment name `MSH`; the
  first segment is shorter than the minimum length needed to contain both
  MSH-1 and a complete 4-character MSH-2; or any subsequent segment does not
  begin with a recognizable segment name.
- **FR-007**: Every structural error the scanner returns MUST identify the
  specific problem (e.g. "missing MSH", "truncated MSH-2", "unrecognized
  segment name") and the byte offset at which it was detected, per
  Constitution Principle III.
- **FR-008**: The scanner MUST accept `\r`, `\n`, and `\r\n` interchangeably
  as segment terminators within a single message.
- **FR-009**: The scanner MUST NOT decode or interpret escape sequences
  within field values — it records delimiter offsets only; escape-sequence
  decoding is explicitly out of scope (deferred to Roadmap spec `1001`).
- **FR-010**: This spec MUST add conformance vectors under
  `fixtures/vectors/scanner/` (a new vector family per spec `003`'s FR-007
  extensibility mechanism) covering: at least one message using non-default
  delimiters end-to-end, and at least one malformed-MSH case per each error
  condition in FR-006.
- **FR-011**: The scanner MUST be implemented as Rust source under
  `crates/core/src/scanner.rs` per the repository layout defined in
  `HL7-PET-Rust-Migration-Plan.md`, as the first Rust engine code produced by
  this migration.

### Key Entities

- **Scan Result / Offset Map**: The scanner's output for one message — an
  ordered list of segment records, each with its start/end byte offset and
  the offsets of every delimiter occurrence within it. Contains no copied
  field/component text.
- **Delimiter Set**: The five characters governing a message's structure
  (field separator from MSH-1; component, repetition, escape, and
  subcomponent separators from MSH-2), resolved once per message from its
  own MSH segment rather than hardcoded.
- **Structural Error**: A specific, located failure reported instead of a
  Scan Result when the message's MSH segment or a later segment is
  malformed, per Constitution Principle III.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of `fixtures/messages/` corpus files using standard
  delimiters scan to byte-identical offset maps whether or not the
  delimiters were read dynamically from MSH-1/MSH-2 — zero regressions for
  the common case.
- **SC-002**: The scanner correctly locates every segment and delimiter
  boundary in a message using non-standard delimiters, verified against at
  least one new conformance vector authored for this spec (FR-010) — the
  specific "MSH-1/MSH-2 must be standard" limitation is confirmed fixed.
- **SC-003**: Every malformed-MSH case enumerated in FR-006 produces a
  distinct, correctly-located structural error, verified against its own
  conformance vector (FR-010) — zero silent mis-scans or panics on malformed
  input.
- **SC-004**: Scanner peak allocation count for a given message is
  independent of that message's field/component/repetition count (varies
  only with segment count), confirmed by a benchmark comparable to the
  methodology in spec `004`'s harness.

## Assumptions

- This spec covers the scanner only (offsets, delimiter resolution,
  structural validation of MSH and segment names). It does not implement
  PATH parsing, query execution, or hierarchy navigation — those are specs
  `006`-`008`, which will consume this scanner's offset output.
- "Encoding characters" per FR-003 always means exactly four characters in
  the fixed order component/repetition/escape/subcomponent, matching MSH-2's
  standard HL7 definition (`^~\&`); MSH-2 declaring a different *count* of
  characters is a structural error (FR-006), not a variant format to
  support.
- Segment-name recognition for the "unrecognized segment name" error
  condition (FR-006) means a non-empty, minimally sane token at the start of
  a segment (e.g. an alphabetic-led 3-character code, consistent with spec
  `001`'s tightened `SEG` grammar rule); this spec does not validate segment
  names against a profile's declared segment list — that remains structural
  validation's job (Roadmap module 2000-2999).
- This is the first spec to introduce a Cargo workspace/crate for the Rust
  core; the exact crate name, workspace structure, and build tooling beyond
  what `HL7-PET-Rust-Migration-Plan.md` already specifies (`crates/core` as
  `hl7pet-core`) are implementation details for `/speckit-plan`, not decided
  here.
- Performance comparison against the Scala baseline (spec `004`) beyond the
  allocation-independence claim in SC-004 is deferred to spec `009`
  (core-perf-validation), which benchmarks specs `005`-`008` together against
  the Scala baseline once more of the core exists.
