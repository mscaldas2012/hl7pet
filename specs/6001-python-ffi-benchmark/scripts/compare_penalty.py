#!/usr/bin/env python3
"""Compares the `hl7pet` Python binding against raw `hl7pet-core` for the
same corpus, producing `penalty-report.json`
(contracts/comparison-artifact-schema.md, spec
6001-python-ffi-benchmark).

Usage: compare_penalty.py <run-directory>

Expects, in <run-directory>:
  - python-results.json               This spec's Python harness output.
  - rust-results-parsing.json
  - rust-results-extraction.json
  - rust-results-hierarchy.json       A fresh `cargo bench -p hl7pet-core`
                                       run (research.md #1's
                                       `PERF_RUN_OUTPUT_DIR` override) --
                                       never spec 009's own committed
                                       artifact (spec.md FR-005).

Writes penalty-report.json to the same directory.
"""

import json
import sys
from pathlib import Path

METRICS = ("throughput", "latencyP50", "latencyP95")

# Python has no equivalent to the Rust harness's byte-accurate custom
# global allocator -- these are never presented as comparable (FR-008,
# research.md #5).
NOT_COMPARABLE_METRICS = [
    "allocationBytesPerOp",
    "allocationCallCount",
    "memoryAllocRateBytesPerSec",
]

# The two messages data-model.md's Scaling Comparison reads (US2, FR-007):
# the representative "typical" ORU^R01 message and the large-high-
# repetition one, both benchmarked with getValue/"OBX-5".
LARGE_MESSAGE_ID = "oru_r01_large_026"


def load_json(path):
    return json.loads(Path(path).read_text())


def match_key(feature, message_id, path_expr):
    """"parsing" rows match by (feature, messageId) only. Rust's pure
    scan() carries pathExpression: null; Python's proxy measurement
    (get_first_value(message, "MSH-1"), research.md #3) necessarily calls
    a real PATH -- the two sides' pathExpression values are never expected
    to agree for this one feature, by design, not by omission."""
    if feature == "parsing":
        return (feature, message_id)
    return (feature, message_id, path_expr)


def parse_side(entries):
    """Returns {match_key: {"pathExpression": ..., metric: value, ...}}."""
    rows = {}
    for r in entries:
        key = match_key(r["feature"], r["messageId"], r.get("pathExpression"))
        rows[key] = {
            "pathExpression": r.get("pathExpression"),
            "throughput": r["throughput"]["value"],
            "latencyP50": r["latencyP50"]["value"],
            "latencyP95": r["latencyP95"]["value"],
        }
    return rows


def penalty_ratio(metric, python_value, rust_value):
    """>1.0 always means Python costs more (research.md #6)."""
    if metric == "throughput":
        return (rust_value / python_value) if python_value else float("inf")
    return (python_value / rust_value) if rust_value else float("inf")


def build_scaling_check(results):
    rows = [
        r
        for r in results
        if r["feature"] == "getValue" and r["pathExpression"] == "OBX-5" and r["metric"] == "throughput"
    ]
    large = next((r for r in rows if r["messageId"] == LARGE_MESSAGE_ID), None)
    typical = next((r for r in rows if r["messageId"] != LARGE_MESSAGE_ID), None)
    if not (large and typical):
        return None
    return {
        "feature": "getValue",
        "pathExpression": "OBX-5",
        "typicalMessageId": typical["messageId"],
        "largeMessageId": large["messageId"],
        "typicalPenaltyRatio": typical["penaltyRatio"],
        "largePenaltyRatio": large["penaltyRatio"],
    }


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    run_dir = Path(sys.argv[1])

    python_data = load_json(run_dir / "python-results.json")

    rust_partials = []
    for target in ("parsing", "extraction", "hierarchy"):
        path = run_dir / f"rust-results-{target}.json"
        if path.exists():
            rust_partials.append(load_json(path))
    if not rust_partials:
        raise SystemExit(f"no rust-results-*.json files found in {run_dir}")

    # FR-001 hard precondition: every input must describe the same corpus.
    corpus_ids = {python_data["corpusId"]} | {p["corpusId"] for p in rust_partials}
    if len(corpus_ids) > 1:
        raise SystemExit(
            f"corpusId mismatch across input files: {sorted(corpus_ids)} -- "
            "refusing to compare results from two different corpora"
        )
    corpus_id = next(iter(corpus_ids))

    rust_entries = []
    engine_failures = list(python_data.get("engineFailures", []))
    for p in rust_partials:
        rust_entries.extend(p["results"])
        engine_failures.extend(p.get("engineFailures", []))

    python_rows = parse_side(python_data["results"])
    rust_rows = parse_side(rust_entries)

    results = []
    all_keys = set(python_rows) | set(rust_rows)
    for key in sorted(all_keys, key=lambda k: (k[0], k[1], k[2] if len(k) > 2 else "")):
        feature, message_id = key[0], key[1]
        py_row = python_rows.get(key)
        rust_row = rust_rows.get(key)

        if py_row is None or rust_row is None:
            missing = "python" if py_row is None else "rust"
            engine_failures.append(
                {
                    "engine": missing,
                    "messageId": message_id,
                    "feature": feature,
                    "description": f"no {missing} result for {key}",
                }
            )
            continue

        for metric in METRICS:
            unit = "ops/us" if metric == "throughput" else "us"
            py_val = py_row[metric]
            rust_val = rust_row[metric]
            results.append(
                {
                    "feature": feature,
                    "messageId": message_id,
                    "pathExpression": py_row["pathExpression"],
                    "metric": metric,
                    "pythonValue": py_val,
                    "rustValue": rust_val,
                    "unit": unit,
                    "penaltyRatio": penalty_ratio(metric, py_val, rust_val),
                }
            )

    report = {
        "runDate": run_dir.name,
        "corpusId": corpus_id,
        "pythonEngineVersion": python_data["pythonEngineVersion"],
        "rustEngineVersion": rust_partials[0]["rustEngineVersion"],
        "pythonHostEnvironment": python_data["hostEnvironment"],
        "rustHostEnvironment": rust_partials[0]["hostEnvironment"],
        "notComparableMetrics": NOT_COMPARABLE_METRICS,
        "results": results,
        "engineFailures": engine_failures,
        "scalingCheck": build_scaling_check(results),
    }

    out_path = run_dir / "penalty-report.json"
    out_path.write_text(json.dumps(report, indent=2))
    print(f"wrote {out_path}")
    print(f"{len(results)} metrics compared, {len(engine_failures)} engine failure(s)")


if __name__ == "__main__":
    main()
