# Hierarchy-Mode Semantics

Companion to `specs/001-path-grammar-spec/contracts/path-grammar.md`. That document
owns PATH *syntax* (including the `->` token appearing in the `PATH`/`CHILD_PATH`
productions); this document owns what `->` *means* at evaluation time. Terminology
(`SEGMENT_EXPR`, `SEG_IDX`, `FIELD_EXPR`, `CHILD_PATH`, etc.) is reused from spec `001`
without redefinition.

This document has two parts: **Section A** describes the current, real, single-hop
behavior of the Scala engine (the Constitution Principle I compatibility floor every
implementation — including the planned Rust core — must match). **Section B** records
the two decisions spec `002`'s planning phase was explicitly tasked with resolving
(profile requirement, multi-level chaining), per `research.md`.

## Section A: Current Behavior (Compatibility Floor)

### A.1 Segment Hierarchy Tree

A **Segment Hierarchy Tree** is built from a message and a profile's
`segmentDefinition` map (segment name → `{cardinality, children}`), rooted at a
synthetic node whose children are the profile's top-level `segmentDefinition` entries.

**Construction rule** (nearest-enclosing-ancestor matching):

1. The first line (`MSH`) always attaches directly under the root, unconditionally.
2. For each subsequent line, take its 3-letter segment name and try to match it as a
   child of the *current* tree position (the most recently attached node).
3. If it matches: attach as a new child of the current position; the current position
   descends to this new node.
4. If it does **not** match: move the current position up to its own parent and retry
   step 2/3 against that parent. Repeat until either (a) a match is found at some
   ancestor level — attach there, and the current position descends from *that*
   ancestor (any deeper levels that were tried and failed are discarded) — or (b) the
   root is reached with no match anywhere in the ancestor chain.
5. In case 4(b) (segment not recognized as a child anywhere up the current ancestor
   chain), the line is **silently dropped from the tree** — no error, no exception.
   The line remains fully present in the underlying flat message (visible to
   static-mode/flat PATH queries) but has no node in the tree and cannot be reached via
   `->`.

`SegmentConfig.cardinality` is **not** consulted anywhere in this construction
algorithm — it is inert metadata at this stage (see A.3).

**Example** (using the shape of the real `COVID_ORC.json` profile:
`MSH → {ORC, OBR → {NTE, OBX → {NTE}, SPM → {OBX}}}`):

```text
MSH
├─ ORC
├─ OBR                  (1st occurrence)
│  ├─ OBX
│  │  └─ NTE
│  └─ SPM
│     └─ OBX
└─ OBR                  (2nd occurrence)
   └─ OBX
```

A trailing `ZZZ` segment (not present anywhere in the profile) appearing after the last
`OBX` above would be silently dropped: the algorithm would try it against the current
position (`OBX`), then `OBR`, then the root, exhaust the ancestor chain, and drop it —
without affecting anything else in the tree.

### A.2 The `->` Operator (Single-Hop)

`SEGMENT_EXPR -> CHILD_PATH` (spec `001`'s `PATH` production) evaluates as follows:

1. Parse the parent side (`seg`, `segIdx`) and child side (`cseg`, `csegIdx`, and the
   child's own `FIELD_EXPR`) using spec `001`'s existing `SEGMENT_EXPR`/`FIELD_EXPR`
   grammar — unchanged from flat-path evaluation.
2. Compute the set of **matching parent occurrences**: scan the whole tree
   (recursively, at every depth — not only root-level children) for every node whose
   segment type equals `seg`. For each such node, check whether its source line number
   is among the occurrences of `seg` selected by `segIdx` (a number, `$LAST`, `*`, or a
   `FILTER`) — this selection is computed the same way flat-path `SEG_IDX` evaluation
   selects occurrences, i.e. over **all** occurrences of `seg` in the message, not
   scoped to any subtree.
3. For every matching parent occurrence, collect its **direct children only** (one
   level down — grandchildren are not included by this step, even if `cseg` would
   otherwise match one). All matching parents' children are combined into a single
   flat, ordered list, **of mixed segment types** (not yet filtered to `cseg`), in the
   order the parents themselves were visited (which follows tree traversal order,
   which in turn follows document order).
4. Walk that combined, still-mixed-type list position by position (0-based). At each
   position, first check whether that entry's segment type equals `cseg`; only if it
   does is it a candidate at all.
5. If `csegIdx` is a `FILTER`, evaluate it against the candidate entry directly (same
   as elsewhere). If `csegIdx` is a plain number, it is compared **directly, with no
   `-1` adjustment, against the entry's 0-based position in the full mixed-type
   combined list from step 3** — not its position among only-`cseg`-typed entries, and
   not adjusted to the 1-based convention used everywhere else in the engine (see A.4).
   If `csegIdx` is absent, every `cseg`-typed candidate qualifies.
6. Apply the child's `FIELD_EXPR` (field/component/subcomponent/index) to each
   remaining line exactly as flat-path evaluation would.
7. The result shape is the same two-dimensional (occurrences × repetitions) structure
   spec `001` FR-004 defines for flat paths — hierarchy adds a selection/scoping step
   before that shape is produced, it does not change the shape itself.

If step 2 or step 4 yields zero occurrences, the result is an empty result (zero
values) — never an exception (Constitution Principle III). This applies uniformly
whether the emptiness is because a specific parent occurrence has no children of the
requested type, or because the parent selector itself matched nothing.

**A precise, verifiable distinction in *how* "empty" is represented**: flat-path
evaluation (`HL7StaticParser.getValue`) collapses *both* "the path doesn't even
syntactically match" and "it matches but finds zero values" down to the same
`None` (`HL7StaticParser.scala:259` for the former, `:285-288`'s explicit
`if (result.isEmpty) None else Option(result)` for the latter). Hierarchy `->`
evaluation does **not** make this same collapse: `getChildrenValues` returns
`Option(result)` unconditionally once the parent side has matched
`SEGMENT_EXPR` syntactically (`HL7ParseUtils.scala:125`) — so a syntactically valid
`->` expression that legitimately finds zero children (step 2 or step 4 above)
yields `Some(emptyArray)`, not `None`. `None` is reserved, in hierarchy mode, for
cases where the *parent side itself* fails to match `SEGMENT_EXPR` at all — which is
exactly what happens in Sections A.5 and A.6 below, where the parent side ends up
containing characters (a literal arrow, or unrecognized syntax) `SEGMENT_EXPR` never
accepts. Concretely: `OBR[2] -> OBX-5` against an `OBR` occurrence with no `OBX`
children returns `Some(Array())` (empty, not absent); `"OBR[1] -> OBX[3]" -> NTE-3`
(Section A.6, parent side already contains an embedded arrow) returns `None`. Both are
externally "zero values, no exception" from a caller's perspective if all they check
is emptiness — but the exact `Option` shape differs, and an implementation aiming for
faithful parity should preserve which one applies where, not treat them as
interchangeable.

### A.3 Cardinality vs. Navigation (spec FR-002 boundary)

`segmentDefinition` cardinality (`[m..n]`) is used **exclusively** by the separate
Validation module (`StructureValidator`, Roadmap 2000-2999) and **only** as a
whole-message, per-segment-type count (e.g. "are there between 1 and 3 `OBR` segments
anywhere in this message") — not scoped to any specific parent occurrence. Neither tree
construction (A.1) nor `->` navigation (A.2) reads or enforces cardinality at all. A
consequence worth stating explicitly: **there is currently no mechanism, anywhere in
the engine, that validates "does this specific parent occurrence have the correct
number of children"** — only "does this segment type appear the correct number of
times globally." A required child (`[1..1]`/`[1..*]`) missing under one particular
parent occurrence, while present under a different occurrence of the same parent type,
is invisible to both navigation (silently returns empty for that occurrence, A.2) and
validation (the global count is still satisfied by the other occurrence's children).

### A.4 Known/Documented Limitation: Child Index Is Unfiltered, Un-Rebased, and Untested

A numeric child index (e.g. `OBX[2]`) does **not** behave like every other indexed
position in this engine. Three compounding, real behaviors (traced directly in
`HL7ParseUtils.scala`, see `research.md` Decision 3):

1. **Cross-parent concatenation**: if the parent selector matches more than one
   occurrence, all of their children are combined into one list first — the index is
   not re-based to "the Nth child of each individual parent."
2. **Cross-type mixing**: the index counts position over *every* child in that
   combined list, regardless of segment type — not only over children matching `cseg`.
   A parent with interleaved child types (the normal case — e.g. `COVID_ORC.json`'s
   `OBR` has `OBX`, `NTE`, `TQ1`, `TQ2`, `CTD`, `FT1`, `CTI`, `SPM` children) can cause
   `OBX[N]` to land on a non-`OBX` position and match nothing, even when an Nth `OBX`
   child clearly exists.
3. **No 1-based adjustment**: unlike the parent side of `->` and every flat-path
   `SEG_IDX`/`FIELD_IDX` (which convert a 1-based user index via
   `segments.slice(segmentIndex - 1, segmentIndex)`), this specific comparison
   (`csegIdx.toInt == i`) uses the raw index directly against a 0-based position, with
   no `-1` adjustment — an inconsistency even in the simplest single-parent,
   single-child-type case.

No test in the Scala test suite exercises a numeric index on the child side of `->`
(existing hierarchy tests either omit the child index or use a `FILTER` instead), so
this appears to be genuinely untested behavior, not a verified design choice. This is
carried forward as-is per Principle I (compatibility floor); it is documented here in
full so no implementation "fixes" it silently, partially, or inconsistently across
bindings. Any future correction (e.g. per-parent-and-per-type re-based, 1-based
indexing) is a breaking-change decision for whichever spec actually changes it — most
likely spec `008` — not an implicit side effect of this document.

### A.5 `->` Without Hierarchy Mode

When hierarchy mode is not enabled, a `->` expression is **not** specially
interpreted — it is passed through as a literal string to flat-path evaluation, which
does not recognize `->` as a token, so it matches nothing and yields an empty result
(zero values, no exception). This is *not* "the child segment gets extracted as if it
were a flat path" (a plausible but incorrect reading of `SPEC.md` §7's "silently falls
back to flat extraction") — the entire combined string (`"OBR[1] -> OBX-5"`, arrow
included) simply fails to match the flat grammar as a whole. The externally observable
outcome (empty, no error) is what implementations must preserve; the specific internal
mechanism (a failed match rather than a deliberate fallback) need not be replicated.

**"No hierarchy mode" is controlled by `buildHierarchy`, not by "no profile" — the two
are not the same switch.** `HL7ParseUtils`'s single-argument constructor
(`new HL7ParseUtils(message)`) sets `buildHierarchy = false` *and* still loads a
default profile (`PhinGuideProfile.json`) into its `profile` field regardless — that
loaded profile is simply never consulted, since `getValue` gates all hierarchy logic
behind `if (buildHierarchy)` before anything touches `profile`. Conversely, the
three-argument constructor called as `new HL7ParseUtils(message, null, true)` — an
explicit `null` profile *with* `buildHierarchy = true` — enters hierarchy mode anyway,
falling back to that same bundled `PhinGuideProfile.json` as the tree's structure. So
"requires a profile" (Section B.1) means hierarchy mode always operates against *some*
declarative profile, not that the caller must always supply one explicitly — a bundled
default can and does satisfy that requirement today. What actually gates hierarchy
behavior is `buildHierarchy` alone.

### A.6 Chained `->` Today Silently Fails

A path containing more than one `->` (e.g. `ORC[1] -> OBR[1] -> OBX-5`) is **not**
today interpreted as two hops. The parent/child split greedily consumes up to the
*last* `->`, leaving a parent side (`"ORC[1] -> OBR[1]"`) that itself contains an
arrow and therefore does not match a plain `SEGMENT_EXPR` — so the whole expression
silently yields an empty result. This is today's actual behavior, not a documented
design. See Section B.2 for what changes if multi-level chaining is adopted.

## Section B: Decisions From This Spec's Planning Research

### B.1 Profile Requirement (resolves spec FR-004)

**Decision**: Hierarchy navigation in the Rust core still **requires an explicit,
declarative profile** (a `segmentDefinition`-equivalent) to determine legal
parent/child pairings — HL7 v2's flat segment sequence carries no self-describing
nesting information, so nothing else could supply this without hard-coding
per-message-type logic (a direct Principle V violation). What changes from today's
Scala engine is *not* whether a profile is required, but *when and how much* structure
gets built from it: the Rust core MUST NOT eagerly materialize a full tree over the
whole message at construction time (Section A.1's approach) — containment for a given
`->` query is computed lazily, on demand, bounded to a single forward scan from the
selected parent occurrence(s) to the next line that is not a descendant under the
same nearest-enclosing-ancestor rule (A.1), using the profile purely as a lookup table
for "is X a legal child of Y," not as an eagerly-built tree object. See `research.md`
Decision 2 for full rationale and rejected alternatives.

### B.2 Multi-Level Navigation (resolves spec FR-005)

**Decision**: Recommend **including** multi-level chaining as a Backward-Compatible
Addition to spec `001`'s `CHILD_PATH` production:

```text
CHILD_PATH ::= SEGMENT_EXPR [ "-" FIELD_EXPR ]
             | SEGMENT_EXPR " -> " CHILD_PATH        (new: recursive, replaces
                                                        Section A.6's silent-failure
                                                        behavior with real N-hop
                                                        navigation)
```

Existing single-hop paths (`SEGMENT_EXPR -> CHILD_PATH` where `CHILD_PATH` has no
further `->`) parse and evaluate exactly as Section A.2 describes — this is a strict
addition, not a change to existing meaning, so no MAJOR version bump is triggered by
the addition itself (Constitution Principle I). What *was* a silently-empty result
(Section A.6) becomes a real multi-hop answer instead — this is a behavior change only
for inputs that previously matched nothing, which `ROADMAP.md`'s Backward-Compatible
Additions convention treats as additive, not breaking.

**Falsifiable performance claim (SC-005)**: each additional hop costs exactly one
bounded forward scan over the line range the *previous* hop already narrowed — no hop
re-scans lines a prior hop excluded, and no full-message tree is ever materialized.
Total work across an N-hop query is `O(message size)` once (the hops partition the
scan, they don't repeat it), the same asymptotic bound Decision B.1 already commits
single-hop navigation to. This claim is falsifiable once spec `008`'s implementation
exists: spec `009` (core-perf-validation) should include a benchmark comparing an
N-hop chained query's cost against N independently-run single-hop queries over
disjoint ranges of the same message — if chaining shows super-linear cost relative to
that baseline, this recommendation should be revisited before shipping multi-level
support, per spec FR-005's explicit performance gate.

**Explicitly not decided here**: whether the child-indexing limitation (A.4) should be
fixed (type-filtered, per-parent re-based, 1-based) as part of introducing chaining.
That is left to spec `008`'s own plan, informed by this document, since it is a
Principle-I breaking-change judgment call, not a consequence of the chaining decision
itself.
