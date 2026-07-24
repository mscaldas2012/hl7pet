#!/usr/bin/env python3
"""Standalone, dependency-free reference reader for a committed Baseline Results
Artifact (spec.md US2, contracts/baseline-artifact-schema.md). Needs no JVM, Maven, or
Scala engine -- only manifest.json + jmh-results.json from one run directory.

Usage:
    python3 read-baseline-example.py <run-date>
    python3 read-baseline-example.py 2026-07-24
"""
import json
import sys
from pathlib import Path


def operation_for(benchmark_name: str) -> str:
    class_name = benchmark_name.rsplit(".", 1)[0]
    if class_name.endswith("ParsingBenchmarks"):
        return "parsing"
    if class_name.endswith("ExtractionBenchmarks"):
        return "extraction"
    return "unknown"


def metrics_from_entry(entry: dict) -> dict:
    """Extract the Metric Result values this one JMH result entry carries,
    per contracts/baseline-artifact-schema.md's field mapping."""
    metrics = {}
    primary = entry["primaryMetric"]
    if entry["mode"] == "thrpt":
        metrics["throughput"] = (primary["score"], primary["scoreUnit"])
    elif entry["mode"] == "sample":
        percentiles = primary.get("scorePercentiles", {})
        unit = primary["scoreUnit"]
        if "50.0" in percentiles:
            metrics["latencyP50"] = (percentiles["50.0"], unit)
        if "95.0" in percentiles:
            metrics["latencyP95"] = (percentiles["95.0"], unit)
        secondary = entry.get("secondaryMetrics", {})
        if "gc.alloc.rate.norm" in secondary:
            alloc = secondary["gc.alloc.rate.norm"]
            metrics["allocation"] = (alloc["score"], alloc["scoreUnit"])
        if "gc.alloc.rate" in secondary:
            mem = secondary["gc.alloc.rate"]
            metrics["memory"] = (mem["score"], mem["scoreUnit"])
    return metrics


def main() -> None:
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <run-date>", file=sys.stderr)
        sys.exit(1)

    run_dir = Path(__file__).parent / sys.argv[1]
    manifest = json.loads((run_dir / "manifest.json").read_text())
    results = json.loads((run_dir / manifest["resultsFile"]).read_text())

    print(f"Baseline run {manifest['runDate']}")
    print(f"  engine:  {manifest['engineCoordinate']}")
    print(f"  corpus:  {manifest['corpusId']}")
    print(f"  host:    {manifest['hostEnvironment']['cpu']}, {manifest['hostEnvironment']['os']}")
    if manifest["excludedMessages"]:
        print(f"  excluded: {manifest['excludedMessages']}")
    print()

    # Group by (operation, benchmark method, messageType) so per-method entries in
    # different JMH modes (thrpt / sample) merge back into one row of metrics.
    rows = {}
    for entry in results:
        operation = operation_for(entry["benchmark"])
        method = entry["benchmark"].rsplit(".", 1)[1]
        message_type = entry.get("params", {}).get("messageType", "(n/a)")
        key = (operation, method, message_type)
        rows.setdefault(key, {}).update(metrics_from_entry(entry))

    for (operation, method, message_type), metrics in sorted(rows.items()):
        print(f"[{operation}] {method} messageType={message_type}")
        for category in ("throughput", "latencyP50", "latencyP95", "memory", "allocation"):
            if category in metrics:
                value, unit = metrics[category]
                print(f"    {category:<12} {value:>12.3f} {unit}")
        print()


if __name__ == "__main__":
    main()
