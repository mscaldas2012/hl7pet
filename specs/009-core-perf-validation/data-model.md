# Data Model: Core Performance Validation

Entities from [spec.md](spec.md)'s Key Entities section, made concrete against
[research.md](research.md)'s decisions. This spec's artifacts are plain JSON (both
engines' native/near-native output plus one derived comparison file), not Rust
types with a public API — there is no `contracts/*-api.md` in the specs `005`-`008`
sense; see [contracts/comparison-artifact-schema.md](contracts/comparison-artifact-schema.md)
for the schema this document summarizes.

## Benchmark Message Corpus (relocated + extended, research.md #1)

`fixtures/messages/perf/corpus-manifest.json` — same shape as spec `004`'s existing
manifest:

| Field | Type | Notes |
|---|---|---|
| `corpusId` | string | `"perf-v2"` — a new id, distinct from spec `004`'s `"interim-v1"`, since the corpus now lives at a new location and gained a new message (Decision 1). |
| `messages` | array | Each entry: `messageId`, `messageType`, `sizeCategory`, `filePath` — unchanged shape. One new entry: `large-hierarchy.hl7`, `sizeCategory: "large-high-repetition"`, a new hierarchy-specific `messageType`. |

## Comparison Run

One execution of `compare_results.py` (research.md #5) producing one
`comparison-report.json`.

| Field | Type | Notes |
|---|---|---|
| `runDate` | string (date) | Matches spec `004`'s `manifest.json` convention. |
| `corpusId` | string | `"perf-v2"` (must match both input files' corpus id — a mismatch is a hard error, not a silent skip, per FR-001). |
| `scalaEngineCoordinate` | string | e.g. `gov.cdc:hl7-pet_2.13:1.2.11`, copied from `scala-results.json`'s run metadata. |
| `rustEngineVersion` | string | The `hl7pet-core` crate version (or git commit) benchmarked. |
| `hostEnvironment` | object | CPU/OS, matching spec `004`'s existing `hostEnvironment` shape (shared machine for both runs, spec.md Technical Context). |
| `results` | array of Comparison Result | See below. |
| `engineFailures` | array of Engine Failure Record | See below; empty array, not omitted, when there are none. |

## Comparison Result

One row per (feature, corpus message, metric) tuple.

| Field | Type | Notes |
|---|---|---|
| `feature` | string | One of `"parsing"`, `"getValue"`, `"getFirstValue"`, `"hierarchy"` (spec.md FR-003). |
| `messageId` | string | Ties back to the Benchmark Message Corpus entry both engines used (FR-001). |
| `pathExpression` | string, nullable | The PATH string benchmarked, for `getValue`/`getFirstValue`/`hierarchy` rows; `null` for `parsing` (no PATH involved). |
| `metric` | string | One of the Constitution's five categories, split per spec `004`'s own already-established convention (research.md #3): `"throughput"`, `"latencyP50"`, `"latencyP95"`, `"allocationBytesPerOp"` (vs. Scala `gc.alloc.rate.norm` — the "allocation" category), `"memoryAllocRateBytesPerSec"` (vs. Scala `gc.alloc.rate` — the "memory" category), plus one Rust-only diagnostic, `"allocationCallCount"` (research.md #3) — see `scalaValue`. |
| `scalaValue` | number, nullable | `null` exactly for `metric: "allocationCallCount"` (no Scala-side equivalent, research.md #3) — never `null` for any other metric, per FR-001's "exact same... messages" and FR-003's minimum feature coverage. |
| `rustValue` | number | Always present. |
| `unit` | string | e.g. `"ops/us"`, `"us"`, `"bytes"`, `"count"` — both values in the same row share one unit. |
| `verdict` | string | One of `"meets"`, `"beats"`, `"regresses"`, or `"not-comparable"` — for `allocationCallCount` (no Scala equivalent) and, discovered during implementation, `memoryAllocRateBytesPerSec` too: that metric is `bytesPerOp * throughput`, so an engine with dramatically higher throughput necessarily shows a higher rate even while using *less* memory per operation — confirmed against the real comparison run, where every one of this metric's rows "regressed" while the same rows' `allocationBytesPerOp` and `throughput` both showed Rust winning by 10-20x. `scalaValue`/`rustValue` are still populated for this metric (for reference); only the verdict is suppressed, so a naive regression count doesn't get inflated by a confounded metric. `allocationBytesPerOp` is the metric that actually answers "which engine uses less memory per call." Never omitted — SC-002 requires every metric to carry an explicit verdict, `not-comparable` included. |

## Engine Failure Record

| Field | Type | Notes |
|---|---|---|
| `engine` | string | `"scala"` or `"rust"` — which engine has no result for this row. |
| `messageId` | string | Which corpus message. |
| `feature` | string | Which feature was being benchmarked. |
| `reason` | string | `"missing-result"` (this message/feature is entirely absent from that engine's run — a genuine gap) or `"path-form-mismatch"` (the *other* engine's run has a different `pathExpression` for this same message/feature — expected when one engine benchmarks a PATH form the other has no per-form benchmark method for, e.g. Rust's FR-004 indexed/filter forms against spec 004's single-form `getValueRepresentativeField`, spec.md Edge Cases/SC-003). Discovered necessary during implementation — an earlier draft used one undifferentiated bucket, which made an expected, spec-anticipated asymmetry look identical to a real gap. |
| `description` | string | What happened (exception message, panic, etc., for `missing-result`; which pathExpression exists on the other engine instead, for `path-form-mismatch`) — never silently dropped from the aggregate (FR-009). |

## Relationship between inputs and output

```text
Scala: mvn compile exec:java (extended BenchmarkRunner, Decision 4)
  -> scala-results.json (JMH native format: thrpt entries for throughput,
     sample entries for p50/p95, secondaryMetrics.gc.alloc.rate.norm for bytes/op)

Rust: cargo bench (benches/{parsing,extraction,hierarchy}.rs, Decision 2)
  -> rust-results.json (this spec's own shape: per feature/message/PATH,
     {throughput, p50, p95, bytesPerOp, allocCallCount})

compare_results.py (Decision 5):
  1. Load both JSON files; assert corpusId matches (hard error otherwise, FR-001)
  2. For each (feature, messageId) both files cover:
       for each of the 5 metrics: build one Comparison Result row (verdict computed
       by comparing rustValue against scalaValue — "beats" if better, "meets" if
       within an agreed noise tolerance carried over from spec 004's SC-003 ±10%,
       "regresses" otherwise; "not-comparable" for allocationCallCount)
  3. For each (feature, messageId) only one file covers, or where either engine's
     raw results carry a recorded failure: emit an Engine Failure Record instead of
     a Comparison Result (FR-009) — never silently skip it
  4. Write comparison-report.json (Comparison Run shape, above)
```
