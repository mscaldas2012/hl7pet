"""`get_value`/`get_first_value` benchmarks (spec 6001-python-ffi-benchmark)
— reuses `crates/core/benches/extraction.rs`'s exact representative-field
mapping and PATH forms (research.md #2), calling the `hl7pet` Python
binding instead of `hl7pet_core` directly.
"""

from __future__ import annotations

import hl7pet

from common.corpus import Corpus, CorpusMessage
from common.output import EngineFailure, MetricValue, ResultRow
from common.timing import sample

GET_FIRST_VALUE_PATH = "PID-5.1"

# Mirrors extraction.rs's representative_field() exactly.
_REPRESENTATIVE_FIELD = {
    "ADT^A01": "PV1-3.1",
    "ADT^A08": "PV1-3.1",
    "ORU^R01": "OBX-5",
    "ORU^R01^HIERARCHY": "OBX-5",
    "VXU^V04": "RXA-5.2",
    "ORM^O01": "OBR-4.2",
}


def _row(feature: str, message_id: str, path_expr: str, stats) -> ResultRow:
    return ResultRow(
        feature=feature,
        message_id=message_id,
        path_expression=path_expr,
        throughput=MetricValue(stats.throughput_ops_per_us, "ops/us"),
        latency_p50=MetricValue(stats.p50_us, "us"),
        latency_p95=MetricValue(stats.p95_us, "us"),
    )


def _run_get_value(
    results: list[ResultRow], failures: list[EngineFailure], message: str, message_id: str, path_expr: str
) -> None:
    try:
        stats = sample(lambda: hl7pet.get_value(message, path_expr))
    except Exception as e:  # noqa: BLE001
        failures.append(
            EngineFailure(
                message_id=message_id,
                feature="getValue",
                description=f"get_value(message, {path_expr!r}) failed: {e}",
            )
        )
        return
    results.append(_row("getValue", message_id, path_expr, stats))


def _run_get_first_value(
    results: list[ResultRow], failures: list[EngineFailure], message: str, message_id: str
) -> None:
    try:
        stats = sample(lambda: hl7pet.get_first_value(message, GET_FIRST_VALUE_PATH))
    except Exception as e:  # noqa: BLE001
        failures.append(
            EngineFailure(
                message_id=message_id,
                feature="getFirstValue",
                description=f"get_first_value(message, {GET_FIRST_VALUE_PATH!r}) failed: {e}",
            )
        )
        return
    results.append(_row("getFirstValue", message_id, GET_FIRST_VALUE_PATH, stats))


def _run_representative(
    results: list[ResultRow], failures: list[EngineFailure], message: CorpusMessage
) -> None:
    field = _REPRESENTATIVE_FIELD.get(message.message_type)
    if field is None:
        return

    segment = field.split("-")[0].split("[")[0]

    # Plain field (FR-004).
    _run_get_value(results, failures, message.content, message.message_id, field)

    # Indexed segment selector (FR-004): same field expression, segment
    # gets an explicit [1].
    field_suffix = field[len(segment):]
    indexed_path = f"{segment}[1]{field_suffix}"
    _run_get_value(results, failures, message.content, message.message_id, indexed_path)

    # Filter clause (FR-004): only for OBX-bearing (ORU^R01-shaped)
    # messages, where OBX-1 (the set id) reliably starts at "1".
    if segment == "OBX":
        _run_get_value(results, failures, message.content, message.message_id, "OBX[@1='1']-5")


def run(corpus: Corpus) -> tuple[list[ResultRow], list[EngineFailure]]:
    results: list[ResultRow] = []
    failures: list[EngineFailure] = []

    for message in corpus.representative_typical_per_type():
        _run_get_first_value(results, failures, message.content, message.message_id)
        _run_representative(results, failures, message)

    large = corpus.unique_by_size_category("large-high-repetition")
    _run_get_value(results, failures, large.content, large.message_id, "OBX-5")

    minimal = corpus.unique_by_size_category("minimal")
    _run_get_first_value(results, failures, minimal.content, minimal.message_id)

    return results, failures
