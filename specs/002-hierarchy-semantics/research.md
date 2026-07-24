# Phase 0 Research: Hierarchy-Mode Semantics Specification

All items below were genuinely open (no reasonable default existed in the spec, or the
spec explicitly deferred the decision here) and are resolved so Phase 1 design has no
outstanding unknowns.

## Source used

Unlike spec `001` (which had to clone the Scala repo to a scratch location), a
persistent local checkout of the actual parity-target repo already existed at
`/Users/m/projects/personal/HL7-PET` (remote `github.com/mscaldas2012/HL7-PET`, not a
submodule or dependency of this repo). The three files actually read for this research:

- `src/main/scala/gov/cdc/hl7/HL7HierarchyParser.scala` — builds the tree.
- `src/main/scala/gov/cdc/hl7/HL7ParseUtils.scala` — evaluates `->` against the tree.
- `src/main/scala/gov/cdc/hl7/StructureValidator.scala` — confirms the cardinality/
  navigation boundary (FR-002).
- `src/main/resources/COVID_ORC.json` (a real profile) and
  `src/test/scala/HL7ParserUtilsTest.scala` (real usage examples), used to ground
  concrete conformance-vector scenarios.

Nothing from this checkout is copied into this repo in bulk; only the specific
behaviors below are extracted and re-expressed as prose/vectors, consistent with
`ROADMAP.md`'s "no hard dependency" convention.

## Decision 1: What "parent" actually means during tree construction

**Decision**: A child segment line attaches to the *nearest enclosing ancestor* (by
current parse position, walking outward from the most recently opened node) whose
profile `children` map contains that segment name — not simply "the most recent
occurrence of any single designated parent type." Concretely, `HL7HierarchyParser`
maintains a stack of (profile-pointer, output-node) pairs; for each incoming segment it
first tries the *current* pointer's children; on failure it pops one level and retries
against the parent, repeating until a match is found (attaching there and discarding the
popped levels) or the stack is exhausted (the segment is silently dropped from the tree
and parser state is restored exactly, via backup stacks, as if the line had never been
seen). Segment-type cardinality (`SegmentConfig.cardinality`) is never read during this
process — it is inert metadata at construction time.

**Rationale**: This is what the actual code does (`HL7HierarchyParser.scala:47-93`), and
it is the compatibility floor spec.md FR-001 requires. It is meaningfully more precise
than `SPEC.md` §3.3's one-line summary ("builds a tree... using the profile's
`segmentDefinition`"), which doesn't describe the backtracking-to-nearest-ancestor
behavior at all — an implementer working from `SPEC.md` prose alone would very
plausibly build "child scope ends at the next sibling occurrence of the *same* parent
type" (a much simpler, and behaviorally different, rule) instead.

**Alternatives considered**:
- Document only `SPEC.md`'s simplified prose: rejected — fails FR-001's "matching the
  current Scala engine's hierarchy-mode behavior" requirement the moment a message has
  more than trivial nesting (e.g. `COVID_ORC.json`'s `OBR -> OBX -> NTE` and
  `OBR -> SPM -> OBX` branches, both real, both in the actual bundled profile).
- Re-derive the rule purely from `SPEC.md` + guessing: rejected for the same reason
  spec `001`'s Decision 2 rejected it — risks encoding a stale/simplified description as
  verified fact.

## Decision 2: Whether Rust hierarchy navigation requires an explicit profile (spec FR-004)

**Decision**: **Yes, a declarative profile (`segmentDefinition`-equivalent) remains
required** to determine legal parent/child pairings — but the Rust core MUST NOT
eagerly materialize a full in-memory tree object over the whole message the way
`HL7HierarchyParser.parseMessageHierarchy` does today. Containment (Decision 1's
nearest-enclosing-ancestor rule) is instead computed lazily, per query, bounded to the
line range actually needed to answer that one `->` expression.

**Rationale**: HL7 v2 messages are a flat, ordered list of 3-letter segment codes with
no self-declared nesting markers — nothing in the message itself says "these OBX lines
belong to this OBR." The nearest-enclosing-ancestor rule (Decision 1) is *inherently*
profile-driven: without a declarative `children` map per segment type, there is no way
to know that an `NTE` following an `OBX` belongs to the `OBX`, not back up to the `OBR`
or `SPM`. This is exactly Constitution Principle V's point (declarative,
versionable profiles over hard-coded per-message-type logic) and matches the Migration
Plan's explicit non-goal of inferring structure. However, requiring a profile does not
require the *eager, whole-message tree* the Scala engine builds — that specific
implementation choice is precisely what Migration Plan Phase 3 says to avoid
("contextual navigation without building a full tree," "evaluate hierarchy lazily
during query execution"), and Constitution Principle II makes it non-negotiable. A
single `->` query only ever needs: (a) the profile's children map to know what counts
as "child of X" at the parsed depth, and (b) one bounded forward scan from the selected
parent occurrence's line to the next line that is *not* a descendant per the same
nearest-enclosing-ancestor rule. Both are computable without building or holding a
tree for the rest of the message.

**Alternatives considered**:
- No profile at all, infer structure from message shape alone: rejected — there is no
  structural signal in raw HL7 v2 to infer nesting from (unlike, say, XML/JSON). Two
  different valid profiles can assign the same flat sequence of segments to different
  trees (e.g. whether a repeating `NTE` belongs to the preceding `OBX` or the enclosing
  `OBR` is a profile decision, not something the bytes alone determine).
  Principle V and the actual Scala precedent both support requiring a profile.
  Reversing this later, if ever justified, would need to go through spec `002`'s own
  amendment process, not be assumed away here.
  Requiring a profile for hierarchy mode is also already what static mode's absence
  documents by contrast (`SPEC.md` §3.3: static mode is profile-free specifically
  *because* it does not support `->`).
- Eagerly build and cache the full tree once (today's Scala approach, just ported
  as-is to Rust): rejected — directly violates Constitution Principle II and the
  Migration Plan's explicit Phase 3 framing; would also silently regress on messages
  where only one or two `->` queries are ever run against a large message (paying
  full-tree cost for a single-hop answer).

## Decision 3: Real, previously undocumented behavior — child index is unfiltered, un-rebased, and untested

**Decision**: Document, as an explicit Known/Documented Limitation of the current
engine (compatibility floor per Principle I, not a bug this spec silently fixes), the
*exact* mechanism `getChildrenValues` uses for a numeric child index (e.g. `OBX[2]`),
which is more surprising than "just" a cross-parent concatenation issue:

1. All children of every matched parent occurrence are first concatenated into one flat
   list, **regardless of segment type** (`children ++= hl7Hierarhy.children` inside
   `recursiveAction`, across every matched parent — `HL7HierarchyParser`/
   `HL7ParseUtils.scala:74-90`).
2. That combined, *still-mixed-type* list is `zipWithIndex`'d
   (`HL7ParseUtils.scala:104`), producing 0-based positions `i` over **every** child
   regardless of its segment type.
3. Only *then*, per position, does the code check `cseg == it.substring(0, 3)` (is
   this position's segment actually of the requested child type) and, if so,
   `childMatch = csegIdx.toInt == i` (`HL7ParseUtils.scala:106-111`) — comparing the
   user's literal index string directly against that raw, mixed-type, 0-based position,
   with **no `-1` adjustment**.

Two compounding consequences: (a) if the matched parent has children of more than one
segment type (the normal case — e.g. `OBR`'s children per `COVID_ORC.json` include
`OBX`, `NTE`, `TQ1`, `TQ2`, `CTD`, `FT1`, `CTI`, `SPM`, interleaved in document order),
`OBX[N]`'s `N` is compared against a position that may not even land on an `OBX` at
all, so a numeric child index can silently match nothing even when the Nth *OBX*
child clearly exists; and (b) unlike every other indexed position in the engine — the
parent side of `->` and every flat-path `SEG_IDX`/`FIELD_IDX`, which all convert a
1-based user index via `segments.slice(segmentIndex - 1, segmentIndex)`
(`HL7StaticParser.scala:138-154`) — this specific child-index comparison has no such
conversion, so even in the single-parent, single-child-type-only case it is off by one
against the 1-based convention documented in `SPEC.md` §3.1 and verified by this
engine's own flat-path tests (`HL7StaticParserUtilsTest.scala:209-211` confirms
`OBX[1]` selects the 1st flat occurrence). No test anywhere in the Scala test suite
(`HL7ParserUtilsTest.scala`, `HL7StaticParserUtilsTest.scala`) exercises a numeric
index on the *child* side of `->` — every hierarchy test either omits the child index
entirely (`OBR[4]->OBX-3.2`) or uses a filter (`@field=value`) instead, so this path
appears to be genuinely untested, not a deliberately verified design choice.

**Rationale**: This is real, verifiable behavior in the source, not a hypothetical edge
case, and it is exactly the kind of "corner found ambiguous/underspecified in `SPEC.md`"
FR-010 requires flagging rather than silently reproducing or silently fixing. Getting
the *precise* mechanism right (not just "cross-parent, unrebased," which understates
it) matters because a future implementer reading only a softened description could
reasonably build a "fixed" version that still doesn't match today's actual output,
defeating the whole point of a compatibility-floor document. It also matters more once
multi-level chaining (Decision 4) is considered, since a naive per-hop reuse of this
same mechanism would compound the surprise at every hop.

**Alternatives considered**:
- Describe only the cross-parent concatenation aspect, omit the type-mixing and
  off-by-one details: rejected after re-reading `HL7ParseUtils.scala` line-by-line —
  understates the actual behavior enough that it would mislead an implementer.
- Silently carry the behavior forward with no comment: rejected — violates FR-010 and
  risks a future implementer "fixing" it inconsistently across Rust/Python/Java
  bindings, creating exactly the Principle IV divergence the constitution warns
  against.
- Treat it as a confirmed bug and redefine the semantics (per-parent-and-per-type
  re-based, 1-based indexing) in this spec: rejected — out of scope per FR-001's
  "compatibility floor" framing, and risky to assert unilaterally given it appears
  untested (the "correct" intended behavior was never pinned down by a test to compare
  against). This is recorded as a finding for spec `008`'s own plan to decide,
  informed by this document, not something to decide unilaterally in a documentation
  spec.

## Decision 4: Whether `->` should support multi-level chaining (spec FR-005)

**Decision**: **Recommend inclusion**, as a Backward-Compatible Addition to spec `001`'s
`CHILD_PATH` production (e.g. `CHILD_PATH ::= SEGMENT_EXPR ["-" FIELD_EXPR] | SEGMENT_EXPR
" -> " CHILD_PATH`, recursive), gated on the following falsifiable performance claim
(SC-005): **each additional hop costs one bounded forward scan over only the previously
narrowed line range, with no re-scan of already-excluded lines and no full-tree
materialization** — i.e. total work across all hops in a single chained query is
`O(message size)` once, the same asymptotic bound as a single-hop query, not
`O(hops × message size)`. This is a direct consequence of Decision 2's lazy,
bounded-scan approach: because containment is already computed as "forward scan from
parent occurrence's line to the next non-descendant line," chaining is just feeding one
hop's narrowed range as the next hop's starting range — no new data structure, no
repeated full-message work.

**Rationale**: The feature request is explicit that performance is the deciding factor,
not desirability alone, and that inclusion must not be the default without a stated
argument (spec FR-005). The underlying containment relationship is *already* inherently
multi-level in the real engine — `COVID_ORC.json`'s actual nesting is four levels deep
(`MSH → OBR → OBX → NTE`, and `MSH → OBR → SPM → OBX`) — so "hierarchy" was never
conceptually single-level; only the *query syntax* (`->`) is currently limited to one
hop, and only because `HL7ParseUtils`'s `CHILDREN_REGEX` splits on a single `->` and the
segment-expression regex on either side doesn't allow a nested `->` (confirmed: because
`.*` is greedy, a path with two or more `->` tokens today, e.g. `ORC[1] -> OBR[1] ->
OBX-5`, splits at the *last* arrow — parent = `"ORC[1] -> OBR[1]"`, child = `"OBX-5"` —
and since the parent side then fails to match the plain segment-expression regex, the
whole query silently returns no results today. This is itself a previously undocumented
behavior worth carrying into the semantics document per FR-010, distinct from Decision
3). Since the lazy, bounded-scan design already recommended in Decision 2 naturally
composes hop-by-hop without extra cost, multi-level chaining is not a separate feature
so much as removing an arbitrary one-hop ceiling from a mechanism that already
generalizes.

**Alternatives considered**:
- Single-hop only, revisit after spec `008` exists to benchmark: this is the fallback
  spec.md explicitly allows (User Story 4) if a performance cost can't be ruled out.
  Rejected as the *primary* recommendation (though recorded here as the documented
  fallback) because the cost argument above doesn't depend on anything spec `008`
  hasn't already committed to (Decision 2's lazy bounded-scan design) — there's no new
  uncertainty multi-level chaining introduces that single-hop lazy navigation doesn't
  already carry. If spec `008`'s actual implementation deviates from Decision 2's
  bounded-scan design for some other reason, this recommendation should be
  re-examined at that point — it is not an unconditional mandate.
- Chain via repeated eager full-tree lookups (mirroring today's Scala approach applied
  N times): rejected outright — this is exactly Decision 2's rejected alternative,
  compounded N times, actively contradicting SC-005's own claim.
- Fix Decision 3's cross-parent index behavior as part of introducing chaining (i.e.
  re-base child indices per-parent once multiple hops exist): explicitly **not**
  decided here — that would be a breaking-change decision belonging to spec `008`'s own
  plan, informed by this document, not smuggled into a documentation spec as a side
  effect of the chaining decision.

## Decision 5: What the static-mode `->` fallback actually does (spec FR-008)

**Decision**: Document precisely — not `SPEC.md` §7's "silently falls back to flat
extraction" phrasing, which is directionally right but imprecise. In static mode
(`buildHierarchy = false`), `getValue` never even inspects `CHILDREN_REGEX`; it passes
the *entire literal string*, arrow and all (e.g. `"OBR[1] -> OBX-5"`), straight to
`HL7StaticParser.getValue`. That string does not match the flat `PATH_REGEX` (which has
no `->` token), so the call returns `None` — an empty result, not an error, and not a
meaningful "extraction of the child segment as if it were a flat path" as the phrase
"falls back to flat extraction" might suggest to a reader. The Rust core SHOULD
preserve the externally observable outcome (empty result, no exception, per
Constitution Principle III) for a `->` expression evaluated without hierarchy mode
enabled, without needing to replicate the *specific* mechanism (a failed regex match)
that produces it in Scala today.

**Rationale**: This is a case where `SPEC.md`'s summary and the actual mechanism
produce the same observable behavior (empty result) but describe it in a way that could
mislead an implementer into building unnecessary fallback-extraction logic that doesn't
actually exist today. Documenting the precise mechanism avoids that.

**Alternatives considered**:
- Repeat `SPEC.md`'s phrasing verbatim: rejected — risks over-implementation of a
  fallback path that isn't real.
- Treat this as a `[NEEDS CLARIFICATION]` discrepancy per FR-011: not applicable here —
  the *observable* result (empty, no error) is not actually contradicted by `SPEC.md`;
  only the *explanatory mechanism* was imprecise, which this decision corrects without
  a factual conflict to escalate.

## Decision 6: Where the formal semantics document and schema live

**Decision**: The semantics document is its own file, `contracts/hierarchy-semantics.md`
(mirroring spec `001`'s `contracts/path-grammar.md`); the vector schema is
`contracts/hierarchy-conformance-vector.schema.json`, extending (via a documented
superset of fields, not a competing format) spec `001`'s
`conformance-vector.schema.json`.

**Rationale**: Consistent with spec `001`'s Decision 5 and the plan template's guidance
that grammars/formal contracts belong in `contracts/`. Keeping the hierarchy schema as
an explicit extension (adding `profile_ref` and reusing `path`/`message_ref`/`expected`
as-is) rather than a from-scratch schema lets spec `003` treat both vector families
uniformly when it promotes them into `fixtures/`.

**Alternatives considered**:
- Fold hierarchy semantics into `data-model.md`: rejected for the same reason spec
  `001` rejected it — spec `008` depends on this document specifically as a build
  contract and should be able to reference it independently.
- Design a wholly new, unrelated vector schema: rejected — needlessly duplicates
  `id`/`message_ref`/`expected` shape that spec `001` already defined and spec `003`
  already expects to consume.
