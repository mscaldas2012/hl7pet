"""PySpark DataFrame column wiring for hl7pet_arrow (spec
6002-arrow-integration, Story 3). See
specs/6002-arrow-integration/contracts/arrow-api.md.

research.md #4 spike (T024), confirmed live against a local PySpark
4.2.0 session: `@pyspark.sql.functions.arrow_udf` DOES support a
struct-typed return (including a struct-of-structs), so
`extract_values_udf` uses it directly -- no `DataFrame.mapInArrow`
fallback needed.

Both factories are thin: given the PATH(s)/profile at UDF-definition time
(a Spark UDF is always parameterized before being applied to a column,
since PATHs aren't themselves DataFrame data), each returns an
`arrow_udf`-decorated closure over `hl7pet_arrow.extract_value`/
`extract_values`.

Known limitation, not silently mishandled: `extract_values_udf`'s return
type is declared as a Spark struct type string with one named field per
requested PATH, so -- unlike the standalone `hl7pet_arrow.extract_values`,
which supports duplicate PATHs by field *position* -- passing the same
PATH twice to `extract_values_udf` is not supported (Spark struct type
strings require distinct field names); pass distinct PATHs when using
this Spark-facing wrapper. The same limitation applies to
`first_values_udf` below, for the same reason.

`first_value_udf`/`values_udf`/`first_values_udf` (post-6002-merge
addition) are the Spark-facing counterparts of `hl7pet_arrow.simplify`:
where `extract_value_udf` returns a struct column the caller has to
navigate (`result.value`/`result.status`), these return a **plain**
column directly -- a flat `string` column for `first_value_udf`, an
`array<array<string>>` column for `values_udf`, and a struct-of-flat-
scalars (one plain `string` field per PATH, not one Result Struct per
PATH) for `first_values_udf`. Each collapses `"no_match"`/`"error"` to
`null` uniformly, same as `hl7pet_arrow.simplify` itself -- use
`extract_value_udf`/`extract_values_udf` instead if that distinction
matters.
"""

from __future__ import annotations

from typing import Any

import pyarrow as pa
from pyspark.sql.functions import arrow_udf

from hl7pet_arrow import extract_value, extract_values
from hl7pet_arrow import simplify as hs

_RESULT_STRUCT_TYPE = "struct<value: array<array<string>>, status: string>"


def extract_value_udf(path: str, profile: dict[str, Any] | None = None) -> Any:
    """Single-PATH extraction as a PySpark column function (Story 1
    Acceptance Scenario 4). Usable as::

        df.withColumn("result", extract_value_udf("PID-5.1")(df["message"]))
    """

    @arrow_udf(_RESULT_STRUCT_TYPE)
    def _extract_value_udf(messages: pa.Array) -> pa.Array:
        return extract_value(messages, path, profile)

    return _extract_value_udf


def extract_values_udf(paths: list[str], profile: dict[str, Any] | None = None) -> Any:
    """Multi-PATH extraction as a PySpark column function (Story 2 usable
    from Spark). Usable as::

        df.select(extract_values_udf(["PID-5.1", "MSH-9"])(df["message"]))

    Requires distinct `paths` -- see module docstring.
    """
    if not paths:
        raise ValueError("paths must not be empty")

    fields = ", ".join(f"`{path}`: {_RESULT_STRUCT_TYPE}" for path in paths)
    return_type = f"struct<{fields}>"

    @arrow_udf(return_type)
    def _extract_values_udf(messages: pa.Array) -> pa.Array:
        return extract_values(messages, paths, profile)

    return _extract_values_udf


def first_value_udf(path: str, profile: dict[str, Any] | None = None) -> Any:
    """Single-PATH extraction as a **plain, flat** PySpark column function
    -- the first field repetition of the first matched occurrence, `null`
    for `"no_match"`/`"error"` rows, no struct navigation needed. Usable
    as::

        df.withColumn("MSH_12", first_value_udf("MSH-12")(df["message"]))
    """

    @arrow_udf("string")
    def _first_value_udf(messages: pa.Array) -> pa.Array:
        return hs.first_value(extract_value(messages, path, profile))

    return _first_value_udf


def values_udf(path: str, profile: dict[str, Any] | None = None) -> Any:
    """Single-PATH extraction as a **plain** `array<array<string>>` column
    -- `status` dropped, `null` for `"no_match"`/`"error"` rows. Usable as::

        df.withColumn("OBX_5", values_udf("OBX-5")(df["message"]))
    """

    @arrow_udf("array<array<string>>")
    def _values_udf(messages: pa.Array) -> pa.Array:
        return hs.values(extract_value(messages, path, profile))

    return _values_udf


def first_values_udf(paths: list[str], profile: dict[str, Any] | None = None) -> Any:
    """Multi-PATH extraction as a struct of **plain, flat** columns -- one
    `string` field per PATH (not one Result Struct per PATH), each the
    first repetition of the first matched occurrence for that PATH. Built
    for exactly the "build a DataFrame with values directly in the
    columns" use case: `.select("result.*")` after this gives one plain
    column per requested PATH, ready to use. Usable as::

        df.select(first_values_udf(["MSH-12", "PID-5.1"])(df["message"]).alias("result"))

    Requires distinct `paths` -- see module docstring.
    """
    if not paths:
        raise ValueError("paths must not be empty")

    fields = ", ".join(f"`{path}`: string" for path in paths)
    return_type = f"struct<{fields}>"

    @arrow_udf(return_type)
    def _first_values_udf(messages: pa.Array) -> pa.Array:
        multi = extract_values(messages, paths, profile)
        columns = [hs.first_value(multi.field(i)) for i in range(len(paths))]
        return pa.StructArray.from_arrays(columns, names=list(paths))

    return _first_values_udf
