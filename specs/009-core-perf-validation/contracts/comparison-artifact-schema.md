# Contract: Comparison Artifact

The interface this spec's own deliverable, `compare_results.py` (research.md #5),
produces — and the one any later spec (or a human) programmatically consuming this
spec's results must be able to parse without reading the harness implementations.
Extends spec `004`'s `contracts/baseline-artifact-schema.md` conventions (run-level
metadata, dated/retained artifacts) rather than inventing a new documentation style.
Three files, committed together under
`specs/009-core-perf-validation/comparison/<run-date>/`.

## Inputs (not this spec's own format, documented here only for traceability)

- `scala-results.json` — JMH's native `-rf json` output, produced by the extended
  `BenchmarkRunner` (research.md #4). Same shape spec `004`'s own
  `contracts/baseline-artifact-schema.md` already documents in full — this contract
  does not repeat that mapping, only extends it with the new `HierarchyBenchmarks`
  entries (identified by the `gov.cdc.hl7.bench.HierarchyBenchmarks` class-name
  prefix, mapping to `feature: "hierarchy"`, the same way `ParsingBenchmarks`/
  `ExtractionBenchmarks` map to `"parsing"`/`getValue`/`getFirstValue`).
- `rust-results.json` — this spec's own Rust harness output shape:

  ```json
  {
    "corpusId": "perf-v2",
    "rustEngineVersion": "string, e.g. a git commit hash or crate version",
    "hostEnvironment": { "cpu": "string", "os": "string", "rustcVersion": "string" },
    "results": [
      {
        "feature": "parsing | getValue | getFirstValue | hierarchy",
        "messageId": "string, matches fixtures/messages/perf/corpus-manifest.json",
        "pathExpression": "string or null (null for parsing)",
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

  `throughput`'s unit is deliberately `ops/us` (not `ops/sec`) to match JMH's
  `primaryMetric.scoreUnit` convention directly — `compare_results.py` does the
  unit conversion once, centrally, rather than requiring the Rust harness and the
  comparison script to independently agree on a different shared unit.

## Output: `comparison-report.json`

The Comparison Run shape ([data-model.md](../data-model.md)):

```json
{
  "runDate": "2026-MM-DD",
  "corpusId": "perf-v2",
  "scalaEngineCoordinate": "gov.cdc:hl7-pet_2.13:1.2.11",
  "rustEngineVersion": "string",
  "hostEnvironment": { "cpu": "string", "os": "string" },
  "results": [
    {
      "feature": "hierarchy",
      "messageId": "large-hierarchy",
      "pathExpression": "OBR[1] -> OBX-5",
      "metric": "latencyP95",
      "scalaValue": 0.0,
      "rustValue": 0.0,
      "unit": "us",
      "verdict": "beats"
    }
  ],
  "engineFailures": []
}
```

**Verdict computation** (per Comparison Result row, `compare_results.py`):
- `"beats"`: `rustValue` is strictly better than `scalaValue` beyond spec `004`'s
  existing ±10% reproducibility tolerance (that spec's SC-003, inherited rather
  than re-derived per spec.md Edge Cases) — for throughput/allocation-favorable
  metrics, "better" is higher; for latency/bytes/rate metrics, "better" is lower.
- `"meets"`: within that ±10% tolerance either direction — a difference this small
  is noise, not a real regression or improvement, per spec `004`'s own
  already-established reproducibility bar.
- `"regresses"`: `rustValue` is worse than `scalaValue` beyond the tolerance —
  Constitution's "MUST NOT regress" is violated for this metric/feature/message.
- `"not-comparable"`: only for `metric: "allocationCallCount"` (research.md #3) —
  `scalaValue` is `null` by construction, so no meets/beats/regresses judgment is
  possible or attempted.

**Postconditions**:
- Every `(feature, messageId)` pair present in *both* input files' results
  produces exactly 6 Comparison Result rows (one per metric) — never fewer, never
  a silently-skipped metric.
- Every `(feature, messageId)` pair present in only one input file, or flagged as
  a failure in either input's `engineFailures`, produces an Engine Failure Record
  in the output instead of any Comparison Result rows for that pair — never both,
  never neither (FR-009, SC-003).
- `corpusId` MUST match between `scala-results.json`'s run metadata and
  `rust-results.json`; a mismatch MUST abort `compare_results.py` with a non-zero
  exit and an explanatory error, never silently proceed comparing two different
  corpora (FR-001's "exact same" requirement, enforced as a hard precondition).

## Stability

Internal to this repository (not a public HL7-PET API) — Constitution Principle I
does not apply. Should not change shape without updating this document.
