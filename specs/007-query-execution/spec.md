# Feature Specification: Query Execution

**Feature Branch**: `007-query-execution`

**Created**: 2026-07-25

**Status**: Draft

**Input**: User description: "Navigate offsets to extract values; validated against the `003` regression suite. (Roadmap module 0-999 Rust Core, Migration Plan Phase 2, spec `007`)"

## User Scenarios & Testing *(mandatory)*

This spec is the piece that finally connects the two prior Phase 2 deliverables:
the message scanner's offsets (spec `005`, `hl7pet_core::scanner::scan`) and the
PATH parser's compiled queries (spec `006`, `hl7pet_core::parser::parse`). Neither
prior spec extracts a value from a message — the scanner only locates segments and
delimiters, and the parser only validates and structures a PATH string. This spec
is the query executor: given a `ScanResult` and a `CompiledPath`, it navigates the
offsets to produce the actual string value(s) the caller asked for. Its "users" are
downstream Rust core components (spec `008`'s hierarchy navigation, and eventually
the language bindings in the `6000`-range) and, transitively, every existing caller
of the Scala engine's `getValue`/`getFirstValue`/`retrieveMultipleSegments`-family
methods that this spec's output must reproduce byte-for-byte for every
standard-delimiter message in the shared regression suite (spec `003`).

### User Story 1 - A single-value PATH returns the correct field/component/subcomponent (Priority: P1)

A caller has already compiled a PATH like `PID-5.1` or `OBX[2]-5` (spec `006`) and
has a scanned message (spec `005`). They need the query executor to walk to the
right segment occurrence, the right field, and — if requested — the right
component or subcomponent within it, and return exactly that substring.

**Why this priority**: This is the base case every other capability in this spec
builds on, and the one the existing Scala `getValue`/`getFirstValue` callers
exercise on nearly every call. Without it, spec `006`'s compiled PATHs have nothing
that actually reads a message.

**Independent Test**: For every non-hierarchy, non-filter vector in
`fixtures/vectors/path/valid.json`, scan the vector's referenced message (spec
`005`), execute the compiled PATH (spec `006`) against the scan result, and confirm
the extracted value matches the vector's Scala-verified expected value exactly.

**Acceptance Scenarios**:

1. **Given** a scanned message with a `PID` segment and a compiled PATH for
   `PID-5.1` (family name component), **When** the query is executed, **Then** the
   executor returns exactly the substring occupying that component, using the
   scanned message's own delimiters (spec `005` FR-005) to split it.
2. **Given** a compiled PATH with no field expression (segment only, e.g. `PID`),
   **When** the query is executed, **Then** the executor returns the full raw
   content of the matched segment occurrence, unsplit.
3. **Given** a compiled PATH whose field expression has no component/subcomponent
   (e.g. `PID-5`), **When** the query is executed, **Then** the executor returns the
   full field content, including any of its own component/repetition delimiters
   unsplit.

---

### User Story 2 - Segment and field index selectors resolve to the right occurrence (Priority: P1)

A message can contain repeated segments (e.g. multiple `OBX`) and a field can
contain repeated values (repetitions, separated by `~`). A caller's compiled PATH
carries a segment index selector (`Numeric`, `Last`, `Star`, or a `Filter`) and,
independently, a field index selector (`Numeric`, `Last`, or `Star`) that pick
which occurrence(s) the query targets.

**Why this priority**: Repeated segments and field repetitions are common in real
HL7 traffic (e.g. multiple `OBX` per `OBR`, multiple phone numbers in one field).
A query executor that only reads the first occurrence of everything would silently
return wrong data for a large fraction of real messages — this is not an edge case,
it is core correctness.

**Independent Test**: For every vector in `fixtures/vectors/path/valid.json` whose
PATH includes a segment or field index selector, execute it against its referenced
message and confirm the result matches the specific occurrence the vector's
expected value identifies (not just "some" occurrence).

**Acceptance Scenarios**:

1. **Given** a message with three `OBX` segments and a compiled PATH `OBX[2]-5`,
   **When** executed, **Then** the executor returns the value from the second `OBX`
   occurrence (1-based), not the first or last.
2. **Given** a compiled PATH using `$LAST` as its segment index (e.g. `OBX[$LAST]-5`),
   **When** executed, **Then** the executor returns the value from the final
   matching segment occurrence in the message, regardless of how many there are.
3. **Given** a compiled PATH using `*` (or an omitted index, which the parser
   treats identically per spec `006`'s data-model) as its segment index, **When**
   executed against a message with multiple matching segments, **Then** the
   executor returns every matching occurrence's value, in message order, not just
   the first.
4. **Given** a field containing multiple `~`-separated repetitions and a compiled
   PATH with a numeric or `$LAST` field index, **When** executed, **Then** the
   executor returns the specific repetition selected, using the message's own
   repetition delimiter (spec `005`).
5. **Given** a segment or field index selector whose 1-based occurrence number
   exceeds the number of occurrences actually present (e.g. `OBX[5]-5` when only
   three `OBX` segments exist), **When** executed, **Then** the executor reports no
   match — the same "no data" outcome as a segment type that is entirely absent
   (spec `003`'s regression suite confirms this is the existing Scala engine's actual
   behavior: `getValue`/`getFirstValue` return no match here, not an error) — never a
   value from a different occurrence and never a value silently clamped to the
   nearest valid index.

---

### User Story 3 - A filter clause selects the matching segment occurrence (Priority: P2)

A compiled PATH's segment index can be a filter clause instead of a plain index —
e.g. "the `OBX` whose field 3 component 1 equals `TEMP`" — carrying a target field
(with optional component/subcomponent), a comparison operator, and one or more
OR'd literal values. The executor must scan matching segment occurrences in order
and select the one(s) whose targeted sub-value satisfies the filter.

**Why this priority**: Filters are how a caller selects "the right" repeated
segment by content rather than by position, which is common when segment order in
a message isn't guaranteed (e.g. picking a specific `OBX` by its observation
identifier rather than assuming it's always third). It builds directly on User
Story 2's occurrence-walking and shares its correctness bar, but is lower priority
because plain positional/`$LAST`/`*` selection (User Story 2) already covers the
more common case and must exist first.

**Independent Test**: For every filter vector in `fixtures/vectors/path/valid.json`
and `fixtures/vectors/path/invalid.json` (as applicable), execute the compiled
filter PATH against its referenced message and confirm the executor selects
exactly the occurrence(s) the vector's expected value identifies.

**Acceptance Scenarios**:

1. **Given** a message with several `OBX` segments and a filter PATH whose
   condition matches exactly one of them, **When** executed, **Then** the executor
   returns the value from that one matching occurrence.
2. **Given** a filter condition that matches more than one occurrence, **When**
   executed, **Then** the executor returns every matching occurrence's value, in
   message order (consistent with User Story 2's `*` behavior).
3. **Given** a filter condition that matches zero occurrences, **When** executed,
   **Then** the executor reports a no-match condition rather than an error or a
   value from a non-matching occurrence.
4. **Given** a filter using an ordering operator (`>`, `>=`, `<`, `<=`) against a
   targeted value that is not numeric, **When** executed, **Then** the executor
   reports a comparison-failure condition rather than panicking or silently
   treating the comparison as false.
5. **Given** a filter with multiple OR'd values (e.g. `@3=TEMP||PULSE`), **When**
   executed, **Then** an occurrence satisfies the filter if its targeted sub-value
   equals *any* one of the OR'd values.

---

### Edge Cases

- What happens when the compiled PATH's segment name has zero matching
  occurrences in the message at all (the segment never appears)? Reported as no
  match — the same outcome as an explicit segment or field index that is out of
  range for what actually is present (see the next bullet); the existing engine
  does not distinguish "this segment type isn't in this message" from "the index I
  asked for doesn't exist here" as two different return shapes, so this executor
  does not invent that distinction either (FR-010).
- What happens when an explicit segment or field index (e.g. `OBX[5]` when only 3
  `OBX` occurrences exist, or `OBX-5[5]` when the field has only 2 repetitions) is
  out of range for what is actually present? No match — verified against the real
  Scala engine (spec `004`'s dependency) to be its actual behavior, not a thrown
  exception or an error condition.
- What happens when a requested field number is higher than the number of fields
  actually present in the matched segment occurrence? Treated as no value present
  (same shape as a genuinely empty field), not an error — the Scala engine's
  existing behavior for this case is the compatibility bar (see FR-010).
- What happens when a requested component or subcomponent number is higher than
  the number of components/subcomponents actually present in the field? Same
  answer as the field case above — absent, not an error.
- What happens when the compiled PATH carries a `child` (hierarchy `->`)? This
  spec's executor does not resolve hierarchy navigation at all (deferred to spec
  `008`) — executing a `CompiledPath` with `child` set is out of scope for this
  spec's public function; see Assumptions.
- What happens when the field/component/subcomponent content itself contains an
  escape sequence (e.g. `\F\`)? This spec returns the raw, undecoded substring —
  escape decoding is spec `1001`'s scope, not this one's.
- What happens when the underlying message content between two delimiters is
  empty (e.g. two consecutive field separators)? The executor returns an empty
  string for that position, distinct from "field not present" (previous edge
  cases) — both must be distinguishable in the output.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The executor MUST accept a `ScanResult` (spec `005`) and a
  `CompiledPath` whose top-level form has no `child` set (spec `006`), and produce
  either the extracted value(s), a no-match outcome, or a comparison-failure
  condition (FR-008) — never a panic, never a partial result alongside a failure
  (Constitution Principle III, consistent with specs `005`/`006`'s own contracts).
- **FR-002**: For a `CompiledPath` with no field expression, the executor MUST
  return the full raw content of each matched segment occurrence, unsplit by any
  delimiter.
- **FR-003**: For a `CompiledPath` with a field expression, the executor MUST
  split the matched segment occurrence's content using the message's own resolved
  delimiters (spec `005`'s `DelimiterSet`, never hardcoded) down to the level
  requested (field only, field+component, or field+component+subcomponent) and
  return exactly that substring.
- **FR-004**: The executor MUST resolve a segment index selector (`Numeric`,
  `Last`, `Star`) against the ordered set of segment occurrences in the message
  whose name matches the compiled path's segment name, per the semantics
  demonstrated in User Story 2.
- **FR-005**: The executor MUST resolve a field index selector (`Numeric`,
  `Last`, `Star`) against the ordered set of repetitions (`~`-delimited) within
  the targeted field, per the semantics demonstrated in User Story 2.
- **FR-006**: When a segment or field index selector resolves to `Star` (or an
  omitted index, treated identically per spec `006`), or to a filter clause
  matching more than one occurrence, the executor MUST return every matching
  value in message order, not only the first.
- **FR-007**: The executor MUST resolve a `FilterClause` (spec `006`) by, for each
  candidate segment occurrence (in message order), extracting the filter's target
  sub-value using the same field/component/subcomponent navigation as FR-003, then
  evaluating the filter's operator against each of the filter's OR'd values in
  turn, selecting the occurrence if any one comparison succeeds.
- **FR-008**: The executor MUST support all six filter operators (`=`, `!=`, `>`,
  `>=`, `<`, `<=`); ordering operators (`>`, `>=`, `<`, `<=`) MUST compare the
  target sub-value and the filter value numerically when both parse as numbers,
  and MUST report a distinct comparison-failure condition (not a silent `false`,
  not a panic) when either side does not parse as a number.
- **FR-009**: The executor's outcomes for each of the following scenarios MUST be
  independently verified, per the existing Scala engine's actual behavior (FR-010):
  (a) one or more values successfully extracted; (b) the requested segment type has
  zero occurrences in the message; (c) a requested index (segment or field) is out
  of range for the occurrences actually present; (d) a filter condition matched zero
  occurrences; and (e) a requested field, component, or subcomponent number is
  beyond what a matched occurrence contains. Per the existing engine's verified
  behavior, (b), (c), (d), and (e) are all a "no data" outcome — the same shape as
  (a) with zero values, never an error and never a value from the wrong occurrence
  or silently clamped to the nearest valid index. Only FR-008's comparison-failure
  condition is a distinguishable non-value outcome.
- **FR-010**: For every standard-delimiter message and non-hierarchy PATH vector
  in the shared regression suite (spec `003`) and `fixtures/vectors/path/`, the
  executor's extracted value(s) MUST match the existing Scala engine's output for
  the equivalent `getValue`/`getFirstValue`/`retrieveMultipleSegments`-family call
  exactly, byte-for-byte — this is the executor's compatibility bar, not merely
  its own internal design intention.
- **FR-011**: The executor MUST NOT attempt to interpret or resolve a
  `CompiledPath`'s `child` field (hierarchy navigation) — that remains spec `008`'s
  responsibility; this spec's public function operates only on the non-hierarchy
  `segment [+ field]` form.
- **FR-012**: The executor MUST NOT decode escape sequences (e.g. `\F\`, `\H\`) in
  any extracted value — it returns the raw substring as it appears in the message;
  decoding is spec `1001`'s scope.
- **FR-013**: The executor MUST NOT copy substrings out of the original message
  where a borrowed reference suffices, consistent with Constitution Principle II
  and the zero-copy precedent set by specs `005`/`006`.
- **FR-014**: This spec MUST add new conformance vectors (extending, not
  replacing, the existing corpus) covering execution-time cases not already
  exercised by specs `001`/`005`/`006`'s vectors: an out-of-range segment index, an
  out-of-range field index, a filter matching zero occurrences, a filter matching
  multiple occurrences, a non-numeric value compared with an ordering operator,
  and a `*`/omitted-index query against a segment with multiple occurrences.

### Key Entities

- **Query Result**: The executor's success output for one executed `CompiledPath`
  — an ordered list of zero or more extracted values (zero for any of the "no data"
  conditions in FR-009(b)-(e)), each still tied to the source message's lifetime (no
  copying, FR-013).
- **Comparison Failure**: The executor's one distinguishable non-value outcome —
  FR-008's condition when an ordering operator is applied to an operand that does
  not parse as a number, identifying the operator involved.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of non-hierarchy, standard-delimiter vectors in the shared
  regression suite (spec `003`) and `fixtures/vectors/path/` produce
  byte-for-byte identical extracted values when executed through this spec's
  executor, compared against the existing Scala engine's output for the same
  message and equivalent query.
- **SC-002**: Every occurrence-selection scenario described in User Stories 2 and
  3 (positional, `$LAST`, `*`/omitted, and filter-based selection, including
  multi-match and zero-match) is covered by at least one passing conformance
  vector.
- **SC-003**: Executing a query never crashes the process for any combination of
  a valid `ScanResult` and a non-hierarchy `CompiledPath`, including against
  malformed-but-scannable messages and indices/filters that match nothing.
- **SC-004**: Extracting a value performs at most one pass over the matched
  segment occurrence(s)' content — no repeated re-scanning of the full message
  per query.

## Assumptions

- The executor's public entry point operates only on the non-hierarchy form of a
  `CompiledPath` (`segment` plus optional `field`, per spec `006`'s data-model).
  Executing a `CompiledPath` whose `child` is set is out of scope for this spec
  and is spec `008`'s responsibility; this spec does not define behavior for that
  case beyond "not handled here."
- Filter-target and field/component/subcomponent navigation reuse the same
  splitting logic (FR-003) for both direct extraction and filter evaluation —
  there is one navigation path, not two independently-maintained ones.
- Numeric comparison for ordering operators (FR-008), when both operands do parse
  as numbers, uses ordinary numeric parsing — no new numeric-format support beyond
  what a caller would expect from decimal/integer literals.
- Verified against the real Scala library (`gov.cdc:hl7-pet_2.13:1.2.11`, spec
  `004`'s Maven Central dependency): an out-of-range segment or field index (FR-009
  (c)) is confirmed to produce no match, the same as (b)/(d)/(e) — not a distinct
  error condition, contrary to an earlier draft of this spec. An ordering operator
  applied to a non-numeric operand is confirmed to be the one case the existing
  engine does *not* handle gracefully (an uncaught `NumberFormatException`); FR-008
  deliberately requires this executor to surface it as a typed, non-panicking
  condition instead of reproducing the crash — the same kind of graceful-rejection
  improvement over a Scala crash that spec `001`'s grammar-tightening already
  established as this migration's convention, not a byte-for-byte parity violation
  (there is no crash "value" for FR-010 to reproduce).
- This spec depends on spec `005` (scanner offsets) and spec `006` (compiled
  PATHs) as already-implemented inputs; it does not modify either's public API.
- Escape-sequence decoding (spec `1001`) and hierarchy navigation (spec `008`)
  are explicitly out of scope, per the "does NOT provide" sections of specs
  `005`/`006`'s own contracts.
