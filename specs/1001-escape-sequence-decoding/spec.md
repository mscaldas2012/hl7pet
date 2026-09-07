# Feature Specification: Escape-Sequence Decoding

**Feature Branch**: `1001-escape-sequence-decoding`

**Created**: 2026-09-05

**Status**: Draft

**Input**: User description: "Escape-sequence decoding: getValue/getFirstValue gain a decodeEscapes parameter defaulting to true, decoding standard HL7 v2 escape sequences (\F\, \S\, \T\, \R\, \E\, \H\, \N\, \X..\, \Zxxx\) in returned values instead of passing them through raw. Passing decodeEscapes=false reproduces today's raw-passthrough behavior exactly. This is a deliberate, documented breaking change (not a Backward-Compatible Addition) requiring a MAJOR version bump and migration guide."

**Clarified scope (2026-09-05)**: decoding applies unconditionally — there is
no runtime opt-out. Every value-extraction method (flat-PATH and
hierarchy-navigated alike, per this spec's Assumptions) always decodes; a
caller cannot ask for the old raw-passthrough behavior for a specific call.
This is a deliberate simplification: since no message in the existing shared
fixtures corpus contains an escape sequence today, this change is
behaviorally invisible to the entire existing regression suite — it only
changes output for values that actually contain an escape sequence, which is
new fixture data this spec itself introduces. See Assumptions.

## User Scenarios & Testing *(mandatory)*

This is a Parsing & Extraction deliverable (Roadmap module `1000`-`1999`, spec
`1001`) that fixes a documented, long-standing limitation of the current
Scala engine (`SPEC.md` §7: "No escaped character support" — HL7 escape
sequences are not decoded, values are returned as-is). Unlike spec `1000`,
this is a **deliberate exception** to the Backward-Compatible-Additions
convention: rather than a parallel method, the existing value-extraction
methods change their default (and only) output. Its "users" are callers of
`hl7pet-core`'s query API — today's dev CLI, and any future language binding
(Python/Java, module `6000`-`6999`) — plus everyone upgrading past this
change who needs to understand what output changed and why.

### User Story 1 - Caller receives correctly decoded values, unconditionally (Priority: P1)

A caller extracting a value that contains an HL7 escape sequence — e.g. a
name field using `\S\` to represent a literal component-separator character,
or a note field using `\H\...\N\` to mark highlighted text — needs the
returned value to contain the actual represented character(s)/text, not the
raw escape syntax, without passing any extra argument to get this, and with
no way to accidentally get the old raw form back.

**Why this priority**: This is the feature's entire reason to exist — it is
the fix for the documented limitation, not an enhancement to it. The other
user story exists only because this one changes existing behavior.

**Acceptance Scenarios**:

1. **Given** a value containing `\F\`, `\S\`, `\T\`, `\R\`, or `\E\`, **When**
   it is extracted, **Then** the returned value contains that message's own
   actual field, component, subcomponent, repetition, or escape delimiter
   character in place of the escape sequence — not a hardcoded standard
   character, since messages may declare non-standard delimiters (spec
   `005`).
2. **Given** a value containing `\H\highlighted text\N\`, **When** it is
   extracted, **Then** the returned value contains `highlighted text` with
   the `\H\`/`\N\` markers removed and no other change to the enclosed text.
3. **Given** a value containing a hexadecimal escape sequence (`\Xdddd\`),
   **When** it is extracted, **Then** the returned value contains the
   character(s) the hexadecimal digits represent.
4. **Given** a value containing no escape sequences at all, **When** it is
   extracted, **Then** the returned value is byte-for-byte identical to
   today's output — this feature only changes output for values that
   actually contain an escape sequence, and no message in the existing
   shared fixtures corpus does today (see Assumptions), so this change is
   invisible to every pre-existing test.

---

### User Story 2 - Caller upgrading consults the migration guide (Priority: P2)

A caller upgrading past this change needs a clear, complete written
explanation of exactly what output changes and why, since there is no
runtime toggle to fall back on.

**Why this priority**: This is a mandatory deliverable of a breaking change
(Constitution Principle I), not optional documentation — but it has no
runtime behavior of its own, so it is lower priority than the behavioral
story above.

**Independent Test**: Read the migration guide and confirm it names every
escape-sequence type this feature decodes and states plainly that decoding
is unconditional, with no opt-out.

**Acceptance Scenarios**:

1. **Given** the migration guide, **When** a caller reads it, **Then** they
   can determine, without reading any code, whether their existing calls'
   output will change, and understand there is no per-call way to prevent
   it if it does.

---

### Edge Cases

- What happens when the escape character appears with no valid sequence
  following it (a trailing, unterminated `\`, or an unrecognized sequence
  type)? It MUST be left completely unmodified in the output — never an
  error, never a dropped character, never a crash.
- What happens with a custom, application-defined sequence (`\Zxxx\`), for
  which no universal decoding rule exists? Its delimiters are removed and
  its inner content is passed through unchanged (see Assumptions) — this
  feature does not provide, and does not require, an application-specific
  lookup table.
- What happens when decoding would change a value's structural meaning —
  e.g. a decoded `\S\` producing a literal component-separator character
  inside what is otherwise a single decoded value? Decoding happens strictly
  after a value has already been extracted and its field/component/
  subcomponent boundaries already resolved; it never re-triggers re-parsing
  of the already-extracted value.
- What happens with back-to-back or adjacent escape sequences (e.g.
  `\H\Bold\N\ and \H\Italic\N\` in one value)? Each sequence decodes
  independently, left to right.
- What happens for a hierarchy-navigated (`->`) value that also contains an
  escape sequence? It decodes the same way as any other extracted value —
  this feature is a value-content transformation applied wherever a value is
  returned, not scoped to one navigation style (see Assumptions).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST decode standard HL7 v2 escape sequences — field
  (`\F\`), component (`\S\`), subcomponent (`\T\`), repetition (`\R\`), and
  escape-character (`\E\`) — within values returned by value-extraction
  methods, by default.
- **FR-002**: Each of `\F\`/`\S\`/`\T\`/`\R\`/`\E\` MUST decode to that
  specific message's own actual delimiter character (as declared in that
  message's `MSH-1`/`MSH-2`), never a hardcoded standard character — a
  message using non-standard delimiters (spec `005`) MUST decode using its
  own delimiters.
- **FR-003**: System MUST decode highlighting markers (`\H\` .. `\N\`) by
  removing both markers from the output while preserving the text between
  them unchanged.
- **FR-004**: System MUST decode hexadecimal-data escape sequences
  (`\Xdddd\`) to the character(s) the hexadecimal digit pairs represent.
- **FR-005**: System MUST decode custom/locally-defined escape sequences
  (`\Zxxx\`) by removing the `\Z`/`\` delimiters and passing the inner
  content through unchanged (Assumptions — no universal decode rule exists
  for these).
- **FR-006**: An escape character not followed by a recognized, complete
  escape sequence (unterminated, or an unrecognized sequence-type character)
  MUST be left completely unmodified in the output — never an error, never a
  panic, never a dropped or altered character.
- **FR-007**: Decoding MUST be unconditional — no value-extraction method
  offers a way to request the pre-change (raw, undecoded) output for a
  specific call. There is exactly one behavior, not a default plus an
  opt-out.
- **FR-008**: This is a documented, deliberate breaking change, not a
  Backward-Compatible Addition (`ROADMAP.md`'s Documented Breaking Changes
  table already pre-registers it): it MUST ship with a version bump and a
  written migration guide (Constitution Principle I), both delivered as
  part of this feature, not deferred.
- **FR-009**: The migration guide MUST name every escape-sequence type this
  feature decodes, state plainly that decoding is now unconditional, and
  say explicitly that no per-call opt-out exists.
- **FR-010**: A value containing no escape sequence at all MUST be returned
  unchanged from today's behavior — this feature MUST NOT alter any output
  that isn't itself the subject of an escape sequence.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every documented escape-sequence type (`\F\`, `\S\`,
  `\T\`, `\R\`, `\E\`, `\H\`/`\N\`, `\Xdddd\`, `\Zxxx\`), a value containing
  it is decoded correctly, verified against a conformance vector for each
  type.
- **SC-002**: 100% of extraction calls against messages containing zero
  escape sequences return output byte-for-byte identical to today's
  behavior — verified by the full pre-existing regression suite continuing
  to pass unmodified (no message in that corpus contains an escape sequence
  today, per Assumptions).
- **SC-003**: The migration guide names every escape-sequence type this
  feature newly decodes and states plainly that no opt-out exists —
  verified by a direct cross-check against FR-001-FR-005's list.

## Assumptions

- Custom, application-defined sequences (`\Zxxx\`) have their delimiters
  stripped and their inner content passed through unchanged, since no
  universal decoding rule exists for them and this feature does not
  introduce an application-specific lookup-table mechanism — a caller
  needing custom `\Z`-sequence interpretation applies it themselves to the
  (already delimiter-stripped) content.
- Decoding is a value-content transformation applied wherever a value is
  returned — including values reached via spec `1000`'s location-aware
  extraction and via hierarchy (`->`, spec `008`) navigation, not only the
  flat-PATH methods spec `1000` itself scoped to. Unlike spec `1000`, this
  feature has no reason to exclude hierarchy or location-aware results: it
  changes what a returned value's *content* looks like, not how a value is
  located. Confirmed as this spec's intended scope: decoding is unconditional
  everywhere a value is returned, with no exceptions carved out for specific
  extraction methods.
- No per-call opt-out exists (clarified 2026-09-05, superseding the
  `decodeEscapes=false` framing in this spec's own Input above, which
  mirrored `ROADMAP.md`'s original Scala-flavored framing before this
  clarification). This is deliberately simpler than Constitution Principle
  I's usual expectation of a way to opt back into prior behavior: since the
  existing shared fixtures corpus contains no escape sequences at all today,
  this change alters no existing test's expected output, so there is no
  real prior behavior being taken away from any caller this repository's own
  regression suite represents. A caller outside this repository who
  genuinely needs pre-decode raw text has no built-in way to get it and must
  handle that themselves (documented plainly in the migration guide, FR-009).
- This feature covers decoding only. Nothing in this spec requires the
  reverse operation (re-encoding a value back into escaped form for writing
  HL7 output) — the existing Scala engine and the Rust migration are
  read/extraction-oriented; encoding is out of scope unless a future spec
  introduces message construction.
- Fixture data exercising escape sequences is kept in its own, separate
  location from the pre-existing shared corpus (not mixed into existing
  messages/vectors), since this feature's conformance vectors need messages
  containing escape sequences that no current fixture has — keeping them
  separate makes clear which fixtures exist specifically to validate
  decoding, and confirms by construction that no pre-existing vector's
  expected output needed to change (SC-002).
