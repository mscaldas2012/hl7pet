"""Scan-proxy benchmarks (spec 6001-python-ffi-benchmark, research.md #3).

`hl7pet` exposes no standalone `scan()` (spec 6000 FR-001 only required
extraction, not raw scanner internals), so the "parsing" feature is
measured as `get_first_value(message, "MSH-1")` -- the cheapest real call
the public API offers, still forcing a full scan+parse+execute pass. This
is a documented proxy, not an isolated scan measurement (see the module
docstring in `crates/python/benches/common/output.py`'s caller,
`run_all.py`, and research.md #3 for the full rationale).
"""

from __future__ import annotations

import hl7pet

from common.corpus import Corpus
from common.output import EngineFailure, MetricValue, ResultRow
from common.timing import sample

PARSING_PROXY_PATH = "MSH-1"


def _run_one(message_id: str, message: str) -> tuple[ResultRow | None, EngineFailure | None]:
    try:
        stats = sample(lambda: hl7pet.get_first_value(message, PARSING_PROXY_PATH))
    except Exception as e:  # noqa: BLE001 -- report, never crash the whole run
        return None, EngineFailure(
            message_id=message_id,
            feature="parsing",
            description=f"get_first_value(message, {PARSING_PROXY_PATH!r}) failed: {e}",
        )

    row = ResultRow(
        feature="parsing",
        message_id=message_id,
        path_expression=PARSING_PROXY_PATH,
        throughput=MetricValue(stats.throughput_ops_per_us, "ops/us"),
        latency_p50=MetricValue(stats.p50_us, "us"),
        latency_p95=MetricValue(stats.p95_us, "us"),
    )
    return row, None


def run(corpus: Corpus) -> tuple[list[ResultRow], list[EngineFailure]]:
    results: list[ResultRow] = []
    failures: list[EngineFailure] = []

    messages = list(corpus.representative_typical_per_type())
    messages.append(corpus.unique_by_size_category("large-high-repetition"))
    messages.append(corpus.unique_by_size_category("minimal"))

    for message in messages:
        row, failure = _run_one(message.message_id, message.content)
        if row is not None:
            results.append(row)
        if failure is not None:
            failures.append(failure)

    return results, failures
