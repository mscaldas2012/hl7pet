# Phase 0 Research: Ambiguous-Position Parent Resolution for Hierarchy PATHs

## 1. Does the real Scala engine have defined behavior for this scenario?

Spec.md left this as an explicit open question rather than assuming an answer. It is
now resolved — **yes, and the real engine gets it right, by construction.**

**Decision**: Treat this as a genuine regression relative to the Scala baseline
(`gov.cdc:hl7-pet_2.13:1.2.11`), not merely a missing nice-to-have. The fix must make
Rust's behavior match Scala's for this scenario.

**Rationale**: Verified live, not just by reading source. `HL7HierarchyParser.scala`
(`mscaldas2012/hl7-pet`, the same checkout spec `002`/`008` verified against) was run
directly — via a small Java driver calling `HL7HierarchyParser.parseMessageHierarchyFromJson`
against the real Maven Central artifact — with this repo's own existing
`fixtures/messages/complex-hierarchy.hl7` and `fixtures/profiles/deep-nested.json`
(the exact profile where `OBX` is legal both under `OBR` and under `OBR`'s `SPM`
child, and `NTE` is legal both under `OBR` and under `OBX`). The real engine's output:

```text
line=4  -> OBR|1|ORDER0002|...
  line=5  -> NTE|1|L|Note attached directly to OBR (first)
  line=6  -> NTE|2|L|Note attached directly to OBR (second)
  line=7  -> OBX|1|CE|OBX-A-CODE...
  line=8  -> OBX|2|CE|OBX-C-CODE...
    line=9  -> NTE|1|L|Note attached to OBX-C, not to OBR directly
  line=10 -> SPM|1|SPECIMEN0001|...
    line=11 -> OBX|1|CE|OBX-UNDER-SPM-CODE...
line=12 -> OBR|2|ORDER0002B|...
```

Line 9's `NTE` is correctly nested under line 8's `OBX` (not directly under `OBR`),
and line 11's `OBX` is correctly nested under line 10's `SPM` (not directly under
`OBR`) — exactly the disambiguation `hl7pet-core`'s `HierarchyProfile::node_for`
currently gives up on. Why the real engine never has this problem:
`HL7HierarchyParser.parseMessageHierarchy` (`HL7HierarchyParser.scala:22-95`) builds
its entire output tree in **one top-down pass over the whole message**, maintaining a
single stack of `(profile node, output node)` pairs as it goes (push on a recognized
child, pop-and-retry on an unrecognized one, restore on total failure). There is no
step anywhere in the Scala algorithm that maps a bare type name to "the" profile
position — every segment's position is determined purely by where it falls in that
one continuous walk. Ambiguity, in the sense `node_for` chokes on, cannot arise in the
Scala design at all: it's a byproduct specific to `hl7pet-core`'s (correct, deliberate)
choice to *not* build a full tree, replacing Scala's one holistic walk with a
two-step "look up the type's one global position, then bounded-scan from there" —
which only works when that lookup is unambiguous.

This is exactly the gap spec `008`'s own contract (`contracts/hierarchy-api.md`,
"What this contract explicitly does NOT provide") already named and deliberately
deferred: *"Resolving an ambiguous parent-side type ... via history-dependent
disambiguation."* This spec is that deferred item, now scoped to be built.

**Alternatives considered**: Assuming, without verifying, that this was a "new
capability beyond Scala" (à la spec `1000`'s located-extraction API, which has no
Scala equivalent) — rejected once the live run showed Scala already handles it;
treating it as a bug fix (parity restoration) is the correct framing, consistent with
this repo's verification-before-Rust-code discipline (specs `001`, `002`, `006`,
`007`, `008`).

## 2. Resolution algorithm shape

**Decision**: Generalize the *same* nearest-enclosing-ancestor matching convention
`direct_children_of_type` already uses correctly for descendant/child-side matching
(`crates/core/src/hierarchy.rs`) so it can also classify **parent-side** occurrences
of an ambiguous type — instead of introducing a different or novel resolution rule.
Concretely: for a query whose parent type is detected ambiguous (`profile.by_name`
mapping to more than one node — the same check `node_for` already does, just acted
on differently), resolve each raw name-selected candidate occurrence's *actual* tree
position by replaying the identical push/pop stack walk from the top of the message
(right after `MSH`, mirroring exactly where `HL7HierarchyParser.scala`'s walk begins)
up to and including that occurrence — rather than guessing from the type name alone.
Once a candidate's real node is known, everything downstream (`direct_children_of_type`
seeded at that node, `apply_child_index`, field resolution) is unchanged.

**Rationale**: This keeps the fix additive to the existing, already-correct local
algorithm rather than replacing it, and guarantees parent-side and child-side
resolution can never disagree with each other (they run the same rule). It also
provably doesn't change any already-correct (unambiguous) result: when a type is
unambiguous, its one global profile position is, by definition, the same node any
top-down replay would also arrive at for every occurrence of that type — no
information earlier in the message could change an unambiguous mapping. This directly
grounds FR-006 (no regression), not just as an assumption but as a property of the
chosen algorithm.

**Alternatives considered**:
- **Eagerly build the full hierarchy tree once per hierarchy query, mirroring
  Scala's `parseMessageHierarchy` exactly** — rejected as the *default* path: this
  reintroduces, for the common (unambiguous) case, precisely the eager-full-tree cost
  spec `008` was written to avoid and spec `009` measured as the dominant Scala-vs-Rust
  gap for hierarchy operations (2-2600x). It remains available conceptually as *what
  the fallback for ambiguous types effectively is* (a full top-down replay), just
  triggered only when needed, not unconditionally.
- **Require a qualified/multi-hop PATH to pre-disambiguate (e.g. chaining through
  `SPM`)** — rejected: multi-hop `->` chaining is a different, already-deferred
  non-goal (spec `006`'s parser rejects a second `" -> "`; `hier-009`/`hier-010` in
  `fixtures/vectors/hierarchy/complex.json` document this separately), and wouldn't
  even address a single-hop ambiguous parent reference on its own.
- **A persistent, cross-call cache of each segment's resolved node, attached to
  `ScanResult`** — a plausible future optimization (amortizing the walk across
  multiple hierarchy PATHs against the same scan) but out of scope here: no existing
  caller runs multiple hierarchy PATHs against one `ScanResult` today, and adding a
  cache raises lifetime/mutability questions this fix doesn't need to answer to be
  correct. Noted as a follow-up opportunity, not a requirement.

## 3. Where the walk needs to start, and how expensive it is

**Decision**: The classification walk for an ambiguous type must start from the top
of the message's segment list (the position right after `MSH`, matching where the
real Scala walk begins) — not from some fixed-size local window around the candidate
occurrence.

**Rationale**: The real algorithm's stack state at any point is a function of *every*
preceding segment, not a bounded lookback — the same message could require an
arbitrarily long run of siblings (e.g. many `NTE`s) before the next state-changing
segment. There is no general shortcut to "the last few lines." This is only paid when
the query's own parent type is ambiguous; the existing fast, purely local,
already-correct path (`node_for` + bounded `direct_children_of_type`) is untouched
for every unambiguous query, which — per the existing hierarchy conformance vectors
(`fixtures/vectors/hierarchy/complex.json`, all of which use the unambiguous `OBR` as
their parent type) — is every case exercised today. A later optimization could bound
the walk to start from the nearest preceding segment whose *own* type is unambiguous
(a cheap "checkpoint") rather than always the true top, but that's a performance
refinement layered on top of a correct O(message segment count) baseline, not
something this spec's correctness requirement depends on.

**Alternatives considered**: See #2 above (the cache alternative applies here too,
as an alternative to re-walking from the top on every call).

## 4. Versioning / breaking-change classification

**Decision**: No MAJOR version bump or migration guide, per the
Backward-Compatible-Additions convention (`ROADMAP.md`).

**Rationale**: Confirmed, not just assumed: today, every `->` PATH whose parent type
is ambiguous in its profile returns `Ok(vec![])` unconditionally (verified directly —
`fixtures/vectors/hierarchy/complex.json` has no vector exercising this because none
could pass). No consumer can be relying on a currently-meaningful result for this
case; there is nothing to preserve behind an opt-out. This mirrors spec `008`'s own
precedent of recording a real correctness fix (the child-index bug) directly rather
than gating it behind a flag, since the "old" behavior there was also an
acknowledged bug, not a documented, intentional contract.

## 5. Test/fixture strategy

**Decision**: Add new hierarchy conformance vectors to
`fixtures/vectors/hierarchy/complex.json` (or a sibling file) exercising `OBX` and
`NTE` as the *parent* side of `->`, reusing the already-existing
`fixtures/messages/complex-hierarchy.hl7` / `fixtures/profiles/deep-nested.json`
pair — no new message or profile fixture needed, since that pair already contains
exactly the ambiguity this spec fixes and is already wired into
`fixtures/scripts/validate_corpus.py`'s coverage reporting.

**Rationale**: Reusing the existing pair keeps this fix's verification directly
comparable to the same fixtures the live Scala verification above used, and confirms
the fix against a message the corpus-validation tooling already tracks. Exact new
vectors (e.g. `OBX -> NTE`, `OBX[2] -> NTE`, `SPM -> OBX`, and a case where an
ambiguous-type occurrence resolves to *no* legal position) are an implementation
task, not decided further here.

## 6. The new global walk needs full backup/restore, unlike the existing bounded scan

**Decision**: The new top-of-message classification walk (#2 above) must replicate
`HL7HierarchyParser.scala`'s full backup/restore behavior on an unrecognized-anywhere
segment (`HL7HierarchyParser.scala:53-54,72-79`: popped levels are saved and restored
so a later segment doesn't permanently lose deeper context). It must **not** simply
reuse `direct_children_of_type`'s existing inner loop as-is, even though that loop is
the reference for the *matching rule* (#2's decision).

**Rationale**: `direct_children_of_type`'s existing pop-on-unrecognized behavior
(`crates/core/src/hierarchy.rs`, the `else if stack.len() > 1` branch) discards
popped levels permanently rather than restoring them. This is safe *for that
function's own narrow purpose* — it only ever records a match at stack depth exactly
2 (a direct child of one already-known parent), and permanently losing a deeper,
already-popped level can never produce a false depth-2 match or suppress a true one,
since the parent's own node is never popped. It is provably equivalent to Scala's
restore-based approach for that one purpose. It is **not** safe for a general,
top-of-message walk that must correctly resolve arbitrary target occurrences at
*any* depth: losing an intermediate level after one unrecognized sibling would
incorrectly fail to resolve a later, legitimately-deeper occurrence. The two loops
share the same matching *rule* (nearest-enclosing-ancestor) but need different
bookkeeping around the unrecognized-anywhere case — this is a real implementation
distinction, not a contradiction of #2's "same rule" decision.

**Alternatives considered**: Reusing `direct_children_of_type`'s loop unmodified for
the new global walk — rejected once traced through the scenario above; would silently
diverge from Scala for messages with an unrecognized segment interleaved between two
differently-nested legitimate segments, a real (if narrow) correctness gap the fix
must not introduce.
