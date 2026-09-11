# Feature Specification: Python FFI Overhead Benchmark

**Feature Branch**: `6001-python-ffi-benchmark`

**Created**: 2026-09-11

**Status**: Draft

**Input**: User description: "Benchmark the Python binding (crates/python, package hl7pet) against the raw Rust core (crates/core, hl7pet-core) to quantify the PyO3 FFI-crossing overhead -- 'the penalty' of calling through Python instead of Rust directly. Reuse the exact same benchmark corpus and methodology spec 009 (core-perf-validation) already established: fixtures/messages/perf/ (corpusId perf-v2), the same representative-message/PATH selection per feature (scan/parsing, getValue/getFirstValue extraction, hierarchy navigation), and a comparably-shaped hand-rolled per-call sampling harness (warmup + measured iterations, nearest-rank p50/p95 percentiles) so throughput and latency numbers are directly comparable in the same units spec 009 already uses (ops/us, us). The comparison baseline is Rust core numbers re-measured in the same benchmarking session/host as the new Python numbers (not the stale 2026-09-04 spec 009 artifact, to avoid cross-run/cross-host noise), producing a penalty ratio (Python latency or throughput vs Rust) per feature/message. Allocation/memory metrics are out of scope or explicitly flagged not-comparable, since Python has no equivalent to the Rust harness's custom byte-accurate global allocator. Output should be a structured, human-readable comparison artifact (JSON + report) under this spec's own directory, following the existing comparison-artifact-schema.md precedent from spec 009 where reasonable."

## User Scenarios & Testing *(mandatory)*

This is a Language Bindings deliverable (Roadmap module 6000-6999, spec
`6001`) — the direct follow-up spec `6000`'s own plan.md deferred
("Python-side throughput benchmarking is out of scope here and belongs to a
future spec," per the migration plan's Phase 5-before-Phase-6 ordering).
Spec `6000` proved the Python binding is *correct* (100% fixtures-corpus
parity outside three documented, pre-existing exclusions); this spec
answers a different question — how much does calling through Python
*cost* versus calling `hl7pet-core` directly from Rust. Its "users" are the
project maintainer deciding whether the binding's PyO3 boundary is cheap
enough to recommend for hot extraction loops, and any future spec that
needs a real number for "the FFI tax" rather than an assumption.

### User Story 1 - Maintainer runs one comparison and sees Python vs. Rust, feature by feature (Priority: P1)

The project maintainer needs to run a single command (or a small,
documented sequence) and get back a report showing, for the same corpus
messages spec `009` already established, how the Python binding and the
raw Rust core compare on throughput and latency (p50/p95) — broken out by
feature (parsing/scan, `getValue`/`getFirstValue` extraction, hierarchy
`->` navigation), with an explicit penalty ratio (Python ÷ Rust) per
metric, not raw numbers a human has to divide by hand.

**Why this priority**: This is the entire point of the spec — without a
same-corpus, side-by-side report with a computed ratio, "how good is the
Python port" stays a guess rather than a checkable fact.

**Independent Test**: Run the harness this spec produces end-to-end and
confirm the resulting report names, for each benchmarked feature, a Python
figure, a Rust figure, and a penalty ratio, all traceable to the same named
corpus message(s) — not independently-sized or independently-composed
inputs on each side.

**Acceptance Scenarios**:

1. **Given** the shared `fixtures/messages/perf/` corpus (`corpusId:
   perf-v2`), **When** the comparison is run, **Then** the report
   identifies, for every metric it reports, the exact corpus message(s)
   both the Python and Rust sides were measured against, by name.
2. **Given** the finished report, **When** the maintainer looks for
   parsing/scan, `getValue`, `getFirstValue`, and hierarchy navigation
   results specifically, **Then** each appears as its own labeled
   row/section with its own penalty ratio, not merged into one aggregate
   figure.
3. **Given** a corpus message present on the Rust side but somehow
   unavailable or erroring on the Python side (or vice versa), **When**
   the comparison is assembled, **Then** the report explicitly flags the
   mismatch rather than silently comparing numbers from different inputs
   or omitting the row.

---

### User Story 2 - Maintainer can tell whether the penalty is a fixed per-call cost or grows with data size (Priority: P1)

Beyond a single ratio, the maintainer needs to know *why* the binding costs
what it costs: is the overhead a roughly constant per-call FFI tax
(argument marshaling, one Python-object allocation per returned value —
expected and acceptable), or does it grow with message size or result
count (which would suggest the binding is doing extra copying or work the
zero-copy Rust core itself doesn't do, a real finding worth fixing).

**Why this priority**: Equal to User Story 1 — a single averaged ratio
can't distinguish "constant tax, fine for hot loops" from "gets worse the
bigger your message is, avoid for bulk extraction," and that distinction is
exactly what makes the number actionable rather than just a data point.

**Independent Test**: Compare the Python-vs-Rust penalty ratio for the same
feature across at least two differently-sized/differently-shaped corpus
messages (e.g. a "typical" message and the large/high-repetition or
large-hierarchy message spec `009`'s corpus already includes) and confirm
the report states explicitly whether the ratio stayed roughly stable or
grew.

**Acceptance Scenarios**:

1. **Given** a feature benchmarked against both a small and a large corpus
   message, **When** the report is generated, **Then** it states whether
   the penalty ratio for that feature is roughly constant or grows with
   message size/result count, using the actual measured numbers, not a
   qualitative guess.

---

### User Story 3 - Maintainer trusts the numbers because they were measured fairly (Priority: P2)

The comparison is only useful if it isn't confounded by cross-run or
cross-host noise, and if metrics with no fair Python equivalent (Rust's
byte-accurate custom global allocator) are labeled honestly rather than
approximated and presented as if directly comparable.

**Why this priority**: Lower than User Stories 1/2 because it's a
trustworthiness safeguard on the primary result, not a new question — but
a penalty number computed against a different day's, possibly different
machine's Rust numbers would be actively misleading, and a fabricated
Python "allocation count" compared against Rust's precise one would be
worse than reporting nothing.

**Independent Test**: Confirm the Rust-side figures used in the report were
produced by the same benchmarking run/session as the Python-side figures
(not loaded from spec `009`'s already-committed 2026-09-04 artifact), and
confirm allocation/memory metrics are either absent or explicitly marked
not directly comparable.

**Acceptance Scenarios**:

1. **Given** the comparison artifact, **When** the maintainer checks its
   metadata, **Then** both the Python-side and Rust-side figures carry the
   same run timestamp and host/environment metadata.
2. **Given** the report includes any allocation or memory figure, **When**
   the maintainer reads it, **Then** it is explicitly labeled as not
   directly comparable to Rust's byte-accurate figures (or omitted
   entirely) — never presented as an equivalent, apples-to-apples number.

---

### Edge Cases

- What happens when a corpus message causes the Python binding to raise
  where the Rust core succeeds (or vice versa)? Per User Story 1
  Acceptance Scenario 3, this MUST be reported explicitly (which side,
  which message, what happened) — never silently excluded.
- What happens when an operation is too fast to measure precisely at this
  scale (spec `009`'s own JMH-microbenchmark-noise finding for its fastest
  calls)? This spec inherits spec `009`'s existing tolerance convention
  rather than re-deriving one, and flags results that don't meet it as
  noisy rather than as a misleadingly precise ratio.
- What happens for a hierarchy PATH, given the Python API's
  `get_value_hierarchy` takes a profile argument `hl7pet_core::execute`
  doesn't need? The benchmark calls each side with the arguments its own
  real API requires (a profile dict for Python, a parsed
  `HierarchyProfile` for Rust) — the *comparison* is throughput/latency
  for equivalent work, not identical call signatures.
- What happens to spec `009`'s existing committed comparison artifacts
  (`specs/009-core-perf-validation/comparison/`) once this spec produces
  its own? They remain untouched as the historical Rust-vs-Scala record;
  this spec's artifact is additional, under its own directory, not a
  replacement.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Both the Python binding and the raw Rust core MUST be
  benchmarked against the exact same named corpus messages — not merely
  corpora of the same size or composition. Every reported metric MUST be
  traceable to specific corpus message identifier(s) both sides actually
  used.
- **FR-002**: The benchmark corpus MUST be the existing
  `fixtures/messages/perf/` corpus (`corpusId: perf-v2`), reusing spec
  `009`'s existing representative-message/PATH selection per feature
  rather than defining a new one — so results are directly
  cross-referenceable against spec `009`'s own artifacts, not just
  internally self-consistent.
- **FR-003**: Benchmarked features MUST include, at minimum: parsing/scan,
  `getValue`/`getFirstValue` extraction, and hierarchy `->` navigation —
  matching spec `009`'s feature list exactly.
- **FR-004**: Within extraction benchmarking, representative PATH
  expressions MUST cover at least one indexed segment/field selector and
  at least one filter clause, matching spec `009`'s FR-004 breadth.
- **FR-005**: The Rust-side figures used for the penalty ratio MUST be
  freshly measured in the same benchmarking run/session as the Python-side
  figures — never loaded from spec `009`'s previously-committed artifact,
  since cross-run/cross-host variance would produce a misleading penalty
  number (User Story 3).
- **FR-006**: For throughput and latency (p50/p95), the report MUST state
  an explicit penalty ratio (Python value ÷ Rust value, or the inverse for
  throughput where higher is better) per feature/message — not raw numbers
  a reader has to compute themselves.
- **FR-007**: For at least one benchmarked feature, the report MUST state
  explicitly whether the penalty ratio stays roughly constant or grows
  across differently-sized/differently-shaped corpus messages (User Story
  2), using actual measured figures from at least two such messages.
- **FR-008**: Allocation/memory metrics MUST NOT be presented as directly
  comparable between the Python and Rust sides — either omitted entirely
  with a documented reason, or explicitly labeled not-comparable, matching
  how spec `009`'s own report already handles its one confounded metric.
- **FR-009**: Results MUST be recorded and persisted as a versioned,
  machine-readable artifact committed to the repository under this spec's
  own directory (metric values with units, which feature/message, each
  side's version, run date, host/environment metadata) — mirroring spec
  `009`'s FR-008 precedent, kept separate from and never overwriting spec
  `009`'s own committed artifacts (Edge Cases).
- **FR-010**: A failure on either side (Python raises, or Rust returns an
  error) for a corpus message the other side handles successfully MUST be
  reported explicitly — which side, which message, what happened — never
  silently excluded from the aggregate.
- **FR-011**: The sampling methodology (warmup/measured iteration counts,
  percentile definition) MUST match spec `009`'s Rust harness shape
  closely enough that latency/throughput units and definitions are
  directly comparable, not merely superficially similar.
- **FR-012**: The corpus MUST remain fully synthetic/fabricated test data —
  inherited from spec `009`'s FR-010/spec `001`'s FR-009; this spec adds
  no new corpus messages, so no new fabrication decision is needed.

### Key Entities

- **Comparison Run**: One execution producing a full Python-vs-Rust
  report; has a corpus identifier, a single run timestamp and
  host/environment metadata shared by both sides, the benchmarked version
  of each side, and a set of Comparison Results.
- **Comparison Result**: One metric's Python figure and Rust figure, tied
  to a specific feature (parsing/scan, `getValue`, `getFirstValue`,
  hierarchy), a specific corpus message, and a computed penalty ratio.
- **Engine Failure Record**: A documented case where one side errored on a
  corpus message the other handled successfully — which side, which
  message, what happened (FR-010).
- **Comparison Artifact**: The committed, versioned, machine-readable
  file(s) capturing one Comparison Run's results, under this spec's own
  directory (FR-009).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A committed comparison artifact exists showing, for the same
  named corpus messages, Python and Rust figures for throughput and
  latency (p50/p95) across parsing/scan, `getValue`, `getFirstValue`, and
  hierarchy navigation.
- **SC-002**: Every throughput/latency metric in the comparison artifact
  carries an explicit computed penalty ratio — zero metrics with numbers
  but no stated ratio.
- **SC-003**: For at least one feature, the report explicitly states
  whether the penalty ratio is roughly constant or grows with
  message/result size, backed by measured figures from at least two
  differently-sized corpus messages.
- **SC-004**: Zero allocation/memory figures in the report are presented
  as directly comparable between Python and Rust without an explicit
  not-comparable label.
- **SC-005**: A reader who has not run either benchmark can determine, from
  the comparison artifact and this spec's documentation alone,
  approximately how many times slower (or by how much higher latency) the
  Python binding is than raw Rust for each benchmarked feature, and
  whether that overhead looks like a fixed per-call cost or one that scales
  with data size.

## Assumptions

- Spec `6000` (the Python binding) is already implemented and is the
  surface under test; this spec measures it, it does not change its
  behavior — unless a genuine defect is found during benchmarking, in
  which case it is reported, not silently worked around.
- This spec reuses spec `009`'s exact corpus and representative-message/
  PATH-form selection rather than defining new ones, for direct
  comparability and to avoid duplicating that design work.
- No fixed "acceptable overhead" pass/fail threshold is imposed. Unlike
  Rust-vs-Scala (a Constitution-mandated non-regression requirement),
  there is no equivalent Constitutional clause governing how much slower a
  language binding may be than the core it wraps — some PyO3 crossing cost
  is inherent and expected. This report is diagnostic/informational; the
  maintainer interprets the numbers and decides what, if anything, to act
  on.
- Benchmarking infrastructure (harness code, comparison scripts) is not
  itself part of the published `hl7pet-core`/`hl7pet-python` crates'
  public surface — Constitution Principle IV (Multi-Language
  Interoperability) does not apply to it, matching spec `009`'s own final
  Assumption.
- The Rust-side figures used for the penalty ratio come from re-running
  `crates/core/benches/{parsing,extraction,hierarchy}` fresh in the same
  session as the new Python-side harness, not from spec `009`'s
  already-committed 2026-09-04 artifact (FR-005) — a deliberate deviation
  from that spec's own "persisted artifact reusable without re-running"
  precedent, justified because host/environment variance across days would
  corrupt a fair penalty number.
