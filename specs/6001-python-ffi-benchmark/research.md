# Phase 0 Research: Python FFI Overhead Benchmark

All decisions below were verified directly against the actual existing
code (`crates/core/benches/{common/*.rs,parsing.rs,extraction.rs,
hierarchy.rs}`, `specs/009-core-perf-validation/scripts/compare_results.py`,
`crates/python/src/lib.rs`, `crates/python/python/hl7pet/__init__.py`), not
assumed from spec 009's documentation alone.

## 1. Where the fresh Rust re-run writes its output

**Decision**: Add an optional `PERF_RUN_OUTPUT_DIR` environment variable to
`crates/core/benches/common/output.rs`'s `run_dir()`. When set, it is used
directly as the run directory; when unset, behavior is byte-for-byte
unchanged (still resolves `specs/009-core-perf-validation/comparison/
<PERF_RUN_DATE or today>`). This spec's own tooling (quickstart.md, tasks)
sets it to `specs/6001-python-ffi-benchmark/comparison/<run-date>/` before
invoking `cargo bench -p hl7pet-core`.

**Rationale**: `run_dir()` currently hardcodes
`../../specs/009-core-perf-validation/comparison` (verified by reading the
function directly) — there is no existing way to point a `cargo bench` run
at a different directory. FR-005 requires the Rust baseline to be
re-measured fresh in the *same session* as the new Python numbers, and
FR-009 requires the resulting artifact to live under this spec's own
directory, never overwriting spec `009`'s. An opt-in env var is the
smallest change that satisfies both: zero behavior change for anyone
running spec `009`'s own existing workflow (the env var is simply unset),
and a clean, colocated output directory for this spec's four JSON files
(three fresh `rust-results-*.json` plus the new `python-results.json`)
without touching `PERF_RUN_DATE`'s existing meaning.

**Alternatives considered**:
- Reuse spec `009`'s already-committed 2026-09-04 artifact as the Rust
  baseline instead of re-running — rejected explicitly by spec.md FR-005/
  Assumptions: a different day, quite possibly a different machine, risks
  a penalty ratio confounded by cross-run variance rather than reflecting
  the actual FFI cost.
- Have the new harness re-run `cargo bench` and then copy
  `specs/009-core-perf-validation/comparison/<today>/rust-results-*.json`
  into this spec's directory as a post-processing step — rejected as
  needless indirection once a one-line opt-in override in the harness
  itself does the same thing without a copy step or without silently
  polluting spec `009`'s own directory with a run that isn't really that
  spec's work.

## 2. Reusing spec 009's exact representative-message/PATH selection

**Decision**: The Python harness's `common/corpus.py` reimplements
`common/corpus.rs`'s `representative_typical_per_type()` and
`unique_by_size_category()` logic identically (same manifest file, same
selection rules), and `extraction.py`/`hierarchy.py` hard-code the exact
same PATH forms and per-message-type field mapping already in
`extraction.rs`'s `representative_field()` (`PV1-3.1` for `ADT^A01`/
`ADT^A08`, `OBX-5` for `ORU^R01`, `RXA-5.2` for `VXU^V04`, `OBR-4.2` for
`ORM^O01`) and `hierarchy.rs`'s four-form `PATH_FORMS` constant
(`"OBR[1] -> OBX-5"`, `"OBR[1] -> OBX[3]-5"`,
`"OBR[1] -> OBX[@5='VAL-1-2']-5"`, `"OBR -> OBX-5"`) against
`large_hierarchy_028`.

**Rationale**: FR-001/FR-002 require the exact same named corpus messages
on both sides — copying the Rust targets' already-established, already
spec-009-verified selection is the only way to guarantee that without
re-litigating scope decisions spec `009` already made (research.md #1's
own addendum there explains *why* only one "typical" message per type is
benchmarked, not all five — reusing it inherits that reasoning rather than
re-deriving it).

**Alternatives considered**: Have the Python harness read the manifest and
apply its own independent selection logic — rejected: any behavioral drift
between the two implementations (e.g. a different tie-break for "first
typical entry per type") would silently break FR-001's same-corpus
guarantee in a way that's easy to miss until the comparison script reports
a confusing `path-form-mismatch`-shaped gap.

## 3. Python's "parsing" feature has no standalone call to benchmark

**Decision**: Python's parsing/scan-proxy benchmark (`parsing.py`) times
`hl7pet.get_first_value(message, "MSH-1")` — the cheapest real call the
public API exposes — and the comparison report labels this row's
`pathExpression` as `"MSH-1"` (not `null`, unlike the Rust side's pure
`scan()` rows) with an explicit note that this is a scan+parse+execute
proxy, not an isolated scan measurement.

**Rationale**: Verified directly against `crates/python/python/hl7pet/
__init__.py`'s re-exports and `crates/python/src/lib.rs`: the binding
exposes `get_value`/`get_value_hierarchy`/`get_first_value`/
`get_value_located`/`get_first_value_located`/`get_values` — no standalone
`scan()` (spec `6000` FR-001 only required extraction, not raw scanner
internals, and no binding entry point wraps `hl7pet_core::scan` alone).
Rather than skip the "parsing" feature entirely (which FR-003 requires at
minimum) or silently compare it against something it isn't, the smallest,
cheapest real call available is used and the proxy is stated plainly in
both the artifact and the report — Constitution Principle V's "documented
rather than silently mishandled" standard, the same standard spec `6000`'s
own `parity_check.py` already applied to its own scope gaps.

**Alternatives considered**:
- Add a new `scan()`-only entry point to the Python binding just for this
  benchmark — rejected: that would be a real, permanent addition to
  `hl7pet-python`'s public surface (Constitution Principle IV/spec `6000`'s
  own sync-tooling territory) purely to serve a diagnostic benchmark;
  disproportionate scope creep for this spec.
- Drop the "parsing" feature from the Python side and only compare
  extraction/hierarchy — rejected: FR-003 requires it, and the proxy
  measurement (the fixed floor cost of any Python call at all: argument
  marshaling in, one scan, one parse, one minimal execute, one value out)
  is itself informative — arguably the purest read of "the FFI tax" since
  it is the smallest unit of real work the binding can be asked to do.

## 4. Sampling methodology parity

**Decision**: `crates/python/benches/common/timing.py` mirrors
`timing.rs`'s `sample()`/`measure_operation()` exactly: 50 warmup calls
(discarded), 500 measured calls timed individually via
`time.perf_counter()`, sorted, nearest-rank percentile
(`sorted[round((n-1) * pct)]`) for p50/p95, throughput as
`n / total_microseconds`.

**Rationale**: FR-011 requires the two harnesses' sampling shape to match
closely enough that latency/throughput definitions are directly
comparable, not merely superficially similar — nearest-rank percentile in
particular is a specific choice (matching JMH's own definition, per spec
`009` research.md #2) that a different percentile method (e.g. linear
interpolation) would silently make non-comparable despite both being
labeled "p50"/"p95". `time.perf_counter()` is Python's standard
monotonic, high-resolution clock — the direct analog of Rust's
`Instant::now()`.

**Alternatives considered**: Python's `timeit` module — rejected: its API
shape (batched loops, `Timer.repeat()`) doesn't naturally produce a
per-call duration list to compute nearest-rank percentiles from without
extra reshaping, and pulling in a benchmarking library (`pytest-benchmark`,
etc.) would add a new dev dependency for no benefit over the ~30 lines
`timing.rs`'s own approach already proves out.

## 5. Allocation/memory metrics: omitted, not approximated

**Decision**: `python-results.json` rows carry only `throughput`,
`latencyP50`, `latencyP95` — no `allocationBytesPerOp`/
`allocationCallCount`/`memoryAllocRateBytesPerSec` fields at all (rather
than present-but-null, or an approximated value from `tracemalloc`).
`compare_penalty.py`'s report states explicitly, once, that allocation/
memory comparison is out of scope for this spec and why.

**Rationale**: FR-008 requires allocation/memory metrics never be presented
as directly comparable. `tracemalloc` measures Python-object allocation
(new `str`/`list`/`LocatedValue` instances the binding creates) but has no
visibility at all into the Rust-side allocations the same call makes below
the PyO3 boundary — a "Python allocation count" from it would answer a
different, narrower question than Rust's byte-accurate global-allocator
figure and risks being read as equivalent. Omitting the fields entirely
(rather than including a caveated approximation) is the more honest choice
and avoids `compare_penalty.py` needing special-case logic to keep a
half-comparable metric out of the ratio computation.

**Alternatives considered**: Use `tracemalloc` and label it clearly
not-comparable, same as `memoryAllocRateBytesPerSec` already is on the
Rust-vs-Scala side — rejected: unlike that metric (a real number that's
merely confounded when compared across throughput levels), a
`tracemalloc` figure would measure a fundamentally different, partial
thing (Python heap only) and its inclusion — even caveated — invites the
exact misreading FR-008 exists to prevent.

## 6. Penalty ratio direction and "constant vs. scaling" determination

**Decision**: `penaltyRatio` is always framed so that **greater than 1.0
means Python costs more**: for throughput, `rustValue / pythonValue`; for
latency (p50/p95), `pythonValue / rustValue`. For User Story 2, the
`getValue` extraction feature is used (FR-007) — it already has both a
"typical" message and the `large-high-repetition` message
(`oru_r01_large_026`) benchmarked with the same `OBX-5` path
(`extraction.rs`'s existing scope, research.md #2), giving two
genuinely different result-set sizes for the same feature and path
without adding any new corpus message. The report states both ratios
side by side with no fixed numeric threshold for "roughly constant" —
Assumptions already establishes this spec imposes no pass/fail bar, so the
comparison is presented as data for the maintainer to read, not reduced to
a single bucketed verdict.

**Rationale**: A single averaged ratio can't answer User Story 2's
question; two same-feature, different-scale data points can, without
requiring a new "how much drift counts as scaling" threshold decision
that no stakeholder has actually asked for yet.

**Alternatives considered**: Define a fixed threshold (e.g. "ratio changes
by more than 20% = scaling") — rejected: an arbitrary threshold invented
here would misrepresent this as a pass/fail gate when spec.md's own
Assumptions explicitly says there isn't one; better to show the numbers
and let the reader judge, consistent with FR-006/FR-007's own "report the
ratio, don't invent a verdict" framing.

## 7. Same-session proof: shared run directory, not string-identical host metadata

**Decision**: Both `python-results.json` and the freshly-written
`rust-results-*.json` land in the same `specs/6001-python-ffi-benchmark/
comparison/<run-date>/` directory (research.md #1), which
`compare_penalty.py` takes as the proof both sides were measured in the
same session. Each side still records its own native host-environment
string encoding (Rust: `std::env::consts::OS`/`ARCH`, e.g. `"macos"`/
`"aarch64"`; Python: `platform.system()`/`platform.machine()`, e.g.
`"Darwin"`/`"arm64"`) rather than the harness trying to normalize them
into one shared vocabulary.

**Rationale**: User Story 3 Acceptance Scenario 1 requires both sides to
"carry the same run timestamp and host/environment metadata" — read as
"provably the same machine and session," not "byte-identical metadata
encoding." Rust's and Python's standard libraries don't share a string
vocabulary for OS/architecture names, and inventing a translation table
between them is unnecessary machinery: the shared run directory (and a
shared `runDate` field both sides' output carries) already proves same-
session measurement unambiguously, which is the actual requirement.

**Alternatives considered**: Write a Rust-string-to-Python-string (or
vice versa) normalization table so `hostEnvironment` reads identically on
both sides — rejected as unneeded complexity for a cosmetic concern the
shared directory already resolves structurally.

## 8. A new comparison script, not an extension of spec 009's

**Decision**: `specs/6001-python-ffi-benchmark/scripts/compare_penalty.py`
is a new, independent script — it does not import from or extend spec
`009`'s `compare_results.py`.

**Rationale**: The two scripts answer different questions with different
output shapes. `compare_results.py` computes a three-way
meets/beats/regresses verdict against a Scala baseline with a fixed ±10%
tolerance (a Constitution-mandated non-regression gate). This spec's
script computes a two-way penalty ratio with explicitly no pass/fail bar
(spec.md Assumptions). Sharing code between them (e.g. a common JSON
loader) would save perhaps a dozen lines at the cost of coupling two
scripts that exist to answer genuinely different questions — not worth it
at this scale, and spec `009`'s own script must stay untouched regardless
(Constitution Principle I doesn't apply, but there's no reason to risk
regressing a script with its own already-verified, committed output).

**Alternatives considered**: Add a `--mode=penalty` flag to
`compare_results.py` — rejected: would require threading a third engine's
worth of parsing/verdict logic through a script whose existing structure
(`parse_scala_entries`/`parse_rust_entries`, `METHOD_TABLE` keyed to JMH
benchmark method names) has no natural place for it without a larger,
riskier refactor of an already-working, already-committed tool.

## 9. `maturin develop` must be `--release` for this benchmark specifically

**Decision**: `quickstart.md`'s Prerequisites and `crates/python/README.md`'s
Benchmarking section both require `maturin develop --release`, explicitly
distinct from spec `6000`'s own quickstart (plain `maturin develop`, correct
for day-to-day development/testing).

**Rationale**: A real methodology bug, found and fixed against an actual
run, not assumed. The first full comparison run (2026-09-11) used a
debug-mode `_hl7pet` extension (installed via the plain `maturin develop`
spec `6000`'s and this spec's own quickstart both originally documented)
against `cargo bench`'s always-release Rust build — a debug/release
mismatch, not a fair comparison. Re-running with `maturin develop
--release` and nothing else changed dropped every throughput penalty
ratio by roughly 3-5x uniformly (e.g. `parsing`: 5.5-7.3x → 1.1-1.5x;
`getFirstValue`: 23.2-36.2x → 4.5-7.2x; `hierarchy`: 14.7-186.4x →
2.7-37.9x) — confirming the debug build, not a real property of the FFI
boundary, was responsible for the bulk of the originally-measured
overhead. The qualitative User Story 2 finding (penalty ratio drops from
the typical to the large message, not rises) held in both runs
(26.0x→12.6x debug; 4.8x→2.5x release) — the debug/release mismatch
inflated the *magnitude* uniformly but did not change the *shape* of the
finding, which is reassuring but was verified, not assumed, before relying
on it. The committed `specs/6001-python-ffi-benchmark/comparison/
2026-09-11/` artifact is the corrected, release-built run; the original
debug-built numbers were discarded, not kept alongside as a second
"reference" run, since presenting them at all invites exactly the
misleading comparison this fix exists to prevent.

**Alternatives considered**: Keep both runs and label the debug one as
such — rejected: a debug-build number has no diagnostic value for "the
FFI tax" (it also includes the cost of unoptimized Rust code, an unrelated
variable this spec was never designed to measure) and would only invite a
future reader to misquote it as if release-comparable.
