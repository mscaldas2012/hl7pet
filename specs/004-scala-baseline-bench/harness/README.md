# Scala Baseline Benchmark Harness

Benchmarks the current Scala HL7-PET engine (`gov.cdc:hl7-pet_2.13:1.2.11`, a normal
Maven dependency — see [../spec.md](../spec.md) FR-002) and commits the results as a
versioned baseline under [../baseline/](../baseline/) for the Rust core migration to
compare against later. Full walkthrough with expected output at
[../quickstart.md](../quickstart.md); result file format at
[../contracts/baseline-artifact-schema.md](../contracts/baseline-artifact-schema.md).

## Build

```bash
mvn compile
```

Resolves `gov.cdc:hl7-pet_2.13:1.2.11` from Maven Central — no local `hl7-pet` checkout,
no vendored source (verify with `./verify-no-vendored-source.sh`).

## Test (harness wiring smoke check only)

```bash
mvn test
```

Not exhaustive engine-correctness coverage — see [../spec.md](../spec.md)'s Testing note
and Roadmap spec `003` (regression-suite).

## Run the benchmark suite

```bash
mvn compile exec:exec
```

Writes `manifest.json` + `jmh-results.json` to a new dated subdirectory under
`../baseline/`. Must be `exec:exec`, not `exec:java` — see the `exec-maven-plugin`
comment in `pom.xml` for why (JMH's forked-VM mode needs a real `java` subprocess).

Takes several minutes: 5 warmup + 5 measurement iterations (1s each) per benchmark
mode, across ~20 benchmark/message-type combinations in both `Throughput` and
`SampleTime` mode, plus JMH's `-prof gc` allocation/memory profiler.

## Project layout

```text
src/main/java/gov/cdc/hl7/bench/
  Corpus.java              loads the synthetic corpus (src/main/resources/corpus/)
  ExclusionLog.java         records parse failures instead of aborting a run
  HostEnvironment.java      captures CPU/OS/JDK/JMH metadata for manifest.json
  ManifestWriter.java       writes manifest.json
  ParsingBenchmarks.java    retrieveFirstSegmentOf / retrieveMultipleSegments
  ExtractionBenchmarks.java getValue / getFirstValue
  BenchmarkRunner.java      main class: runs JMH, then ManifestWriter
src/main/resources/corpus/  interim synthetic HL7 message corpus + corpus-manifest.json
src/test/java/.../HarnessWiringTest.java
verify-no-vendored-source.sh
```
