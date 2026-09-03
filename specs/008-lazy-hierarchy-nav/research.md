# Research: Lazy Hierarchy Navigation

Companion to [plan.md](plan.md). Each decision below resolves a design question
spec.md left to this planning phase, or documents a fact discovered while verifying
this spec's design against the real Scala engine — both `HL7HierarchyParser.scala`
and `HL7ParseUtils.scala` (`mscaldas2012/hl7-pet`, the same source spec `002`'s own
`research.md` traced), consulted from a local checkout at
`/Users/m/projects/personal/HL7-PET/src/main/scala/gov/cdc/hl7/`.

## Decision 1: The bounded, per-occurrence forward scan (spec FR-003, Constitution Principle II)

**Decision**: For one matching parent occurrence, direct-child resolution replicates
`HL7HierarchyParser.parseMessageHierarchy`'s nearest-enclosing-ancestor algorithm
(`HL7HierarchyParser.scala:38-93`), but scoped to a **local stack seeded with only the
parent occurrence's own profile node** — never the full ancestor chain from `MSH`,
and never any state from earlier in the document. This works because of a structural
property of the real algorithm: whenever an occurrence of a given segment type is
successfully attached to the tree at all (not dropped per case 4(b)), it is *always*
attached at that type's one canonical position in the profile — the algorithm's
popping loop (`HL7HierarchyParser.scala:80-87`) always continues until it finds
*some* legal match, and since a given segment type occupies exactly one node in an
unambiguous profile (Decision 2), that eventual match is always the same node
regardless of how much document history preceded it. So the *top* of the real
algorithm's stack at the moment a parent occurrence is attached is always
`[..parent's static ancestor chain.., parent_node]` — a value this spec can compute
directly from the profile alone, with no message walk required to reconstruct it.

Concretely, per matching parent occurrence:

1. Look up the parent segment type's node in the profile (`O(1)`, precomputed at
   `HierarchyProfile` construction) and its **static ancestor chain** (parent's
   parent, grandparent, ... up to the synthetic root — also precomputed once per
   profile, `O(profile depth)`, independent of message size).
2. Seed a local stack with exactly `[parent_node]`.
3. Walk `ScanResult.segments` forward from the line immediately after the parent
   occurrence. For each line's segment type `T`:
   - If `T` matches a child of the stack's top node: push that child node. If the
     stack's length just became `2` (i.e. `T` is a *direct* child of `parent_node`)
     and `T == cseg`, record this span as a same-type direct-child candidate
     (FR-007's corrected, type-filtered, per-parent list — see Decision 3).
   - Else, if the stack has more than one entry, pop and retry against the new top
     (mirrors `HL7HierarchyParser.scala:80-87`'s "try next segment on stacks").
   - Else (the stack is back down to just `[parent_node]` and `T` still doesn't
     match `parent_node` itself): check `T` against the precomputed static ancestor
     chain (including the synthetic root's own top-level entries). If `T` matches
     there, the line belongs to some ancestor's other subtree or a sibling
     occurrence — **stop scanning**, this is the "next line that is not a
     descendant" boundary (FR-003). If `T` matches nowhere at all — not in the
     open local stack, not in the static ancestor chain — it is unrecognized
     exactly as spec `002` Section A.1 case 4(b) documents: **silently drop it**
     (FR-005) and continue scanning with the stack unchanged (mirrors
     `HL7HierarchyParser.scala:70-79`'s explicit backup-and-restore on this exact
     case — the real engine does not advance or regress position for a dropped
     line either).
4. Stop at end of message if no boundary line is found first.

**Rationale**: This is `O(the parent occurrence's own scoped line range)` plus one
`O(profile depth)` ancestor-chain lookup — never `O(message size)`, and never a
per-message tree of any kind, satisfying SC-002 and Constitution Principle II
directly. It is also provably equivalent to the real engine's observable behavior
for computing one parent occurrence's direct children, given Decision 2's
unambiguous-profile assumption.

**Alternatives considered**:
- Replay the full `parseMessageHierarchy` stack simulation from `MSH` up through the
  target occurrence's own line, then continue locally from there: rejected as
  unnecessary — it would correctly resolve an *ambiguous parent type* (Decision 2),
  but at the cost of `O(message prefix length)` per query for a case no existing
  vector's *parent* side exercises.
- Build the tree once per message on first hierarchy query, cache it, reuse for
  subsequent queries against the same `ScanResult`: rejected — this is exactly the
  "eager, whole-message tree" spec `002` Section B.1 and Constitution Principle II
  rule out, regardless of whether it happens on the first call or up front; FR-013
  and this spec's own edge cases are explicit that repeated queries must not amortize
  cost through a retained tree.

## Decision 2: A segment type repeated at multiple tree positions is normal profile data, not malformed — only *parent*-side resolution needs to be unambiguous

**Decision (corrected during implementation — supersedes this decision's original
planning-phase draft)**: `HierarchyProfile` is a small node arena (`Vec` of nodes,
each holding a `children: HashMap<String, usize>` and a `parent: Option<usize>`
index) built once from a `segmentDefinition` JSON document. A segment name is
**allowed** to occupy more than one position in the tree — `by_name` records every
position a name occupies (`HashMap<String, Vec<usize>>`), and construction never
fails because of this. `node_for(name)`, used only to resolve the *parent*-side type
of a `->` expression into its one starting node, returns `None` (folding into
FR-006's "no qualifying children" outcome) both when a name is absent from the
profile and when it maps to more than one node — the only case where ambiguity
actually matters for this spec's design. `cardinality` strings are read from the
JSON but not parsed into a structured type or validated — Section A.3 (spec `002`)
already establishes that navigation never consults cardinality at all (that is the
Validation module's job, Roadmap 2000-2999) — so this spec stores nothing it does
not use.

**What the original planning-phase draft got wrong, and how implementation caught
it**: the original decision rejected *any* repeated segment name, anywhere in the
tree, as malformed (`ProfileError::DuplicateSegmentType`), reasoning that Decision
1's `O(profile depth)`-lookup design requires a segment type's tree position to be
knowable independent of document history. Implementing against the actual fixture
profiles — not a hand-picked example — immediately falsified the premise that "no
existing fixture profile" has this shape: `fixtures/profiles/deep-nested.json`
(consumed by 5 of `complex.json`'s 6 vectors) places `OBX` as a legal child both
directly under `OBR` and under `OBR`'s `SPM` child, and `NTE` both directly under
`OBR` and under `OBR`'s `OBX` child — a completely ordinary shape (a leaf/observation
segment type recurring at more than one nesting depth), not an edge case. The
`hierarchy_vectors` integration test (T014) failed immediately on `hier-005` with
this design, which is what surfaced the mistake before it shipped.

The actual fix is narrower than the original draft assumed: Decision 1's local,
per-parent-occurrence bounded scan **never needed** a globally unique name-to-node
map for descendant matching in the first place — `profile.nodes[top].children.get(seg_type)`
already disambiguates correctly by construction, because `top` is a specific node
index (e.g. "OBR" or "SPM"), not a bare type name, and each node's own `children` map
only lists *that node's* legal children. The only place global uniqueness ever
mattered was resolving the `->` expression's own *parent*-side type into a starting
node — and every existing vector's parent type (`OBR`, in all 10 vectors) is
genuinely unique in both fixture profiles. Rejecting the whole profile at
construction time for an ambiguity that only matters for parent-side resolution, and
that no vector's parent side ever exercises, was fixing the wrong layer.

Note on `cyclical` profiles (spec.md FR-012's illustrative example, carried over
from spec `002`'s own Edge Cases): a `segmentDefinition` JSON document is a plain
nested-object tree — a true reference cycle (a node listing an ancestor as its own
child) is not representable in JSON's tree-shaped grammar at all, unlike a
graph-with-IDs format. With `DuplicateSegmentType` removed, `ProfileError` has a
single variant, `InvalidJson` — malformed/wrong-shaped JSON is the only reachable
"malformed profile" condition this representation can actually encounter.

**Alternatives considered**:
- Keep rejecting ambiguous names outright, but only when the ambiguous name is
  actually used as a `->` expression's parent segment type (a query-time check
  instead of a construction-time one): rejected in favor of the simpler design
  above — since `node_for` already needs to distinguish "absent" from "present," and
  both fold into the same safe "no qualifying children" outcome at the call site
  (FR-006), adding a distinguishable error for the ambiguous-parent case would be a
  new `QueryError`/`HierarchyError` variant for a condition no existing vector
  reaches, contradicting Decision 3's "no new query-time error type" reasoning for
  the same reason.
- Full-history replay specifically for an ambiguous parent type, falling back to
  Decision 1's fast path when unambiguous: rejected as the same unnecessary
  complexity Decision 1's own Alternatives Considered already rejected, now doubly
  unmotivated since no existing vector's parent side is ever ambiguous.
- Represent cardinality as a parsed `(min, max)` pair now, anticipating the
  Validation module's future need: rejected (YAGNI) — Roadmap 2000-2999 owns
  cardinality semantics and can add its own representation against the same
  `fixtures/profiles/*.json` shape when it exists; duplicating that decision here
  serves no consumer this spec has.

## Decision 3: No new query-time error type — `QueryError` is reused as-is

**Decision**: `execute_hierarchy` returns `Result<Vec<Vec<&'m str>>, QueryError>` —
the exact type spec `007` already defined, not a new `HierarchyError` wrapper. The
only query-time failure mode this spec can produce is the same one spec `007`
already has: a `FilterClause` (on either the parent or child side's `SEG_IDX`) uses
an ordering operator against a non-numeric operand
(`QueryError::NonNumericComparison`). Every other "nothing matched" condition —
parent selector matched zero occurrences, a matching parent had zero direct children
of the requested type, no profile was supplied (FR-009) — is `Ok(vec![])`,
identical in kind to spec `007`'s existing philosophy (data-model.md).

**Rationale**: Introducing a parallel error enum for hierarchy queries, when the
actual failure conditions are a strict subset of what `QueryError` already
enumerates, would fragment error handling for callers with no corresponding benefit
— Constitution Principle III cares about the *distinction* between absence and
structural-precondition violation, not about which module produced the violation.
`ProfileError` (Decision 2) is a genuinely separate concern: it is a property of the
*profile itself*, checked once at `HierarchyProfile` construction, independent of
any particular query — never returned from `execute_hierarchy`.

**Alternatives considered**:
- A `HierarchyError` enum wrapping `QueryError` plus a hierarchy-specific variant:
  rejected — there is no hierarchy-specific query-time failure to wrap; it would be
  a single-variant pass-through adding an unnecessary type for callers to match on.

## Decision 4: Two existing conformance vectors' `expected` values must change (spec FR-007, FR-010)

**Decision**: `fixtures/vectors/hierarchy/basic.json`'s `hier-004` and
`complex.json`'s `hier-008` — both tagged `semantic_rules: ["A.4-cross-parent-child-
indexing"]` and `known_limitation: "A.4-cross-parent-child-indexing"` — currently
encode the real Scala engine's *buggy* child-index output (cross-type, un-rebased),
verified live against it per spec `002`'s own process. Since spec.md's Clarifications
already decided to **fix** this bug (FR-007) rather than reproduce it, these two
vectors' `expected`/`expected_lines` values are corrected as part of this spec's
implementation, and their `known_limitation` field is removed (it is no longer a
limitation, by design):

| Vector | Path | Old `expected` (buggy, reproduces the bug) | New `expected` (FR-007's fix) |
|---|---|---|---|
| `hier-004` | `OBR[1] -> OBX[1]-3` (`basic-two-level.json` / `basic-hierarchy.hl7`) | `[["OBX-Q-CODE^Second Observation^LN"]]` (line 6 — `OBX[1]` incorrectly lands on the *2nd* `OBX` due to the 0-based, un-rebased index) | `[["OBX-P-CODE^First Observation^LN"]]` (line 5 — the type-filtered, 1-based, per-parent index correctly selects the *1st* `OBX` child of `OBR[1]`) |
| `hier-008` | `OBR[1] -> OBX[1]-3` (`deep-nested.json` / `complex-hierarchy.hl7`) | `[]` (the buggy cross-type index lands on position 1 of `OBR`'s *mixed-type* direct-child list — `NTE` at line 6 — which fails the `cseg == OBX` check, so nothing matches at all despite two real `OBX` children existing) | `[["OBX-A-CODE^Direct Child A^LN"]]` (line 7 — type-filtered to `[OBX-A (l7), OBX-C (l8)]` first, then 1-based index 1 selects `OBX-A`) |

Both traces were re-derived by hand against `HL7HierarchyParser.scala`/
`HL7ParseUtils.scala`'s actual logic (Decision 1) and against this spec's corrected
algorithm, then confirmed by running the actual implementation against them
(`hierarchy_vectors`, T020) — both failed against the *old* values exactly as
predicted before the fixture edit, then passed immediately after it, independently
corroborating the hand trace. `semantic_rules` is left as `["A.4-cross-parent-child-
indexing"]` for both — the vector still exercises that documented rule from
`contracts/hierarchy-semantics.md`, it's simply demonstrating the corrected
treatment of it now — and the schema's closed `semantic_rules` enum (spec `002`)
needs no new value as a result. Only `known_limitation` is removed (it is no longer
a limitation, by design).

**Rationale**: FR-010 requires this module to reproduce every existing vector's
`expected` value *as this spec defines correct behavior* — silently leaving these
two vectors pointing at the old, superseded (buggy) values would make the corrected
implementation *fail* its own conformance suite, or would require quietly
special-casing two vectors' outcomes, either of which contradicts FR-007's whole
point. This is the same category of finding spec `007`'s own planning surfaced
(`plan.md`: "corrected an earlier planning draft") — verified against the real
engine before writing Rust code, not assumed.

**Alternatives considered**:
- Leave `hier-004`/`hier-008` unchanged and add a documented exception carving them
  out of FR-010's "10/10" requirement: rejected — this would leave stale, misleading
  fixture data in the shared regression suite (spec `003`) that no longer describes
  either the old behavior (superseded) or the new one (not yet reflected), which is
  strictly worse than correcting the two values directly.
- Delete the two vectors instead of correcting them: rejected — they are exactly the
  right coverage for the corrected indexing behavior (a single-parent multi-child
  case and a mixed-sibling-type case); deleting them would reduce, not preserve,
  conformance coverage for the very code path FR-007 changes.

## Decision 5: `serde`/`serde_json` promoted from dev- to runtime dependency (spec FR-014)

**Decision**: `HierarchyProfile::from_json` uses `serde`/`serde_json` (already
present at `1.0.151` in `Cargo.lock` as a `hl7pet-core` dev-dependency since spec
`007`) via a small, crate-private `#[derive(Deserialize)]` struct mirroring
`segmentDefinition`'s recursive JSON shape exactly. Neither type is ever part of
`hierarchy`'s public API (contracts/hierarchy-api.md) — the public `HierarchyProfile`
and `ProfileError` are plain Rust types produced *from* that private intermediate
representation, never exposing it.

**Rationale**: Satisfies the user's explicit direction (an established JSON parser,
not a hand-rolled one) and FR-014's two conditions: `serde_json` is pure Rust with no
system/C-library build step (does not obstruct cross-compiling for future Python
wheels or Java JARs), and it was already vetted and pinned by this exact workspace
for the identical purpose (deserializing structured JSON fixtures) — promoting a
dependency already resolved in `Cargo.lock` is lower-risk than introducing a new one.

**Alternatives considered**:
- A hand-rolled minimal parser for just the `segmentDefinition` shape, preserving
  `hl7pet-core`'s zero-runtime-deps property through this spec too: explicitly
  rejected per the user's direction and [[project_dependency_policy]] — the
  zero-runtime-deps property was never a goal in its own right, only a proxy for
  "doesn't complicate the Python/Java bindings," which `serde_json` doesn't.
- A smaller/alternative JSON crate (e.g. `miniserde`, `nanoserde`): rejected —
  `serde_json` is already resolved in this exact workspace for this exact purpose;
  introducing a second JSON library for no functional gain adds dependency-graph
  noise without upside.

## Decision 6: Two vectors (`hier-009`, `hier-010`) use a two-hop PATH and are out of scope, discovered during implementation

**Decision (found during implementation, not anticipated in planning)**:
`fixtures/vectors/hierarchy/complex.json`'s `hier-009` and `hier-010` both use the
PATH `"OBR[1] -> OBX[3] -> NTE-3"` — two `" -> "` hops. `hier-009` (`semantic_rules:
["A.6-chained-arrow-silently-empty"]`, `expected: null`) documents the real Scala
engine's actual behavior for a chained arrow today (spec `002` Section A.6: the
parent side greedily consumes up to the last arrow, fails to match `SEGMENT_EXPR`,
yields nothing). `hier-010` (`semantic_rules: ["B.2-multi-level-navigation"]`,
a real expected value) was added anticipating spec `002` Section B.2's multi-hop
recommendation being implemented by *this* spec. Since spec.md's Clarifications
deferred multi-hop chaining instead, and spec `006`'s parser already rejects a
second `" -> "` outright (`ParseErrorKind::MultipleHierarchyHops`, confirmed
unchanged, T023), **both vectors fail to parse under this implementation** —
`hl7pet_core::parse("OBR[1] -> OBX[3] -> NTE-3")` returns `Err` before
`execute_hierarchy` is ever reached, regardless of which vector's `expected` value
is being checked against. Both are excluded from `hierarchy_vectors`'s dispatch loop
(`is_multi_hop`, skipping any PATH with more than one `" -> "`), with the exclusion
documented in the test file's module doc comment, not silently treated as passing or
silently dropped from the count.

**Why this wasn't caught during planning**: research.md and plan.md's earlier drafts
verified `hier-004`/`hier-008` (Decision 4) by hand because their `known_limitation`
tag made them stand out as needing attention; `hier-009`/`hier-010` carry no such
flag pointing at a spec-008-relevant concern on their face (`hier-009`'s tag
describes *current* Scala behavior, not a forward-looking one), and their PATH
string wasn't inspected closely enough during planning to notice the second arrow.
The `hierarchy_vectors` integration test (T014) failing to even *parse* two of the
ten target vectors is what surfaced this — a category of finding this project
consistently treats as something to fix and document immediately, not patch around
(spec `007`'s own planning-vs-implementation corrections are the established
precedent).

**Rationale for exclusion over any other fix**: neither vector can be made to pass
without either (a) implementing multi-hop chaining now (reopening the Clarifications
decision this spec already made deliberately) or (b) rewriting the vectors to a
different, single-hop PATH (which would stop them from exercising `A.6`/`B.2` at
all, defeating their purpose for whichever future spec does implement multi-hop).
Excluding them, documented, preserves both the vectors themselves (unchanged, ready
for that future spec) and this spec's own scope boundary.

**Alternatives considered**:
- Implement multi-hop chaining now to make `hier-010` pass, treating its existence
  as evidence the deferral decision should be revisited: rejected — the deferral was
  a deliberate scope decision (spec.md Clarifications), not an oversight; discovering
  a fixture vector anticipating the *other* choice doesn't retroactively change the
  facts that motivated deferring (single-hop is the compatibility floor with real
  conformance vectors; multi-hop has none, and no existing Rust caller needs it yet).
- Treat a `ParseError` on these two vectors as an acceptable/expected test outcome
  and assert on that instead of skipping them: rejected — `hier-009`'s `expected:
  null` happens to coincide with "no match," but a parse-time rejection and a
  runtime empty match are different categories (data-model.md's `QueryError` vs. a
  `ParseError` from a wholly different module); asserting a `ParseError` where the
  vector's schema declares a `getValue`/`expected` outcome would be testing the
  wrong thing under a coincidentally-matching label.

## Decision 7: `hier-011` added — the corpus had no vector proving cross-parent isolation

**Decision (post-implementation, per user request)**: `messages/basic-hierarchy.hl7`
gained a second `OBR` occurrence with its own two `OBX` children, appended after the
existing content (lines 1-6 unchanged, so `hier-001`/`hier-002`/`hier-004`'s
`expected_lines` remain valid). `fixtures/vectors/hierarchy/basic.json` gained
`hier-011`: `"OBR[2] -> OBX-3"`, expecting only the second `OBR`'s two `OBX` values.

**Rationale**: every pre-existing single-`OBR`-per-message vector (`hier-001`,
`hier-002`, `hier-004`) proves *that* `->` extracts children, but not that it
*excludes* a sibling parent's children — with only one `OBR` in the message, a
(hypothetical) implementation that ignored parent-scoping entirely and just
returned "every `OBX` in the message" would have passed every existing
`basic-hierarchy.hl7` vector identically. `complex-hierarchy.hl7` does have two
`OBR`s, but every vector against it uses `deep-nested.json`'s deeper profile,
entangling this specific question with nested-child exclusion. `hier-011` isolates
exactly the property in question: `OBR[1]` and `OBR[2]` must return disjoint
results. Manually confirmed live via the CLI before adding the vector:
`OBR[1] -> OBX-3` returns the first pair, `OBR[2] -> OBX-3` returns the second pair,
and unindexed `OBR -> OBX-3` returns all four in document order.

**Alternatives considered**:
- Add the second `OBR` to a *new* message file instead of extending
  `basic-hierarchy.hl7`: rejected — `basic-hierarchy.hl7` already exists
  specifically for `basic-two-level.json`-profile, non-nested scenarios, and
  appending (not editing in place) is provably non-breaking for every vector
  already referencing it (confirmed by re-running `hierarchy_vectors` and
  `scanner_regression` after the edit, both still passing).
- Give `hier-011` a novel `semantic_rules` tag documenting "cross-parent isolation"
  specifically: rejected in favor of reusing `"A.2-single-hop-basic"` (already used
  by `hier-001`) — same schema-avoidance reasoning as Decision 4: no new enum value
  needed, the vector still exercises that same documented rule, just against a
  message that can actually falsify a scoping bug.
