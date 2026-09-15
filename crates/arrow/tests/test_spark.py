"""PySpark DataFrame column wiring smoke test (spec 6002-arrow-integration,
Story 3, quickstart.md step 4). A local (`local[1]`) session applies
`hl7pet_arrow.spark.extract_value_udf`/`extract_values_udf` to a small
DataFrame built from `fixtures/messages/` and confirms `.collect()`
matches the non-Spark `hl7pet_arrow.extract_value`/`extract_values`
results for the same messages/PATHs (Story 1 Acceptance Scenario 4,
Story 3 Acceptance Scenarios 1-2).

Also covers the post-6002-merge simplified UDFs (`first_value_udf`/
`values_udf`/`first_values_udf`), which return plain columns directly --
no struct navigation on the Spark side.
"""

from __future__ import annotations

from pathlib import Path

import pyarrow as pa
import pytest

import hl7pet_arrow as ha
import hl7pet_arrow.simplify as hs
import hl7pet_arrow.spark as has

pyspark = pytest.importorskip("pyspark")
from pyspark.sql import SparkSession  # noqa: E402

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"


@pytest.fixture(scope="module")
def spark():
    session = (
        SparkSession.builder.appName("hl7pet_arrow-test_spark")
        .master("local[1]")
        .config("spark.ui.enabled", "false")
        .getOrCreate()
    )
    session.sparkContext.setLogLevel("ERROR")
    yield session
    session.stop()


def _messages() -> list[str | None]:
    return [
        (FIXTURES / "messages/baseline.hl7").read_text(),
        (FIXTURES / "messages/filter-example.hl7").read_text(),
        None,
    ]


def test_extract_value_udf_matches_standalone_extract_value(spark):
    messages = _messages()
    df = spark.createDataFrame([(m,) for m in messages], ["message"])

    spark_result = (
        df.withColumn("result", has.extract_value_udf("MSH-12")(df["message"]))
        .select("result")
        .collect()
    )
    spark_values = [row["result"].asDict() for row in spark_result]

    standalone = ha.extract_value(pa.array(messages), "MSH-12")
    standalone_values = [row.as_py() for row in standalone]

    assert spark_values == standalone_values


def test_extract_values_udf_matches_standalone_extract_values(spark):
    messages = _messages()
    paths = ["MSH-12", "PID-5.1"]
    df = spark.createDataFrame([(m,) for m in messages], ["message"])

    spark_result = (
        df.select(has.extract_values_udf(paths)(df["message"]).alias("result"))
        .collect()
    )
    spark_values = [
        {path: row["result"][path].asDict() for path in paths} for row in spark_result
    ]

    standalone = ha.extract_values(pa.array(messages), paths)
    standalone_values = [
        {path: standalone.field(i)[row_idx].as_py() for i, path in enumerate(paths)}
        for row_idx in range(len(messages))
    ]

    assert spark_values == standalone_values


def test_first_value_udf_matches_standalone_simplify_first_value(spark):
    messages = _messages()
    df = spark.createDataFrame([(m,) for m in messages], ["message"])

    spark_result = df.withColumn(
        "MSH_12", has.first_value_udf("MSH-12")(df["message"])
    ).select("MSH_12").collect()
    spark_values = [row["MSH_12"] for row in spark_result]

    standalone = hs.first_value(ha.extract_value(pa.array(messages), "MSH-12")).to_pylist()

    assert spark_values == standalone


def test_values_udf_matches_standalone_simplify_values(spark):
    messages = _messages()
    df = spark.createDataFrame([(m,) for m in messages], ["message"])

    spark_result = df.withColumn(
        "OBX_5", has.values_udf("OBX-5")(df["message"])
    ).select("OBX_5").collect()
    spark_values = [row["OBX_5"] for row in spark_result]

    standalone = hs.values(ha.extract_value(pa.array(messages), "OBX-5")).to_pylist()

    assert spark_values == standalone


def test_first_values_udf_gives_one_plain_column_per_path(spark):
    messages = _messages()
    paths = ["MSH-12", "PID-5.1"]
    df = spark.createDataFrame([(m,) for m in messages], ["message"])

    wide = df.select(
        has.first_values_udf(paths)(df["message"]).alias("result")
    ).select("result.*")
    spark_values = [row.asDict() for row in wide.collect()]

    standalone = ha.extract_values(pa.array(messages), paths)
    standalone_values = [
        {path: hs.first_value(standalone.field(i))[row_idx].as_py() for i, path in enumerate(paths)}
        for row_idx in range(len(messages))
    ]

    assert spark_values == standalone_values
    assert set(wide.schema.fieldNames()) == set(paths)


def test_first_values_udf_rejects_empty_paths_list():
    with pytest.raises(ValueError, match="paths must not be empty"):
        has.first_values_udf([])
