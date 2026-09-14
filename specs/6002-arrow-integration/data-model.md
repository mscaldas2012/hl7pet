# Phase 1 Data Model: Arrow Integration for PySpark & PyArrow

Source: spec.md Key Entities, resolved against research.md decisions 1-5.
This feature has no persistent storage — every entity below is an in-memory
Arrow value, either passed in by the caller or produced fresh per call from
`hl7pet-core`.

## Message Column (input)

An Arrow `Utf8`/`LargeUtf8` array of raw HL7 v2 message strings, one entry
per row; nullable entries are permitted (FR-007).

| Representation | Arrow type |
|---|---|
| PyArrow, standalone | `pyarrow.Array` (`string`/`large_string`) |
| PySpark column | Spark `StringType` column, seen Arrow-side as `pa.Array` inside an `arrow_udf` |

A null entry at row *i* produces a null `Result Struct` (below) at row *i*
for every requested PATH — never an error, never a dropped row (FR-007).

## PATH Expression / PATH List (input)

Unchanged from the existing binding: a PATH string (optionally using `->`
for hierarchy mode), or a non-empty ordered list of them for multi-PATH
extraction. Validated eagerly, once, before any row is processed (FR-009,
mirrors the existing binding's parse-time validation) — a syntactically
invalid PATH or an empty PATH list raises immediately and no Arrow
computation happens, exactly like a bad argument to a normal function call
(this is a call-level precondition, not a per-row outcome, so it is *not*
expressed via the per-row `status` field below).

## Segment-Hierarchy Profile (input)

Unchanged: the same `segmentDefinition`-shaped `dict`/JSON object already
consumed by `get_value_hierarchy`. One profile per call, applied to every
row and every hierarchy-mode PATH in that call (spec Assumptions). Required
whenever any requested PATH uses `->`; a hierarchy-mode PATH supplied
without a profile raises eagerly, same timing as an invalid PATH above.

## Result Struct (output) — resolves FR-010

The core new design element (research.md #5). A single requested PATH's
per-row outcome is a 2-field Arrow struct:

```text
Struct<
  value:  List<List<Utf8>>   -- nullable; outer = matched segment occurrences
                                 (message document order), inner = field
                                 repetitions -- identical nesting to the
                                 existing plain binding's list[list[str]]
  status: Utf8               -- one of "ok" | "no_match" | "error"
>
```

| `status` | `value` | Meaning | Mirrors (plain binding) |
|---|---|---|---|
| `"ok"` | non-null, non-empty | PATH matched >=1 occurrence | `get_value(...)` returns a non-empty list |
| `"no_match"` | `null` | Row scanned fine; PATH matched nothing | `get_value(...)` returns `None` |
| `"error"` | `null` | Row could not be evaluated — message failed to scan (e.g. missing/truncated MSH), or a filter applied an ordering operator to a non-numeric operand | `get_value(...)` would have raised `Hl7ScanError`/`Hl7QueryError` |

A non-scan structural failure that the existing binding also surfaces as an
exception for a *single* call — `Hl7PathError` (bad PATH), `Hl7ProfileError`
(bad profile) — is **not** a per-row `status` value, because both are
call-level preconditions already rejected eagerly above (FR-008/FR-009),
before any row's `status` is computed. `Hl7QueryError` (non-numeric filter
comparison) *is* per-row like a scan failure, since it depends on a
particular row's field content, not the PATH/profile alone — both fold into
the single `"error"` status rather than adding a fourth value, since both
represent "this row could not be evaluated" from the caller's perspective;
distinguishing the two sub-cases further is not required by any spec FR and
would only add cases a caller has to handle for no behavioral gain. Named
`"error"` rather than `"scan_error"` precisely because it already covers
more than scan failures.

### Single-PATH extraction output (Story 1 / FR-001)

One `Result Struct` array, one entry per input row — itself a single Arrow
`Array` (a `StructArray`), satisfying FR-001's "returns an Arrow array of
results."

### Multi-PATH extraction output (Story 2 / FR-002)

A struct-of-structs: one outer field per requested PATH (field name = the
PATH string itself, or the caller-supplied alias — see contracts/arrow-api.md),
each an inner `Result Struct` as defined above:

```text
Struct<
  "<path_1>": Result Struct,
  "<path_2>": Result Struct,
  ...
>
```

Every input row is scanned exactly once regardless of how many outer
fields are requested (SC-002) — the per-PATH results are all derived from
that one scan before being packed into the struct-of-structs.

## Demo Notebook (artifact, not runtime data)

`notebooks/arrow_pyspark_demo.ipynb` — not a data entity, listed here only
because spec.md's Key Entities section names it as one. Evaluated by
execution (FR-011/SC-005), not by schema.
