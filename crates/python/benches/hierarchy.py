"""`get_value_hierarchy` benchmarks (spec 6001-python-ffi-benchmark) --
reuses `crates/core/benches/hierarchy.rs`'s exact four-form `PATH_FORMS`
constant against `large_hierarchy_028`, calling the `hl7pet` Python
binding instead of `hl7pet_core` directly.
"""

from __future__ import annotations

import json

import hl7pet

from common.corpus import Corpus
from common.output import EngineFailure, MetricValue, ResultRow
from common.timing import sample

# Mirrors hierarchy.rs's PATH_FORMS exactly, for a true row-for-row
# comparison.
PATH_FORMS = (
    "OBR[1] -> OBX-5",
    "OBR[1] -> OBX[3]-5",
    "OBR[1] -> OBX[@5='VAL-1-2']-5",
    "OBR -> OBX-5",
)


def run(corpus: Corpus) -> tuple[list[ResultRow], list[EngineFailure]]:
    results: list[ResultRow] = []
    failures: list[EngineFailure] = []

    for message in corpus.hierarchy_eligible():
        assert message.profile_json is not None  # hierarchy_eligible() guarantees this
        try:
            profile = json.loads(message.profile_json)
        except json.JSONDecodeError as e:
            failures.append(
                EngineFailure(
                    message_id=message.message_id,
                    feature="hierarchy",
                    description=f"profile JSON failed to parse: {e}",
                )
            )
            continue

        for path_expr in PATH_FORMS:
            try:
                stats = sample(lambda: hl7pet.get_value_hierarchy(message.content, path_expr, profile))
            except Exception as e:  # noqa: BLE001
                failures.append(
                    EngineFailure(
                        message_id=message.message_id,
                        feature="hierarchy",
                        description=f"get_value_hierarchy(message, {path_expr!r}, profile) failed: {e}",
                    )
                )
                continue

            results.append(
                ResultRow(
                    feature="hierarchy",
                    message_id=message.message_id,
                    path_expression=path_expr,
                    throughput=MetricValue(stats.throughput_ops_per_us, "ops/us"),
                    latency_p50=MetricValue(stats.p50_us, "us"),
                    latency_p95=MetricValue(stats.p95_us, "us"),
                )
            )

    return results, failures
