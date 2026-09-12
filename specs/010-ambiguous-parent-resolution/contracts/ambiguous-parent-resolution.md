# Contract: Ambiguous-Position Parent Resolution (amends spec `008`'s hierarchy API)

`execute_hierarchy`'s public signature (spec `008`,
`specs/008-lazy-hierarchy-nav/contracts/hierarchy-api.md`) does **not** change:

```rust
pub fn execute_hierarchy<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
    profile: Option<&HierarchyProfile>,
) -> Result<Vec<Vec<&'m str>>, QueryError>;
```

This document amends only the **behavior** for one previously-excluded input shape;
every other postcondition in spec `008`'s contract (return shape, error variants,
panic-freedom, borrow-not-copy) is unchanged and re-affirmed here by reference.

## What changes

Spec `008`'s contract listed, under "What this contract explicitly does NOT
provide":

> Resolving an **ambiguous parent-side type** — a segment type used as a `->`
> expression's parent that occupies more than one position in `segmentDefinition` —
> via history-dependent disambiguation. `node_for` (data-model.md) returns `None` for
> this case, folding into FR-006's "no qualifying children" outcome, same as an
> absent type.

**This bullet is retired by this spec.** The corresponding update to
`specs/008-lazy-hierarchy-nav/contracts/hierarchy-api.md` MUST remove or strike it
and cross-reference this document, so the limitation registry doesn't describe a gap
that no longer exists.

## Revised behavior for `path.child.is_some()` and `profile.is_some(p)`

Spec `008`'s step 3 (`contracts/hierarchy-api.md`) is refined as follows — only the
**parent selection** sub-step changes; everything after "the seed node is known" is
unchanged:

1. Parent selection (spec `007`'s `resolve_segment_candidates`, unchanged): find every
   segment occurrence in `scan` matching `path.segment.name`, then apply
   `path.segment.index` (`[N]`, `$LAST`, `*`, filter) — purely by raw name and
   document order, **not** filtered or reordered by profile validity (data-model.md;
   spec.md FR-003). This step is identical whether or not the type is ambiguous.
2. **(Revised)** For each selected candidate occurrence, resolve its seed node:
   - If `path.segment.name` is *unambiguous* in `p` (maps to exactly one tree
     position) — `node_for`'s existing O(1) lookup, byte-for-byte unchanged from spec
     `008`.
   - If `path.segment.name` is *ambiguous* in `p` (maps to more than one position) —
     resolve via this occurrence's Segment Occurrence Classification (data-model.md):
     replay the same nearest-enclosing-ancestor rule `direct_children_of_type` already
     uses, from the top of the message, up to and including this occurrence. Yields
     either one specific tree node, or "unrecognized."
   - An "unrecognized" outcome for a given occurrence means that occurrence
     contributes no children — it is dropped from the candidate set, not an error
     (FR-004), exactly as an occurrence whose type is entirely absent from the
     profile already is today.
3. Per-parent bounded direct-child scan (`direct_children_of_type`, unchanged),
   seeded at each candidate's now-correctly-resolved node instead of a single
   shared node → per-parent `csegIdx` resolution (unchanged) → concatenation across
   matching parents, in document order → `child.field` resolution (unchanged).

## Postconditions (delta from spec `008`)

- **(New)** For a query whose parent type is ambiguous in `p`, `values` now reflects
  each candidate occurrence's *real* children — no longer unconditionally `Ok(vec![])`
  — matching the real Scala engine's own output for the equivalent message/profile
  (research.md #1).
- **(Reaffirmed, unchanged)** For a query whose parent type is unambiguous in `p`,
  `values` is byte-for-byte identical to spec `008`'s existing behavior (FR-006;
  data-model.md's equivalence argument).
- **(Reaffirmed, unchanged)** Never panics; every `&'m str` still borrows directly
  from `scan.message`; still no full-message tree is cached or retained across calls
  for the unambiguous path. For the ambiguous path, the per-call classification walk
  touches segment metadata already produced by spec `005`'s scanner — no new
  allocation proportional to field count, only (at most) one small resolved-node
  record per segment occurrence actually classified.

## Verification

Per research.md #1, correctness for this contract is checked against the real
`gov.cdc:hl7-pet_2.13:1.2.11` engine's own output for
`fixtures/messages/complex-hierarchy.hl7` + `fixtures/profiles/deep-nested.json` (the
same pair already in this repo's fixtures, already annotated with line numbers),
extended with new `fixtures/vectors/hierarchy/` entries for `OBX`/`NTE`-as-parent
PATHs (research.md #5) — not a new, separately-sourced fixture.

## New semantic rule: `A.7-ambiguous-parent-resolution`

`fixtures/schemas/hierarchy-conformance-vector.schema.json`'s `semantic_rules` enum
gains one new value, `A.7-ambiguous-parent-resolution`, for vectors exercising this
spec's fix. Registered here rather than by editing spec `002`'s own
`contracts/hierarchy-semantics.md` — that document is Section A's historical record of
*current behavior as of spec `002`'s own writing* (Section A never gained new entries
when spec `008` later fixed Section A.4's child-index bug either; that fix is recorded
in spec `008`'s own contract and `ROADMAP.md`'s Documented Breaking Changes table, the
same pattern this spec follows).

**A.7 — Ambiguous parent-side type resolution**: When a `->` expression's parent-side
segment type occupies more than one position in the loaded profile's tree, each
selected occurrence resolves to its real tree position using the message's actual
document order (the same nearest-enclosing-ancestor rule as A.1, extended to the
parent side — see this document's "Revised behavior" section above), rather than
`node_for`'s O(1) lookup unconditionally yielding no match. An occurrence that
corresponds to no legal position given the real structure contributes no children
(absence, not an error — consistent with A.1's unrecognized-segment-dropped rule).
