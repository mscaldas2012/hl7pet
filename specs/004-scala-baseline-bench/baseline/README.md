# Committed Baseline Results

Each subdirectory here is one Benchmark Run (see
[../data-model.md](../data-model.md)), named by its run date (`YYYY-MM-DD`), containing:

- `manifest.json` — run-level metadata: engine version, corpus id, host/environment,
  excluded messages, and a pointer to the results file.
- `jmh-results.json` — JMH's native structured output, the source of every metric value.

The exact field-by-field schema and the JMH → Metric Result mapping are documented in
[../contracts/baseline-artifact-schema.md](../contracts/baseline-artifact-schema.md) —
read that first if you're consuming this data programmatically (e.g. Roadmap spec `009`,
`core-perf-validation`).

A minimal, dependency-free reference reader is provided at
[read-baseline-example.py](read-baseline-example.py).

## Rules

- **Never overwrite a run in place.** Refreshing the baseline (a new Scala engine
  version, a new corpus, a different machine) adds a new dated subdirectory; existing
  ones are retained as historical record (spec.md FR-010).
- **Runs are not comparable across different host environments.** Check each run's
  `manifest.json.hostEnvironment` before diffing two runs' numbers (spec.md Edge Cases).

## Reproducibility (spec.md SC-003)

Verified by running the harness twice on the same machine (`BenchmarkRunner` tuned to
`forks(3)`, 3 warmup + 3 measurement iterations per fork — see the rationale comment in
`../harness/src/main/java/gov/cdc/hl7/bench/BenchmarkRunner.java`) and diffing throughput
scores. Result: 16 of the 20 distinct benchmark/message-type combinations landed within
the ±10% target (average deviation 5.25%). The 4 that didn't were consistently the
**fastest** calls in the suite — `retrieveFirstSegmentOf` on typical/large messages,
2-4µs per call — deviating up to ~18% between runs, while every call at or above ~5µs
stayed under 8%.

This is a known characteristic of nanosecond-scale JVM microbenchmarking, not a harness
defect: a few nanoseconds of OS-scheduling/safepoint jitter is negligible on a 10µs call
but a large relative swing on a 2-3µs one. Increasing `forks` from 1 to 3 measurably
reduced variance (versus a `forks(1)` run that had similar entries deviating by up to
~29%) but did not eliminate it for these specific fastest calls. Treat the `retrieveFirstSegment*`
benchmarks' exact throughput figures as noisier than the rest of this baseline; the
relative ordering across message types and the extraction-benchmark figures are stable.
