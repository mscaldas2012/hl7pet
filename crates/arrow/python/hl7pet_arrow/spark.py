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
this Spark-facing wrapper.
"""

from __future__ import annotations

from typing import Any

import pyarrow as pa
from pyspark.sql.functions import arrow_udf

from hl7pet_arrow import extract_value, extract_values

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
