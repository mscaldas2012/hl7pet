# Research: PATH Parser

All items below were resolvable from this repo's own governing documents
(`HL7-PET-Rust-Migration-Plan.md`, `.specify/memory/constitution.md`, prior specs
`001`-`005`) — no external unknowns required a `[NEEDS CLARIFICATION]` marker in
plan.md's Technical Context.

## 1. No parser-combinator dependency

**Decision**: Hand-written recursive-descent parser / state machine over the PATH
string's bytes, with zero new dependencies (`hl7pet-core` stays runtime-dependency-free,
continuing spec `005`'s policy).

**Rationale**: `HL7-PET-Rust-Migration-Plan.md` names this exact deliverable a "small
hand-written parser/state machine," and spec `005`'s research.md #2 already flagged
that "the actual PATH grammar... does warrant a real parser" as this spec's scope,
distinct from the scanner's simpler linear delimiter walk. The grammar itself
(`specs/001-path-grammar-spec/contracts/path-grammar.md`) is small and LL(1)-friendly —
every production is disambiguated by its next 1-2 characters (`[`, `-`, `.`, `@`, `->`)
— so a hand-written recursive-descent parser is not meaningfully harder to write
correctly than wiring up `nom`/`pest`, and avoids taking on a dependency's own parsing
behavior inside the component Principle II holds to the strictest allocation
discipline.

**Alternatives considered**:
- `nom` — rejected: adds a runtime dependency for a grammar small enough that a
  hand-written parser is comparably simple, and breaks the "zero runtime deps"
  convention spec `005` established for this crate.
- `pest` (grammar-file-driven) — rejected: same dependency concern, plus its
  generated-code approach makes borrowed-slice output (FR-011) less direct to control
  than hand-written code that decides exactly when to slice vs. copy.

## 2. New-vector expected-value verification methodology

**Decision**: The 4 new vectors this spec adds to `fixtures/vectors/path/` (FR-012)
MUST have their `expected`/`expected_lines` fields computed by actually running the
expression against the real Scala library dependency spec `004` already wired up
(`gov.cdc:hl7-pet_2.13:1.2.11`, Maven Central, no vendored source), the same
verification discipline spec `001` used for its original 17 vectors — not hand-derived
from grammar semantics alone, even where the derivation looks obvious (e.g. an OR'd
filter matching two `OBX` occurrences).

**Rationale**: Spec `001`'s own SC-004 required every conformance vector be "verified
against the real Scala library... with zero discrepancies," and `ROADMAP.md`'s
Constitution-adjacent convention throughout this migration is "never guess, always
verify." Reusing the same Maven Central dependency spec `004` already established costs
nothing new (no vendored source, no build-time fetch beyond what spec `004` already
does) and keeps this spec's additions held to the identical bar as the vectors they
extend, rather than a weaker "looks right" standard just because this spec itself
doesn't evaluate against messages.

**Alternatives considered**: Hand-deriving expected values from the grammar's
documented semantics (e.g. `FILTER`'s `VALUE { "||" VALUE }` OR-list, already described
in `contracts/path-grammar.md`) — rejected: this spec's own parser only needs
accept/reject truth to test itself (SC-001/SC-002 check parse success/failure, not
extracted values), but the vectors it adds live in the *shared* `fixtures/` corpus and
will also be consumed by spec `007` (query execution) to check extracted values match —
an unverified guess there would silently seed a wrong ground truth for a spec that
hasn't been planned yet.

## 3. Compiled representation shape: flat struct with an optional child, not a recursive tree

**Decision**: `CompiledPath` is a single struct holding one `SegmentExpr`, one optional
`FieldExpr`, and one optional `ChildPath` (itself a `SegmentExpr` + optional
`FieldExpr`, not a recursive `CompiledPath`) — mirroring the grammar's actual shape
(`PATH ::= SEGMENT_EXPR [FIELD_EXPR] | SEGMENT_EXPR -> CHILD_PATH`, and `CHILD_PATH`
has the same two-part shape one level deep, per `contracts/path-grammar.md`).

**Rationale**: The grammar does not define `CHILD_PATH` recursively today — spec `001`'s
Non-Goals explicitly says multi-hop chaining is a *proposed future* addition owned by a
later spec, not current syntax. Modeling `child` as `Option<ChildPath>` (a fixed,
non-recursive type) rather than `Option<Box<CompiledPath>>` makes "only one hop is
representable" a property the type system enforces, not just a parser-time check that
could silently drift from the type if the parser changes later.

**Alternatives considered**: `Option<Box<CompiledPath>>` (self-referential, supporting
arbitrary chain depth) — rejected for now: it would make the type capable of
representing something the current grammar doesn't allow, moving the "single-hop only"
enforcement entirely into parser logic instead of partly into the type itself; revisit
if/when spec `008` actually lands the recursive `CHILD_PATH` grammar addition
`contracts/path-grammar.md` anticipates.

## 4. Filter value list representation

**Decision**: `FilterClause.values` is a `Vec<&str>` with a parser-enforced invariant of
at least one element (never constructed empty) — no dedicated `NonEmptyVec` type.

**Rationale**: The grammar's `VALUE { "||" VALUE }` always yields at least one value by
construction (there's no valid syntax for zero values), so the invariant is naturally
guaranteed by the only code path that builds a `FilterClause`. Introducing a
`NonEmptyVec`-style wrapper type for a single internal field, with no external crate
already in use for it, would add ceremony without a caller-visible correctness gain the
plain `Vec` doesn't already have here.

**Alternatives considered**: `(String, Vec<String>)` (first value separate from the
rest) — rejected: makes iteration over "all values" awkward for callers (spec `007`)
with no benefit, since the OR semantics treat all values identically regardless of
position.

## 5. How SC-004 ("parse cost paid once, reuse doesn't re-parse") is verified without an evaluator

**Decision**: SC-004 is satisfied by construction and verified by a unit test that
constructs one `CompiledPath` and passes shared (`&CompiledPath`) references to
multiple simulated call sites — not a throughput benchmark. `CompiledPath` exposes no
method that could trigger re-parsing (there is no `evaluate()` on it in this spec's
scope; that arrives with spec `007`), so "reuse without re-parsing" is structurally true
once the type has no such method, and the test exists to pin that invariant against
future accidental regressions (e.g. someone later adding a convenience method that
silently re-parses internally).

**Rationale**: A real allocation/throughput comparison needs an evaluator to actually
exercise reuse against multiple messages, which doesn't exist until spec `007` — exactly
the deferral spec `005`'s plan.md already established for its own SC-004 (allocation
count), pushing full baseline comparison to spec `009`. Testing the structural
invariant now (no re-parse path exists) is the strongest claim this spec alone can make
and matches the "verified against real library" ethos in spirit without inventing a
premature evaluator.

**Alternatives considered**: A criterion-based micro-benchmark comparing "parse once,
reuse N times" vs. "parse N times" — rejected as premature: with no `evaluate()` to call
N times yet, the benchmark would only be timing `Clone`/pointer-copy overhead of
`CompiledPath` itself, not the actual reuse this spec's claim is about.

## 6. Parse error position units

**Decision**: `ParseError.offset` is a byte offset into the PATH string (`usize`,
0-based), matching spec `005`'s `ScanError.offset` convention for message offsets —
not a `(line, column)` pair (PATH strings are always single-line) and not a character
count (would require UTF-8-aware indexing for no benefit, since valid PATH syntax is
ASCII-only per every terminal in the grammar).

**Rationale**: Consistency with the sibling `ScanError` type's offset convention
(`specs/005-message-scanner/contracts/scanner-api.md`) keeps error-handling code in
`crates/core` uniform; PATH strings have no line structure to report, and every
grammar terminal (`SEG`, digits, operators, `@`, `'`, `->`) is ASCII, so byte offset and
character offset always coincide — no distinction needs to be drawn.

**Alternatives considered**: 1-based offset (rejected: `ScanError` and idiomatic Rust
slicing are both 0-based; switching PATH errors to 1-based would be the one
inconsistency in the crate).
