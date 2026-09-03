# Feature Specification: Lazy Hierarchy Navigation

**Feature Branch**: `008-lazy-hierarchy-nav`

**Created**: 2026-09-02

**Status**: Draft

**Input**: User description: "Lazy contextual parent-to-child hierarchy navigation for compiled PATH queries: evaluate CompiledPath.child (the '->' hop) against a message's scanned offsets without materializing a full segment tree, resolving the nearest-enclosing-ancestor semantics documented in spec 002 and building on spec 007's query executor and spec 006's parser output."

## Clarifications

### Session 2026-09-02

- Q: Spec `002` Section B.2 recommended multi-level `->` chaining (e.g. `ORC[1] -> OBR[1] -> OBX-5`), gated on a performance claim, and named this spec as the likely implementer. Should spec `008` include it now, or defer it? → A: Defer — spec `008` ships single-hop `->` navigation only; multi-hop chaining becomes a separate future spec once single-hop is implemented and proven.
- Q: Spec `002` Section A.4 documents a real, apparently-untested Scala bug in child-index resolution on `->`'s right-hand side (unfiltered by segment type, concatenated across matched parents before indexing, no 1-based adjustment). Since spec `008` is the first Rust implementation of this behavior at all, should it reproduce the bug as the compatibility floor, or fix it now as a documented breaking change? → A: Fix now — spec `008`'s child-index resolution is type-filtered, re-based per matching parent, and 1-based, documented as a Breaking Change in `ROADMAP.md`'s Documented Breaking Changes table with a MAJOR version bump (Constitution Principle I).

## User Scenarios & Testing *(mandatory)*

This is a Rust Core / Engine Migration deliverable (Migration Plan Phase 3,
Roadmap module 0-999, spec `008`) — the first spec in this module to produce
hierarchy-aware runtime behavior rather than documentation (`001`, `002`) or a
flat-path-only engine module (`005`-`007`). Its "users" are callers of
`hl7pet-core`'s query API who need `->` (hierarchy) PATHs resolved, and the
regression/perf specs (`003`, `009`) that validate this module against the
Scala baseline.

### User Story 1 - Caller resolves a single-hop hierarchy PATH against a scanned message (Priority: P1)

A caller who already has a `ScanResult` (spec `005`) and a `CompiledPath` with
`child: Some(_)` (spec `006`) needs to get back the actual value(s) the `->`
expression addresses — e.g. `OBR[1] -> OBX-5` against a message with multiple
`OBR` occurrences, each with its own set of child `OBX` segments — without the
executor materializing a full hierarchy tree over the whole message first.

**Why this priority**: This is the feature's entire reason to exist. Spec
`007`'s executor explicitly refuses to resolve `CompiledPath.child`
(`query-api.md`'s precondition: `path.child` MUST be `None`) and the `hl7pet`
dev CLI currently prints a warning and evaluates only the parent segment/field
when a `->` PATH is given. Without this spec, hierarchy PATHs are parseable
(spec `006`) but not executable — the migration's Phase 2 output has no
Phase 3 consumer.

**Independent Test**: Given `fixtures/vectors/hierarchy/basic.json`'s
`hier-001` vector (`OBR[1] -> OBX-3` against `basic-hierarchy.hl7` and
`basic-two-level.json`), call the new hierarchy-resolving entry point and
confirm the returned values match the vector's `expected` exactly.

**Acceptance Scenarios**:

1. **Given** a scanned message with two `OBR` occurrences, each followed by a
   different number of `OBX` children, **When** `OBR[1] -> OBX-5` is executed,
   **Then** only the first `OBR` occurrence's `OBX` children are returned, in
   document order, with the second `OBR`'s children excluded entirely.
2. **Given** a matching parent occurrence with zero children of the requested
   child segment type, **When** the hierarchy PATH is executed, **Then** the
   result is empty (no values) — never an error, never a panic.
3. **Given** the same PATH and message, **When** it is executed twice against
   the same `ScanResult`/`CompiledPath` pair, **Then** both calls return
   identical results and neither call scans lines outside the range bounded
   by the selected parent occurrence(s) and their immediate successors (no
   full-message tree is built to answer a single-hop query).

---

### User Story 2 - Existing hierarchy conformance vectors validate the implementation byte-for-byte (Priority: P1)

The shared regression suite (spec `003`) already carries
`fixtures/vectors/hierarchy/basic.json` and `complex.json` — 10 vectors
authored during spec `002` and verified live against the real Scala library,
covering single-hop scoping, the unrecognized-segment-drop rule, the
static-mode fallback, and the untested child-index limitation (`SPEC.md`
Section A.4). This implementation needs to reproduce every one of those
vectors' documented `expected` output exactly, the same way specs `005`-`007`
each closed the loop against their own fixture families before being
considered done — with one discovered-during-implementation carve-out:
`hier-009` and `hier-010` use a two-hop PATH (`"OBR[1] -> OBX[3] -> NTE-3"`),
which spec `006`'s parser already rejects outright (`MultipleHierarchyHops`)
and which exercises exactly the multi-hop capability User Story 3 defers —
these 2 are excluded from this story's scope, not silently miscounted as
passing (research.md #6).

**Why this priority**: Equal to User Story 1 — a hierarchy executor with no
proof it matches the documented compatibility floor (spec `002`,
Constitution Principle I) is not verifiably correct, regardless of whether it
compiles and runs.

**Independent Test**: Run the 8 single-hop vectors in
`fixtures/vectors/hierarchy/` (10 total, minus `hier-009`/`hier-010`) end-to-
end (scan → parse → hierarchy-execute) and confirm 8/8 match their
documented `expected` value, including the vectors whose `expected` is
`null` (static-mode fallback, zero children).

**Acceptance Scenarios**:

1. **Given** `hier-002` (`buildHierarchy: false`, i.e. no profile supplied),
   **When** the same `->` PATH is executed without hierarchy mode enabled,
   **Then** the result is empty, matching spec `002` Section A.5's documented
   fallback behavior.
2. **Given** every single-hop vector in `fixtures/vectors/hierarchy/basic.json`
   and `complex.json` (8 of the 10 — `hier-009`/`hier-010` excluded per this
   story's scope), **When** each is executed against its `message_ref` and
   `profile_ref`, **Then** all 8 reproduce their `expected`/`expected_lines`
   exactly.

---

### User Story 3 - Multi-level chained navigation is explicitly deferred, not silently dropped (Priority: P3)

Spec `002` Section B.2 recommended extending `->` to support more than one
hop (e.g. `ORC[1] -> OBR[1] -> OBX-5`) as a Backward-Compatible Addition to
`CHILD_PATH`, but gated that recommendation on a falsifiable performance
claim, and `path-grammar.md`'s Non-Goals section named this spec as the
most likely place to land it. Per this spec's own Clarifications, that
recommendation is **deferred**, not implemented here: spec `008` ships
single-hop `->` navigation only, matching spec `006`'s parser, which already
rejects a second `" -> "` at parse time (`ParseErrorKind::MultipleHierarchyHops`).

**Why this priority**: Lowest of the three stories — it produces no runtime
behavior in this spec. Its only job is to make sure spec `002`'s
recommendation is recorded as a deliberate deferral (with a named follow-up
path) rather than silently going unaddressed, so a future reader doesn't
have to re-discover that the recommendation exists.

**Independent Test**: A reader of this spec's Clarifications and Assumptions
sections can determine, without consulting spec `002` or `006` source
directly, that multi-hop chaining was recommended, is not implemented by
spec `008`, and remains available as a future spec once single-hop
navigation (User Stories 1/2) is implemented and validated.

**Acceptance Scenarios**:

1. **Given** spec `008`'s implementation, **When** a caller supplies a PATH
   with two `->` hops, **Then** `hl7pet_core::parse` continues to reject it
   exactly as spec `006` does today (`MultipleHierarchyHops`) — this spec
   introduces no parser or grammar change for multi-hop.
2. **Given** this spec's documentation, **When** a reader looks for whether
   multi-hop chaining was considered, **Then** they find the deferral
   decision and rationale recorded, not silence.

---

### Edge Cases

- What happens when the parent segment selector (`SEG_IDX`) matches zero
  occurrences at all (e.g. `OBR[5]` when only 2 `OBR` segments exist)? Must
  resolve to an empty result — no error — per spec `002` Section A.2's
  "step 2/step 4 yields zero occurrences" rule.
- What happens when a segment type appears in the message that the supplied
  profile does not recognize as a legal child anywhere in the current
  ancestor chain (spec `002` Section A.1, case 4(b))? The line MUST be
  silently excluded from hierarchy navigation while remaining fully visible
  to flat/non-hierarchy queries over the same `ScanResult` — never dropped
  from the underlying scan, never an error.
- What happens when the child segment index (`OBX[2]` on the child side) is
  numeric? Per this spec's Clarifications, the real engine's documented bug
  (spec `002` Section A.4: untested, unfiltered by type, un-rebased) is
  **not** reproduced — spec `008` resolves the index against only
  same-type candidates, re-based per matching parent occurrence, 1-based
  (FR-007), as a documented Breaking Change.
- What happens when `->` is used but no profile is supplied (static/flat
  mode)? Per spec `002` Section A.5, the whole `->` expression must yield an
  empty result, not a partial flat-path evaluation of either side alone.
- What happens when the same `CompiledPath`/`ScanResult` pair is queried
  many times in a loop (e.g. once per row of a batch job)? Per spec `002`'s
  Section B.1 lazy-navigation decision and Constitution Principle II
  (zero-copy/lazy), no per-call cost should scale with total message size
  beyond the single bounded forward scan each call already requires — no
  tree should be built once and reused, nor rebuilt unnecessarily on repeat
  calls with an unchanged profile.
- What happens when a profile itself is structurally malformed (e.g. a cycle
  in `segmentDefinition`'s children)? Per spec `002`'s Edge Cases, this is a
  structural-precondition violation (Constitution Principle III), not a
  silent empty result — this spec's planning phase MUST define the concrete
  error/panic-avoidance behavior, consistent with spec `005`'s `ScanError`
  and spec `007`'s `QueryError` precedent of a located, typed error rather
  than a panic.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `hl7pet-core` MUST provide a way to execute a `CompiledPath`
  (spec `006`) whose `child` field is `Some(_)` against a `ScanResult` (spec
  `005`) and a profile, returning the same two-dimensional (occurrences ×
  repetitions) result shape spec `007`'s `execute()` already returns for
  flat paths — hierarchy scoping is a selection step before that shape is
  produced, not a different shape (per spec `002` Section A.2, step 7).
- **FR-002**: Parent-occurrence selection MUST reuse spec `007`'s existing
  `SEG_IDX` resolution (`Numeric`/`$LAST`/`*`/`FilterClause`) evaluated over
  every occurrence of the parent segment type in the message — not scoped to
  any subtree — matching spec `002` Section A.2 step 2 exactly.
- **FR-003**: For each matching parent occurrence, child-line resolution MUST
  be computed via a single bounded forward scan from that occurrence to the
  next line that is not its descendant under the nearest-enclosing-ancestor
  rule (spec `002` Section A.1) — this module MUST NOT construct a full
  segment hierarchy tree over the whole message at any point, per spec `002`
  Section B.1's decision and Constitution Principle II.
- **FR-004**: Hierarchy navigation MUST require an explicit profile (a
  `segmentDefinition`-equivalent lookup of legal parent/child pairings) to
  determine containment, per spec `002` Section B.1 — the profile is
  consulted purely as a lookup table, never eagerly compiled into a
  whole-tree object.
- **FR-005**: A segment line whose type is not recognized as a legal child
  anywhere in the current ancestor chain (spec `002` Section A.1 case 4(b))
  MUST be excluded from hierarchy results without raising an error, while
  remaining fully queryable via non-hierarchy (flat) execution against the
  same `ScanResult`.
- **FR-006**: When the parent selector matches zero occurrences, or a
  matching parent occurrence has zero qualifying children, the result MUST
  be empty (no values) — never an error, never a panic — consistent with
  spec `007`'s `QueryError` design, which already treats "no data present"
  uniformly regardless of cause.
- **FR-007**: Child-side numeric index behavior is **corrected**, per this
  spec's Clarifications, rather than reproducing spec `002` Section A.4's
  documented Scala bug. A numeric child index (e.g. `OBX[2]`) MUST: (a) be
  filtered to only candidates matching the child segment type (`cseg`)
  before counting position — no cross-type mixing; (b) be re-based
  independently per matching parent occurrence — no cross-parent
  concatenation into one combined list; and (c) use standard 1-based
  indexing, consistent with every other `SEG_IDX`/`FIELD_IDX` in the engine.
  This is a documented Breaking Change (`ROADMAP.md` convention) relative to
  the Scala engine's behavior, requiring the MAJOR version bump and
  migration guide Constitution Principle I mandates — recorded in
  `ROADMAP.md`'s Documented Breaking Changes table.
- **FR-008**: Multi-hop `->` chaining (spec `002` Section B.2's
  recommendation) is **deferred**, per this spec's Clarifications — spec
  `008` MUST NOT extend spec `001`'s `CHILD_PATH` grammar production or spec
  `006`'s `ChildPath` parser type to be recursive. `hl7pet_core::parse` MUST
  continue to reject a second `" -> "` exactly as spec `006` does today
  (`ParseErrorKind::MultipleHierarchyHops`). This spec's documentation MUST
  record the deferral and rationale so spec `002`'s recommendation is not
  silently lost (User Story 3).
- **FR-009**: A `->` expression evaluated with no profile supplied (static
  mode) MUST yield an empty result, matching spec `002` Section A.5's
  documented fallback behavior — the parent and child sides MUST NOT be
  independently evaluated as flat paths.
- **FR-010**: This module MUST be validated against the 8 single-hop vectors
  among the 10 existing `fixtures/vectors/hierarchy/` vectors (spec
  `002`/`003`), reproducing each vector's `expected`/`expected_lines`
  (including the `null`-expected static-mode and unrecognized-segment
  vectors) exactly, before being considered complete. `hier-009`/`hier-010`
  (a two-hop PATH, discovered during implementation to be unparseable under
  spec `006`'s existing grammar and to test exactly the multi-hop capability
  FR-008 defers) are explicitly out of scope for this requirement — excluded
  from the validating test suite with the exclusion documented, not silently
  treated as passing (research.md #6).
- **FR-011**: *(Removed — moot per FR-008's deferral decision. A multi-hop
  conformance vector requirement applies only if/when a future spec
  implements multi-hop chaining.)*
- **FR-012**: A structurally malformed profile (e.g. a cyclical
  `segmentDefinition` children graph) MUST be rejected with a located,
  typed error at the point hierarchy navigation is attempted — never a
  panic, and never a silent empty result masking a precondition violation
  (Constitution Principle III), consistent with spec `005`'s `ScanError` and
  spec `007`'s `QueryError` precedent.
- **FR-013**: Repeated execution of hierarchy queries against the same
  `ScanResult` and profile MUST NOT incur cost that scales with the whole
  message beyond what a single bounded forward scan per call already
  requires — no full-tree structure is built once and cached, since FR-003
  already forbids building one at all.
- **FR-014**: Profile parsing MUST use an established JSON parsing library
  rather than a hand-rolled parser — this spec's planning phase does not
  need to weigh a bespoke-parser option, resolving the profile-representation
  question the Assumptions section previously left fully open. However, the
  resulting `Hierarchy Profile` type MUST be a plain Rust representation,
  decoupled from that library's own types at `hl7pet-core`'s public module
  boundary (no `serde_json::Value`-shaped type, or equivalent, appears in
  this module's public API) — because this crate's ultimate purpose is to be
  consumed through language bindings (Python via `PyO3`, Java via `JNI`/
  `JNA`, Roadmap module 6000-6999, Migration Plan Phase 4-5), and a JSON
  library's own types leaking across that future FFI boundary would
  constrain or complicate those bindings later. Any dependency added for
  this spec MUST also be pure-Rust with no system/C-library build
  requirement, so it does not obstruct cross-compiling this crate for the
  wheel/JAR targets those future bindings will need.

### Key Entities

- **Hierarchy Profile**: The Rust representation of a `segmentDefinition`
  map (segment name → cardinality + children) used purely as a legal-child
  lookup table (FR-004) — a new type this spec introduces, since no Rust
  profile representation exists yet in `hl7pet-core`.
- **Matching Parent Occurrence**: One specific occurrence of the parent
  segment type selected by the `->` expression's `SEG_IDX` (FR-002), scoped
  independently of any tree structure.
- **Bounded Child Scan Range**: The line range, computed lazily per matching
  parent occurrence (FR-003), from that occurrence to the next line that is
  not its descendant per the nearest-enclosing-ancestor rule — the unit of
  work this spec's lazy-navigation approach performs instead of building a
  tree.
- **Child-Index Fix**: The Clarifications-recorded decision (FR-007) that the
  numeric child-index limitation (spec `002` Section A.4) is corrected —
  type-filtered, per-parent re-based, 1-based — as a documented Breaking
  Change, not preserved as-is.
- **Multi-Hop Deferral**: The Clarifications-recorded decision (FR-008) that
  `->` chaining beyond one hop is deferred past this spec, not implemented.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 8/8 single-hop vectors among the 10 existing
  `fixtures/vectors/hierarchy/` vectors pass end-to-end (scan → parse →
  hierarchy-execute) with output matching their documented
  `expected`/`expected_lines` exactly (`hier-009`/`hier-010`'s two-hop PATH
  is out of scope, FR-010).
- **SC-002**: For a message and profile of any size, a single-hop hierarchy
  query's work is bounded by the size of the matching parent occurrence(s)'
  scoped line ranges, not the whole message — demonstrated by a dedicated
  test analogous to spec `005`'s allocation-counting precedent, confirming
  no full-tree structure is ever constructed.
- **SC-003**: The child-index fix (FR-007) is implemented exactly as
  specified (type-filtered, per-parent re-based, 1-based) and recorded in
  `ROADMAP.md`'s Documented Breaking Changes table with a MAJOR version
  bump and migration guide before this spec is marked complete. (Both
  FR-007 and the multi-hop decision, FR-008, are already resolved by this
  spec's Clarifications — fix, and defer, respectively — so nothing here is
  left open for planning to decide.)
- **SC-004**: A malformed profile (invalid JSON, or JSON that doesn't match
  `segmentDefinition`'s expected shape) never causes a panic — confirmed by a
  dedicated panic-safety test, mirroring spec `006`'s pathological-input
  precedent. (A cyclical `segmentDefinition` — this criterion's original
  illustrative example — turned out to be unrepresentable in this shape's
  plain nested-JSON grammar, confirmed against the real Scala source during
  planning, research.md #2; a segment type legally repeating at multiple
  tree positions, discovered during implementation to be normal, common
  profile data rather than malformed, is handled correctly, not rejected —
  research.md #2's corrected design.)
- **SC-005**: *(Removed — moot per FR-008's deferral decision. Applies only
  if/when a future spec implements multi-hop chaining, at which point spec
  `002` Section B.2's performance claim must still be validated before that
  spec ships it.)*

## Assumptions

- Specs `005` (scanner), `006` (parser), and `007` (query execution) are
  already implemented and are consumed, not modified — per this spec's
  Clarifications (multi-hop deferred), spec `006`'s `ChildPath` type stays
  non-recursive; no grammar or parser change is in scope here.
- `fixtures/vectors/hierarchy/` (spec `002`/`003`, 10 vectors, 8 of which are
  in scope per FR-010) and `fixtures/profiles/{basic-two-level,deep-nested}.json`
  are the primary conformance target; no new Scala verification is needed for
  behavior already covered by those vectors, since spec `002` already
  verified them live against the real Scala engine. `deep-nested.json`
  legitimately places `OBX` and `NTE` as legal children at more than one
  nesting depth — discovered during implementation to require a design
  correction (research.md #2), not something this spec's design could assume
  away as rare or invalid.
- The `hl7pet` dev CLI's current hierarchy warning
  (`crates/cli/src/main.rs`: "hierarchy PATH is not evaluated yet (spec
  008)") is a convenience artifact, not a roadmap deliverable in its own
  right (per that file's own header comment); updating it to actually
  evaluate hierarchy PATHs is a natural follow-up once this spec lands but
  is not itself a success criterion here.
- `hl7pet-core` has shipped as zero-runtime-deps through specs `005`-`007`;
  this spec breaks that property deliberately by adding an established JSON
  parsing dependency for profile parsing (FR-014) rather than hand-rolling
  one, on the explicit condition that the dependency stays pure-Rust and
  never surfaces through this module's public API — preserving the crate's
  suitability for the Python/Java bindings that are this project's ultimate
  target (Roadmap 6000-6999), which zero-runtime-deps was never a goal in
  its own right for, just a proxy for "doesn't complicate those bindings."
  The planning phase picks the specific crate (e.g. `serde_json` or
  equivalent) and confirms it meets FR-014's pure-Rust, no-system-library
  constraint.
- Spec `009` (core-perf-validation) is the spec responsible for benchmarking
  this module against the spec `004` Scala baseline at a whole-suite level;
  this spec's own SC-002/SC-005 benchmarks are scoped to proving this
  spec's own lazy-navigation and multi-hop performance claims, not a full
  parity benchmark against Scala.
