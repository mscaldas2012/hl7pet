# Data Model: Python FFI Overhead Benchmark

Entities from [spec.md](spec.md)'s Key Entities section, made concrete
against [research.md](research.md)'s decisions. This spec's artifacts are
plain JSON (each side's near-native harness output plus one derived
comparison file), not Rust/Python types with a public API — there is no
`contracts/*-api.md` in the specs `005`-`008`/`6000` sense; see
[contracts/comparison-artifact-schema.md](contracts/comparison-artifact-schema.md)
for the schema this document summarizes.

## Comparison Run

One execution of `compare_penalty.py` (research.md #8) producing one
`penalty-report.json`.

| Field | Type | Notes |
|---|---|---|
| `runDate` | string (date) | Shared directory name both sides' raw output lands in (research.md #7) — the proof both sides were measured in the same session. |
| `corpusId` | string | `"perf-v2"` — must match across `python-results.json` and all three `rust-results-*.json` files; a mismatch is a hard error, not a silent skip (FR-001, mirroring spec `009`'s own precedent). |
| `pythonEngineVersion` | string | The installed `hl7pet` package version benchmarked. |
| `rustEngineVersion` | string | The `hl7pet-core` crate version benchmarked. |
| `pythonHostEnvironment` | object | `{os, arch}` in Python's own `platform` module vocabulary (research.md #7 — not normalized to match Rust's). |
| `rustHostEnvironment` | object | `{os, arch}` in Rust's `std::env::consts` vocabulary, copied through from `rust-results-*.json`. |
| `results` | array of Comparison Result | See below. |
| `engineFailures` | array of Engine Failure Record | See below; empty array, not omitted, when there are none. |
| `notComparableMetrics` | array of string | Always `["allocationBytesPerOp", "allocationCallCount", "memoryAllocRateBytesPerSec"]` with a one-line reason (FR-008/research.md #5) — stated once at the run level rather than per row, since the Python side never produces these fields at all. |

## Comparison Result

One row per (feature, corpus message, PATH expression, metric) tuple, for
each of `"throughput"`, `"latencyP50"`, `"latencyP95"`.

| Field | Type | Notes |
|---|---|---|
| `feature` | string | One of `"parsing"`, `"getValue"`, `"getFirstValue"`, `"hierarchy"` (spec.md FR-003) — `"parsing"` rows are the scan-proxy measurement (research.md #3), not an isolated `scan()` call. |
| `messageId` | string | Ties back to `fixtures/messages/perf/corpus-manifest.json`, identical to the message both `crates/core/benches/*.rs` and `crates/python/benches/*.py` used (FR-001/FR-002). |
| `pathExpression` | string | The PATH string benchmarked. Never `null` (unlike spec `009`'s `data-model.md`, where Rust's pure `scan()` rows had none) — the `"parsing"` feature's proxy row always carries `"MSH-1"` (research.md #3). |
| `metric` | string | `"throughput"`, `"latencyP50"`, or `"latencyP95"` — the three metrics this spec compares (FR-006); allocation/memory metrics never appear here (see `notComparableMetrics` above). |
| `pythonValue` | number | Always present — this row wouldn't exist if either side failed (see Engine Failure Record instead). |
| `rustValue` | number | Always present. |
| `unit` | string | `"ops/us"` or `"us"` — both values in the same row share one unit. |
| `penaltyRatio` | number | `rustValue / pythonValue` for `"throughput"`; `pythonValue / rustValue` for `"latencyP50"`/`"latencyP95"` — always framed so **greater than 1.0 means Python costs more** (research.md #6, FR-006). No pass/fail label attached (spec.md Assumptions — diagnostic only). |

## Engine Failure Record

| Field | Type | Notes |
|---|---|---|
| `engine` | string | `"python"` or `"rust"` — which side has no result for this row. |
| `messageId` | string | Which corpus message. |
| `feature` | string | Which feature was being benchmarked. |
| `description` | string | What happened (exception message, panic, etc.) — never silently dropped from the aggregate (FR-010). |

## Scaling Comparison (User Story 2, FR-007)

Not a separate JSON entity — a derived view `compare_penalty.py` prints
(and includes as a short `scalingCheck` object in `penalty-report.json`)
by picking the `"getValue"` feature's two differently-sized `OBX-5` rows
that already exist in `results` (the representative-typical `ORU^R01`
message and the `large-high-repetition` message, research.md #6):

| Field | Type | Notes |
|---|---|---|
| `feature` | string | Always `"getValue"` for this spec (FR-007's minimum — the one feature both a "typical" and a "large" same-path measurement already exist for). |
| `pathExpression` | string | Always `"OBX-5"`. |
| `typicalMessageId` / `largeMessageId` | string | The two `messageId`s compared. |
| `typicalPenaltyRatio` / `largePenaltyRatio` | number | Each message's own `throughput` `penaltyRatio` from `results`, repeated here for direct side-by-side reading — not recomputed. |

## Relationship between inputs and output

```text
Rust: PERF_RUN_OUTPUT_DIR=specs/6001-python-ffi-benchmark/comparison/<date> \
      cargo bench -p hl7pet-core                          (research.md #1)
  -> rust-results-{parsing,extraction,hierarchy}.json      (unchanged shape,
     contracts/comparison-artifact-schema.md's existing ResultRow)

Python: python crates/python/benches/run_all.py \
        --out specs/6001-python-ffi-benchmark/comparison/<date>  (research.md #4)
  -> python-results.json                                    (this spec's
     ResultRow-minus-allocation shape, contracts/comparison-artifact-schema.md)

compare_penalty.py (research.md #8):
  1. Load all four JSON files from the same run directory; assert corpusId
     matches across all of them (hard error otherwise, FR-001)
  2. For each (feature, messageId, pathExpression) both sides cover:
       for each of throughput/latencyP50/latencyP95: build one Comparison
       Result row with a computed penaltyRatio (research.md #6)
  3. For each (feature, messageId, pathExpression) only one side covers, or
     where either side's raw results carry a recorded failure: emit an
     Engine Failure Record instead (FR-010) — never silently skip it
  4. Derive the Scaling Comparison from the two getValue/OBX-5 rows (FR-007)
  5. Write penalty-report.json (Comparison Run shape, above)
```
