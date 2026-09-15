# Contract: `hl7pet_arrow` Public API

Package: `hl7pet_arrow` (own `maturin` wheel, `crates/arrow`, independent of
the existing `hl7pet` package per plan.md's Structure Decision). Compiled
extension module: `hl7pet_arrow._hl7pet_arrow` (PyO3, mirrors `crates/python`'s
`_hl7pet` naming convention); re-exported from `hl7pet_arrow/__init__.py`.

Every function below raises before touching any row's data — never
mid-batch — for a call-level precondition violation (data-model.md): an
invalid PATH, an empty PATH list, a hierarchy PATH with no profile, or an
invalid profile JSON. These reuse the existing binding's typed exceptions
(`Hl7PathError`, `Hl7ProfileError`), imported from `hl7pet` so callers
already handling those exceptions elsewhere don't need a second exception
hierarchy for the Arrow surface (Backward-Compatible Additions convention —
this is additive reuse, not a new one). This makes `hl7pet` (spec `6000`'s
package, the `crates/python` wheel) a **runtime Python dependency of
`hl7pet_arrow`**, declared in `crates/arrow/pyproject.toml`'s `dependencies`
(plan.md Primary Dependencies) — not a Cargo dependency, since
`crates/python` builds a `cdylib` with no `lib`/`rlib` output for another
Rust crate to link against. `hl7pet_arrow`'s Rust code obtains the exception
type objects the same way `crates/python`'s own `get_value_hierarchy`
already imports the stdlib `json` module — `py.import("hl7pet")?` at the
PyO3 boundary, not a Rust-level type reference.

## `extract_value`

```python
def extract_value(
    messages: pyarrow.Array,           # Utf8/LargeUtf8, nullable entries OK
    path: str,
    profile: dict | None = None,       # required iff `path` uses "->"
) -> pyarrow.StructArray:              # one Result Struct per row (data-model.md)
    ...
```

- **Given** a well-formed `messages` array and a non-hierarchy `path`,
  **returns** a `StructArray` the same length as `messages`, one
  `{value, status}` entry per row (spec Story 1, Acceptance Scenario 1).
- **Given** a hierarchy-mode `path` (`->`) and a `profile`, **behaves**
  identically except hierarchy-navigated per Story 1 Acceptance Scenario 2.
- **Given** a hierarchy-mode `path` with `profile=None`, **raises**
  `Hl7ProfileError` immediately (no partial row processing).
- **Given** a syntactically invalid `path`, **raises** `Hl7PathError`
  immediately.
- **Given** a null entry in `messages` at row *i*, **produces**
  `{value: null, status: "no_match"}` at row *i* — a missing message is
  treated the same as "nothing to match against," not an error, since
  there is no message content to have failed to scan.

## `extract_values`

```python
def extract_values(
    messages: pyarrow.Array,
    paths: list[str],                  # non-empty; duplicates allowed (spec Edge Cases)
    profile: dict | None = None,       # required iff any path uses "->"
) -> pyarrow.StructArray:              # struct-of-structs, one outer field per path
    ...
```

- **Given** `paths = ["PID-5.1", "MSH-9"]`, **returns** a `StructArray`
  with two top-level fields named `"PID-5.1"` and `"MSH-9"`, each an inner
  Result Struct (data-model.md). A caller narrows to one field the same way
  they'd narrow any Arrow/Spark struct column: `result.field("PID-5.1")`
  (PyArrow) or `.select("result.`PID-5.1`")` (Spark, backtick-quoted since
  a PATH string can contain characters that aren't bare identifiers).
- **Given** `paths = []`, **raises** `ValueError` immediately (FR-008) —
  a plain Python exception, not an `Hl7*Error`, since this is a Python-API
  argument-shape violation with no `hl7pet-core` equivalent to mirror
  (there is no "empty PATH list" concept inside `hl7pet-core` itself).
- **Given** the same `path` string twice in `paths`, **returns** two
  independent outer fields, both computed (spec Edge Cases) — field
  *position*, not name uniqueness, is what a duplicate-safe caller should
  rely on; `pyarrow.StructArray.field(i)` (positional) works even when two
  fields share a name, `.field("name")` does not.
- Every row across every requested `path` is derived from exactly one scan
  of that row's message (SC-002) — this is an internal implementation
  requirement, verified by `crates/arrow/tests/test_scan_count.py`, not
  something the caller observes in the return shape.

## `extract_value_located` / `extract_values_located`

Post-6002-merge addition, not in the original spec.md — located
counterparts of `extract_value`/`extract_values`, mirroring the plain
binding's `get_value_located`/`get_value_hierarchy_located` (specs
`1000`/`011`) the same way the non-located pair mirrors `get_value`/
`get_value_hierarchy`. Identical signatures, call-level precondition
behavior (raises before any row is touched, same exceptions table below),
and `status` semantics as `extract_value`/`extract_values` — the only
difference is the per-row payload shape:

```python
def extract_value_located(
    messages: pyarrow.Array,
    path: str,
    profile: dict | None = None,
) -> pyarrow.StructArray:              # {values, status} per row -- see below

def extract_values_located(
    messages: pyarrow.Array,
    paths: list[str],
    profile: dict | None = None,
) -> pyarrow.StructArray:              # struct-of-structs, one {values, status} per path
```

Located Result Struct shape (data-model.md's located extension):

```text
Struct<
  values: List<
    Struct<
      value: List<Utf8>   -- non-empty; one entry per field repetition
      line:  UInt64        -- 1-based; shared by every repetition in this occurrence
    >
  >                         -- one entry per matched segment occurrence; null iff status != "ok"
  status: Utf8
>
```

`line` lives once per occurrence, not once per repetition — every
repetition within one segment occurrence provably shares it
(`hl7pet-core`'s own guarantee: `crates/core/src/query.rs`'s
`execute_located_values_from_same_occurrence_share_one_line` test), so
storing it per repetition would be pure redundancy. `extract_values_located`
has the identical duplicate-PATH/field-position behavior as
`extract_values` (spec Edge Cases).

## `hl7pet_arrow.simplify` (pure Python, no new native code)

Post-6002-merge addition. Plain-column helpers over `extract_value`/
`extract_values`' Result Struct output, for a caller who doesn't need the
`status`/no-match-vs-error distinction:

```python
def values(result: pyarrow.StructArray) -> pyarrow.Array:
    """result.field("value") -- List<List<Utf8>>, status dropped."""

def first_value(result: pyarrow.StructArray) -> pyarrow.Array:
    """Flat Utf8: first repetition of first occurrence."""
```

Both accept any Result Struct array — `extract_value(...)`'s direct
output, or one field of `extract_values(...)`'s struct-of-structs
(`multi_result.field(path_or_index)`). Both collapse `"no_match"` and
`"error"` rows to `null` uniformly (matching `get_first_value`'s own
null-for-absence convention) — this is a deliberate simplification, not an
oversight; a caller who needs to tell the two apart uses `extract_value`/
`extract_values` directly. Not applicable to the located Result Struct
shape (`extract_value_located`'s `values` field is a different shape,
`List<Struct<value, line>>`, not `List<List<Utf8>>`).

## `hl7pet_arrow.spark` (pure Python, Story 3)

```python
def extract_value_udf(
    path: str,
    profile: dict | None = None,
) -> pyspark.sql.column.Column:
    """Usable as df.withColumn("result", extract_value_udf("PID-5.1")(df["message"]))."""

def extract_values_udf(
    paths: list[str],
    profile: dict | None = None,
) -> pyspark.sql.column.Column:
    """Usable as df.select(extract_values_udf(["PID-5.1", "MSH-9"])(df["message"]))."""
```

Both are thin factories: given the PATH(s)/profile at UDF-definition time
(mirroring how a Spark UDF is always parameterized before being applied to
a column, since PATHs aren't themselves DataFrame data), each returns an
`arrow_udf`-decorated callable (research.md #4) closing over `hl7pet_arrow.
extract_value`/`extract_values`. If the research.md #4 struct-output spike
finds `arrow_udf` cannot return a `StructType` column in PySpark 4.2 today,
`extract_values_udf` falls back to `DataFrame.mapInArrow` internally — a
change to this file's implementation notes only, not to the two functions'
signatures or return contract above.

## Existing plain-binding exceptions reused here

| Class | Raised by this API for |
|---|---|
| `hl7pet.Hl7PathError` | Syntactically invalid PATH (either function) |
| `hl7pet.Hl7ProfileError` | Hierarchy PATH with missing/invalid profile |
| `ValueError` (builtin) | Empty `paths` list (`extract_values` only) |

Per-row structural failure (`hl7pet.Hl7ScanError`/`Hl7QueryError`'s
call-level equivalents) is deliberately **not** in this table — it never
raises in this API; it's the `status: "error"` field per data-model.md's
Result Struct (FR-010).
