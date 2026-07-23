# Feature Specification: PATH Grammar Specification & Conformance Vectors

**Feature Branch**: `001-path-grammar-spec`

**Created**: 2026-07-22

**Status**: Draft

**Input**: User description: "Formal PATH grammar specification and conformance test vectors"

## Clarifications

### Session 2026-07-23

- Q: Must source HL7 messages used in conformance vectors be synthetic (no real patient data)? → A: Yes — all conformance-vector messages MUST be synthetic/fabricated test data for library development and testing; no real PHI, even de-identified, is ever included.
- Q: When a conformance vector, verified against the real Scala library (SC-004), contradicts what `SPEC.md` documents, which one wins? → A: Neither by default — escalate every such discrepancy as a `[NEEDS CLARIFICATION]` for a case-by-case human decision; do not silently prefer either source.
- Q: Should this spec define a concrete schema (field names/types) for the conformance vector file format, or leave it for spec `003` to design? → A: Define it concretely now (JSON schema in FR-011), including a stable `id` field per vector for cross-referencing by later specs.
- Q: Should the conformance vector suite include syntactically invalid PATH strings, not just valid ones? → A: Yes, but validity is purely grammatical, independent of whether the referenced segment/field exists: `PID`, `PID-1`, `PID[1]-1`, `PID-3.2` are all valid; `XYZ-99` is also valid (well-formed) and MUST evaluate to zero values, not a parse error; `999[1].1.1` is invalid because `999` isn't a valid segment name and `.` is used where the grammar requires `-`. Invalid vectors cover grammar violations only, never "well-formed but references nothing."

## User Scenarios & Testing *(mandatory)*

This is a documentation/specification deliverable (Migration Plan Phase 1,
Roadmap module 0-999 "Rust Core", spec `001`) rather than a runtime feature.
Its "users" are the people and processes that will build against or verify
against it.

### User Story 1 - Rust core implementer builds the PATH parser from the spec alone (Priority: P1)

An engineer implementing the Rust PATH parser (Roadmap spec `006`) needs an
unambiguous, complete formal grammar for PATH expressions, so they can build
a parser without reverse-engineering the existing Scala source or guessing
at edge-case behavior.

**Why this priority**: This is the entire point of the spec — Constitution
Principle I (Path Contract Stability) requires the PATH grammar to remain
backward compatible across every implementation and language port. Without a
formal, standalone grammar document, that principle is unenforceable — there
would be nothing to check new implementations against except the Scala
source code itself.

**Independent Test**: Give the grammar document (with no access to Scala
source) to someone unfamiliar with HL7-PET and ask them to classify 20
sample PATH strings (mix of valid and invalid) into grammar productions or
"invalid, violates production X." Success = 100% correct classification
using only the document.

**Acceptance Scenarios**:

1. **Given** the grammar document, **When** a reader looks up the production
   for a segment index filter (e.g. `OBX[@3.1='94500-6']-5`), **Then** they
   can find `FILTER` fully defined with no forward reference to an
   undefined term.
2. **Given** the grammar document, **When** a reader encounters `MSH-21[3].1`,
   **Then** they can derive, from the grammar alone, that this addresses the
   third repetition of MSH field 21, component 1.

---

### User Story 2 - Regression-suite author gets canonical conformance vectors (Priority: P1)

The author of Roadmap spec `003` (regression suite) needs a set of canonical
PATH conformance vectors — path string, source message, expected result —
covering every grammar production, so the same vectors can be run against
the Scala baseline, the Rust core, and (later) the Python/Java bindings to
prove parity.

**Why this priority**: Conformance vectors are what make Principle I
checkable in CI rather than just written down. Without them, "the grammar
didn't change" is an assertion nobody can verify mechanically.

**Independent Test**: Feed each conformance vector's PATH string and source
message into the *current Scala library* (external repo,
[mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet)) and confirm
the actual output matches the vector's documented expected result.

**Acceptance Scenarios**:

1. **Given** a conformance vector for `OBR[1] -> OBX-5`, **When** it is run
   against the Scala library in hierarchy mode, **Then** the actual output
   matches the vector's expected result exactly.
2. **Given** the full vector set, **When** each vector's grammar production
   is tallied, **Then** every production in the grammar document is
   exercised by at least one vector.

---

### User Story 3 - Future contributor understands what this document does and does not cover (Priority: P2)

A contributor reading this spec later needs to know where PATH *syntax*
(this spec) ends and hierarchy *semantics* (Roadmap spec `002`) begins, so
they don't duplicate or contradict work across the two specs.

**Why this priority**: Prevents scope drift and duplicated/conflicting
documentation between `001` and `002` as both evolve.

**Independent Test**: Ask a contributor to state, after reading this spec's
Assumptions section, which document owns "what does `->` mean when OBR
repeats." They should correctly name spec `002`, not this one.

**Acceptance Scenarios**:

1. **Given** this document, **When** a reader looks for parent→child
   cardinality rules, **Then** they find a pointer to spec `002` rather than
   an attempted (and possibly inconsistent) explanation here.

---

### Edge Cases

- What happens when a PATH string is syntactically valid but semantically
  meaningless in static mode (e.g. uses `->` without hierarchy mode)? The
  grammar document must state this is a syntax/semantics split: the string
  parses, but evaluation behavior is out of this spec's scope (see FR-002).
- How are the documented Known Limitations from `SPEC.md` §7 (no escaped
  character support, fixed `MSH-1`/`MSH-2` delimiters, single-clause filter
  syntax) reflected in the grammar itself vs. left as prose caveats? The
  grammar must encode every limitation that is actually a syntax constraint
  (e.g. filters supporting only one field+component clause); limitations
  that are evaluation-time behavior (e.g. escape sequences not being
  decoded) are noted as prose, not grammar productions.
- What happens when a conformance vector's expected result depends on
  `removeEmpty` behavior? Each such vector must state which `getValue`
  overload/flag it exercises, since `Option`-returning behavior differs
  between them.
- What happens when a PATH matches multiple segment occurrences *and* each
  occurrence has multiple field repetitions at once (e.g. `OBX-5` across 3
  `OBX` segments, each with 2 repetitions of field 5)? The expected result
  must be documented as fully two-dimensional (3 outer entries × 2 inner
  entries each), not flattened to a single list of 6 values — flattening
  would silently discard which repetitions belong to which segment
  occurrence.
- What happens when a conformance vector, once actually run against the
  real Scala library per SC-004, produces a result that contradicts what
  `SPEC.md` documents? There is no default resolution — this MUST be
  escalated as a `[NEEDS CLARIFICATION]` for a case-by-case human decision
  (see FR-010), never silently resolved in favor of either the
  documentation or the observed runtime behavior.
- What distinguishes a syntactically invalid PATH from a syntactically
  valid PATH that simply matches nothing? Validity is purely grammatical:
  `PID`, `PID-1`, `PID[1]-1`, and `PID-3.2` are all valid regardless of
  whether the referenced segment/field actually exists in a given message
  or profile. `XYZ-99` is syntactically valid (three-letter segment name,
  dash, numeric field) even though no such segment/field is typically
  defined — it MUST evaluate to zero values (FR-004), not a parse error.
  `999[1].1.1` is syntactically invalid: `999` is not a valid 3-letter
  alphabetic segment name, and `.` is used where the grammar requires `-`
  between the segment and field expressions.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The grammar document MUST formally define, in EBNF or
  equivalent formal notation, every production referenced in `SPEC.md` §3.1:
  `PATH`, `SEGMENT_EXPR`, `SEG`, `SEG_IDX`, `FILTER`, `OPERATOR`,
  `FIELD_EXPR`, `FIELD_IDX`, `CHILD_PATH`, plus `field_num`, `comp_num`,
  `subcomp_num`.
- **FR-002**: The grammar document MUST include the `->` (hierarchy child)
  operator as a syntax token (it appears in the `PATH` production) but MUST
  NOT define its evaluation semantics (parent-scoping, cardinality-driven
  tree building) — that is Roadmap spec `002`'s responsibility. This spec
  MUST link to `002` at the point where hierarchy mode is mentioned.
- **FR-003**: A conformance vector suite MUST be produced covering: every
  grammar production at least once, every Known Limitation in `SPEC.md` §7
  that is a syntax-level constraint, every Quick-Start example in
  `SPEC.md` §8, and syntactically invalid PATH strings per FR-012.
- **FR-004**: Each conformance vector MUST specify, in a structured
  (machine-readable) format: the PATH string, a source HL7 message (inline
  or by reference to a shared fixture), the `getValue`/`getFirstValue`
  variant and flags exercised, and the exact expected result. A PATH MUST
  be treated as returning **zero, one, or many values** — never assumed to
  resolve to a single scalar. Results are two-dimensional: one axis for
  matching segment occurrences (e.g. multiple `OBX` segments), one axis for
  field repetitions within a match (e.g. `PID-5[2]`). The vector suite MUST
  include at least one case per axis (a path matching multiple segment
  occurrences, and a path matching multiple field repetitions within one
  segment) so implementations can't collapse either axis by accident.
  `getFirstValue` is the sole scalar-returning exception and MUST be
  labeled as such in its vectors.
- **FR-005**: The grammar document and conformance vectors MUST be
  reviewable by both a human (readable prose + EBNF) and a machine
  (structured vector file conforming to FR-011), since spec `003` will
  consume the vectors programmatically.
- **FR-006**: Any corner of PATH syntax found to be ambiguous or
  underspecified in `SPEC.md` during extraction MUST be flagged explicitly
  in this document as a resolved decision with rationale, not silently
  guessed at.
- **FR-007**: Deliverables from this spec (grammar document, conformance
  vectors) live under `specs/001-path-grammar-spec/`. Promoting the
  conformance vectors into the shared `fixtures/` corpus described in
  `HL7-PET-Rust-Migration-Plan.md` is in scope for spec `003`
  (regression-suite), not this spec.
- **FR-008**: Each conformance vector SHOULD additionally record the
  1-based source line number each expected value was extracted from, even
  though the current Scala `getValue`/`getFirstValue` API does not expose
  it. This is forward-looking metadata only — it does not change this
  spec's grammar or the result shape defined in FR-004 — collected now so a
  future location-aware extraction API (planned, not yet specified) can
  reuse these vectors instead of requiring a second data-collection pass.
- **FR-009**: Every source HL7 message used in a conformance vector MUST be
  synthetic/fabricated test data. Real patient data — including de-identified
  real messages — MUST NOT be used, even if sourced from the external Scala
  repo's own test fixtures. This applies to every conformance vector
  produced by this spec and to any message examples this spec commits under
  its own directory.
- **FR-010**: When verifying a conformance vector against the real Scala
  library (SC-004) reveals a discrepancy with what `SPEC.md` documents,
  neither source MUST be silently trusted. Each such discrepancy MUST be
  raised as a `[NEEDS CLARIFICATION]` item for a case-by-case human
  decision, distinct from the general ambiguity-flagging required by
  FR-006 (this is a factual contradiction discovered during verification,
  not an underspecified corner of the documentation).
- **FR-011**: Conformance vectors MUST be stored as an array of JSON
  records, each conforming to this schema:

  | Field | Type | Required | Description |
  |---|---|---|---|
  | `id` | string | Yes | Stable, unique identifier (e.g. `"path-001"`), used for cross-referencing by later specs (e.g. `1000`, `1001` reusing this spec's line-number metadata) |
  | `path` | string | Yes | The PATH expression under test |
  | `message_ref` | string | Yes | Relative path to the synthetic source HL7 message file (FR-009) |
  | `method` | `"getValue"` \| `"getFirstValue"` | Yes | Which extraction method/variant this vector exercises |
  | `flags` | object | No | Method-specific flags, e.g. `{"removeEmpty": true}` |
  | `expected` | two-dimensional array of strings (or a single string for `getFirstValue`) | Yes | Expected result, per the shape defined in FR-004 |
  | `expected_lines` | array mirroring `expected`'s shape, of 1-based line numbers | No | Per FR-008 |
  | `grammar_productions` | array of strings | Yes | Which grammar production(s) from FR-001 this vector exercises (drives SC-003 coverage tracking) |
  | `known_limitation` | string or null | No | Name of the `SPEC.md` §7 Known Limitation this vector demonstrates, if any |

  File organization (one file per production family vs. one combined file)
  is left to the implementer as long as every record matches this schema.
  For invalid-PATH vectors (FR-012), `expected` MUST be the literal string
  `"INVALID"` instead of a value array.

- **FR-012**: The conformance vector suite MUST also cover syntactically
  invalid PATH strings — strings that violate the grammar defined in
  FR-001 itself (e.g. a non-alphabetic or wrong-length segment name, `.`
  used where `-` is required, an unterminated filter clause) — with at
  least one vector per grammar production where a violation is
  structurally possible. This is strictly narrower than "matches no data":
  a well-formed PATH referencing a nonexistent segment or field (e.g.
  `XYZ-99`) is syntactically **valid** and MUST use FR-004's zero-values
  result, never the `"INVALID"` marker — only actual grammar violations
  (e.g. `999[1].1.1`) qualify as invalid vectors.

### Key Entities

- **PATH Expression**: A string addressing data within an HL7 message
  (segment, optional field/component/subcomponent, optional index/filter,
  optional hierarchy child).
- **Grammar Production**: A named formal rule (e.g. `SEGMENT_EXPR`) defining
  a syntactic category of the PATH language.
- **Conformance Vector**: A tuple of (PATH string, source message,
  extraction variant/flags, expected result) used to verify an
  implementation's behavior against the documented grammar.
- **Filter Clause**: The `@field[.comp] OPERATOR 'value'` syntax used inside
  a `SEG_IDX` to select a segment occurrence by field value.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the Quick-Start examples in `SPEC.md` §8 parse
  successfully against the documented grammar with no undefined
  productions.
- **SC-002**: The grammar document contains zero forward or undefined
  references — every nonterminal used is defined within the document or
  explicitly delegated to spec `002`.
- **SC-003**: At least one conformance vector exists per grammar production,
  per syntax-level Known Limitation from `SPEC.md` §7, and one invalid-PATH
  vector per production where a grammar violation is structurally possible
  (FR-012) — minimum ~25 vectors given the current grammar's production
  count.
- **SC-004**: 100% of conformance vectors, when manually run against the
  external Scala library, either match the documented expected result, or
  have an FR-010 discrepancy escalation opened and resolved before this
  spec is considered complete (i.e. no vector ships unverified or with a
  silently-ignored mismatch).

## Assumptions

- No Rust, Python, or Java code exists yet for this spec — it is a
  documentation/data deliverable consumed by later specs (`002`, `003`,
  `006`).
- `SPEC.md` §3.1 (this repo) is the primary source of truth; the external
  Scala repo ([mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet))
  is consulted only to resolve ambiguities `SPEC.md` doesn't settle, and per
  prior direction nothing is fetched automatically — any message examples
  pulled from there are downloaded once and committed under this spec's
  directory.
- Hierarchy-mode evaluation semantics (what parent-scoping actually does) is
  explicitly out of scope here and owned by Roadmap spec `002` — see
  `contracts/path-grammar.md`'s Non-Goals section for exactly where the
  syntax/semantics line is drawn.
- Turning these vectors into an executable, CI-wired regression suite under
  `fixtures/` is out of scope here and owned by Roadmap spec `003`; that
  spec consumes the JSON format and schema defined in FR-011 as-is rather
  than redesigning it.
