# Data Model: Ambiguous-Position Parent Resolution for Hierarchy PATHs

This spec adds no new persisted or public data type. It extends the meaning of
existing spec `002`/`008` concepts (`HierarchyProfile`, `ScanResult`'s segments) with
one new internal concept needed to resolve them correctly. Field/type names below are
descriptive of the concept, not a mandated Rust signature — `tasks.md` and the
implementation decide exact names.

## Existing entities (unchanged shape, extended behavior)

### `HierarchyProfile` (spec `008`)

Unchanged public shape (`from_json`, opaque node arena). Its internal `by_name: HashMap<String, Vec<usize>>` already records, for every segment type name, every tree
position it occupies (spec `008`'s own doc comment already anticipated this — see
`crates/core/src/hierarchy.rs`'s comment on `by_name`, written when spec `008` first
discovered `deep-nested.json`'s repeated `OBX`/`NTE` placements). This spec is the
first to *use* that "more than one position" case productively instead of folding it
into `node_for`'s `None`.

### `ScanResult` / `SegmentSpan` (spec `005`)

Unchanged. The resolution walk this spec adds reads only what `scan()` already
collected — each segment's name and span — never re-parsing message bytes.

## New concept: Segment Occurrence Classification

Not a new public struct — the *result* of resolving one specific segment occurrence
(identified by its position in `scan.segments`) to the one `HierarchyProfile` tree
node it actually occupies, given the real, complete sequence of segments before it in
the message.

| Aspect | Description |
|---|---|
| Input | A `ScanResult`, a `HierarchyProfile`, and a segment occurrence's index into `scan.segments`. |
| Output | The resolved tree node index (an existing `HierarchyProfile` internal node id), or "unrecognized" if no legal position matches given the real preceding structure (FR-004). |
| Determinism | Total and deterministic — the nearest-enclosing-ancestor walk (same rule as `direct_children_of_type`, and as the real Scala engine's `HL7HierarchyParser`) always produces exactly one answer or "unrecognized," never an ambiguous result requiring a guess (spec.md Edge Cases). |
| Cost | O(number of segments from the top of the message through the target occurrence) when the type is ambiguous; O(1) lookup (today's existing `node_for`) when it is not — the two paths are chosen by an existing, already-cheap check (`by_name[type].len()`). |
| Lifetime | Ephemeral — computed as needed for one `execute_hierarchy` call; not cached or attached to `ScanResult` (research.md #2's deferred future-optimization note). |

### Relationship to existing resolution flow (spec `008`'s data-model.md)

Today: `direct_children_of_type` derives its seed node via
`profile.node_for(parent_type)` — a single global lookup, `None` when ambiguous.

After this fix: for a **parent occurrence** whose type is ambiguous, the seed node
instead comes from that occurrence's own Segment Occurrence Classification (the walk
above), computed once per matching candidate. For a parent occurrence whose type is
*not* ambiguous, the seed node is still `node_for`'s existing O(1) global lookup,
completely unchanged — the classification concept only exists, and is only computed,
for the ambiguous case. Everything downstream of "the seed node is known"
(`direct_children_of_type` itself, `apply_child_index`, field resolution) is
byte-for-byte unchanged code, per plan.md's Constitution Check equivalence argument.

## Key Entities (from spec.md, mapped to code)

- **Hierarchy Profile** → `HierarchyProfile` (spec `008`, unchanged shape).
- **Segment Occurrence** → an index into `ScanResult.segments` (spec `005`,
  unchanged shape); this spec's new logic reads it, never mutates or extends it.
- **Ambiguous Segment Type** → any type name where
  `HierarchyProfile`'s internal `by_name` map has more than one entry — already
  computed today, just newly consulted (rather than short-circuited to `None`) for
  parent-side resolution.
