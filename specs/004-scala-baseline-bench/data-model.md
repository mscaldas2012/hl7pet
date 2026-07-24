# Data Model: Scala Baseline Benchmark Harness

Entities carried over from [spec.md](spec.md)'s Key Entities section, made concrete
against the design decisions in [research.md](research.md).

## Benchmark Run

One execution of the harness against the Scala engine.

| Field | Type | Notes |
|---|---|---|
| `runDate` | ISO-8601 date | Also the name of the `baseline/<run-date>/` directory this run's artifacts live under (plan.md Project Structure). |
| `engineCoordinate` | string | Fixed as `gov.cdc:hl7-pet_2.13:1.2.11` (research.md #2) unless a deliberate refresh to a newer version is chosen — in which case it's a new, separately dated run (spec.md FR-010), never an overwrite. |
| `corpusId` | string | Identifies which corpus version was used — the interim corpus (research.md #4) today, or spec `003`'s shared corpus once migrated. |
| `hostEnvironment` | object | CPU model, OS, JDK vendor/version, JMH version — needed because SC-003's variance target and the Edge Cases' "runs on different hardware aren't directly comparable" rule both depend on knowing what produced a given run. |
| `excludedMessages` | array of `{messageId, reason}` | Populated per spec.md FR-007 / Edge Cases when a corpus message fails to parse; empty on a clean run. |
| `metricResults` | array of Metric Result | The actual measurements (below). |

This is written as `manifest.json` — see
[contracts/baseline-artifact-schema.md](contracts/baseline-artifact-schema.md).

## Metric Result

One measured value from a Benchmark Run.

| Field | Type | Notes |
|---|---|---|
| `operation` | enum: `parsing` \| `extraction` | Per research.md #3 — `parsing` = `retrieveFirstSegmentOf`/`retrieveMultipleSegments` (`ParsingBenchmarks`), `extraction` = `getValue`/`getFirstValue` (`ExtractionBenchmarks`, end-to-end parse+extract, labeled as such per FR-006). |
| `metricCategory` | enum: `throughput` \| `latencyP50` \| `latencyP95` \| `memory` \| `allocation` | Matches spec.md FR-001's five required categories. |
| `messageType` | string | e.g. `ADT^A01` — results are per message type, not a single aggregate (FR-006). |
| `value` | number | The measured figure. |
| `unit` | string | e.g. `ops/us`, `us/op`, `B/op`, `MB/sec` — explicit per value, never implied (FR-005, FR-008). |

In practice these are *not* hand-written by the harness: JMH's own `-rf json` output
(`jmh-results.json`) has one flat entry per `(benchmark method, mode, params)`
combination — `operation` comes from the benchmark method's declaring class,
`messageType` from a separate `params.messageType` field (not embedded in the benchmark
name), and `metricCategory` depends on which `mode` the entry is (`thrpt` →
`throughput`; `sample` → `latencyP50`/`latencyP95` plus the `-prof gc` secondary metrics
for `allocation`/`memory`). `Metric Result` above is the logical shape a consumer reads
*out of* that raw JMH structure — see
[contracts/baseline-artifact-schema.md](contracts/baseline-artifact-schema.md) for the
exact, verified-against-a-real-run field mapping, so this spec doesn't reinvent a result
format JMH already produces.

## Baseline Results Artifact

The committed pair of files representing one Benchmark Run, under
`baseline/<run-date>/`:

- `manifest.json` — one Benchmark Run record (above), hand-written by the harness's
  `ManifestWriter` at the end of a run.
- `jmh-results.json` — JMH's native structured output, the source of all Metric Result
  values, referenced (not duplicated) by the manifest.

Retained permanently once committed (FR-010) — a later refresh adds a new dated
subdirectory rather than replacing this one.

## Benchmark Message Corpus

The set of synthetic HL7 messages benchmarked against.

| Field | Type | Notes |
|---|---|---|
| `corpusId` | string | e.g. `interim-v1` — referenced by `manifest.json`'s `corpusId`. |
| `messages` | array of `{messageId, messageType, sizeCategory, filePath}` | `sizeCategory` is one of `minimal` \| `typical` \| `large-high-repetition`, per research.md #4's requirement to include at least one of each. |

Lives under `harness/src/main/resources/corpus/`, synthetic/fabricated only —
real patient data, including de-identified real messages, is disallowed (spec.md FR-009).
