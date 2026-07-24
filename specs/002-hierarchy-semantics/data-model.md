# Data Model: Hierarchy-Mode Semantics Specification

This feature produces documentation/data artifacts, not a running system, so
"entities" here are the conceptual objects the semantics document and conformance
vectors are built from — not database tables or runtime objects. Entities already
defined by spec `001` (PATH Expression, Grammar Production, Filter Clause, Synthetic
HL7 Message) are reused, not redefined.

## Entities

### Segment Hierarchy Tree

The parent→child structure built from a profile's `segmentDefinition` at
hierarchy-construction time (today's Scala behavior) or navigated contextually,
lazily, per query (candidate Rust behavior, per `contracts/hierarchy-semantics.md`
Section B.1).

| Field | Description |
|---|---|
| `root` | Synthetic node, not itself a real segment; its children are the profile's top-level `segmentDefinition` entries |
| `nodes` | Each a Segment Hierarchy Node |

**Relationships**: built from a Profile (specifically its `segmentDefinition`) applied
to a Synthetic HL7 Message; navigated by `->` PATH Expressions.

**Validation rules**: constructed only via the nearest-enclosing-ancestor rule
(`contracts/hierarchy-semantics.md` Section A.1) — a segment not recognized as a child
anywhere up the current ancestor chain is dropped from the tree, never an error.

### Segment Hierarchy Node

One occurrence of a segment within the tree.

| Field | Description |
|---|---|
| `line_number` | 1-based source line number, matching spec `001` FR-008's line-number convention |
| `segment_type` | 3-letter segment name |
| `children` | Ordered list of child Segment Hierarchy Nodes (direct children only) |

**Relationships**: exactly one parent (except `root`); zero or more children.

**Validation rules**: a node's children are determined solely by the profile's
`children` map for that node's `segment_type` — a node's `SegmentConfig.cardinality`
does not affect which children it has (Section A.3: cardinality is not read during
tree construction).

### Parent Occurrence

The specific Segment Hierarchy Node(s) selected by a `->` expression's parent-side
`SEGMENT_EXPR` (segment name + `SEG_IDX`).

| Field | Description |
|---|---|
| `segment_type` | Matched against every node in the tree, at any depth (recursive search, not root-only) |
| `selected_by` | The `SEG_IDX` selector applied — evaluated over *all* occurrences of `segment_type` in the message, not scoped to any subtree |

**Relationships**: zero or more Segment Hierarchy Nodes; its children (one level,
combined across all selected occurrences) form the candidate pool for the child-side
match (see Hierarchy Conformance Vector's `A.2-multi-parent-combined-children` rule and
`A.4-cross-parent-child-indexing` limitation).

### SegmentConfig / Cardinality

The `[m..n]` occurrence-count contract attached to a segment within
`segmentDefinition`.

| Field | Description |
|---|---|
| `cardinality` | e.g. `[1..1]`, `[0..*]` |
| `children` | Map of child segment name → nested SegmentConfig (defines legal tree structure, consumed by Segment Hierarchy Tree construction) |

**Relationships**: `children` drives Segment Hierarchy Tree construction (Section A.1);
`cardinality` is consumed *only* by the separate Validation module
(`StructureValidator`), as a whole-message per-segment-type count — never scoped to a
specific Parent Occurrence, and never read during `->` navigation itself (Section A.3).

**Validation rules**: an invalid `cardinality` string is a structural-precondition
violation (`HL7ParseError`, Constitution Principle III), consistent with `SPEC.md` §6 —
this spec does not redefine that, only clarifies navigation never touches this field.

### Hierarchy Conformance Vector

One machine-checkable test case for `->` evaluation. Schema matches spec FR-007 and
`contracts/hierarchy-conformance-vector.schema.json` exactly — this table is the
human-readable mirror of that schema.

| Field | Type | Required | Notes |
|---|---|---|---|
| `id` | string | Yes | Unique across the whole vector suite; distinct namespace from spec `001`'s `path-*` ids (e.g. `hier-001`) |
| `path` | string | Yes | The PATH Expression under test, using `->` |
| `profile_ref` | string | Yes | Relative path to a synthetic `segmentDefinition` profile — new relative to spec `001`, since flat paths need no profile |
| `message_ref` | string | Yes | Relative path to a Synthetic HL7 Message |
| `method` | `"getValue"` \| `"getFirstValue"` | Yes | |
| `flags` | object | No | e.g. `{"removeEmpty": true}` |
| `expected` | 2D array of strings, or a single string (for `getFirstValue`) | Yes | |
| `expected_lines` | array mirroring `expected`'s shape, of 1-based ints | No | |
| `semantic_rules` | array of rule identifiers from `contracts/hierarchy-semantics.md` (e.g. `A.2-single-hop-basic`) | Yes | Drives SC-003 coverage tracking, playing the role spec `001`'s `grammar_productions` played there |
| `known_limitation` | Documented Limitation name, or null | No | e.g. `A.4-cross-parent-child-indexing` |

**Validation rules**:
- `id` MUST be unique within the suite and MUST NOT collide with spec `001`'s vector
  ids if the two suites are ever merged by spec `003`.
- If `method` is `"getFirstValue"`, `expected` MUST be a scalar string, not a 2D array.
- Every message a vector's `message_ref` points to MUST be synthetic (FR-012); every
  profile a vector's `profile_ref` points to MUST also be synthetic/fabricated, never a
  real jurisdiction profile copied from the external Scala repo.

**Lifecycle** (state transitions) — identical to spec `001`'s Conformance Vector
lifecycle:

```text
Drafted --(run against real Scala library, SC-004)--> Verified
Drafted --(run against real Scala library, SC-004)--> Discrepancy Found
Discrepancy Found --(FR-011: raised as [NEEDS CLARIFICATION])--> Escalated
Escalated --(human decision made)--> Verified
```

### Profile Requirement Decision

The researched, documented answer to whether Rust hierarchy navigation mandates an
explicit profile (spec FR-004).

| Field | Description |
|---|---|
| `decision` | "Profile required; full eager tree materialization not required" (see `contracts/hierarchy-semantics.md` Section B.1) |
| `rationale` | Recorded in `research.md` Decision 2 |

**Relationships**: informs spec `008`'s planned API surface (profile-required
constructor/navigation entry points).

### Multi-Level Navigation Decision

The researched, documented answer to whether `->` supports more than one hop, and the
performance rationale behind it (spec FR-005).

| Field | Description |
|---|---|
| `decision` | "Recommend inclusion, as a Backward-Compatible Addition to `CHILD_PATH`" (see `contracts/hierarchy-semantics.md` Section B.2) |
| `performance_claim` | "O(message size) total across all hops, hops partition rather than repeat the scan" — falsifiable once spec `008` exists to benchmark (spec `009`) |
| `rationale` | Recorded in `research.md` Decision 4 |

**Relationships**: extends spec `001`'s `CHILD_PATH` production if adopted; informs
spec `008`'s implementation scope and spec `009`'s benchmark design.
