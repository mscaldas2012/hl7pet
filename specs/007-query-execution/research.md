# Phase 0 Research: Query Execution

No `[NEEDS CLARIFICATION]` markers remain in spec.md's Technical Context (there is no
Technical Context in spec.md itself — spec.md is implementation-free by design; this
document resolves the *design* decisions plan.md's own Technical Context section
needed, working from spec.md's FRs and the existing `hl7pet-core` crate specs `005`/
`006` already built). Each entry below: Decision / Rationale / Alternatives considered.

## 1. Single execution path producing both `getValue`- and `getFirstValue`-shaped output

**Decision**: One core function, `execute(scan, path) -> Result<Vec<Vec<&'a str>>, QueryError>`,
walks each matching segment occurrence exactly once and returns the outer/inner
structure the existing conformance vectors already use for `getValue` (outer = matched
segment occurrences in message order, inner = field repetitions matched within each).
`getFirstValue`'s shape (`Option<&'a str>`) is a pure, allocation-free derivation —
`result.first().and_then(|reps| reps.first()).copied()` — never a second walk over the
message.

**Rationale**: `fixtures/schemas/conformance-vector.schema.json` (spec `001`) already
defines exactly this two-dimensional shape for `getValue` vectors and a scalar/`null`
shape for `getFirstValue` ones — the vector format was designed anticipating this
spec, not invented here. Computing both from one walk satisfies SC-004 (at most one
pass over matched content) and FR-013 (no copying) directly, and avoids maintaining
two independently-written extraction algorithms that could silently drift apart
(the exact bug class Principle I exists to prevent).

**Alternatives considered**: Two independent public functions (`get_value`,
`get_first_value`) each re-walking the message — rejected: doubles the surface area
to keep in sync for no behavioral benefit, and risks the two silently disagreeing on
edge cases (out-of-range, filter-no-match) that this spec's FR-009 requires to be
handled identically regardless of which shape a caller asks for.

## 2. Mapping FR-009's outcomes onto Rust's `Result`/empty-collection idioms

**Decision**:
- **(a)** One or more values found → `Ok(vec)` with at least one entry.
- **(b)** The requested segment name has zero occurrences in the message at all → `Ok(vec![])` (an empty outer vec) — never `Err`.
- **(c)** An explicit `Numeric`/`$LAST` segment-occurrence or field-repetition index is out of range for the occurrences actually present → `Ok(vec![])` — **not** an error (revised; see Verification below).
- **(d)** A filter clause matches zero candidate occurrences → `Ok(vec![])` — same representation as (b)/(c).
- **(e)** A requested field/component/subcomponent number is beyond what a matched occurrence contains → `Ok(...)` with an empty string (`""`) at that position, not an error and not an omitted entry.
- The **only** genuine `Err(QueryError::NonNumericComparison { operator })` case is FR-008: an ordering operator (`>`, `>=`, `<`, `<=`) applied to an operand that does not parse as a number.

**Verification (supersedes this decision's original draft)**: The first draft of this
decision reasoned from Constitution Principle III's own worked example — "an
out-of-range field index in `splitFields`" — being named as a legitimate error case,
and concluded (c) should be `Err`. That reasoning was checked against the real Scala
library (`gov.cdc:hl7-pet_2.13:1.2.11`, spec `004`'s Maven Central dependency) before
any Rust code was written, per this migration's standing policy (spec `006`'s
research.md #2/tasks.md T016) of confirming behavior empirically rather than from
prose alone. The check: `HL7StaticParser.getValue("OBX[5]-5", ...)` against
`multi-obx.hl7` (only 3 `OBX` occurrences) returns `None`; `getValue("OBX-5[5]", ...)`
against `multi-repetition.hl7` (field 5 has only 2 repetitions) also returns `None` —
neither throws. This directly falsifies the original (c) = `Err` design: whatever
`splitFields` itself does internally, `getValue`/`getFirstValue` — the methods this
executor's compatibility bar (FR-010) actually targets — catch or absorb that case
into an ordinary no-match result. (b)/(c)/(d) are therefore all the same "no data"
outcome Principle III's rationale text is actually about ("no data present" vs.
"message is malformed" — not further subdivided within "no data present"), and Rust's
idiomatic mechanism for that is an empty collection, not an `Err`. `path-zero-values-nonexistent` (`XYZ-99`, `expected: null`) already confirmed (b); the same
verification run additionally confirmed a segment-only PATH's shape (decision #5) and
a multi-match filter's shape (decision #6's `path-filter-multi-match`) directly
against the real engine, reused as this spec's new vectors' ground truth.

By contrast, `HL7StaticParser.getValue("OBX[@5>'100']-5", ...)` against
`multi-obx.hl7` (field 5 holds text like `"Positive"`, not a number) throws an
uncaught `NumberFormatException` — confirming FR-008's premise that the *existing*
engine does not handle this gracefully. FR-008 deliberately requires this executor to
surface that as a typed `Err`, not reproduce the crash (research.md #4 elaborates:
this is the same kind of graceful-rejection improvement spec `001`'s grammar-
tightening already established as this migration's convention when the alternative is
an uncaught exception with nothing meaningful for FR-010 to byte-for-byte match).

**Alternatives considered**: A single unified `QueryOutcome` enum with one variant per
FR-009 letter (including distinct `SegmentAbsent`/`FilterNoMatch`/`OutOfRange`
variants) — rejected as over-fitting the spec's prose to the type signature: it would
force every caller to match on a distinction (why zero came back) that Scala's
existing, compatibility-bar `getValue`/`getFirstValue` callers never had, and that the
verification above confirms the real engine does not surface either. Keeping (c) as a
distinct `Err` variant (the original draft) was rejected once verification falsified
its premise — building it anyway would violate FR-010's byte-for-byte compatibility
bar for no benefit, since no vector in the corpus would ever need to reach it.

## 3. Shared navigation between direct extraction and filter evaluation

**Decision**: One internal function, `extract_subvalue(segment_content, delimiters,
field_num, component, subcomponent) -> &str`, implements the field→component→
subcomponent splitting (empty string when a level is requested but absent, per
decision #2(e)). Both the top-level `FieldExpr` path (FR-003) and `FilterClause`
evaluation (FR-007) call this same function — a filter's target is just another
`FieldExpr`-shaped navigation applied to a candidate occurrence before the
match/no-match decision, not a separately implemented traversal.

**Rationale**: FR-007 and FR-003 describe the identical splitting operation applied
to two different purposes (produce vs. compare); spec.md's own Assumptions section
states this explicitly ("there is one navigation path, not two independently
maintained ones"). A single implementation also means a future fix to splitting logic
(e.g. an edge case in empty-component handling) cannot fix direct extraction while
leaving filter evaluation silently stale, or vice versa.

**Alternatives considered**: Duplicating the split logic inline within filter
evaluation for a marginal branch-prediction win — rejected; this is a Rust library
call, not a hot-path macro-expansion concern, and the correctness risk of divergence
outweighs any plausible performance gain (unverified, and this spec has no throughput
target — Technical Context).

## 4. Numeric comparison for ordering filter operators

**Decision**: For `>`, `>=`, `<`, `<=`, both the filter's target sub-value (from
decision #3's `extract_subvalue`) and each of the filter's OR'd literal values are
parsed via `str::parse::<f64>()`. If both parse successfully, compare numerically. If
either fails to parse, return `Err(QueryError::NonNumericComparison { .. })` rather
than treating the comparison as `false` or panicking (FR-008, spec.md User Story 3
Acceptance Scenario 4).

**Rationale**: `f64` parsing is a standard-library-only, dependency-free way to
recognize integer and decimal numeric literals (matching the kind of values HL7
numeric fields carry, e.g. `OBX-2` numeric observation values) without introducing a
number-parsing crate into a still-zero-runtime-dependency crate (consistent with
specs `005`/`006`'s own "zero new runtime deps" precedent). No existing conformance
vector exercises an ordering operator today (all existing filter vectors use `=`), so
there is no prior Scala-verified numeric-format convention this spec must match yet;
FR-014's new non-numeric-comparison vector only needs to prove the *rejection* path,
not pin down fractional/scientific-notation edge cases. Verified against the real
Scala library (decision #2's Verification note): `HL7StaticParser.getValue` for
`OBX[@5>'100']-5` against `multi-obx.hl7` (where field 5 holds non-numeric text like
`"Positive"`) throws an uncaught `NumberFormatException` rather than returning `None`
or `false` — the existing engine has no graceful behavior here at all to match
byte-for-byte, which is precisely why FR-008 requires this executor to invent one
(a typed `Err`) rather than reproduce a crash.

**Alternatives considered**: Byte-wise lexicographic string comparison for ordering
operators — rejected: it produces wrong answers for the common case ("10" < "9"
lexicographically), and FR-008 explicitly requires numeric comparison when both sides
parse as numbers.

## 5. Segment-only PATH (no `FieldExpr`) output shape

**Decision**: When `CompiledPath.field` is `None`, each matched segment occurrence
contributes a single-element inner vector containing that occurrence's full raw
content (`ScanResult`'s segment span, unsplit) — e.g. a `PID` query against a message
with one `PID` segment yields `vec![vec![<full PID line>]]`.

**Rationale**: FR-002 requires "the full raw content of each matched segment
occurrence, unsplit by any delimiter"; wrapping it in a one-element inner vector keeps
the output shape uniform with the field-expression case (outer = occurrences, inner =
repetitions-or-one-value) rather than introducing a second, differently-shaped return
type for this one case. No existing conformance vector exercises a segment-only PATH
(every current `fixtures/vectors/path/valid.json` entry includes a field expression),
so this spec's FR-014 vector additions include one (plan.md Project Structure).
Verified directly against the real Scala library (decision #2's Verification note):
`HL7StaticParser.getValue("PID", ...)` against `messages/baseline.hl7` returns
`Some([["PID|1||SYN00001^^^FAKEFACILITY^MR||SYNTHETIC^PATIENT^A||19800101|M"]])` — a
single occurrence, single-element inner array holding the full raw segment line,
exactly this decision's shape.

**Alternatives considered**: A separate `Vec<&'a str>` (one dimension, not two) for
the segment-only case — rejected: forces callers (and this spec's own test harness)
to branch on whether a `CompiledPath` had a field expression before knowing which
shape to expect, when the existing vector schema already treats `getValue`'s output
as uniformly two-dimensional regardless of PATH shape.

## 6. New conformance vectors this spec adds (FR-014)

Six new entries in `fixtures/vectors/path/valid.json` (no schema change — the
existing `conformance-vector.schema.json` from spec `001` already fits), covering
exactly the execution-time cases not already exercised by the 14 existing vectors. All
six were computed by running the actual PATH against the real Scala library
(`gov.cdc:hl7-pet_2.13:1.2.11`, spec `004`'s Maven Central dependency,
`HL7StaticParser.getValue`/`getFirstValue`) — the same verification policy spec `006`'s
tasks.md T016 used — not hand-derived, and this is also what surfaced decision #2's
revision (the two "out-of-range" vectors were originally expected to record an error,
until verification showed the real engine returns no match):

| id | `path` | `message_ref` | `method` | Verified `expected` |
|---|---|---|---|---|
| `path-segment-only` | `PID` | `messages/baseline.hl7` | `getValue` | `[["PID\|1\|\|SYN00001^^^FAKEFACILITY^MR\|\|SYNTHETIC^PATIENT^A\|\|19800101\|M"]]` (decision #5) |
| `path-segidx-out-of-range` | `OBX[5]-5` | `messages/multi-obx.hl7` (only 3 `OBX` occurrences) | `getValue` | `null` — no match, not an error (decision #2) |
| `path-fieldidx-out-of-range` | `OBX-5[5]` | `messages/multi-repetition.hl7` (field 5 has only 2 repetitions) | `getValue` | `null` — no match, not an error (decision #2) |
| `path-filter-no-match` | `OBX[@3.1='NO-SUCH-CODE']-5` | `messages/multi-obx.hl7` | `getValue` | `null` |
| `path-filter-multi-match` | `OBX[@11='F']-5` | `messages/multi-obx.hl7` (all 3 `OBX` share observation-result-status `F` in field 11) | `getValue` | `[["Positive"], ["Negative"], ["Equivocal"]]` (message order); `getFirstValue` → `"Positive"` |
| `path-filter-nonnumeric-ordering` | `OBX[@5>'100']-5` | `messages/multi-obx.hl7` (field 5 holds non-numeric text) | `getValue` | throws `NumberFormatException` on the real engine (decision #4) — this executor's vector instead records the `QueryError::NonNumericComparison` sentinel (tasks.md T017), since there is no crash "value" to reproduce byte-for-byte |

All six reuse existing `fixtures/messages/*.hl7` files already referenced by other
`path` vectors rather than introducing new synthetic messages — each scenario turned
out to be reachable against data already present in those files, discovered by
inspecting their content directly rather than assumed in advance.

## 7. Occurrence numbering convention

**Decision**: A segment's 1-based occurrence index counts only among segments
sharing its name, in message order (e.g. the 2nd `OBX` regardless of what other
segment types appear between occurrences) — not a position among all segments in the
message. A field's 1-based repetition index counts `~`-delimited slices left to
right within the targeted field.

**Rationale**: This is not a new decision this spec introduces — it is the existing,
already-Scala-verified convention the current `fixtures/vectors/path/valid.json`
vectors already encode (`path-segidx-number`'s `PID[1]-5`, `path-fieldidx-specific`'s
`OBX-5[2]`), and spec `006`'s `SegIndex`/`FieldIndex` types carry no information that
would support any other numbering scheme. Restated here only so `data-model.md`'s
resolution algorithm has an explicit, citable rule rather than leaving it implicit.

**Alternatives considered**: None — this is confirmed by existing fixture data, not
an open design choice.
