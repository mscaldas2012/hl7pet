#!/usr/bin/env python3
"""Compares Scala JMH results against this spec's own Rust harness results
for the same corpus, producing comparison-report.json
(contracts/comparison-artifact-schema.md, spec 009-core-perf-validation).

Usage: compare_results.py <run-directory>

Expects, in <run-directory>:
  - jmh-results.json   Scala's native JMH JSON output (spec 004's
                        BenchmarkRunner, extended with HierarchyBenchmarks
                        per research.md #4). JMH's own output carries no
                        corpusId or messageId -- both engines already read
                        the same fixtures/messages/perf/corpus-manifest.json
                        (research.md #1), so this script reads that file
                        directly (not a copy) to resolve each Scala
                        @Param-driven result back to a specific messageId
                        and to get the authoritative corpusId, rather than
                        depending on spec 004's separate run-specific
                        manifest.json artifact.
  - rust-results-parsing.json
  - rust-results-extraction.json
  - rust-results-hierarchy.json
                        This spec's own Rust harness output (one file per
                        `cargo bench` target, research.md #2's design --
                        avoids the write race a single shared file would
                        need to coordinate across three separate binaries).

Writes comparison-report.json to the same directory.
"""

import json
import sys
from pathlib import Path

# spec 004's own already-established reproducibility bar (that spec's SC-003),
# inherited rather than re-derived (spec.md Edge Cases).
TOLERANCE = 0.10

# "Better" direction per metric -- throughput is better higher; everything
# else (latency, bytes) is better lower.
HIGHER_IS_BETTER = {"throughput"}

# memoryAllocRateBytesPerSec is bytesPerOp * throughput -- a rate, not a
# per-operation figure. Discovered against the real comparison run: every
# regression this script initially reported was on this one metric, while
# throughput and allocationBytesPerOp *both* showed Rust winning by 10-20x on
# the very same rows (e.g. adt_a01_001/getValue: Rust beats Scala 18x on
# throughput and uses 9x fewer bytes/op, yet still "regresses" on this rate
# metric) -- because an engine doing dramatically more operations per second
# necessarily moves more total bytes per second too, even while using far
# less memory *per operation*. Comparing this rate directly between two
# engines with very different throughput is confounded, not a real
# regression signal (Constitution Principle V: document rather than silently
# mishandle a misleading comparison, the same way spec 004's own
# contracts/baseline-artifact-schema.md already flags this exact metric as
# "a proxy figure... a deliberate simplification" for its own Scala-only
# baseline -- that caveat was written for a single-engine report and doesn't
# survive cross-engine comparison at different throughputs). Reported for
# reference only; `allocationBytesPerOp` is the metric that actually answers
# "which engine uses less memory per call," and never shows this artifact.
RATE_CONFOUNDED_METRICS = {"memoryAllocRateBytesPerSec"}

# Maps each Scala JMH benchmark method name to (feature, path expression or
# None, messageId resolution rule). Resolution rules:
#   ("by_type",)          resolve via params.messageType: first manifest
#                         entry with that messageType and sizeCategory ==
#                         "typical" (mirrors each *State.setUp()'s own
#                         `.filter(...).findFirst()` in the Java source).
#   ("fixed", messageId)  always this specific corpus message.
REPRESENTATIVE_PATH_BY_TYPE = {
    "ADT^A01": "PV1-3.1",
    "ADT^A08": "PV1-3.1",
    "ORU^R01": "OBX-5",
    "VXU^V04": "RXA-5.2",
    "ORM^O01": "OBR-4.2",
}

METHOD_TABLE = {
    "retrieveFirstSegment": ("parsing", None, ("by_type",)),
    "retrieveFirstSegmentLarge": ("parsing", None, ("fixed", "oru_r01_large_026")),
    "retrieveFirstSegmentMinimal": ("parsing", None, ("fixed", "adt_a01_minimal_027")),
    "retrieveMultipleSegmentsLarge": ("parsing", None, ("fixed", "oru_r01_large_026")),
    "getFirstValuePatientLastName": ("getFirstValue", "PID-5.1", ("by_type",)),
    "getValueRepresentativeField": ("getValue", None, ("by_type",)),  # path varies by type
    "getValueRepeatingFieldLarge": ("getValue", "OBX-5", ("fixed", "oru_r01_large_026")),
    "getFirstValueMinimal": ("getFirstValue", "PID-5.1", ("fixed", "adt_a01_minimal_027")),
    "getValueSingleHopPlain": ("hierarchy", "OBR[1] -> OBX-5", ("fixed", "large_hierarchy_028")),
    "getValueSingleHopIndexedChild": ("hierarchy", "OBR[1] -> OBX[3]-5", ("fixed", "large_hierarchy_028")),
    "getValueSingleHopFilteredChild": (
        "hierarchy",
        "OBR[1] -> OBX[@5='VAL-1-2']-5",
        ("fixed", "large_hierarchy_028"),
    ),
    "getValueAllParentsCombined": ("hierarchy", "OBR -> OBX-5", ("fixed", "large_hierarchy_028")),
}


def load_json(path):
    return json.loads(Path(path).read_text())


def resolve_message_id(manifest_by_type_typical, rule, params):
    kind = rule[0]
    if kind == "fixed":
        return rule[1]
    if kind == "by_type":
        message_type = params["messageType"]
        entry = manifest_by_type_typical.get(message_type)
        if entry is None:
            raise ValueError(f"no typical message found for messageType {message_type!r}")
        return entry
    raise ValueError(f"unknown resolution rule: {rule}")


def build_typical_index(manifest):
    """First manifest entry per (messageType, "typical") -- mirrors each
    Java *State.setUp()'s own `.filter(...).findFirst()` (first in array
    order, i.e. first in manifest["messages"])."""
    index = {}
    for entry in manifest["messages"]:
        if entry["sizeCategory"] != "typical":
            continue
        index.setdefault(entry["messageType"], entry["messageId"])
    return index


def parse_scala_entries(jmh_entries, manifest_by_type_typical):
    """Returns {(feature, messageId, pathExpression): {metric: value}}."""
    scala = {}
    for entry in jmh_entries:
        method = entry["benchmark"].rsplit(".", 1)[-1]
        table_entry = METHOD_TABLE.get(method)
        if table_entry is None:
            continue  # not a benchmark this comparison covers
        feature, fixed_path, rule = table_entry

        message_id = resolve_message_id(manifest_by_type_typical, rule, entry.get("params", {}))
        path_expr = fixed_path
        if path_expr is None and feature == "getValue":
            path_expr = REPRESENTATIVE_PATH_BY_TYPE[entry["params"]["messageType"]]

        key = (feature, message_id, path_expr)
        row = scala.setdefault(key, {})

        if entry["mode"] == "thrpt":
            row["throughput"] = entry["primaryMetric"]["score"]  # already ops/us
        elif entry["mode"] == "sample":
            pct = entry["primaryMetric"]["scorePercentiles"]
            row["latencyP50"] = pct["50.0"]  # us/op
            row["latencyP95"] = pct["95.0"]  # us/op
            secondary = entry.get("secondaryMetrics", {})
            if "gc.alloc.rate.norm" in secondary:
                row["allocationBytesPerOp"] = secondary["gc.alloc.rate.norm"]["score"]  # B/op
            if "gc.alloc.rate" in secondary:
                # MB/sec (binary mega, JMH/GCProfiler convention) -> bytes/sec
                row["memoryAllocRateBytesPerSec"] = secondary["gc.alloc.rate"]["score"] * 1024 * 1024

    return scala


def parse_rust_entries(rust_partials):
    """Returns {(feature, messageId, pathExpression): {metric: value}} plus
    the merged engineFailures list."""
    rust = {}
    failures = []
    corpus_ids = set()

    for partial in rust_partials:
        corpus_ids.add(partial["corpusId"])
        failures.extend(partial.get("engineFailures", []))
        for r in partial["results"]:
            key = (r["feature"], r["messageId"], r.get("pathExpression"))
            rust[key] = {
                "throughput": r["throughput"]["value"],
                "latencyP50": r["latencyP50"]["value"],
                "latencyP95": r["latencyP95"]["value"],
                "allocationBytesPerOp": r["allocationBytesPerOp"]["value"],
                "memoryAllocRateBytesPerSec": r["memoryAllocRateBytesPerSec"]["value"],
                "allocationCallCount": r["allocationCallCount"]["value"],
            }

    if len(corpus_ids) > 1:
        raise SystemExit(
            f"rust-results-*.json disagree on corpusId: {sorted(corpus_ids)} -- "
            "the three bench targets must be run against the same corpus"
        )

    return rust, failures, next(iter(corpus_ids))


METRIC_UNITS = {
    "throughput": "ops/us",
    "latencyP50": "us",
    "latencyP95": "us",
    "allocationBytesPerOp": "bytes",
    "memoryAllocRateBytesPerSec": "bytes/sec",
    "allocationCallCount": "count",
}


def verdict(metric, scala_value, rust_value):
    if metric == "allocationCallCount" or metric in RATE_CONFOUNDED_METRICS:
        return "not-comparable"
    if scala_value == 0:
        return "meets" if rust_value == 0 else "regresses"
    ratio = rust_value / scala_value
    better = ratio > 1.0 if metric in HIGHER_IS_BETTER else ratio < 1.0
    within_tolerance = abs(ratio - 1.0) <= TOLERANCE
    if within_tolerance:
        return "meets"
    return "beats" if better else "regresses"


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    run_dir = Path(sys.argv[1])

    jmh_entries = load_json(run_dir / "jmh-results.json")

    manifest_path = Path(__file__).resolve().parents[3] / "fixtures" / "messages" / "perf" / "corpus-manifest.json"
    manifest = load_json(manifest_path)
    manifest_by_type_typical = build_typical_index(manifest)

    rust_partials = []
    for target in ("parsing", "extraction", "hierarchy"):
        path = run_dir / f"rust-results-{target}.json"
        if path.exists():
            rust_partials.append(load_json(path))
    if not rust_partials:
        raise SystemExit(f"no rust-results-*.json files found in {run_dir}")

    scala = parse_scala_entries(jmh_entries, manifest_by_type_typical)
    rust, engine_failures, rust_corpus_id = parse_rust_entries(rust_partials)

    # FR-001 hard precondition: both sides must be measuring the same corpus.
    manifest_corpus_id = manifest["corpusId"]
    if rust_corpus_id != manifest_corpus_id:
        raise SystemExit(
            f"corpusId mismatch: rust-results-*.json say {rust_corpus_id!r}, "
            f"but fixtures/messages/perf/corpus-manifest.json says {manifest_corpus_id!r} -- "
            "refusing to compare results from two different corpora"
        )

    results = []
    all_keys = set(scala) | set(rust)
    for key in sorted(all_keys, key=lambda k: (k[0], k[1], k[2] or "")):
        feature, message_id, path_expr = key
        scala_row = scala.get(key)
        rust_row = rust.get(key)

        if scala_row is None or rust_row is None:
            missing_engine = "scala" if scala_row is None else "rust"
            # Distinguish an expected PATH-form-only mismatch (this exact
            # messageId+feature exists on the other engine too, just under a
            # different pathExpression -- e.g. Rust's indexed/filter FR-004
            # forms, which spec 004's ExtractionBenchmarks has no per-form
            # equivalent for) from a message genuinely absent from one
            # engine's run entirely (spec.md Edge Cases/SC-003).
            other = scala if missing_engine == "scala" else rust
            same_message_other_path = any(k[0] == feature and k[1] == message_id for k in other)
            reason = "path-form-mismatch" if same_message_other_path else "missing-result"
            engine_failures.append(
                {
                    "engine": missing_engine,
                    "messageId": message_id,
                    "feature": feature,
                    "reason": reason,
                    "description": (
                        f"no {missing_engine} result for path {path_expr!r}"
                        + (" (present under a different pathExpression on that engine)" if same_message_other_path else "")
                    ),
                }
            )
            continue

        for metric, unit in METRIC_UNITS.items():
            rust_value = rust_row[metric]
            scala_value = scala_row.get(metric) if metric != "allocationCallCount" else None
            results.append(
                {
                    "feature": feature,
                    "messageId": message_id,
                    "pathExpression": path_expr,
                    "metric": metric,
                    "scalaValue": scala_value,
                    "rustValue": rust_value,
                    "unit": unit,
                    "verdict": verdict(metric, scala_value, rust_value) if scala_value is not None else "not-comparable",
                }
            )

    report = {
        "runDate": run_dir.name,
        "corpusId": rust_corpus_id,
        "scalaEngineCoordinate": "gov.cdc:hl7-pet_2.13:1.2.11",
        "rustEngineVersion": rust_partials[0]["rustEngineVersion"],
        "hostEnvironment": rust_partials[0]["hostEnvironment"],
        "results": results,
        "engineFailures": engine_failures,
    }

    out_path = run_dir / "comparison-report.json"
    out_path.write_text(json.dumps(report, indent=2))
    print(f"wrote {out_path}")

    regressions = [r for r in results if r["verdict"] == "regresses"]
    print(f"{len(results)} metrics compared, {len(regressions)} regression(s), {len(engine_failures)} engine failure(s)")


if __name__ == "__main__":
    main()
