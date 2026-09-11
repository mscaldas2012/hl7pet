"""Writes `python-results.json` (spec 6001-python-ffi-benchmark,
contracts/comparison-artifact-schema.md, research.md #5) — the Python
harness's own ResultRow-minus-allocation shape: no
`allocationBytesPerOp`/`allocationCallCount`/`memoryAllocRateBytesPerSec`
fields at all, since Python has no equivalent to the Rust harness's
byte-accurate custom global allocator (spec.md FR-008).

Unlike the three separate Rust bench binaries (which each write their own
`rust-results-<target>.json` to avoid a write race, `crates/core/benches/
common/output.rs`), the Python harness runs everything in one process
(`run_all.py`), so all three features' rows collect into one list and are
written together in a single call — no split-file/merge step needed.
"""

from __future__ import annotations

import json
import platform
from dataclasses import dataclass
from importlib import metadata
from pathlib import Path


@dataclass(frozen=True)
class MetricValue:
    value: float
    unit: str

    def to_json(self) -> dict:
        return {"value": self.value, "unit": self.unit}


@dataclass(frozen=True)
class ResultRow:
    feature: str
    message_id: str
    path_expression: str
    throughput: MetricValue
    latency_p50: MetricValue
    latency_p95: MetricValue

    def to_json(self) -> dict:
        return {
            "feature": self.feature,
            "messageId": self.message_id,
            "pathExpression": self.path_expression,
            "throughput": self.throughput.to_json(),
            "latencyP50": self.latency_p50.to_json(),
            "latencyP95": self.latency_p95.to_json(),
        }


@dataclass(frozen=True)
class EngineFailure:
    message_id: str
    feature: str
    description: str
    engine: str = "python"

    def to_json(self) -> dict:
        return {
            "engine": self.engine,
            "messageId": self.message_id,
            "feature": self.feature,
            "description": self.description,
        }


def python_engine_version() -> str:
    try:
        return metadata.version("hl7pet")
    except metadata.PackageNotFoundError:
        return "unknown"


def write_results(
    out_dir: Path,
    corpus_id: str,
    results: list[ResultRow],
    engine_failures: list[EngineFailure],
) -> Path:
    payload = {
        "corpusId": corpus_id,
        "pythonEngineVersion": python_engine_version(),
        "hostEnvironment": {"os": platform.system(), "arch": platform.machine()},
        "results": [r.to_json() for r in results],
        "engineFailures": [f.to_json() for f in engine_failures],
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / "python-results.json"
    path.write_text(json.dumps(payload, indent=2))
    print(f"wrote {path}")
    return path
