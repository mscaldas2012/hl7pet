# Research: Scala Baseline Benchmark Harness

All items below were resolved during planning; no `NEEDS CLARIFICATION` markers remain
in the Technical Context.

## 1. Benchmarking approach: JMH on plain Java/Maven, not sbt/Scala

**Decision**: Drive all measurements with JMH (Java Microbenchmark Harness) from a
plain Java 11 + Maven project. Do not use sbt, Scala, or a hand-written timing loop.

**Rationale**:
- JMH is the de facto standard for JVM microbenchmarking. It handles JIT warm-up,
  dead-code elimination, and forking automatically — directly supporting spec SC-003
  ("repeated runs within ±10% variance"), which a hand-rolled `System.nanoTime()` loop
  would not reliably achieve.
- JMH ships a built-in `-prof gc` profiler that reports allocation rate
  (`gc.alloc.rate.norm`, bytes/op) and supports `-rf json` for structured output —
  covering FR-001's throughput/latency/allocation requirements and FR-005/FR-008's
  "machine-readable, documented artifact" requirement largely out of the box, minimizing
  custom code this harness has to own and get right.
- Confirmed via `javap -classpath hl7-pet_2.13-1.2.11.jar gov.cdc.hl7.HL7StaticParser`
  against the real jar downloaded from Maven Central: the Scala engine's static API
  (`getValue`, `getFirstValue`, `splitFields`, `retrieveSegment`, etc.) compiles to
  plain `public static` Java methods. A pure-Java harness can call it directly — no
  Scala compiler or sbt is needed to *consume* the library, only to have built it
  upstream. This keeps FR-003 ("runnable on a machine that never cloned the Scala
  repo, using only standard Maven tooling") honest.

**Alternatives considered**:
- Hand-written `System.nanoTime()` loops — rejected: JIT warm-up and dead-code-elimination
  pitfalls make naive timing loops unreliable and would fail the SC-003 reproducibility
  target.
- sbt + a Scala benchmarking library (e.g. a ScalaMeter-style setup) — rejected:
  reintroduces an sbt/Scala build-tool requirement for a task that doesn't need one. The
  constitution's target runtime is "JVM 11+, Scala 2.13.x" for the *engine*, not a
  mandate that every consumer of that engine also build with sbt.
- Async-profiler / JFR for allocation/memory profiling — not required initially; JMH's
  built-in `-prof gc` already satisfies FR-001's allocation-count metric at much lower
  setup cost. Left as a possible future addition, not blocking.

## 2. Declaring the Scala engine as a Maven dependency

**Decision**: The harness's `pom.xml` declares:

```xml
<dependency>
  <groupId>gov.cdc</groupId>
  <artifactId>hl7-pet_2.13</artifactId>
  <version>1.2.11</version>
</dependency>
```

**Rationale**: Confirmed by fetching the actual published POM
(`https://repo1.maven.org/maven2/gov/cdc/hl7-pet_2.13/1.2.11/hl7-pet_2.13-1.2.11.pom`) —
it declares `org.scala-lang:scala-library:2.13.13` as a normal (non-optional,
non-provided) dependency, so Maven's transitive resolution pulls the required Scala
runtime automatically; no manual Scala runtime wiring is needed in the harness's own
`pom.xml`. Explicitly pinning the same `scala-library` version in the harness's own
`pom.xml` (via `<dependencyManagement>`) is a minor best practice to avoid silent
diamond-dependency version drift later, but is not required for the dependency to
resolve correctly today.

Artifact and repository facts confirmed directly (not assumed):
- Hosted on Maven Central (`repo1.maven.org`), publicly resolvable, no authentication.
- Release is PGP-signed (`.asc` files present alongside every artifact), consistent
  with a genuine Central Sonatype release rather than a snapshot or mirror.
- This differs from the `mscaldas2012/hl7-pet` GitHub repository's own `build.sbt`
  (organization `gov.cdc.hl7`, version `1.2.10`, GitHub Packages publishing present but
  commented out) — the Central-published `gov.cdc:hl7-pet_2.13:1.2.11` coordinate is a
  separate, newer, publicly-resolvable release and is the one this harness depends on.

**Alternatives considered**: Shading/relocating the Scala library into a fat jar —
unnecessary; this harness is not itself published as a dependency of anything else, so
plain transitive resolution is simpler and sufficient.

## 3. Mapping "parsing" vs. "extraction" operations to real API calls

**Decision**:
- **Parsing throughput** benchmarks call `HL7StaticParser.splitFields` /
  `retrieveSegment` / `retrieveFirstSegmentOf` — raw structural scanning of the message
  string, no PATH-string evaluation.
- **Extraction throughput** benchmarks call `HL7StaticParser.getValue` /
  `getFirstValue` — full PATH-string evaluation against a raw message.

**Rationale**: Confirmed via `javap` against the real jar that `getValue`/`getFirstValue`
both take the raw message `String` and a path `String` as direct arguments — there is no
separate "parse once into an object, extract many times" step exposed by the Scala
engine's static-mode API. This means "extraction throughput," for the Scala engine, is
necessarily an end-to-end parse+extract number, not extraction-in-isolation. FR-006
requires results to be broken out per operation specifically so this distinction is
visible to whoever consumes the artifact later (Roadmap spec `009`) — the results
artifact must label the extraction figures accordingly rather than implying a
directly-comparable "extraction-only" cost the way a hypothetical parse-once Rust API
might report it.

**Alternatives considered**: Benchmark only `getValue`/`getFirstValue` and skip a
separate parsing-only figure — rejected, FR-001 explicitly requires both categories, and
a scan-only number (via `splitFields`/`retrieveSegment`) gives a meaningful figure the
Rust scanner (Roadmap spec `005`) can eventually be compared against even though the
Scala engine re-derives it internally on every `getValue` call.

## 4. Interim benchmark corpus

**Decision**: A small, hand-authored corpus (~20-30 synthetic HL7 v2 messages) covering
a handful of common message types (e.g. `ADT^A01`, `ORU^R01`, `VXU^V04`), including at
least one large/high-repetition message and one minimal message, committed under
`harness/src/main/resources/corpus/`.

**Rationale**: Matches the synthetic-only convention already established by specs `001`
(FR-009) and `002` (FR-012). Keeps this spec independently implementable today without
blocking on Roadmap spec `003` (regression-suite), consistent with the constitution
listing both as parallel Phase 1 deliverables rather than strictly sequential ones.

**Alternatives considered**: Waiting for spec `003` before implementing this spec at
all — rejected; this spec is its own Roadmap entry specifically so it doesn't block on
that dependency, per spec.md's Edge Cases and Assumptions.
