# Research: Message Scanner

All items below were resolvable from this repo's own governing documents
(`HL7-PET-Rust-Migration-Plan.md`, `.specify/memory/constitution.md`, `SPEC.md`, prior
specs `001`-`004`) — no external unknowns required a [NEEDS CLARIFICATION] marker in
plan.md's Technical Context.

## 1. Rust edition and MSRV

**Decision**: Rust stable channel, edition 2021. No Minimum Supported Rust Version is
pinned beyond "current stable."

**Rationale**: The constitution's Performance & Portability Standards only require the
Rust core to "build on stable Rust" — it does not name a minimum version, and this is
the first spec to touch Rust tooling at all, so there is no existing CI/toolchain
config to match. Edition 2021 is the current stable default for `cargo new` and has no
feature gaps relevant to this spec's scope (no `let-else`, GATs, or other
edition-2024-only syntax needed).

**Alternatives considered**: Pinning an explicit MSRV (e.g. via `rust-version` in
`Cargo.toml`) was considered, but rejected for this spec — premature without a CI
matrix or a downstream consumer (Python/Java bindings, Phase 5) whose own minimum
requirements would drive the choice. Revisit once Phase 5 bindings define their build
requirements, or sooner if a specific stable feature becomes load-bearing.

## 2. Dependency policy for `hl7pet-core`

**Decision**: Zero runtime dependencies — `hl7pet-core` links only `std`.
`serde`/`serde_json` are dev-dependencies only, used exclusively by the integration
test that deserializes `fixtures/vectors/scanner/*.json`.

**Rationale**: `HL7-PET-Rust-Migration-Plan.md`'s Repository Layout explicitly
describes `crates/core` as "zero FFI deps" so its performance can't be hidden behind
FFI marshaling in its own benchmarks; extending that to "zero deps, period" for this
first, foundational spec keeps the crate trivially auditable and avoids taking on a
dependency's transitive allocation behavior inside a component whose entire job is
allocation discipline (SC-004). A parser-combinator crate (`nom`, `pest`) was
considered for the scan loop but is unnecessary: this is a linear single-pass byte
scan for delimiter/segment boundaries, not a grammar (the actual PATH grammar, which
does warrant a real parser, is spec `006`'s scope, and the Migration Plan itself calls
for a "small hand-written parser/state machine" there — the same hand-written ethos
applies even more directly to this simpler scan).

**Alternatives considered**:
- `nom`/`pest` — rejected: no grammar to parse here, only linear delimiter scanning;
  would add a dependency for no expressiveness this spec needs.
- `memchr` (fast byte search) — plausible future optimization, deliberately deferred:
  this spec's success criteria (SC-004) are about allocation *count*, not raw
  throughput, and introducing it now without a benchmark target to justify it would be
  premature. Revisit in spec `009` if profiling shows the naive scan is a bottleneck.

## 3. Delimiter-occurrence storage shape

**Decision**: One flat `Vec<DelimiterOccurrence>` for the whole scan (all segments
combined), plus one `Vec<SegmentSpan>` for segment boundaries — two total heap
allocations per successful scan, both sized once up front from an initial counting
pass, not grown incrementally per delimiter found.

**Rationale**: Spec.md SC-004 requires scan allocation *count* to vary only with
segment count, not field/component/repetition count. A design that allocated a
`Vec<usize>` *per segment* for that segment's delimiter offsets would still scale
allocation count with segment count (technically compliant), but a single
message-wide `Vec` is strictly simpler, has fewer allocations in absolute terms (2
regardless of segment count), and avoids the nested-Vec shape that most resembles the
per-field object model Principle II forbids. Sizing both `Vec`s from an initial pass
(count segments and delimiters, then allocate once) avoids the reallocation-on-growth
pattern of repeated `push`, keeping the "single pass" character of FR-001 honest even
though it technically means two passes over the bytes (one counting, one recording) —
this is discussed further in item 6 below.

**Alternatives considered**: Per-segment `Vec<usize>` (rejected: more total
allocations, more code, no benefit over the flat design). A fixed-size array sized to
message length (rejected: wastes memory proportional to message length rather than
delimiter count, and still needs a fallback for pathological inputs).

## 4. Which characters count as tracked "delimiter occurrences"

**Decision**: All five MSH-1/MSH-2-declared characters are tracked as occurrences:
field separator, component separator, repetition separator, escape character, and
subcomponent separator (`DelimiterKind` in data-model.md has five variants).

**Rationale**: Resolves an apparent scope gap between spec.md's FR-001 (which
describes "field/component/repetition/subcomponent delimiter occurrence" without
naming escape) and User Story 1's Acceptance Scenario 2 (which explicitly lists
"field-separator, component-separator, repetition-separator, escape-character, and
subcomponent-separator occurrences" as what a downstream caller can ask for). The
Acceptance Scenario is the more specific, testable statement of the contract, and
tracking the escape character's raw positions costs nothing extra in this spec's
single-pass design (it's classified by the same byte-comparison switch as the other
four) — while explicitly *not* decoding escape sequences (FR-009) is preserved,
since recording where the escape character byte appears is a location fact, not an
interpretation of the value that follows it.

**Alternatives considered**: Tracking only the three structural separators
(field/component/repetition) plus subcomponent, omitting escape — rejected as
contradicting User Story 1's explicit acceptance scenario, and it would leave spec
`1001` (escape-sequence decoding) with no offset data to build on later, working
against `ROADMAP.md`'s "Backward-Compatible Additions" convention.

## 5. Segment name storage

**Decision**: `SegmentSpan` stores only `start`/`end` byte offsets. Segment name is
never stored separately — it is always derived on demand as
`&message[span.start..span.start + 3]`, a borrowed slice.

**Rationale**: Storing a segment name as an owned `String` (or even a fixed `[u8; 3]`
copy) would be exactly the kind of small-but-per-segment allocation/copy Principle II
warns against when it's avoidable, and it is avoidable here — the name is always the
first three bytes of the span and the underlying message outlives the `ScanResult`
that borrows it (data-model.md's lifetime design).

**Alternatives considered**: A `[u8; 3]` copy (rejected: unnecessary since the source
`&str` is guaranteed to outlive `ScanResult` — there's no borrow-checker reason to
copy, only convenience, and convenience isn't a listed goal here).

## 6. "Single pass" vs. a counting pre-pass

**Decision**: FR-001's "single pass" requirement is interpreted as: the scanner does
not build any full object model or copy field text, and the *conceptual* algorithm is
a single left-to-right walk of the message. The reference implementation MAY use an
internal two-pass structure (count, then fill pre-sized `Vec`s) purely as an allocation
optimization, since this produces *fewer* total allocations than a naive single-pass
`push`-based `Vec` (which reallocates and copies on growth) — the two-pass version is
still O(message length) time and touches each byte a small constant number of times,
never proportional to field/component count separately.

**Rationale**: SC-004 (allocation count independent of field count) is the load-bearing,
testable requirement; "single pass" in FR-001 is best read as ruling out the
Scala-style full materialization (splitting into a `Vec<Vec<String>>` object model),
not as a literal ban on reading the byte stream twice. A `push`-based single literal
pass would still satisfy FR-001's literal words but risks *more* reallocation-driven
allocations under growth, working against SC-004. Where the two requirements are in
tension, this spec resolves it in SC-004's favor since that is the constitutionally
load-bearing principle (II).

**Alternatives considered**: Strict single-pass with a growable `Vec` (`Vec::new()` +
`push`) — rejected as the literal-but-weaker reading; accepted only as a fallback if
implementation finds the two-pass approach materially more complex than its allocation
benefit justifies (a call left to `/speckit-tasks`/implementation, not re-litigated
here).

## 7. Segment-name validity check for FR-006's "unrecognized segment name" error

**Decision**: A segment name is considered recognizable if it is exactly 3 bytes,
alphabetic-led (first character `A`-`Z`), matching spec `001`'s already-tightened `SEG`
grammar rule (`ROADMAP.md`'s Documented Breaking Changes table: "`SEG` requires an
alphabetic first character"). No check against a profile's declared segment list is
performed — that remains Validation-module (2000-2999) responsibility per spec.md's
Assumptions.

**Rationale**: Reusing spec `001`'s already-settled, already-conformance-vector-backed
rule avoids inventing a second, possibly inconsistent definition of "valid segment
name" in the same codebase, and keeps this spec's error condition narrowly scoped to
what MSH-parsing structurally requires (a decodable segment stream), not full semantic
validation.

**Alternatives considered**: Full HL7 segment name enumeration (rejected: out of
scope, brittle against new/custom Z-segments, and explicitly deferred to the
Validation module by spec.md's own Assumptions).

## 8. New vector family schema shape

**Decision**: `fixtures/schemas/scanner-conformance-vector.schema.json` follows the
same top-level pattern as spec `001`/`002`'s vector schemas (`id`, `message_ref`, plus
an `expected` shape) but replaces the PATH-specific `path`/`method`/`grammar_productions`
fields with scanner-specific ones: `expected_delimiters` (the resolved `DelimiterSet`)
and `expected_segments` (array of `{start, end}`) for success vectors, or
`expected_error` (`{kind, offset}`) for malformed-MSH vectors — mutually exclusive per
vector, mirroring how spec `001`'s schema makes `expected: "INVALID"` mutually
exclusive with `expected_lines`. Full schema is in
[contracts/scanner-conformance-vector.schema.json](contracts/scanner-conformance-vector.schema.json).

**Rationale**: Reuses the established `id`/`message_ref` conventions (so existing
corpus-wide uniqueness/reference-resolution checks in
`fixtures/scripts/validate_corpus.py` need no scanner-specific special-casing beyond
schema registration) while representing what the scanner actually produces — offsets
and an optional structural error — rather than forcing scanner vectors into the
PATH-vector's `path`/`expected` value shape, which doesn't apply here.

**Alternatives considered**: Reusing spec `001`'s schema as-is with `path` left empty —
rejected: `additionalProperties: false` and several PATH-specific required fields
(`method`, `grammar_productions`) don't map onto scanner semantics; forcing the fit
would produce vectors that lie about what they're testing.
