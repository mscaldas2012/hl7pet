# Feature Specification: Core Performance Validation

**Feature Branch**: `009-core-perf-validation`

**Created**: 2026-09-03

**Status**: Draft

**Input**: User description: "Benchmark the Rust core (specs 005-008: scanner, PATH parser, query execution, lazy hierarchy navigation) against the existing Scala engine's throughput, latency, memory, and allocation numbers, running both engines against the same shared fixtures/ corpus, covering as many features as practical: getValue, getFirstValue, and hierarchy (->) navigation, confirming the Rust core meets or beats the Scala baseline and that its zero-copy/lazy design targets are actually met, not just asserted."

## User Scenarios & Testing *(mandatory)*

This is a Rust Core / Engine Migration deliverable (Migration Plan Phase 6,
Roadmap module 0-999, spec `009`) — the spec `004`/`005`-`008` chain's payoff:
until now, "the Rust core doesn't regress performance versus Scala"
(Constitution's Performance & Portability Standards) and "zero-copy/lazy
evaluation" (Constitution Principle II) have been argued from design and
proven only at unit-test scale (allocation-counting tests scoped to a single
call). This spec is where both claims get measured for real, on the same
workload, side by side. Its "users" are the project maintainer deciding
whether the Rust core is ready to replace the Scala engine in practice, and
future specs (Language Bindings, 6000-6999) that need to know the underlying
engine's real performance characteristics before building on top of it.

### User Story 1 - Maintainer runs one comparison and sees Rust vs. Scala, feature by feature (Priority: P1)

The project maintainer needs to run a single command (or a small, documented
sequence) and get back a report showing, for the same corpus of messages,
how the Rust core and the Scala engine compare on throughput, latency
(p50/p95), memory, and allocation count — broken out by feature (parsing,
`getValue`, `getFirstValue`, hierarchy `->` navigation), not collapsed into
one aggregate number that hides which specific capability is faster or
slower.

**Why this priority**: This is the entire point of the spec. Without a
same-corpus, side-by-side report, "is the Rust core ready" stays a matter of
confidence in the design rather than a checkable fact — and spec `004`'s own
baseline was captured on an interim corpus that predates the shared
`fixtures/` corpus (Roadmap spec `003`), so today there is no comparison at
all that both engines were measured on identical input.

**Independent Test**: Run the harness(es) this spec produces end-to-end and
confirm the resulting report names, for each benchmarked feature, a Scala
figure and a Rust figure for the same metric, computed from the same named
corpus messages — not two independently-sized or independently-composed
message sets.

**Acceptance Scenarios**:

1. **Given** the shared `fixtures/` corpus, **When** the comparison is run,
   **Then** the report identifies, for every metric it reports, the exact
   corpus message(s) both engines were measured against, by name.
2. **Given** a corpus message present when the Scala side runs but somehow
   unavailable when the Rust side runs (or vice versa), **When** the
   comparison is assembled, **Then** the report explicitly flags the mismatch
   rather than silently comparing numbers from different inputs.
3. **Given** the finished report, **When** the maintainer looks for
   `getValue`, `getFirstValue`, and hierarchy (`->`) navigation results
   specifically, **Then** each appears as its own labeled row/section, not
   merged into a single "extraction" figure.

---

### User Story 2 - Confirm the Rust core does not regress performance versus Scala (Priority: P1)

The Constitution's Performance & Portability Standards state the Rust core
"MUST NOT regress parsing throughput, extraction throughput, memory usage,
allocation count, or latency versus the Scala baseline it replaces." This
spec needs to produce a clear pass/fail (or "regression, with detail")
verdict against that requirement — not just numbers a human has to
eyeball-compare.

**Why this priority**: Equal to User Story 1 — a report with numbers but no
verdict against the actual Constitutional requirement doesn't answer the
question this spec exists to answer. Every prior Rust-core spec (`005`-`008`)
explicitly deferred this exact validation to spec `009`; this is where that
deferred obligation is finally discharged.

**Independent Test**: For each of the five metric categories the
Constitution names (parsing throughput, extraction throughput, memory usage,
allocation count, latency), the report states explicitly whether the Rust
core meets or beats the Scala figure, or regresses — and by how much.

**Acceptance Scenarios**:

1. **Given** a metric where Rust meets or beats Scala, **When** the report is
   generated, **Then** that metric is marked as passing, with the actual
   numbers shown.
2. **Given** a metric where Rust regresses versus Scala, **When** the report
   is generated, **Then** that metric is marked as a regression — not hidden,
   averaged away, or silently omitted — per Constitution Principle V's
   "documented rather than silently mishandled."

---

### User Story 3 - Confirm the zero-copy/lazy design claims hold at realistic scale, not just in a unit test (Priority: P2)

Specs `005` and `007` each proved, via a dedicated unit test, that a single
call's allocation count is independent of message size or repetition count.
This spec needs to confirm the same structural property holds across the
full benchmark corpus and across every feature (including hierarchy
navigation's "no full-message tree" claim, spec `008` SC-002) — a broader,
corpus-wide confirmation of a claim so far only checked one call at a time.

**Why this priority**: Lower than User Stories 1/2 because it's a
reinforcement of already-tested structural properties, not a new question —
but still real value: a subtle regression that only shows up on certain
corpus messages (not the one hand-picked unit test input) would otherwise go
undetected.

**Independent Test**: Run the Rust benchmarks' allocation-count measurements
across every corpus message and confirm the counts scale the way specs `005`
(scanner), `007` (query execution), and `008` (hierarchy navigation) each
already claim — not just for the one message each spec's own unit test used.

**Acceptance Scenarios**:

1. **Given** the full corpus, **When** allocation counts are measured for
   `execute_hierarchy` across every hierarchy-eligible message, **Then** no
   message shows allocation behavior consistent with a full-message tree
   having been built (spec `008` FR-003).

---

### Edge Cases

- What happens when a corpus message causes the Scala engine to throw, or
  the Rust core to return an error, where the other engine succeeds? Per
  User Story 1 Acceptance Scenario 2's principle, this MUST be reported
  explicitly (which engine, which message, what happened) — never silently
  excluded from the aggregate the way an ordinary "no data" outcome would be.
- What happens when the two engines' fastest/smallest-scale operations are
  too fast to measure precisely (the JVM-microbenchmark noise spec `004`'s
  own baseline already documented for its fastest calls)? This spec inherits
  spec `004`'s existing ±10% reproducibility tolerance (its SC-003) rather
  than re-deriving a new one, and must flag results that don't meet it as
  noisy rather than as a false regression or false pass.
- What happens when a feature has no natural Scala equivalent to compare
  against (there is none here — `getValue`/`getFirstValue`/hierarchy `->`
  all exist on both engines — but the question matters for scoping which
  PATH forms to benchmark: index selectors, filters, `$LAST`/`*`)? Every
  benchmarked PATH form MUST exist and be meaningful on both engines; a form
  unique to one engine (there are none as of this spec) would be documented
  as Rust-only and excluded from the comparative verdict, not silently
  compared against nothing.
- What happens to spec `004`'s existing committed baseline artifacts (dated,
  retained, never overwritten per that spec's FR-010) once this spec
  produces a new same-corpus comparison? They remain as the historical
  interim-corpus record; this spec's own comparison artifact is additional,
  not a replacement, following the same "each baseline is a separately
  dated, retained artifact" convention.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Both engines MUST be benchmarked against the exact same named
  corpus messages — not merely corpora of the same size or composition.
  Every reported metric MUST be traceable to specific corpus message
  identifier(s) that both the Scala run and the Rust run actually used.
- **FR-002**: The benchmark corpus MUST be (or be built from) the shared
  `fixtures/` corpus (Roadmap spec `003`), completing the migration off spec
  `004`'s interim synthetic corpus that spec's own status explicitly deferred
  ("migrating to the shared `fixtures/` corpus is a natural follow-up, not
  done in this PR"). How the existing Scala harness's per-message-type/
  per-size-category benchmark design (spec `004`) reconciles with
  `fixtures/`'s existing organization (by purpose/fixture family, not message
  type/size category) is a planning-phase design decision, not specified
  here.
- **FR-003**: Benchmarked features MUST include, at minimum: raw structural
  parsing/scanning (spec `004`'s existing "parsing" category, spec `005`'s
  Rust equivalent), `getValue` and `getFirstValue` extraction (spec `004`'s
  existing "extraction" category, spec `007`'s Rust equivalent), and
  hierarchy `->` navigation (spec `008`'s Rust deliverable) — a feature spec
  `004`'s existing Scala benchmarks do not cover at all today and this spec
  MUST add.
- **FR-004**: Within `getValue`/`getFirstValue` benchmarking, representative
  PATH expressions MUST cover more than the trivial single-field case:
  at least one indexed segment/field selector (spec `001`'s `SEG_IDX`/
  `FIELD_IDX`), at least one filter clause (spec `001`'s `FILTER`), and at
  least one hierarchy expression (spec `002`'s `->`) — covering breadth of
  PATH capability without requiring a distinct benchmark method per PATH
  form, since all resolve through the same `getValue`/`getFirstValue`-shaped
  call on both engines.
- **FR-005**: For each of the five metric categories the Constitution names
  (parsing throughput, extraction throughput, memory usage, allocation
  count, latency including p50/p95), the resulting report MUST state an
  explicit verdict — meets/beats Scala, or regresses, with the actual
  figures — not present numbers without a verdict a reader has to compute
  themselves.
- **FR-006**: A regression on any metric MUST be reported explicitly and
  distinctly from a pass — never averaged into an aggregate that could mask
  it, per Constitution Principle V and User Story 2's Acceptance Scenario 2.
- **FR-007**: The comparison MUST be run per corpus message (or a clearly
  documented representative subset), broken out by feature, matching spec
  `004`'s existing FR-006 precedent ("results broken out per operation...
  not a single aggregate number") extended to cover the new hierarchy
  feature and the Rust side.
- **FR-008**: Results MUST be recorded and persisted as a versioned,
  machine-readable artifact committed to the repository (mirroring spec
  `004`'s FR-005/FR-008 precedent: metric values with units, which
  feature/operation, which corpus message(s), each engine's version, run
  date, host/environment metadata) — so a later spec can load and compare
  against it without re-running either engine, the same guarantee spec
  `004`'s own SC-004 already established for its own baseline.
- **FR-009**: An engine failure on a specific corpus message (an unexpected
  error/exception from either engine on input the other engine handles
  successfully) MUST be reported explicitly — which engine, which message,
  what happened — never silently excluded from the aggregate (Edge Cases).
- **FR-010**: The corpus MUST remain fully synthetic/fabricated test data;
  real patient data, including de-identified real messages, MUST NOT be used
  — matching spec `001` FR-009, spec `002` FR-012, and spec `004` FR-009.
- **FR-011**: This spec's own comparison run(s) MUST NOT overwrite spec
  `004`'s previously committed baseline artifact(s) in place — each is a
  separately dated, retained artifact, matching spec `004`'s existing FR-010
  convention (Edge Cases).
- **FR-012**: Rust-side allocation-count measurements MUST be captured across
  the full benchmark corpus for every benchmarked feature, not only the
  single hand-picked message each of specs `005`/`007`/`008`'s own unit
  tests already used — confirming (or falsifying) those specs' "allocation
  count independent of message size" claims at corpus scale (User Story 3).

### Key Entities

- **Comparison Run**: One execution producing a full Rust-vs-Scala report;
  has a corpus identifier, a run timestamp, host/environment metadata, the
  benchmarked version of each engine, and a set of Comparison Results.
- **Comparison Result**: One metric's Rust figure and Scala figure, tied to
  a specific feature (parsing / `getValue` / `getFirstValue` / hierarchy),
  a specific corpus message (or documented representative subset), and a
  pass/regression verdict against the Constitution's non-regression
  requirement.
- **Engine Failure Record**: A documented case where one engine errored on a
  corpus message the other handled successfully — which engine, which
  message, what happened (FR-009).
- **Comparison Artifact**: The committed, versioned, machine-readable file
  (or set of files) capturing one Comparison Run's results, for later specs
  to load without re-running either engine (FR-008).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A committed comparison artifact exists showing, for the same
  named corpus messages, Rust and Scala figures for all five Constitution
  metric categories, across parsing, `getValue`, `getFirstValue`, and
  hierarchy navigation.
- **SC-002**: Every metric in the comparison artifact carries an explicit
  meets/beats-or-regresses verdict against the Scala baseline — zero metrics
  with numbers but no stated verdict.
- **SC-003**: Zero corpus-message/feature combinations are silently excluded
  from the report for a reason other than an explicitly documented Engine
  Failure Record or PATH-form/feature mismatch (Edge Cases).
- **SC-004**: Rust-side allocation counts, measured across the full
  benchmark corpus, are independent of message size/repetition count for
  every benchmarked feature — confirming specs `005`/`007`/`008`'s
  allocation-independence claims at corpus scale, not just their original
  single-message unit tests.
- **SC-005**: A reader who has not run either benchmark can determine, from
  the comparison artifact and this spec's documentation alone, whether the
  Rust core currently satisfies the Constitution's "MUST NOT regress"
  requirement in full, partially, or not at all — and if not in full,
  exactly which metric(s)/feature(s) regress and by how much.

## Assumptions

- Specs `005` (scanner), `006` (parser), `007` (query execution), and `008`
  (hierarchy navigation) are already implemented and are the Rust surface
  under test; this spec measures them, it does not change their behavior.
- Spec `004`'s existing Scala JMH harness (`specs/004-scala-baseline-bench/harness/`)
  is extended, not replaced — its Maven-dependency-only, no-vendored-source
  constraint (that spec's FR-002/FR-003, SC-005) carries forward unchanged
  to whatever hierarchy-mode benchmarks this spec adds to it.
  `HL7ParseUtils`/`HL7HierarchyParser` (hierarchy mode) are already part of
  the same Maven-resolved `gov.cdc:hl7-pet_2.13:1.2.11` dependency spec `004`
  already declared — no new Scala dependency is needed to add hierarchy
  benchmarks.
- The specific mechanism for reconciling spec `004`'s existing
  message-type/size-category corpus design with `fixtures/`'s
  purpose-organized layout (FR-002) is a planning-phase decision, not
  guessed here — as is the specific Rust benchmarking approach/tooling
  (e.g. `criterion` as a dev-only dependency, or a lighter custom timing
  harness matching the existing crate's allocation-counting precedent). Any
  new dependency this spec adds is dev/bench-only, never a runtime
  dependency of the published `hl7pet-core` crate, so it does not implicate
  [[project_dependency_policy]]'s public-API/cross-compilation constraints
  the way spec `008`'s `serde_json` promotion did.
- "As many features as practical" (the feature request) is bounded by
  FR-003/FR-004's minimum feature/PATH-form list; additional PATH forms
  (e.g. `$LAST`, `*`, OR'd filter values) may be added during planning if
  they fit the same benchmark-method shape without material extra harness
  work, but are not required for this spec to be considered complete.
- This spec produces a fresh, same-corpus comparison rather than merely
  loading spec `004`'s existing interim-corpus baseline (a deliberate scope
  choice beyond that spec's own SC-004, which anticipated spec `009` only
  needing to load the existing artifact, not necessarily re-run Scala) —
  because the existing baseline was never run against `fixtures/`, and
  covers no hierarchy-mode benchmarks at all, so it cannot answer this
  spec's own User Stories on its own.
- Benchmarking infrastructure (harness code, comparison scripts) is not
  itself part of the published `hl7pet-core`/`hl7pet-cli` crates' public
  surface — Constitution Principle IV (Multi-Language Interoperability) does
  not apply to it the way it would to a new user-facing capability.
