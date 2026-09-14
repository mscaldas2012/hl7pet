# Feature Specification: Arrow Integration for PySpark & PyArrow

**Feature Branch**: `6002-arrow-integration`

**Created**: 2026-09-14

**Status**: Draft

**Input**: User description: "Create the necessary code to be able to use the hl7pet library with PySpark and PyArrow. Build a full Jupyter notebook to demonstrate its use. Provide (at minimum) two extraction mechanisms: a single-PATH mechanism and a mechanism that accepts an array of PATHs and returns all results at once, computed in one pass per message. JSON-template/profile-based whole-message transform (a third, richer mechanism inspired by the lib-bumblebee library) is out of scope for this spec, deferred to a later one."

## User Scenarios & Testing *(mandatory)*

This is a Language Bindings deliverable (Roadmap module 6000-6999, spec
`6002`) and realizes Phase 4 ("Arrow Integration") of
`HL7-PET-Rust-Migration-Plan.md`, building on top of the plain Python
binding delivered by spec `6000`. Its users are data engineers who already
hold HL7 v2 messages as a column in a PySpark DataFrame or a PyArrow
Table/RecordBatch (e.g. loaded from files, a message queue, or a warehouse
table) and want to extract structured field values from every message
without leaving the columnar/distributed pipeline they're already in — no
row-by-row Python loop, no manual Arrow packing.

Per the module's existing single-call-vs-batched convention (established by
spec `6000`'s `get_value`/`get_values`), this spec ships two extraction
mechanisms side by side:

1. **Single-PATH extraction** over a column of messages — analogous to
   `get_value`, but operating on a whole column at once.
2. **Multi-PATH (array-of-paths) extraction** over a column of messages —
   analogous to `get_values`, computing every requested PATH from a single
   pass per message rather than re-scanning the message once per PATH.

A third mechanism — transforming a whole message into a JSON document via a
template or a segment-hierarchy profile (comparable to what the `lib-bumblebee`
library does today) — is explicitly out of scope for this spec and deferred to
a future one, once its design is discussed on its own.

### User Story 1 - Extract one field across a column of HL7 messages (Priority: P1)

A data engineer has a PySpark DataFrame (or a PyArrow Table) with one column
holding raw HL7 v2 message text, one message per row. They apply hl7pet's
single-PATH extraction to that column with one PATH expression (e.g.
`PID-5.1`) and get back a new column holding that field's value for every
row, with no per-row Python code and no manual conversion between Arrow and
plain Python types.

**Why this priority**: This is the minimum capability that makes hl7pet
usable inside a PySpark/PyArrow pipeline at all. Every other story in this
spec builds on it.

**Independent Test**: Build a small PyArrow Table (or PySpark DataFrame) from
messages in the shared `fixtures/messages/` corpus, run single-PATH
extraction with a PATH that has a corresponding `fixtures/vectors/path/`
vector, and confirm the output column matches that vector's `expected` value
row-for-row.

**Acceptance Scenarios**:

1. **Given** a column of HL7 messages and one non-hierarchy PATH expression,
   **When** the data engineer runs single-PATH extraction, **Then** the
   returned column has one entry per input row, each matching what the
   existing plain Python `get_value` returns for that row's message and
   PATH.
2. **Given** a column of HL7 messages and one hierarchy-mode PATH (using the
   `->` operator) plus a segment-hierarchy profile, **When** the data
   engineer runs single-PATH extraction, **Then** the returned column
   matches what the existing plain Python `get_value_hierarchy` returns for
   each row's message.
3. **Given** a PATH that matches nothing in a given row's message, **When**
   extraction runs, **Then** that row's output is the column's idiomatic
   "no value" representation (never a Python-level exception raised out of
   the whole call).
4. **Given** the same column of messages, **When** the data engineer runs
   single-PATH extraction inside a PySpark job and, separately, against a
   standalone PyArrow Table outside of Spark, **Then** both produce
   identical results for the same input.

---

### User Story 2 - Extract multiple fields across a column of HL7 messages in one pass (Priority: P1)

A data engineer wants several fields out of each message (e.g. patient ID,
observation date, and every OBX-5 result) without scanning each message once
per field. They apply hl7pet's multi-PATH extraction to the message column
with a list of PATH expressions and get back one result per PATH per row,
computed from a single per-message pass.

**Why this priority**: This is the primary performance-sensitive path for
realistic pipelines, which typically extract many fields per message, not
one. It mirrors spec `6000`'s `get_values` batching rationale, extended to a
whole column instead of a single message.

**Independent Test**: Using the same fixtures-derived Table, run multi-PATH
extraction with a list of PATHs that each have their own
`fixtures/vectors/path/` vector, and confirm every PATH's output matches its
own vector's `expected` value, row-for-row, in one call.

**Acceptance Scenarios**:

1. **Given** a column of HL7 messages and a list of PATH expressions
   (non-hierarchy, hierarchy, or a mix), **When** the data engineer runs
   multi-PATH extraction, **Then** the result exposes one set of values per
   PATH per row, each matching what calling single-PATH extraction
   separately for that PATH would have returned.
2. **Given** a list of PATHs, **When** multi-PATH extraction runs, **Then**
   each input message is scanned once regardless of how many PATHs were
   requested, not once per PATH.
3. **Given** an empty list of PATHs, **When** multi-PATH extraction is
   invoked, **Then** the call is rejected as invalid input rather than
   silently returning an empty result.

---

### User Story 3 - Use the extraction mechanisms directly from PySpark (Priority: P2)

A data engineer working in a PySpark job (not just a standalone PyArrow
script) registers hl7pet's single-PATH and multi-PATH extraction as
DataFrame-column operations and calls them with `.withColumn(...)` /
`.select(...)`, the same way they'd use any other PySpark column function,
without hand-writing Arrow-to-Spark glue code themselves.

**Why this priority**: PySpark is explicitly named as a target consumer
(alongside standalone PyArrow, Pandas, Polars, Databricks, DuckDB per the
migration plan's Phase 4 goals). Without a ready-to-import PySpark entry
point, every adopter would have to hand-roll the same wiring.

**Independent Test**: In a local PySpark session, build a DataFrame from the
fixtures corpus, apply the single-PATH and multi-PATH operations via
DataFrame API calls, `.collect()` the result, and confirm it matches the
non-Spark PyArrow-only result for the same input.

**Acceptance Scenarios**:

1. **Given** a PySpark DataFrame with a message column, **When** the data
   engineer applies single-PATH extraction via the DataFrame API, **Then** a
   new column appears with the extracted values, usable in further
   DataFrame operations (filtering, joining, writing out).
2. **Given** a PySpark DataFrame with a message column, **When** the data
   engineer applies multi-PATH extraction via the DataFrame API, **Then**
   the requested fields become accessible as DataFrame columns.

---

### User Story 4 - Evaluate hl7pet's Arrow support via a runnable demo (Priority: P3)

A prospective adopter (or a maintainer validating the feature) opens a
single Jupyter notebook that loads sample HL7 messages into both a
standalone PyArrow Table and a PySpark DataFrame, exercises single-PATH
extraction, multi-PATH extraction, and hierarchy-mode PATHs on both, and
shows a simple before/after comparison against calling the existing
row-by-row plain Python binding — without needing to read any spec or
source file first.

**Why this priority**: This is how the capability gets demonstrated and
adopted; it doesn't add new extraction behavior itself; it's a consumer of
Stories 1-3.

**Independent Test**: Run the notebook top-to-bottom in a clean environment
with the project's declared dependencies installed and confirm every cell
executes without error and produces the output it narrates.

**Acceptance Scenarios**:

1. **Given** a clean environment with the project's dependencies installed,
   **When** the notebook is run top-to-bottom, **Then** every cell completes
   without error.
2. **Given** the notebook's comparison cell, **When** it runs, **Then** it
   shows the row-by-row plain-Python baseline and the columnar Arrow-based
   result agreeing on the same data.

---

### Edge Cases

- What happens when the message column contains a null/missing entry for a
  given row? (That row's output across every requested PATH should also be
  null/missing, never crash the whole call.)
- What happens when the message column contains a structurally malformed HL7
  message (e.g. missing MSH segment) for some rows but not others? See
  FR-010 (clarified below) for the resolved per-row error-handling
  behavior.
- What happens when a hierarchy-mode PATH is requested without supplying a
  profile? (Rejected as invalid input, consistent with the existing plain
  Python binding's `get_value_hierarchy` requiring a profile.)
- What happens when the same PATH string appears twice in a multi-PATH
  request? (Both occurrences are honored independently; the result exposes
  a value for each requested PATH position/name, not de-duplicated away.)
- What happens when the input column is empty (zero rows)? (Returns an
  empty result of the correct shape, not an error.)
- What happens when a PATH expression itself is syntactically invalid
  (e.g. malformed grammar)? (Rejected up front as invalid input, before any
  per-row processing — this is a property of the PATH itself, not of any
  particular message, so it does not need per-row handling.)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a single-PATH extraction mechanism
  that, given a column of HL7 messages and one PATH expression, returns one
  result per input row.
- **FR-002**: The system MUST provide a multi-PATH extraction mechanism
  that, given a column of HL7 messages and a list of PATH expressions,
  returns one result per PATH per input row, computed from a single pass
  per message regardless of how many PATHs were requested.
- **FR-003**: Both extraction mechanisms MUST support hierarchy-mode PATHs
  (the `->` operator) given a segment-hierarchy profile, in addition to
  non-hierarchy PATHs, with results matching the existing plain Python
  binding's `get_value_hierarchy` for the same message/PATH/profile.
- **FR-004**: Both extraction mechanisms MUST produce results that exactly
  match the existing plain Python binding's `get_value`/`get_values` for
  the same message(s) and PATH(s) — this feature changes how extraction is
  invoked (columnar vs. per-message), not what it returns.
- **FR-005**: Both extraction mechanisms MUST be usable directly against a
  standalone PyArrow Table/RecordBatch, without requiring a PySpark session.
- **FR-006**: Both extraction mechanisms MUST also be directly usable as
  PySpark DataFrame column operations (e.g. via `.withColumn`/`.select`),
  without the caller having to hand-write Arrow marshaling code.
- **FR-007**: A row whose message value is null/missing MUST produce a
  null/missing result for every requested PATH on that row, without
  aborting the call for other rows.
- **FR-008**: The system MUST reject an empty PATH list passed to multi-PATH
  extraction as invalid input, distinct from "no matches found." (An
  arbitrary number of PATHs, one or more, MUST be supported.)
- **FR-009**: The system MUST reject a syntactically invalid PATH expression
  (in either mechanism) up front, before per-row processing begins, mirroring
  the existing plain Python binding's parse-time PATH validation.
- **FR-010**: When a given row's message is structurally malformed (fails to
  scan — e.g. missing/truncated MSH), that row's result for every requested
  PATH MUST surface as that row's own null/error indicator, and processing
  of the remaining rows in the same call MUST continue rather than the
  entire call raising and discarding every row's results. The call itself
  MUST still distinguish "this row failed to scan" from "this row scanned
  fine but the PATH matched nothing," per Constitution Principle III's
  existing error/no-data distinction, without requiring the caller to
  catch a raised exception to get partial results back.
- **FR-011**: The project MUST ship one runnable Jupyter notebook
  demonstrating: loading HL7 messages into both a PyArrow Table and a
  PySpark DataFrame, single-PATH extraction, multi-PATH extraction,
  hierarchy-mode extraction via both mechanisms, and a comparison against
  the existing row-by-row plain Python binding.
- **FR-012**: The existing plain Python binding's public API (`get_value`,
  `get_values`, `get_value_hierarchy`, and the `_located` variants) MUST
  remain unchanged by this feature, per the Backward-Compatible Additions
  convention — this spec adds new, additive capability alongside it.
- **FR-013**: JSON-template/profile-based whole-message-to-JSON
  transformation (the `lib-bumblebee`-style capability) is explicitly OUT of
  scope for this spec.

### Key Entities

- **Message Column**: An ordered collection of raw HL7 v2 message strings
  (one per row), the shared input to both extraction mechanisms; may contain
  null/missing entries.
- **PATH Expression**: The existing hl7pet PATH syntax identifying a field
  (or, with `->`, a hierarchy-navigated field) to extract; unchanged from
  the existing binding.
- **PATH List**: An ordered, non-empty collection of PATH Expressions
  supplied to multi-PATH extraction; duplicates are permitted and each is
  honored independently.
- **Segment-Hierarchy Profile**: The existing `segmentDefinition`-shaped
  profile data already consumed by `get_value_hierarchy`, required whenever
  any PATH in a request uses the `->` operator.
- **Extraction Result**: The per-row, per-PATH output of either mechanism —
  a value, an explicit "no match" indicator, or an explicit "row failed to
  scan" indicator (FR-010), never conflated with one another.
- **Demo Notebook**: The runnable artifact from FR-011, evaluated as part of
  this feature's own acceptance rather than as separate documentation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A data engineer can go from "column of raw HL7 messages" to
  "column of extracted field values" in a single function call, for both
  the single-PATH and multi-PATH mechanisms, with no per-row Python loop
  and no manual Arrow type conversion written by the caller.
- **SC-002**: Extracting 10 fields from the same set of messages via
  multi-PATH extraction does not cost meaningfully more than extracting 1
  field via single-PATH extraction on the same messages (single pass per
  message, not one pass per requested field).
- **SC-003**: 100% of the shared fixtures corpus's `path` and `hierarchy`
  vectors, when run through either columnar extraction mechanism, produce
  results identical to the existing plain Python binding's results for the
  same vectors.
- **SC-004**: Both mechanisms produce identical results whether invoked
  against a standalone PyArrow Table or from within a PySpark DataFrame
  pipeline, for the same input data.
- **SC-005**: A new adopter can run the demo notebook top-to-bottom in a
  clean environment and see every one of Stories 1-3's capabilities working
  on real sample data, with no cell failing and no undocumented manual step.
- **SC-006**: A batch containing a mix of well-formed and structurally
  malformed messages completes in one call, with well-formed rows'
  results unaffected by the malformed rows present elsewhere in the same
  batch.

## Assumptions

- PyArrow and (for Stories 3-4) PySpark are treated as the consuming
  environment's own dependencies, not bundled as required dependencies of
  `hl7pet-core` itself — consistent with the existing dependency policy
  that `hl7pet-core` stays pure-Rust with nothing FFI/columnar-specific
  leaking through its own public surface.
- A single segment-hierarchy profile applies to an entire extraction call
  (i.e. to every row and every hierarchy-mode PATH in that call), matching
  the existing plain Python binding's `get_value_hierarchy(message, path,
  profile)` shape — not a per-row/per-message profile. Per-row profiles are
  not a known use case today and are left out of scope unless a real need
  surfaces later.
- Multi-PATH extraction's result is organized as one named output per
  requested PATH (so a caller can address each field by its PATH string),
  rather than a single opaque blob the caller must parse further.
- The demo notebook runs against a local/standalone PySpark session (no
  cluster setup, no external data source) — it demonstrates correctness and
  usage, not distributed-scale performance.
- Escape-sequence decoding (spec `1001`) and located (line-numbered)
  extraction (specs `1000`/`011`) are available to this feature the same way
  they're already available to the existing plain Python binding, since
  both mechanisms are required (FR-004) to match that binding's output
  exactly; this spec does not re-decide either behavior.
