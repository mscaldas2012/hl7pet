# Contract: Baseline Results Artifact

This is the interface Roadmap spec `009` (`core-perf-validation`) programmatically
consumes (spec.md FR-008, SC-004) — it must be understandable and parseable without
reading this harness's implementation. Two files, committed together under
`specs/004-scala-baseline-bench/baseline/<run-date>/`.

## `manifest.json`

```json
{
  "runDate": "2026-07-24",
  "engineCoordinate": "gov.cdc:hl7-pet_2.13:1.2.11",
  "corpusId": "interim-v1",
  "hostEnvironment": {
    "cpu": "string, e.g. Apple M2 Pro / Intel Xeon Platinum 8375C",
    "os": "string, e.g. macOS 14.5 / Ubuntu 22.04",
    "jdkVendorVersion": "string, e.g. Temurin 11.0.23",
    "jmhVersion": "string, e.g. 1.37"
  },
  "excludedMessages": [
    { "messageId": "string", "reason": "string" }
  ],
  "resultsFile": "jmh-results.json"
}
```

Consumers MUST treat `runDate` as the stable identifier for a Benchmark Run — never
overwritten in place (spec.md FR-010). `excludedMessages` is `[]` on a clean run, never
omitted (spec.md FR-007 requires it to always be reportable, even when empty).

## `jmh-results.json`

JMH's own native `-rf json` output — not a custom format. It is a flat array with one
entry per `(benchmark method, mode, params)` combination — e.g. a `@Param`-driven method
run in both `Throughput` and `SampleTime` mode across 5 message types produces 10
entries, not 1. Verified directly against a real run of this harness (2026-07-24
baseline) rather than assumed; the fields relevant to this contract are:

| JMH field | Maps to Data Model | Notes |
|---|---|---|
| `benchmark` | `operation` | Fully-qualified `class.method`, e.g. `gov.cdc.hl7.bench.ExtractionBenchmarks.getValueRepresentativeField`. The class name prefix (`ParsingBenchmarks` vs. `ExtractionBenchmarks`) is `operation` (`parsing` vs. `extraction`) per research.md #3. It does **not** encode `messageType` — that's a separate field (below). |
| `params.messageType` | `messageType` | A separate top-level object, e.g. `{"messageType": "ADT^A01"}` — present only for the `@Param`-driven benchmark methods (one per corpus message type, per spec.md FR-006). The four methods dedicated to the large/high-repetition and minimal messages (`retrieveFirstSegmentLarge`, `retrieveMultipleSegmentsLarge`, `getValueRepeatingFieldLarge`, `retrieveFirstSegmentMinimal`, `getFirstValueMinimal`) have no `params` object; identify those by benchmark name instead. |
| `mode` | which `metricCategory` values apply | `"thrpt"` entries carry the `throughput` category (via `primaryMetric.score`); `"sample"` entries carry `latencyP50`/`latencyP95` (via `primaryMetric.scorePercentiles`) and the `allocation`/`memory` secondary metrics (below) — both modes exist per benchmark method because `@BenchmarkMode({Throughput, SampleTime})` produces one full JMH result entry per mode, not one entry with both. |
| `primaryMetric.scoreUnit` | `unit` (`throughput` category) | e.g. `ops/us` — note this is **operations per microsecond**, not per second; convert if comparing against a differently-scaled Rust number. |
| `primaryMetric.score` | `value` (`throughput` category) | Only meaningful on `mode: "thrpt"` entries. |
| `primaryMetric.scorePercentiles."50.0"` | `value` (`latencyP50` category) | Only meaningful on `mode: "sample"` entries; unit is `primaryMetric.scoreUnit` (typically `us/op`). |
| `primaryMetric.scorePercentiles."95.0"` | `value` (`latencyP95` category) | Same. |
| `secondaryMetrics."gc.alloc.rate.norm".score` | `value` (`allocation` category) | Populated only on `mode: "sample"` entries (this harness enables `-prof gc` for every run, per research.md #1); unit is `B/op`. No `·` prefix in the JSON key — that bullet only appears in JMH's human-readable console table, not the JSON. |
| `secondaryMetrics."gc.alloc.rate".score` | `value` (`memory` category) | JMH's `GCProfiler` has no direct per-invocation heap-snapshot metric; this harness uses allocation rate (`MB/sec`) as the `memory` category's proxy figure — a deliberate simplification (research.md #1's "Alternatives considered": Async-profiler/JFR heap snapshots were deferred as unnecessary for this baseline). |

`manifest.json` only carries the run-level metadata JMH itself doesn't know about
(engine version, corpus id, host description, exclusions); it never re-derives or
duplicates a value from `jmh-results.json`.

## Stability

This schema is internal to this repository (not a public HL7-PET API), so Constitution
Principle I (Path Contract Stability) does not apply to it. It should still not change
shape without updating this document and checking Roadmap spec `009`'s consumer code,
once that spec exists.
