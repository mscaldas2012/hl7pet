# Contract: Comparison Artifact

The interface this spec's own deliverable, `compare_penalty.py`
(research.md #8), produces — and the one any later spec (or a human)
programmatically consuming this spec's results must be able to parse
without reading the harness implementations. Extends spec `009`'s
`contracts/comparison-artifact-schema.md` conventions (run-level metadata,
dated/retained artifacts under this spec's own directory) rather than
inventing a new documentation style. Four input files plus one output
file, committed together under
`specs/6001-python-ffi-benchmark/comparison/<run-date>/`.

## Inputs

- `rust-results-{parsing,extraction,hierarchy}.json` — unchanged shape,
  produced by the existing `crates/core/benches/{parsing,extraction,
  hierarchy}.rs` targets (verified directly against `benches/common/
  output.rs`'s actual `ResultRow`/`PartialResults` structs — this is the
  real current shape, not spec `009`'s own contract doc, which documents a
  slightly different `hostEnvironment` shape than the code that actually
  ships):

  ```json
  {
    "corpusId": "perf-v2",
    "rustEngineVersion": "0.1.0",
    "hostEnvironment": { "os": "macos", "arch": "aarch64" },
    "results": [
      {
        "feature": "getValue",
        "messageId": "adt_a01_001",
        "pathExpression": "PV1-3.1",
        "throughput": { "value": 0.0, "unit": "ops/us" },
        "latencyP50": { "value": 0.0, "unit": "us" },
        "latencyP95": { "value": 0.0, "unit": "us" },
        "allocationBytesPerOp": { "value": 0.0, "unit": "bytes" },
        "memoryAllocRateBytesPerSec": { "value": 0.0, "unit": "bytes/sec" },
        "allocationCallCount": { "value": 0, "unit": "count" }
      }
    ],
    "engineFailures": []
  }
  ```

- `python-results.json` — this spec's own new Python harness output shape,
  deliberately a subset of the Rust shape above (no allocation fields,
  research.md #5):

  ```json
  {
    "corpusId": "perf-v2",
    "pythonEngineVersion": "0.1.0",
    "hostEnvironment": { "os": "Darwin", "arch": "arm64" },
    "results": [
      {
        "feature": "getValue",
        "messageId": "adt_a01_001",
        "pathExpression": "PV1-3.1",
        "throughput": { "value": 0.0, "unit": "ops/us" },
        "latencyP50": { "value": 0.0, "unit": "us" },
        "latencyP95": { "value": 0.0, "unit": "us" }
      }
    ],
    "engineFailures": []
  }
  ```

  `hostEnvironment` uses Python's own `platform.system()`/
  `platform.machine()` vocabulary, not normalized to match Rust's
  `std::env::consts` strings (research.md #7 — the shared run directory,
  not string-identical fields, is what proves same-session measurement).
  `throughput`'s unit is `ops/us`, matching the Rust side exactly, so
  `compare_penalty.py` never needs a unit conversion.

## Output: `penalty-report.json`

The Comparison Run shape ([data-model.md](../data-model.md)):

```json
{
  "runDate": "2026-MM-DD",
  "corpusId": "perf-v2",
  "pythonEngineVersion": "0.1.0",
  "rustEngineVersion": "0.1.0",
  "pythonHostEnvironment": { "os": "Darwin", "arch": "arm64" },
  "rustHostEnvironment": { "os": "macos", "arch": "aarch64" },
  "notComparableMetrics": [
    "allocationBytesPerOp",
    "allocationCallCount",
    "memoryAllocRateBytesPerSec"
  ],
  "results": [
    {
      "feature": "getValue",
      "messageId": "adt_a01_001",
      "pathExpression": "PV1-3.1",
      "metric": "throughput",
      "pythonValue": 0.0,
      "rustValue": 0.0,
      "unit": "ops/us",
      "penaltyRatio": 0.0
    }
  ],
  "engineFailures": [],
  "scalingCheck": {
    "feature": "getValue",
    "pathExpression": "OBX-5",
    "typicalMessageId": "oru_r01_001",
    "largeMessageId": "oru_r01_large_026",
    "typicalPenaltyRatio": 0.0,
    "largePenaltyRatio": 0.0
  }
}
```

**Penalty ratio computation** (per Comparison Result row,
`compare_penalty.py`, research.md #6):

- `"throughput"`: `rustValue / pythonValue`.
- `"latencyP50"` / `"latencyP95"`: `pythonValue / rustValue`.
- Both framed so **greater than 1.0 always means Python costs more** —
  there is no `"meets"`/`"beats"`/`"regresses"` verdict, unlike spec
  `009`'s contract; this is diagnostic-only (spec.md Assumptions).

**Postconditions**:

- Every `(feature, messageId, pathExpression)` triple present in *both*
  `python-results.json` and the matching `rust-results-*.json` produces
  exactly 3 Comparison Result rows (`throughput`/`latencyP50`/
  `latencyP95`) — never fewer, never a silently-skipped metric.
- Every triple present in only one side's results, or flagged as a
  failure in either side's own `engineFailures`, produces an Engine
  Failure Record in the output instead of any Comparison Result rows for
  that triple — never both, never neither (FR-010).
- `corpusId` MUST match across `python-results.json` and all three
  `rust-results-*.json` files; a mismatch MUST abort `compare_penalty.py`
  with a non-zero exit and an explanatory error, never silently proceed
  comparing two different corpora (FR-001, enforced as a hard
  precondition, mirroring spec `009`'s own).
- `scalingCheck` MUST always be present (FR-007) — its two messages are
  fixed (`representative_typical_per_type()`'s `ORU^R01` entry and
  `unique_by_size_category("large-high-repetition")`), so it never has a
  "not found" case as long as both sides successfully benchmarked
  `getValue`/`OBX-5` for both messages.

## Stability

Internal to this repository (not a public HL7-PET API) — Constitution
Principle I does not apply. Should not change shape without updating this
document.
