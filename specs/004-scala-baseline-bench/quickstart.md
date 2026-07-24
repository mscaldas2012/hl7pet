# Quickstart: Scala Baseline Benchmark Harness

Validates spec.md's user stories end-to-end: capturing a baseline (US1), producing a
committed artifact readable without the harness (US2), and building on a clean machine
via Maven alone (US3).

## Prerequisites

- JDK 11+
- Maven 3.9+
- **No local checkout of the Scala `hl7-pet` repository, and no local Maven settings
  pointing at a private/GitHub Packages repository** — the whole point of this
  validation run is confirming `gov.cdc:hl7-pet_2.13:1.2.11` resolves from public
  Maven Central alone (spec.md FR-002, FR-003, SC-002).

## 1. Confirm the dependency resolves with no local Scala checkout (US3)

```bash
cd specs/004-scala-baseline-bench/harness
mvn -o dependency:resolve 2>&1 | grep -q "hl7-pet_2.13" && echo "OK: hl7-pet resolved" || echo "FAIL: not resolved"
```

Run once with `mvn dependency:resolve` (no `-o`/offline flag) on a machine with an empty
local `~/.m2/repository` for `gov.cdc` and `org.scala-lang` to prove first-time
resolution works with zero prior setup.

**Expected outcome**: Maven downloads `hl7-pet_2.13-1.2.11.jar` and its transitive
`scala-library` dependency directly from Maven Central; no credentials prompt, no
manual jar placement.

## 2. Run the benchmark suite (US1)

```bash
mvn clean test               # JUnit 5 smoke check (harness wiring only)
mvn compile exec:exec        # the actual JMH benchmark suite
```

`exec:exec` (not `exec:java`) is required: JMH forks a child JVM per benchmark, and that
child reconstructs its classpath from `java.class.path`, which only a real `java`
subprocess (`exec:exec`) sets correctly — `exec:java` runs in-process via Maven's own
classloader and the fork fails with `ClassNotFoundException:
org.openjdk.jmh.runner.ForkedMain`. See `pom.xml`'s `exec-maven-plugin` configuration.

**Expected outcome**: `mvn clean test` runs the JUnit 5 smoke check (harness wiring
only — corpus loads, dependency is callable); `mvn compile exec:exec` executes the JMH
benchmark suite across the interim corpus (research.md #4) and writes:

```text
specs/004-scala-baseline-bench/baseline/<today's-date>/manifest.json
specs/004-scala-baseline-bench/baseline/<today's-date>/jmh-results.json
```

Confirm both files exist and `manifest.json`'s `excludedMessages` is `[]` (or, if
non-empty, that each entry has a `reason` — spec.md FR-007).

## 3. Confirm the artifact is self-describing (US2)

Without opening any harness source file, open `manifest.json` and
`jmh-results.json` directly and confirm, per
[contracts/baseline-artifact-schema.md](contracts/baseline-artifact-schema.md):

- Every metric category from spec.md FR-001 (throughput, latencyP50, latencyP95,
  memory, allocation) is present for at least one message type.
- Results are broken out per `operation` (`parsing` vs. `extraction`) — not a single
  aggregate number (FR-006).
- `manifest.json`'s `engineCoordinate`, `corpusId`, `runDate`, and `hostEnvironment`
  are all populated (FR-005).

## 4. Confirm reproducibility (SC-003)

```bash
mvn compile exec:exec   # run #2, same machine
```

Compare the new `baseline/<today's-date2>/jmh-results.json` throughput and latency
figures against run #1's. (Two runs on the same date collide on the directory name —
`BenchmarkRunner` appends a `-2`, `-3`, ... suffix automatically per spec.md FR-010,
rather than overwriting.)

**Expected outcome, as actually verified**: the large majority (16/20 in the reference
run — see [baseline/README.md](baseline/README.md)'s Reproducibility section) land
within ±10%, average deviation ~5%. The fastest calls in the suite (`retrieveFirstSegmentOf`,
2-4µs/op) can swing further — this is expected JVM microbenchmarking noise at that time
scale, not a harness defect, and is documented rather than hidden.

## 5. Confirm no vendored source anywhere (SC-005)

```bash
grep -rIl "package gov.cdc.hl7" --include="*.scala" . || echo "OK: no vendored Scala source found"
```

**Expected outcome**: `OK: no vendored Scala source found`.
