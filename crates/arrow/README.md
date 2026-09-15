# hl7pet_arrow (PyArrow / PySpark)

Apache Arrow integration for [`hl7pet-core`](../core) (spec
`6002-arrow-integration`, Phase 4 of `HL7-PET-Rust-Migration-Plan.md`):
extract HL7 v2 field values from a whole Arrow/PyArrow column of messages
at once, zero-copy across the PyO3 boundary via the Arrow C Data Interface
(`pyo3-arrow`), usable standalone or as PySpark DataFrame column
operations. Depends on `hl7pet-core` only at the Cargo level, and on the
existing [`hl7pet`](../python) package at the Python level (reuses its
`Hl7PathError`/`Hl7ProfileError` exceptions — see below).

## Install (development)

```bash
python3 -m venv .venv-arrow && source .venv-arrow/bin/activate
pip install maturin "pyarrow>=18"

# hl7pet_arrow depends on hl7pet at the Python level -- build it first,
# into the same venv.
(cd ../python && maturin develop)
maturin develop   # builds hl7pet-core + this crate, installs `hl7pet_arrow`
```

## Quickstart

```python
import pyarrow as pa
import hl7pet_arrow as ha

messages = pa.array([open("message1.hl7").read(), open("message2.hl7").read(), None])

# Single-PATH: one Arrow StructArray back, {value, status} per row.
result = ha.extract_value(messages, "MSH-12")
result.to_pylist()
# -> [{'value': [['2.5.1']], 'status': 'ok'},
#     {'value': [['2.5.1']], 'status': 'ok'},
#     {'value': None, 'status': 'no_match'}]      # a null message row, not an error

# Multi-PATH: one scan per message, one outer struct field per requested
# PATH (by position -- duplicate PATHs are allowed and each is computed
# independently; a Result Struct's own status distinguishes "no match"
# from "row failed to scan", never a raised exception mid-batch).
multi = ha.extract_values(messages, ["MSH-12", "PID-5.1"])
multi.field(0).to_pylist()   # MSH-12 column
multi.field(1).to_pylist()   # PID-5.1 column

# Hierarchy-mode ("->") PATHs need the same segmentDefinition profile the
# plain hl7pet.get_value_hierarchy already consumes.
import json
profile = json.load(open("profile.json"))
obs = ha.extract_value(messages, "OBR[1] -> OBX-5", profile)
```

A call-level precondition (an invalid PATH, an empty `paths` list, a
hierarchy PATH with no profile) raises immediately, before any row is
touched -- `Hl7PathError`/`Hl7ProfileError` are the same exception types
the plain `hl7pet` package raises, imported at the PyO3 boundary rather
than re-declared:

| Raised for | Exception |
|---|---|
| Syntactically invalid PATH | `hl7pet.Hl7PathError` |
| Hierarchy PATH with missing/invalid profile | `hl7pet.Hl7ProfileError` |
| Empty `paths` list (`extract_values` only) | `ValueError` (builtin -- no `hl7pet-core` equivalent to mirror) |

A **per-row** structural failure (a message that fails to scan, or a
non-numeric filter comparison) never raises -- it's that row's own
`status: "error"` in the Result Struct, distinct from `"no_match"`,
without the caller catching an exception mid-batch. See
[`specs/6002-arrow-integration/contracts/arrow-api.md`](../../specs/6002-arrow-integration/contracts/arrow-api.md)
for the full contract and
[`specs/6002-arrow-integration/data-model.md`](../../specs/6002-arrow-integration/data-model.md)
for the Result Struct's exact shape.

## Located extraction (source line numbers)

`extract_value_located`/`extract_values_located` are the located
counterparts of `extract_value`/`extract_values` -- same PATH/profile
handling and per-row `status`, but each matched segment occurrence is
paired with its 1-based source line (mirroring the plain binding's
`get_value_located`/`LocatedValue`, specs `1000`/`011`):

```python
obs = ha.extract_value_located(messages, "OBR[1] -> OBX-5", profile)
obs.to_pylist()
# -> [{'values': [{'value': ['POS'], 'line': 5}, {'value': ['NEG'], 'line': 6}],
#      'status': 'ok'}]
```

`line` lives once per occurrence, not once per repetition -- a repeating
field (`value` with more than one entry) still carries a single `line`,
since every repetition within one segment occurrence always shares it
(`hl7pet-core`'s own guarantee, not assumed here).

## Plain-column helpers (`hl7pet_arrow.simplify`)

For a caller who doesn't need the `status`/no-match-vs-error distinction
and just wants a column to drop straight into a DataFrame, built on top of
`extract_value`/`extract_values`' output with no new native code:

```python
import hl7pet_arrow.simplify as hs

result = ha.extract_value(messages, "OBX-5")
hs.values(result)        # List<List<Utf8>> column, status dropped
hs.first_value(result)   # flat Utf8 column: first repetition of first occurrence

# Works on one field of extract_values' struct-of-structs too:
multi = ha.extract_values(messages, ["OBX-5", "MSH-12"])
hs.first_value(multi.field("MSH-12"))
```

Both collapse `"no_match"` and `"error"` rows to `null` -- if you need to
tell those apart, use `extract_value`/`extract_values` directly.

## PySpark

```python
from pyspark.sql import SparkSession
import hl7pet_arrow.spark as has

spark = SparkSession.builder.master("local[1]").getOrCreate()
df = spark.createDataFrame([(msg,) for msg in my_messages], ["message"])

df.withColumn("result", has.extract_value_udf("PID-5.1")(df["message"]))
df.select(has.extract_values_udf(["PID-5.1", "MSH-9"])(df["message"]).alias("result"))
```

Both factories wrap `extract_value`/`extract_values` as `arrow_udf`-based
PySpark column functions (PySpark 4.2's Arrow-native UDF API -- no pandas
conversion step). One limitation: `extract_values_udf`'s return type is a
Spark struct type with one named field per PATH, which requires distinct
field names -- unlike the standalone `extract_values`, it does not support
duplicate PATHs.

## Testing

```bash
pytest tests/                # fixtures-corpus parity + PySpark wiring tests
cargo test -p hl7pet-arrow   # Rust unit tests, including the SC-002 scan-count proof
```

`tests/test_parity.py` verifies this crate's output against the shared
`fixtures/vectors/{path,hierarchy}/` corpus, cross-checked against the
existing plain `hl7pet.get_value`/`get_value_hierarchy`. A handful of
vectors are documented, pinned-by-id exclusions (`getFirstValue`-only
vectors, `buildHierarchy: false`, multi-hop `->` chaining) -- see the
file's module docstring for the exact list and why each is out of scope,
not a gap. `tests/test_located.py` runs the identical corpus/exclusion set
against `extract_value_located`, cross-checked against
`hl7pet.get_value_located`/`get_value_hierarchy_located`'s actual line
numbers, not just that a line is present. `tests/test_simplify.py` covers
the `hl7pet_arrow.simplify` helpers.

## Demo notebook

[`notebooks/arrow_pyspark_demo.ipynb`](../../notebooks/arrow_pyspark_demo.ipynb)
(repo root) exercises every mechanism above against a standalone PyArrow
Table and a local PySpark session, ending with a side-by-side comparison
against the row-by-row plain `hl7pet` binding.
