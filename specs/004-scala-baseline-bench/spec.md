# Feature Specification: Scala Baseline Benchmark Harness

**Feature Branch**: `004-scala-baseline-bench`

**Created**: 2026-07-24

**Status**: Draft

**Input**: User description: "scala-baseline-bench. make sure for this you use maven dependency - do not copy scala's HL7-pet source code here or reference it locally anywhere - this should work on any machine via maven dependencies"

## User Scenarios & Testing *(mandatory)*

Like specs `001` and `002`, this is an infrastructure/tooling deliverable
(Migration Plan Phase 1, Roadmap module 0-999 "Rust Core", spec `004`) rather
than an end-user-facing feature. Unlike `001`/`002`, it produces both a
runnable harness and committed data, not just documentation. Its "users" are
the people and later specs that build, run, and depend on it — principally
whoever benchmarks the current Scala engine before the Rust rewrite starts,
and the author of Roadmap spec `009` (`core-perf-validation`), who compares
Rust core numbers against this spec's committed baseline.

### User Story 1 - Migration engineer captures the Scala baseline before Rust work begins (Priority: P1)

Before any Rust core code is written, an engineer needs documented,
reproducible performance numbers for the current Scala engine — parsing
throughput, extraction throughput, memory usage, allocations, and latency —
so that later Rust work has something concrete to beat, rather than a vague
"should be faster" goal.

**Why this priority**: This is the entire point of the spec. Without a
captured baseline, Roadmap spec `009` (core-perf-validation) has nothing to
validate the Rust core against, and Constitution Principle II (Zero-Copy &
Lazy Evaluation) becomes an unverifiable claim rather than a measured one.

**Independent Test**: Run the harness against the Scala engine once, and
confirm it produces a results artifact reporting all five metric categories
(parsing throughput, extraction throughput, memory usage, allocations,
latency) for the benchmark corpus, with no category silently missing.

**Acceptance Scenarios**:

1. **Given** the benchmark harness and its target message corpus, **When**
   the harness is run against the current Scala engine, **Then** it produces
   a single results artifact reporting throughput, memory, allocation, and
   latency numbers, broken out per operation (parsing vs. extraction).
2. **Given** a completed benchmark run, **When** the results artifact is
   inspected, **Then** it also records the exact Scala engine version, the
   corpus/fixture identifier used, and the run's date and host/environment
   description — not just the raw numbers.

---

### User Story 2 - Later Rust benchmarks compare against the committed baseline without re-running Scala (Priority: P1)

The author of Roadmap spec `009` needs to compare the Rust core's measured
performance against the Scala engine's numbers. They should not need Scala,
sbt, or a JVM installed to do this — they need a committed, versioned data
file they can read and diff against.

**Why this priority**: Re-running the Scala engine on every future Rust
benchmark comparison would tie Rust CI to a JVM toolchain, defeating the
purpose of a "baseline" (a fixed reference point) and contradicting the
Migration Plan's stated approach of exporting the numbers once and committing
them as static data.

**Independent Test**: Delete or ignore the harness's build tooling entirely
and confirm that spec `009`'s comparison logic can still load and use the
committed baseline results file on its own.

**Acceptance Scenarios**:

1. **Given** the committed baseline results file, **When** a reader with no
   JVM or Maven installed opens it, **Then** every metric value is present
   and its unit and meaning are unambiguous without cross-referencing the
   harness's source code.
2. **Given** the committed baseline results file, **When** spec `009`'s
   comparison logic consumes it programmatically, **Then** it can identify,
   per operation and message type, which metric to compare a new Rust number
   against.

---

### User Story 3 - New contributor rebuilds the harness on a clean machine using only Maven (Priority: P2)

A contributor who has never cloned the Scala `hl7-pet` repository, and has
no local copy of its source, wants to (re)run the benchmark harness to
refresh the baseline. They should be able to do this using only standard
Maven tooling to fetch the Scala engine as a declared dependency — not by
being handed a source tree or a private jar file.

**Why this priority**: This is the explicit constraint from the feature
request. Vendoring the Scala source (or pointing a build file at a local
filesystem path) would silently reintroduce a maintenance burden the
Migration Plan already decided against ("no submodule, no build-time fetch"
from the *Rust* repo's perspective — this spec's harness is the one place a
real dependency on the Scala artifact is allowed to exist, and it must be
declared, not implicit).

**Independent Test**: On a machine with no prior `hl7-pet` (Scala) checkout
and nothing under this repository referencing one, run the harness's build
using only its declared Maven dependency and confirm it resolves the Scala
engine without any manual file copying.

**Acceptance Scenarios**:

1. **Given** a machine with only standard Maven tooling installed and no
   local copy of the Scala engine's source, **When** the harness project is
   built, **Then** the Scala engine is resolved as a normal versioned Maven
   dependency (declared by group/artifact/version coordinates), with no
   vendored source and no build-file reference to a local filesystem path.
2. **Given** this repository's full source tree, **When** it is searched for
   a copy of the Scala engine's source code, **Then** none is found anywhere
   in the repository.

---

### Edge Cases

- What happens when the harness's declared Maven dependency coordinate
  cannot be resolved at build time (artifact not published, network
  unavailable, credentials missing for a private repository)? The harness
  build MUST fail loudly with a clear error identifying the unresolved
  dependency, not silently skip benchmarking or fall back to a bundled copy.
- What happens when a fixture message fails to parse under the Scala engine
  during a benchmark run? That message MUST be excluded from the affected
  metric's results with a logged reason, rather than aborting the entire run
  or silently corrupting the aggregate numbers.
- What happens when the harness is re-run on different hardware than the
  originally committed baseline? The results artifact MUST record enough
  host/environment metadata (e.g. CPU, OS, JVM version) that a reader can
  tell two runs are not directly comparable, since this spec does not
  attempt to normalize numbers across machines.
- What happens if the shared golden-message corpus (Roadmap spec `003`,
  `regression-suite`) does not yet exist when this spec is implemented? The
  harness MUST still be able to run against a documented, versioned subset of
  representative messages committed under this spec's own directory; migrating
  to the shared corpus once spec `003` exists is expected, not blocking.
- What happens when the Scala engine's published version changes (a new
  release) after a baseline has already been committed? The existing
  baseline MUST remain unchanged and readable as a historical record; a
  refreshed baseline is a new, separately dated results artifact, not an
  in-place overwrite.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The harness MUST measure, for the current Scala HL7-PET
  engine, at minimum: parsing throughput, field/value extraction throughput,
  memory usage, allocation counts, and latency (including at least a p50 and
  a p95 figure), matching the metric categories called out in
  `HL7-PET-Rust-Migration-Plan.md` Phase 6.
- **FR-002**: The harness MUST obtain the Scala engine exclusively as a
  versioned dependency declared through the standard Maven dependency
  mechanism (explicit groupId/artifactId/version coordinates), resolved from
  Maven Central — confirmed available and publicly resolvable, no
  authentication required, as `gov.cdc:hl7-pet_2.13:1.2.11`. It MUST NOT
  include a copy of the Scala engine's source code anywhere in this
  repository, and MUST NOT reference a local filesystem path to a Scala
  checkout (e.g. no `system`-scope dependency, no relative-path project
  reference).
- **FR-003**: The harness's build MUST be runnable end-to-end on a machine
  that has never cloned the Scala `hl7-pet` repository, using only standard
  Maven tooling to fetch the declared dependency.
- **FR-004**: The harness MUST run its measurements against a documented,
  versioned corpus of representative HL7 messages — reusing the shared
  golden-message corpus from Roadmap spec `003` once it exists, or a
  documented subset committed under this spec's own directory in the
  meantime (per Edge Cases) — so that later Rust-core benchmark runs
  (Roadmap spec `009`) measure a comparable workload.
- **FR-005**: The harness MUST record and persist its results as a
  versioned, machine-readable artifact committed to this repository,
  including: each metric value and its unit, the operation it applies to
  (parsing vs. extraction), the message corpus/fixture identifier used, the
  benchmarked Scala engine's version, the run date, and host/environment
  metadata (per Edge Cases).
- **FR-006**: The harness MUST report results broken out per operation
  (parsing vs. extraction) rather than a single aggregate number, so later
  comparisons can target specific Rust core components (scanner vs. query
  execution, per Roadmap specs `005`-`007`).
- **FR-007**: The harness MUST report which, if any, corpus messages were
  excluded from a run (e.g. parse failures) along with a reason, rather than
  silently dropping them from the aggregate numbers.
- **FR-008**: The committed results artifact's format MUST be documented
  clearly enough that Roadmap spec `009` (`core-perf-validation`) can load
  and compare against it programmatically, without needing to read the
  harness's implementation.
- **FR-009**: Every message used by the harness MUST be synthetic/fabricated
  test data; real patient data, including de-identified real messages, MUST
  NOT be used, matching spec `001`'s FR-009 and spec `002`'s FR-012.
- **FR-010**: Re-running the harness to refresh the baseline MUST NOT
  overwrite a previously committed baseline results artifact in place; each
  captured baseline is a separately dated, retained artifact (per Edge
  Cases).

### Key Entities

- **Benchmark Run**: A single execution of the harness; has a benchmarked
  Scala engine version, a corpus identifier, a run timestamp, and
  host/environment metadata, plus a set of Metric Results.
- **Metric Result**: A captured measurement (metric category — throughput,
  latency, memory, or allocations; operation — parsing or extraction; value;
  unit) tied to a specific Benchmark Run and message corpus.
- **Baseline Results Artifact**: The committed, versioned, machine-readable
  file (or set of files) capturing one Benchmark Run's Metric Results,
  intended for long-term comparison by Roadmap spec `009`.
- **Benchmark Message Corpus**: The set of representative HL7 messages used
  as harness input — ideally the shared golden-message corpus from Roadmap
  spec `003`, or a documented interim subset (per FR-004).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A committed, versioned baseline exists in the repository
  capturing parsing throughput, extraction throughput, memory usage,
  allocation counts, and latency (p50/p95) for the current Scala engine,
  across the full benchmark corpus.
- **SC-002**: The benchmark harness builds and runs successfully from a
  clean checkout of this repository, on a machine with no local copy of the
  Scala engine's source, using only Maven-resolved dependencies — verified
  by at least one such clean-machine run.
- **SC-003**: Repeated harness runs on the same, unchanged machine produce
  throughput and latency figures within an acceptable variance (target:
  ±10%) of each other, confirming the numbers are stable enough to be a
  meaningful comparison baseline rather than noise.
- **SC-004**: Roadmap spec `009` can load and compare against the committed
  baseline results artifact without executing the Scala engine or the
  benchmark harness itself.
- **SC-005**: A search of the full repository confirms zero copies of Scala
  `hl7-pet` source code and zero local-filesystem-path references to a
  Scala checkout, at any point after this spec is implemented.

## Assumptions

- This spec's harness is the one place in this repository a real,
  declared dependency on the Scala engine is allowed to exist — the
  Migration Plan's "no submodule, no build-time fetch" statement describes
  the *Rust* core and its own build, not this benchmarking tool, which
  exists specifically to measure the Scala baseline once and export the
  result as static data (`HL7-PET-Rust-Migration-Plan.md`, Repository
  Layout section).
- The Scala engine's Maven coordinate is `gov.cdc:hl7-pet_2.13:1.2.11`,
  confirmed published to Maven Central (`repo1.maven.org`, PGP-signed
  release artifacts) and resolvable without authentication or any special
  repository configuration. This is the version the harness benchmarks
  against unless a newer release is deliberately chosen during planning; if
  so, FR-010's "each baseline is a separately dated artifact" rule means the
  new version gets its own results artifact rather than overwriting this
  one. (Note this differs from the upstream `mscaldas2012/hl7-pet` GitHub
  repository's own `build.sbt`, which lists `gov.cdc.hl7` / version `1.2.10`
  and has GitHub Packages publishing commented out — the Central-published
  `gov.cdc:hl7-pet_2.13:1.2.11` artifact is the one this spec depends on,
  not that repository's local build output.)
- Consuming the already-committed Baseline Results Artifact (e.g. from spec
  `009`) requires no Maven dependency, Scala toolchain, or JVM at all — only
  *producing or refreshing* a baseline requires the harness and its Maven
  dependency (Constitution Principle IV does not apply here, since this is
  benchmarking tooling, not a language binding).
- The shared golden-message corpus from Roadmap spec `003`
  (`regression-suite`) is the intended long-term input for this harness;
  since `003` has not been implemented as of this writing, an interim,
  documented subset of representative messages is an acceptable starting
  point (per Edge Cases and FR-004).
- "Representative" messages means a mix of message types and sizes
  reasonably expected in production use (drawing on the same message types
  already referenced by `SPEC.md` and specs `001`/`002`), not an exhaustive
  enumeration of every possible HL7 v2 message shape.
- Benchmark numbers are inherently host-dependent; this spec does not
  attempt to produce numbers portable across machines, only numbers that are
  internally consistent and well-documented enough for Roadmap spec `009`
  to reason about relative (not absolute) improvement.
