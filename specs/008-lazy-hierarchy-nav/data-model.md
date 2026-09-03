# Data Model: Lazy Hierarchy Navigation

Entities carried over from [spec.md](spec.md)'s Key Entities section, made concrete
against [research.md](research.md)'s decisions. Types are given in Rust since this
spec's deliverable is Rust source (`crates/core/src/hierarchy.rs`); see
[contracts/hierarchy-api.md](contracts/hierarchy-api.md) for the full public API
surface. Inputs (`ScanResult`, `CompiledPath` and its constituents, `QueryError`) are
defined by specs `005`-`007` and are not redefined here — only referenced.

## HierarchyProfile

The Rust representation of a `segmentDefinition` map (spec.md's "Hierarchy Profile"
Key Entity) — a small node arena, independent of any specific message, used purely
as a legal-child lookup table (FR-004). Opaque to callers: no field is public.

```rust
pub struct HierarchyProfile {
    nodes: Vec<ProfileNode>,              // index 0 is always the synthetic root
    by_name: HashMap<String, Vec<usize>>, // every position a name occupies
}

struct ProfileNode {
    children: std::collections::HashMap<String, usize>, // segment name -> node index
    parent: Option<usize>,                               // None only for the root
}
```

**Construction**: `HierarchyProfile::from_json(json: &str) -> Result<Self, ProfileError>`
parses `json`'s `segmentDefinition` object (the same shape
`fixtures/profiles/{basic-two-level,deep-nested}.json` already use — a map of
segment name to `{ "cardinality": "...", "children": { ...same shape... } }`)
recursively into the arena, via a crate-private `#[derive(Deserialize)]`
intermediate (`RawProfile`/`RawSegmentDef`, research.md #5) that never escapes this
module. `cardinality` is read and discarded — not stored, not validated (research.md
#2's "stores nothing it does not use").

**A segment name recurring at more than one tree position is allowed** — `by_name`
records every position a name occupies (research.md #2, corrected during
implementation: `deep-nested.json` legitimately places `OBX` and `NTE` at more than
one nesting depth — normal profile data, not malformed). Construction never fails
because of this; it only fails for genuinely invalid JSON (`ProfileError::InvalidJson`).
Construction always fully succeeds or fully fails, never partially builds a profile.

**Derived, precomputed at construction (not stored as separate public fields, but as
the concrete mechanism `hierarchy.rs`'s algorithm depends on)**:
- `node_for(name: &str) -> Option<usize>` — looks up `by_name`, used only to resolve
  a `->` expression's *parent*-side segment type into its one starting node. Returns
  `Some` only when the name maps to exactly one position; `None` both when the name
  is absent and when it is ambiguous (more than one position) — research.md #2's
  corrected design establishes that descendant matching during the bounded scan
  never needs this map at all (each node's own `children` map already disambiguates
  correctly by construction), so only parent-side ambiguity can ever matter here, and
  no existing vector's parent type is ever ambiguous.
- `ancestor_chain(node: usize) -> Vec<usize>` — walks `parent` pointers from `node`
  up to (and including) the root; `O(profile depth)`, computed on demand per query
  rather than cached per node (profile depth is small — the existing fixture
  profiles are at most 3 levels deep — so caching would save negligible work at the
  cost of `O(profile size)` extra memory retained for the profile's lifetime).

## ProfileError

The one non-panic failure output for `HierarchyProfile::from_json` — reserved for a
structurally invalid *profile document*, never returned by `execute_hierarchy`
itself (research.md #3; that function reuses `QueryError` exclusively). A single
variant (research.md #2, corrected during implementation — a repeated segment name
is valid data, not an error condition):

| Variant | Fields | Corresponds to |
|---|---|---|
| `InvalidJson` | `message: String` | The input is not well-formed JSON, or does not match `segmentDefinition`'s expected shape (e.g. `children` present but not an object). Wraps `serde_json::Error`'s `Display` output as an owned `String` — the `serde_json::Error` type itself never crosses this module's public boundary (FR-014). |

Mirrors `ScanError`/`ParseError`/`QueryError`'s existing precedent: `Clone`, `Eq`,
implements `std::error::Error` and `Display` via a manual `impl` — no error-derive
crate dependency beyond what `serde_json` itself already pulls in. Exhaustive — no
catch-all variant.

## Hierarchy Query Result shape

Identical to spec `007`'s `execute()` — `Result<Vec<Vec<&'m str>>, QueryError>`, the
same two-dimensional (occurrences × repetitions) shape, tied to the message's
lifetime `'m` (FR-001, spec `002` Section A.2 step 7: hierarchy scoping is a
selection step before this shape is produced, not a different shape). No new
success type is introduced.

## Matching Parent Occurrence (internal)

Produced identically to spec `007`'s "Occurrence Candidate"
(`resolve_segment_candidates`, now `pub(crate)`, plan.md Project Structure) — this
spec adds no new parent-selection logic, per FR-002's explicit reuse requirement.

## Bounded Child Scan Range (internal)

The mechanism research.md #1 defines: for one `Matching Parent Occurrence`, a local
stack of `usize` node indices (seeded with `[node_for(parent_type)]`) walked forward
over `ScanResult.segments[parent_line + 1 ..]`, using `ancestor_chain(parent_node)`
only at the point the local stack is exhausted (to distinguish "exited the parent's
subtree" from "unrecognized everywhere, silently dropped"). Not a public type — an
algorithm, not a data structure retained past one query.

```text
fn direct_children_of_type<'m>(
    scan: &ScanResult<'m>,
    profile: &HierarchyProfile,
    parent_span: SegmentSpan,
    cseg: &str,
) -> Vec<SegmentSpan> {
    // research.md #1's walk: seed local stack at parent's node, push on match,
    // pop-and-retry within the local stack, and on local-floor exhaustion check
    // the static ancestor chain to decide "stop" vs. "drop and continue."
    // Records a span when the local stack reaches depth 2 (a direct child) and
    // its type equals `cseg` — already type-filtered, per FR-007.
}
```

## Corrected child-index resolution (spec FR-007)

Given, per matching parent occurrence, its `direct_children_of_type(cseg)` list
(already type-filtered and already scoped to that one parent — never combined
across multiple matching parents before indexing, unlike the original Scala
behavior spec `002` Section A.4 documented):

| `csegIdx` | Resolution |
|---|---|
| `None` / `SegIndex::Star` | Every entry in `direct_children_of_type`, in document order. |
| `SegIndex::Numeric(n)` | The entry at 1-based position `n` within `direct_children_of_type` — `vec![]` if `n` is out of range for *this specific parent's* type-filtered list (FR-006; matches spec `007`'s existing out-of-range-is-empty convention, `resolve_segment_candidates`'s `Numeric` arm). |
| `SegIndex::Last` | The last entry of `direct_children_of_type` (empty if the list itself is empty). |
| `SegIndex::Filter(clause)` | `filter_matches` (now `pub(crate)`, unchanged) evaluated against each entry in `direct_children_of_type`, in order — identical semantics to spec `007`'s parent-side filter evaluation, just applied to this narrower, per-parent, type-filtered candidate set. |

Results from each matching parent occurrence are concatenated, in the parent
occurrences' own document order (FR-002), to form the final flat span list before
`resolve_field_values` (spec `007`, now `pub(crate)`) applies `path.child.field`.

## Relationship to specs `005`-`007`'s types

```text
CompiledPath<'p>  (spec 006, borrows PATH string)
  - segment: SegmentExpr<'p> { name, index: Option<SegIndex<'p>> }   // parent side
  - child:   Option<ChildPath<'p>>
               ChildPath { segment: SegmentExpr<'p>, field: Option<FieldExpr> } // child side

execute_hierarchy(scan: &ScanResult<'m>, path: &CompiledPath<'p>,
                   profile: Option<&HierarchyProfile>):
  1. path.child is None -> delegate to query::execute(scan, path) unchanged (profile
     unused; flat paths never touch HierarchyProfile at all)
  2. path.child is Some(child) and profile is None -> Ok(vec![]) (FR-009)
  3. path.child is Some(child) and profile is Some(profile):
     a. resolve_segment_candidates(scan, path.segment.name, path.segment.index)
        (spec 007, pub(crate), unchanged)              -> Matching Parent Occurrences
     b. for each: direct_children_of_type(scan, profile, parent_span, child.segment.name)
        (this spec, research.md #1)                     -> per-parent type-filtered list
     c. apply child.segment.index (csegIdx) per-parent (this spec's corrected table,
        above)                                           -> per-parent selected spans
     d. concatenate across parents, in parent-occurrence document order
     e. resolve_field_values(child.field, ..., scan.delimiters) (spec 007,
        pub(crate), unchanged) per selected span          -> Vec<Vec<&'m str>>
  4. returns Result<Vec<Vec<&'m str>>, QueryError> (research.md #3 — no new error type)
```

The output's lifetime is tied to the message (`ScanResult<'m>`'s `'m`), independent
of the PATH string's lifetime (`CompiledPath<'p>`'s `'p`) and of `HierarchyProfile`'s
own lifetime (fully owned, no borrow from either the message or the PATH string) —
the same independence spec `007`'s data-model.md already established for `'m`/`'p`,
extended here with a third, unrelated, owned input.
