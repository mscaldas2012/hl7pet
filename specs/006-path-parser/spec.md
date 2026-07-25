# Feature Specification: PATH Parser

**Feature Branch**: `006-path-parser`

**Created**: 2026-07-25

**Status**: Draft

**Input**: User description: "Hand-written PATH parser/state machine replacing regex; compiles paths into reusable query objects. (Roadmap module 0-999 Rust Core, Migration Plan Phase 2, spec `006`)"

## User Scenarios & Testing *(mandatory)*

This spec turns the formal grammar spec `001` documented (`fixtures/vectors/path/`,
`contracts/path-grammar.md`) into working Rust core code: a parser that takes a
raw PATH string and produces a structured, reusable representation of it. Its
"users" are downstream Rust core components — the query executor (spec `007`)
and lazy hierarchy navigation (spec `008`) — that need a validated, structured
PATH instead of a raw string, and transitively every caller who writes a PATH
into a rules file or config today and currently only finds out it's malformed
when the current Scala engine throws mid-evaluation.

### User Story 1 - Malformed PATHs are rejected at parse time with a precise reason (Priority: P1)

A rule/config author writes a PATH string (e.g. in a rules file) that turns
out to be malformed — a bad index token, an invalid filter operator, the
wrong separator between segment and field. Today's Scala engine's regex
matches many of these anyway and then throws an uncaught exception
(`NumberFormatException`, `MatchError`) deep in evaluation. This parser must
catch the same mistakes at parse time, before any message is touched, with a
specific, located reason.

**Why this priority**: This is the exact defect spec `001`'s grammar contract
was written to close (see `contracts/path-grammar.md` Notes #2/#3) and the
reason this parser is hand-written rather than a thin regex wrapper. Without
it, spec `001`'s tightened grammar is just documentation with nothing
enforcing it.

**Independent Test**: Feed the parser every entry in
`fixtures/vectors/path/invalid.json` and confirm each is rejected with a
distinct, located syntax error — none panics, none is silently accepted,
none produces a partial result.

**Acceptance Scenarios**:

1. **Given** a PATH whose segment index bracket contains a non-numeric,
   non-`$LAST`, non-`*`, non-filter token (e.g. `PID[ABC]-1`), **When** the
   parser runs, **Then** it returns a syntax error identifying the invalid
   `SEG_IDX` token and its position in the string, with no compiled result.
2. **Given** a PATH whose filter uses an operator outside `=`, `!=`, `>`,
   `>=`, `<`, `<=` (e.g. `OBX[@3=='9945-3']-5`), **When** the parser runs,
   **Then** it returns a syntax error identifying the invalid operator.
3. **Given** a PATH using `.` where the grammar requires `-` between a
   segment expression and a field expression (e.g. `OBX[1].5`), **When** the
   parser runs, **Then** it returns a syntax error, not a best-effort guess
   at intent.
4. **Given** any string in `fixtures/vectors/path/invalid.json`, **When** the
   parser runs, **Then** it never panics or crashes the process, regardless
   of how malformed the input is.

---

### User Story 2 - A parsed PATH is compiled once and reused across many messages (Priority: P1)

A caller evaluating the same PATH against a high-volume stream of HL7
messages needs to pay the parsing cost exactly once, then reuse the
resulting compiled form for every message, rather than re-parsing (or
re-compiling a regex) on every single evaluation.

**Why this priority**: This is the roadmap's defining goal for this spec
("compiles paths into reusable query objects") and the foundation spec `007`
(query execution) builds directly on — Constitution Principle II (Zero-Copy &
Lazy Evaluation) requires that repeated evaluation not re-pay avoidable cost.

**Independent Test**: Parse a single valid PATH string once, in isolation
from any HL7 message, and confirm the resulting compiled representation (a)
does not reference or depend on any specific message or scanner output (spec
`005`), and (b) can be inspected/reused without re-parsing the original
string.

**Acceptance Scenarios**:

1. **Given** a valid PATH string, **When** it is parsed, **Then** parsing
   succeeds without any HL7 message, scanner offset, or hierarchy profile
   being supplied — the parser's only input is the PATH string itself.
2. **Given** a compiled PATH produced from a single parse call, **When** it
   is reused across multiple (simulated) evaluation calls, **Then** no
   re-parsing of the original string occurs for any of them.

---

### User Story 3 - The compiled PATH exposes structured fields, not just a validated string (Priority: P2)

Downstream components (query execution, spec `007`; hierarchy navigation,
spec `008`) need to read the segment name, index selector, field/component/
subcomponent numbers, filter clauses, and hierarchy hop off a parsed PATH as
structured data, without re-deriving that information by re-scanning the
original string themselves.

**Why this priority**: P2, not P1, because it doesn't block this spec's own
correctness (User Stories 1-2 already establish parse/reject and
reuse) — but it is the actual contract specs `007`-`008` will build against,
matching how spec `005`'s `scanner-api.md` documents itself as "the contract
specs `006`/`007` build on."

**Independent Test**: Parse a representative PATH from each shape in
`fixtures/vectors/path/valid.json` (bare segment, numeric/`$LAST`/`*` index,
field with component/subcomponent, filter, repetition index, hierarchy hop)
and confirm the compiled representation exposes each piece as distinguishable
structured data (e.g. the filter's operator and value list are individually
readable, not embedded in an opaque string).

**Acceptance Scenarios**:

1. **Given** `PID[1]-5`, **When** parsed, **Then** the compiled result
   exposes segment name `PID`, numeric segment index `1`, and field number
   `5`, as distinct fields.
2. **Given** `OBX[@3.1='94500-6']-5`, **When** parsed, **Then** the compiled
   result exposes the filter's target (field `3`, component `1`), operator
   `=`, and value list `["94500-6"]` as distinct, readable fields — not just
   confirmation that the string was valid.
3. **Given** `OBR[1] -> OBX-5`, **When** parsed, **Then** the compiled result
   exposes the parent segment expression (`OBR[1]`) and the child path
   (`OBX-5`) as distinct fields, without attempting to navigate or evaluate
   the hierarchy relationship itself.

---

### Edge Cases

- What happens with an empty or whitespace-only PATH string? Rejected as a
  syntax error — the grammar's `PATH` production requires at minimum a
  `SEGMENT_EXPR`, and no string qualifies as one.
- What happens with a syntactically valid PATH that addresses a
  segment/field unlikely to exist in any real message (e.g. `XYZ-99`,
  `fixtures/vectors/path/valid.json`'s `path-zero-values-nonexistent`)? It
  parses successfully — whether it matches real data is an evaluation-time
  (spec `007`) concern, not a parse-time one, per `contracts/path-grammar.md`.
- What happens with a filter that OR's multiple values (`@3='A||B'`) or adds
  a subcomponent (`@3.1.2='...'`)? Both are valid per the grammar and MUST
  parse into a filter with a multi-element value list / a populated
  subcomponent field respectively — neither combination is covered by the
  existing 17 `fixtures/vectors/path/` vectors, so new vectors are needed
  (FR-010).
- What happens with optional whitespace around a filter's operator
  (`OBX[@3.1 = '94500-6']-5`)? MUST parse identically to the no-whitespace
  form — also not covered by the existing vector set (FR-010).
- What happens with a hierarchy chain of more than one `->` hop (e.g.
  `ORC[1] -> OBR[1] -> OBX-5`)? Rejected as a syntax error. The current
  grammar's `CHILD_PATH` production (`contracts/path-grammar.md`) is
  single-hop only; multi-hop chaining is a proposed future addition the
  grammar file explicitly assigns to a later spec ("likely `008`"), not
  this one (see Assumptions).
- What happens with a `SEG_IDX`/`FIELD_IDX` value of `0`, or a value with
  leading zeros? Accepted — the grammar treats any syntactically valid
  `NUMBER` as parseable; whether `0` is a meaningful occurrence index is a
  semantic question for evaluation (spec `007`), not a parse-time rejection,
  per `contracts/path-grammar.md`'s Non-Goals.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The parser MUST accept exactly the strings defined as valid by
  `specs/001-path-grammar-spec/contracts/path-grammar.md`'s grammar, and
  reject every other string — no broader, no narrower.
- **FR-002**: For a valid PATH, the compiled representation MUST expose the
  segment name and, when present, its index selector (numeric occurrence,
  `$LAST`, `*`, or a filter clause).
- **FR-003**: For a valid PATH that includes a field expression, the compiled
  representation MUST expose the field number, its optional index selector
  (numeric, `$LAST`, or `*`), and its optional component and subcomponent
  numbers.
- **FR-004**: For a valid PATH whose segment index is a filter clause, the
  compiled representation MUST expose the filter's target (field number, and
  optional component/subcomponent numbers), its operator, and the ordered
  list of one or more OR'd literal values.
- **FR-005**: For a valid PATH using the hierarchy operator
  (`SEGMENT_EXPR " -> " CHILD_PATH`), the compiled representation MUST expose
  the parent segment expression and the child path as distinct fields, and
  MUST NOT attempt to resolve or evaluate the navigation itself (that
  remains spec `008`'s responsibility).
- **FR-006**: The parser MUST NOT panic or crash the process for any input
  string, well-formed or not (Constitution Principle III); a malformed PATH
  always produces a returned error value, never an unhandled exception.
- **FR-007**: The parser MUST NOT return a compiled representation alongside
  an error — a given parse call MUST produce exactly one of a compiled
  result or an error, never both, never neither.
- **FR-008**: Every rejected PATH's error MUST identify both which grammar
  rule was violated and the position (byte or character offset) within the
  PATH string where the violation was detected.
- **FR-009**: Parsing MUST be a pure function of the PATH string alone — it
  MUST NOT read or depend on any HL7 message, scanner output (spec `005`),
  or hierarchy profile; the same PATH string always compiles to the same
  representation.
- **FR-010**: The compiled representation MUST be reusable across an
  unbounded number of downstream evaluations without re-parsing the
  original string.
- **FR-011**: Parsing MUST NOT copy substrings of the PATH string where a
  borrowed reference suffices (segment names, filter values, etc.),
  consistent with Constitution Principle II.
- **FR-012**: This spec MUST add new conformance vectors to
  `fixtures/vectors/path/` (extending, not replacing, spec `001`'s set)
  covering the combinations identified in Edge Cases as not already present:
  a filter with multiple OR'd values, a filter with a subcomponent, optional
  whitespace around a filter operator, and rejection of a multi-hop
  hierarchy chain.

### Key Entities

- **Compiled PATH**: The parser's output for one valid PATH string — a
  structured representation of segment name, segment index selector, field/
  component/subcomponent numbers, an optional filter clause, and, for
  hierarchy-mode PATHs, the distinct parent and child path expressions.
  Reusable across any number of later evaluations against different
  messages; contains no reference to message data.
- **Filter Clause**: The parsed form of a `SEG_IDX`'s `@field.comp.subcomp
  OPERATOR 'value||value...'` alternative — target field/component/
  subcomponent numbers, one of the six comparison operators, and an ordered
  list of one or more literal values.
- **Parse Error**: A specific, located failure reported instead of a
  Compiled PATH when the input string does not match the grammar, per
  Constitution Principle III — identifies the violated grammar rule and the
  offset at which the violation was detected.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% (11/11) of `fixtures/vectors/path/valid.json` entries
  parse successfully into a compiled representation, with zero errors.
- **SC-002**: 100% (6/6) of `fixtures/vectors/path/invalid.json` entries are
  rejected with a located, rule-specific syntax error, with zero panics and
  zero partial results.
- **SC-003**: Every new edge case this spec identifies (OR'd filter values,
  filter subcomponent, whitespace-tolerant operator, multi-hop-hierarchy
  rejection) is covered by its own new conformance vector (FR-012), verified
  with zero discrepancies against `contracts/path-grammar.md`.
- **SC-004**: A compiled PATH's memory footprint does not grow with the
  number of times it is subsequently reused for evaluation — parsing cost is
  paid exactly once per distinct PATH string, confirmed by a test or
  benchmark comparable to spec `005`'s allocation-counting methodology.

## Assumptions

- This spec covers the PATH parser only: PATH string in, compiled
  representation or syntax error out. It does not evaluate a compiled PATH
  against message offsets (spec `007`) or resolve hierarchy navigation (spec
  `008`) — those specs consume this one's compiled representation as their
  contract, the same relationship spec `005`'s `scanner-api.md` documents
  for the scanner.
- Per `contracts/path-grammar.md`'s Non-Goals, `CHILD_PATH` is single-hop
  only in the grammar as currently defined. Multi-hop chaining (e.g. `ORC[1]
  -> OBR[1] -> OBX-5`) is a proposed future Backward-Compatible Addition
  owned by a later spec ("likely `008`"), gated on a performance claim not
  yet made — this spec parses only today's single-hop form and rejects
  chains of more than one `->` as a syntax error.
- Numeric range or semantic validation of `SEG_IDX`/`FIELD_IDX` values (e.g.
  whether `0` is a meaningful occurrence index) is out of scope, per
  `contracts/path-grammar.md`'s Non-Goals — any syntactically valid `NUMBER`
  is accepted at parse time; matching real data is spec `007`'s concern.
- The compiled representation's exact in-memory types/shape are a
  `/speckit-plan` implementation decision; this spec defines only the
  information it must expose (FR-002 through FR-005), not concrete Rust
  types or an AST layout.
- This spec targets the Rust core (`crates/core`), consistent with spec
  `005`. It does not modify or retire the current Scala engine's
  regex-based parser, which remains in place until a later migration phase
  addresses it.
- No dependency on spec `005`'s scanner exists or is introduced by this
  spec (FR-009) — the PATH parser and message scanner are independent
  components that both feed spec `007`'s query executor.
