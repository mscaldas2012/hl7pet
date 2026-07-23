# Data Model: PATH Grammar Specification & Conformance Vectors

This feature produces documentation/data artifacts, not a running system, so
"entities" here are the conceptual objects the grammar document and conformance
vectors are built from — not database tables or runtime objects.

## Entities

### PATH Expression

A string addressing data within an HL7 message.

| Field | Description |
|---|---|
| `raw` | The literal PATH string, e.g. `OBX[2]-5.1` |
| `segment` | 3-letter segment name (`SEG` production) |
| `segment_index` | Optional occurrence selector: a number, `$LAST`, `*`, or a `FILTER` clause |
| `field` | Optional field number |
| `field_index` | Optional field repetition selector: a number or `$LAST` |
| `component` | Optional component number |
| `subcomponent` | Optional subcomponent number |
| `hierarchy_child` | Optional nested `CHILD_PATH` after `->` (syntax only — see spec FR-002; evaluation semantics are spec `002`'s scope) |

**Relationships**: composed of one or more Grammar Productions; evaluated (conceptually,
by whatever engine implements the grammar) against a Synthetic HL7 Message to produce
the `expected` result recorded on a Conformance Vector.

**Validation rules**: every `PATH Expression` used in a Conformance Vector MUST be
classifiable as exactly one of: (a) syntactically valid and matching data, (b)
syntactically valid and matching no data — zero-values result, MUST NOT be treated as
invalid (see spec Edge Cases, `XYZ-99` example), or (c) syntactically invalid — a
genuine grammar violation (see spec Edge Cases, `999[1].1.1` example).

### Grammar Production

A named formal rule in the PATH grammar.

| Field | Description |
|---|---|
| `name` | One of: `PATH`, `SEGMENT_EXPR`, `SEG`, `SEG_IDX`, `FILTER`, `OPERATOR`, `FIELD_EXPR`, `FIELD_IDX`, `CHILD_PATH`, `field_num`, `comp_num`, `subcomp_num` (FR-001) |
| `definition` | The EBNF rule body, as written in `contracts/path-grammar.md` |
| `references` | Other production names this rule's body refers to (used to check FR-001/SC-002's "no undefined references" requirement) |

**Relationships**: referenced by PATH Expressions when parsed; tracked by Conformance
Vector's `grammar_productions` field for SC-003 coverage counting.

**Validation rules**: every name in `references` MUST resolve to another Grammar
Production defined in this document, or be explicitly delegated to spec `002` (only
true for the hierarchy evaluation semantics behind `CHILD_PATH`/`->`, per FR-002) —
zero dangling references (SC-002).

### Conformance Vector

One machine-checkable test case. Schema matches spec FR-011 and
`contracts/conformance-vector.schema.json` exactly — this table is the human-readable
mirror of that schema, not a separate definition.

| Field | Type | Required | Notes |
|---|---|---|---|
| `id` | string | Yes | Stable, unique across the whole vector suite |
| `path` | string | Yes | The PATH Expression under test |
| `message_ref` | string | Yes | Relative path to a Synthetic HL7 Message |
| `method` | `"getValue"` \| `"getFirstValue"` | Yes | |
| `flags` | object | No | e.g. `{"removeEmpty": true}` |
| `expected` | 2D array of strings, a single string (for `getFirstValue`), or the literal `"INVALID"` | Yes | |
| `expected_lines` | array mirroring `expected`'s shape, of 1-based ints | No | Per FR-008 |
| `grammar_productions` | array of Grammar Production names | Yes | Drives SC-003 coverage |
| `known_limitation` | Known Limitation name, or null | No | |

**Validation rules**:
- `id` MUST be unique within the suite.
- If `method` is `"getFirstValue"`, `expected` MUST be a scalar string, not a 2D array.
- If `path` is syntactically invalid (see PATH Expression validation rules), `expected`
  MUST be exactly `"INVALID"` and `grammar_productions` MUST name the production(s)
  whose constraint is violated.
- If `path` is syntactically valid but matches no data (e.g. `XYZ-99`), `expected` MUST
  be an empty array (or empty per the `method`'s shape) — never `"INVALID"`.
- Every message a vector's `message_ref` points to MUST be synthetic (FR-009).

**Lifecycle** (state transitions):

```text
Drafted --(run against real Scala library, SC-004)--> Verified
Drafted --(run against real Scala library, SC-004)--> Discrepancy Found
Discrepancy Found --(FR-010: raised as [NEEDS CLARIFICATION])--> Escalated
Escalated --(human decision made)--> Verified
```

A vector is not considered complete (part of this spec's deliverable) until it reaches
`Verified`. No vector ships in the `Drafted` or `Escalated` state (SC-004).

### Filter Clause

The `@field[.comp] OPERATOR 'value'` sub-structure used inside a `SEG_IDX`.

| Field | Description |
|---|---|
| `field_num` | Field number being filtered on |
| `comp_num` | Optional component number |
| `operator` | One of `=`, `!=`, `>`, `>=`, `<`, `<=` |
| `values` | One or more literal values, `\|\|`-separated for OR semantics |

**Relationships**: sub-structure of `SEG_IDX` (a Grammar Production).

### Known Limitation

A documented constraint from `SPEC.md` §7, relevant here only if it's a syntax-level
constraint (evaluation-time limitations, like undecoded escape sequences, are out of
this spec's scope per spec Edge Cases — that's spec `1001`'s territory).

| Field | Description |
|---|---|
| `name` | Short identifier, e.g. `"single-clause-filter"` |
| `description` | Prose from `SPEC.md` §7 |
| `classification` | Always `"syntax-level"` for entries referenced by this spec's vectors |

**Relationships**: referenced by zero or more Conformance Vectors via
`known_limitation`.

### Synthetic HL7 Message

A fabricated (non-real-patient) HL7 v2 message used as a Conformance Vector's input.

| Field | Description |
|---|---|
| `path` | File path under `messages/` |
| `content` | Raw HL7 v2 text |

**Validation rules**: MUST NOT contain real patient data, even de-identified (FR-009).
Referenced by one or more Conformance Vectors via `message_ref`.
