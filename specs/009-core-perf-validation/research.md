# Research: Core Performance Validation

Companion to [plan.md](plan.md). Each decision below was verified against the
actual, existing harness code and its real output (`specs/004-scala-baseline-bench/`,
`specs/004-scala-baseline-bench/baseline/2026-07-24/jmh-results.json`) — not assumed.

## Decision 1: Promote spec 004's interim-v1 corpus into `fixtures/messages/perf/`, extended with one new large hierarchy message

**Decision**: Relocate spec `004`'s existing 27-message "interim-v1" corpus (currently
bundled at `specs/004-scala-baseline-bench/harness/src/main/resources/corpus/`, loaded
via `corpus-manifest.json`'s `{messageId, messageType, sizeCategory, filePath}` shape)
to `fixtures/messages/perf/`, keeping the exact same manifest shape and message
content unchanged. Add one new message, `large-hierarchy.hl7` (many `OBR`
occurrences, each with many `OBX` children — synthetic, per FR-010), paired with a
new profile, `fixtures/profiles/large-hierarchy.json`, tagged in the manifest as
`sizeCategory: "large-high-repetition"`, `messageType` a new hierarchy-eligible
category. Both the Scala harness (`Corpus.java`'s resource path) and the new Rust
harness (a loader in `benches/common/corpus.rs`) read this single relocated
directory and manifest — satisfying spec.md FR-001's "exact same named corpus
messages" by construction, not by convention.

**Rationale**: Verified two real facts before deciding anything: (1) `fixtures/messages/*.hl7`
(spec `003`'s existing shared corpus) contains only tiny, purpose-built correctness
fixtures — the largest is 12 lines — with nothing resembling spec `004`'s own
"large-high-repetition" (100-`OBX`) message; a throughput/latency comparison run
entirely on 4-12-line messages would be dominated by fixed per-call overhead, not the
scaling behavior specs `005`-`008` actually make claims about. (2) Spec `004`'s
"interim-v1" corpus already has exactly the representative-message-type-plus-large-message
richness this spec needs, and its own status explicitly deferred migrating it to
`fixtures/` — completing that deferred migration, rather than inventing a third
corpus design, directly satisfies both spec `004`'s own unfinished FR-004 and this
spec's FR-002. Neither existing corpus (spec `003`'s or spec `004`'s) has any
hierarchy-eligible large message, since spec `004` predates hierarchy mode (specs
`002`/`008`) entirely — a new one is unavoidable and is the only actual gap.

**Alternatives considered**:
- Benchmark using spec `003`'s existing tiny `fixtures/messages/*.hl7` fixtures
  as-is, no new large message: rejected — produces throughput/latency numbers
  dominated by fixed overhead rather than the scaling behavior this spec's User
  Story 3 needs to confirm (a bounded-scan claim is only meaningfully testable
  against a message large enough that "bounded" and "unbounded" would visibly
  differ).
- Keep spec `004`'s corpus where it is and have the new Rust harness read
  `specs/004-scala-baseline-bench/harness/src/main/resources/corpus/` directly:
  rejected — that path is Maven-resource-shaped (Java classpath resource
  convention) and conceptually belongs to one spec's toolchain, not the shared,
  cross-language `fixtures/` root every other spec's corpus already lives under;
  leaving it there also leaves spec `004`'s own deferred migration undone forever.
- Invent an entirely new, third benchmark corpus design (e.g. procedurally
  generated messages) rather than reusing spec `004`'s existing, already-designed
  one: rejected — spec `004`'s corpus was already deliberately designed for exactly
  this purpose (representative message types, a `minimal` and a `large-high-repetition`
  size category) with real engineering care; reusing it is strictly less work and
  no less rigorous.

## Decision 2: A custom Rust timing harness, not `criterion`

**Decision**: The new Rust benchmarks (`crates/core/benches/`) use a small,
hand-written sampling harness (`benches/common/timing.rs`): a warmup phase
(discarded), then N measured iterations each timed individually via
`std::time::Instant`, collected into a `Vec<Duration>`. Throughput is
`iterations / sum(durations)`; latency percentiles (p50/p95, matching the
Constitution's list) are computed by sorting the samples and indexing — the same
statistical shape JMH's `sample` mode already produces (verified directly in
`jmh-results.json`: `secondaryMetrics` carries `p0.50`/`p0.90`/`p0.95`/etc. from
per-invocation sampling, not from the separate `thrpt`-mode entries). `[[bench]]`
targets are declared `harness = false` in `Cargo.toml`, so this is a plain `fn
main()` binary, not Rust's unstable built-in `#[bench]` (which requires nightly —
every prior spec's Technical Context has committed to stable Rust only) and not
`criterion`.

**Rationale**: `criterion` is the conventional choice for Rust benchmarking, but two
things pushed away from it here: it would be this spec's only new dependency
anywhere in the repository (worth avoiding for a comparison whose entire point is
measuring the crate under test, not exercising a benchmarking library's own
overhead/methodology), and its default reporting is built around mean/median with
confidence intervals rather than JMH-style p50/p95 percentiles as a first-class
output — matching JMH's exact percentile definition (this spec's whole premise is a
literal, same-methodology comparison) is more directly achievable by computing
percentiles from raw per-call samples directly than by first confirming exactly how
a third-party tool's report format maps onto that definition. A hand-rolled
sampling loop is a small amount of code, fully under this project's own control,
and mirrors `src/test_alloc.rs`'s existing precedent of preferring a small,
precisely-specified, dependency-free instrument over a general-purpose library
where the project's own definition of the measurement matters more than breadth of
features.

**Alternatives considered**:
- `criterion` as a `hl7pet-core` `[dev-dependencies]` entry: rejected per above —
  not needed for a bench-only, non-shipped concern, and the project consistently
  prefers minimal, purpose-built instrumentation (`test_alloc.rs`) over pulling in
  a general-purpose library when a precise, small amount of custom code suffices.
- Rust's built-in unstable `#[bench]` attribute (`test` crate): rejected outright —
  requires nightly Rust, contradicting every prior spec's stable-only Technical
  Context and Constitution's "Rust core MUST build on stable Rust."

## Decision 3: Allocation/memory metric reconciliation — bytes allocated is the comparable unit, call count stays a Rust-only diagnostic

**Decision**: The Rust harness's standalone `#[global_allocator]`
(`benches/common/alloc.rs`) tracks three figures, extending `src/test_alloc.rs`'s
existing call-counting design: (a) allocation **call count** (already established
by specs `005`/`007`/`008`'s own unit tests, kept here as a corpus-wide
confirmation of those claims, spec.md User Story 3); (b) total **bytes allocated
per operation** (summing each `Layout::size()` at `alloc`/`realloc` time,
normalized per iteration) — comparable against the Scala side's existing
`secondaryMetrics."gc.alloc.rate.norm"` (bytes/op), spec `004`'s own already-
established "allocation" Constitution-metric category (verified in that spec's
own `contracts/baseline-artifact-schema.md`, not just in the raw JSON); and
(c) a derived **allocation rate** (bytes-per-op × measured throughput = bytes/sec)
comparable against `secondaryMetrics."gc.alloc.rate"` (MB/sec), spec `004`'s own
already-established "memory" category — the two GC-profiler-derived categories
that spec already split apart and documented, reused here rather than re-invented.
The comparison report's allocation/memory sections therefore compare (b) and (c)
as genuine cross-engine verdict metrics, and report Rust's allocation **call
count** (a) as a same-engine-only diagnostic (useful for confirming specs
`005`/`007`/`008`'s own "allocation count independent of size" claims, not for a
Rust-vs-Scala number, since the JVM exposes no equivalent call-count metric).

**Rationale**: Discovered by reading both the real `jmh-results.json` output and
spec `004`'s own already-written contract rather than assuming JMH's allocation
metric shape or inventing a new category split — that spec had already reasoned
through exactly this reconciliation (`gc.alloc.rate.norm` as "allocation",
`gc.alloc.rate` as "memory", explicitly calling the latter "a proxy figure...a
deliberate simplification") for its own Scala-only baseline; reusing its category
boundaries keeps this spec's comparison consistent with the document a reader
would already consult to understand what "memory usage" means for this project,
rather than presenting a second, subtly different definition. Rust's throughput
figure (Decision 2) makes deriving (c) from (b) free — no separate rate-tracking
instrumentation needed.

**Alternatives considered**:
- Only report call count on the Rust side, with no bytes-allocated figure, leaving
  allocation entirely non-comparable cross-engine: rejected — bytes-allocated is
  cheap to add (one extra field in the same existing allocator hook) and is the
  metric category the Constitution actually names ("allocation count" in the
  Constitution's prose maps most naturally onto JMH's actual bytes-based metric,
  since that's the only allocation metric spec `004`'s own baseline already
  captures).
- Attempt to estimate a Scala-side "allocation call count" via bytecode
  instrumentation or a different profiler: rejected as disproportionate engineering
  for a number the JVM doesn't naturally expose and that spec `004`'s already-
  built, already-verified `GCProfiler` wiring has no need to be replaced to get.

## Decision 4: New `HierarchyBenchmarks.java`, using the same profile-loading path spec 002 already documented

**Decision**: Add `HierarchyBenchmarks.java` alongside spec `004`'s existing
`ParsingBenchmarks`/`ExtractionBenchmarks`, benchmarking `HL7ParseUtils`'s
hierarchy-mode `getValue` (the three-argument constructor with `buildHierarchy =
true`, per spec `002`'s own research trace of `HL7HierarchyParser.scala`) against
representative hierarchy PATH expressions (a plain single-hop `->`, at least one
indexed/filtered form, per spec.md FR-004), using `large-hierarchy.hl7`'s profile
(Decision 1) loaded via `HL7HierarchyParser.parseMessageHierarchyFromJson`-style
JSON deserialization — the same Jackson/`DefaultScalaModule`-based mechanism spec
`002`'s own research already traced in the real source, not a new profile-loading
path invented for this spec. Registered in `BenchmarkRunner.java`'s existing
`.include(...)` list alongside the other two classes; same `GCProfiler`, same
`forks(3)`/warmup/measurement configuration, for consistency with the rest of the
suite (spec `004`'s existing reproducibility rationale, its own `SC-003`).

**Rationale**: Spec `004`'s Scala baseline covers zero hierarchy-mode calls today
(confirmed by reading `ExtractionBenchmarks.java`/`ParsingBenchmarks.java` directly
— both exclusively call `HL7StaticParser`) — spec.md FR-003 requires this spec to
add it, and the natural, lowest-risk way is a class matching the existing two
classes' shape exactly, reusing the already-Maven-resolved dependency's own
hierarchy-mode entry points rather than anything new.

**Alternatives considered**:
- Skip Scala-side hierarchy benchmarking, treating hierarchy as Rust-only
  performance data with no baseline to compare against: rejected — this is
  explicitly what spec.md FR-003 and the user's own request ("as many features as
  possible... hierarchy") rule out; a "comparison" with nothing on one side isn't
  one.

## Decision 5: Comparison verdict is a small Python script, matching this repo's existing tooling-script precedent

**Decision**: `specs/009-core-perf-validation/scripts/compare_results.py` loads
`scala-results.json` (JMH's native JSON, both `thrpt` and `sample` mode entries)
and `rust-results.json` (this spec's own Rust harness output, data-model.md's
shape), matches entries by corpus message identifier and feature/operation name,
and emits `comparison-report.json` — one row per (feature, message, metric) tuple,
each carrying both engines' figures, units, and an explicit
`meets`/`beats`/`regresses` verdict (spec.md FR-005/FR-006).

**Rationale**: Python, not a new Rust or Java tool, matching
`fixtures/scripts/validate_corpus.py`'s existing precedent as this repo's language
for cross-cutting, read-only analysis/reporting scripts that don't belong inside
either engine's own codebase. No new tooling-language dependency introduced.

**Alternatives considered**:
- A Rust binary (e.g. a `hl7pet-cli` subcommand) performing the comparison:
  rejected — would make the comparison logic itself part of the crate under test's
  own toolchain, and gains nothing over a small Python script for a one-shot,
  read-two-JSON-files-emit-a-third task.
