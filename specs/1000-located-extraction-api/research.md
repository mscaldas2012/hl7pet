# Phase 0 Research: Located Extraction API

No `[NEEDS CLARIFICATION]` markers remain in `plan.md`'s Technical Context — this
feature's scope, dependencies, and constraints were already settled by
`ROADMAP.md`'s spec `1000` entry and the pre-existing `expected_lines` fixture
metadata (spec `001` FR-008). The findings below are the concrete technical
decisions made while confirming that Technical Context against the real source,
not resolutions of open unknowns.

## 1. Where the line number comes from

**Decision**: A value's 1-based line number is the 1-based position of its
segment occurrence within `ScanResult.segments`, captured during the same
filtering pass `resolve_segment_candidates` (`crates/core/src/query.rs`)
already performs — not a new scan, not a byte-offset-to-line conversion.

**Rationale**: Verified directly against `crates/core/src/scanner.rs`:
`scan()` appends one `SegmentSpan` per segment in document order, so
`scan.segments[i]`'s line is simply `i + 1`. This is also exactly the
convention the shared fixtures corpus already uses:
`fixtures/vectors/path/valid.json`'s `path-obx5-occurrences` vector records
`expected_lines: [[4], [5], [6]]` for three `OBX` occurrences that are,
respectively, the 4th, 5th, and 6th segments in that vector's source message —
segment position, not a byte offset or a count of `\n`/`\r` characters, which
this decision reproduces exactly.

**Refinement found while checking call sites** (before any code was written):
`resolve_segment_candidates` (`crates/core/src/query.rs`) is not private to
`execute()` — it is `pub(crate)` and already called directly by
`crates/core/src/hierarchy.rs`'s `execute_hierarchy` (spec `008`), plus five
of its own existing unit tests. Changing its return type in place from
`Vec<SegmentSpan>` to something carrying an index would ripple into
`hierarchy.rs`, contradicting plan.md's Project Structure ("hierarchy.rs —
untouched, out of scope"). Corrected design: introduce a new
`pub(crate) fn resolve_segment_candidates_indexed<'m>(...) ->
Result<Vec<(usize, SegmentSpan)>, QueryError>` containing the actual
`.enumerate().filter(...)` logic, and make the existing
`resolve_segment_candidates` a thin wrapper over it
(`.into_iter().map(|(_, span)| span).collect()`). This keeps the existing
function's signature, behavior, and all six existing call sites (`execute()`,
`hierarchy.rs`, five unit tests) completely unchanged — satisfying spec.md
FR-008 for this internal helper too, not just the public `execute()` — while
still computing the index within one single filtering pass, not a second one.

**Alternatives considered**:
- *Re-deriving the line number from `SegmentSpan.start` by counting segment
  separators up to that byte offset*: rejected — strictly more work than
  necessary (an extra scan over the message text) for information the scanner
  already encodes implicitly via segment order, and it risks disagreeing with
  the scanner's own segment boundaries if the counting logic ever diverges
  from how `scan()` actually splits segments.
- *Storing an explicit line number field on `SegmentSpan` itself (spec `005`)*:
  rejected — would touch a prior, already-shipped spec's data structure for a
  need entirely local to this one, and `SegmentSpan`'s position in
  `ScanResult.segments` already *is* that information; adding a redundant
  field risks the two ever disagreeing.
- *Changing `resolve_segment_candidates`'s return type in place*: rejected
  after finding its real call sites (above) — would touch `hierarchy.rs` and
  existing tests for a need local to this feature.

## 2. Shape of the "first located value" convenience

**Decision**: `first_located` is a thin wrapper that calls `execute_located`
and returns its first entry's first value (or `None`), not an independently
implemented traversal.

**Rationale**: Verified against `crates/core/src/query.rs` and
`crates/cli/src/main.rs` that no dedicated `execute_first`/`getFirstValue`
core function exists today — the CLI's existing `--first` flag already gets
"first value" behavior by slicing `execute()`'s output client-side, not via a
separate core entry point. `first_located` follows that same established
precedent, which also trivially satisfies spec.md SC-004 (no extra pass): it
is the exact same single call as `execute_located`, only truncated.

**Alternatives considered**:
- *A separate short-circuiting traversal that stops at the first match*:
  rejected as premature optimization — `execute_located`'s underlying
  `resolve_segment_candidates`/`resolve_field_values` calls are already
  bounded by the number of matching segment occurrences (typically small,
  spec `007`/`009`), and no existing precedent (`execute`/`getFirstValue`)
  short-circuits either.

## 3. Dependency and API-surface footprint

**Decision**: Zero new Cargo dependencies; two new `pub` functions
(`execute_located`, `first_located`) and one new `pub` type (`LocatedValue`)
added to `crates/core/src/query.rs` and re-exported from `lib.rs`, reusing the
existing `QueryError` type unchanged (no new variant).

**Rationale**: Confirmed via `crates/core/Cargo.toml` (only `serde`/
`serde_json`, both pre-existing for spec `008`'s `HierarchyProfile` parsing,
unrelated to this feature) that this feature needs nothing new. This keeps
the crate's public API pure-Rust and free of anything that would need to leak
through the eventual Python/Java binding boundary (module `6000`-`6999`),
per the project's dependency policy.

**Alternatives considered**:
- *A new sibling module (`located.rs`)*: rejected — the feature is small
  enough (two functions, one struct) that a new module would add navigation
  overhead without a matching increase in cohesion; `query.rs` is already the
  home of extraction-shape decisions (spec `007`), and this feature is
  exactly such a decision.

## 4. Benchmark scope

**Decision**: No dedicated JMH-style comparative benchmark (unlike specs
`004`/`009`); correctness is validated via `expected_lines`-based conformance
vectors, and the "no extra pass" performance claim (SC-004) is validated via
a counting-allocator unit test.

**Rationale**: Per `ROADMAP.md`, this is "a new capability (no current Scala
equivalent)" — there is no existing Scala `getValueLocated` to compare
against, so the Constitution's "benchmark against the existing baseline"
requirement has no baseline to target for this specific capability. The
allocation-counting pattern (already used in specs `005` and `008`) directly
proves the zero-copy, no-extra-scan claim without requiring a full JMH
harness extension for a capability the JVM baseline cannot express at all.

**Alternatives considered**:
- *Extending spec `009`'s `cargo bench` harness with a `located` benchmark
  target now*: deferred, not rejected outright — reasonable future work once
  this feature ships, but not required to validate this spec's own success
  criteria (which are about correctness and no added scanning, not absolute
  throughput).

## 5. `execute_located`'s allocation count vs. `execute`'s (found while writing T017)

**Finding**: `execute_located` performs exactly one more heap allocation than
`execute` per matched segment occurrence whose PATH has a field expression —
not the "identical count" plan.md's Technical Context and the original T017
task wording assumed. Traced with a temporary diagnostic test (isolating
`resolve_segment_candidates`/`_indexed` and `resolve_field_values`/
`_located`): `execute`'s final step, `.into_iter().map(|rep|
extract_component(...)).collect()`, maps `&str -> &str` — the standard
library's same-element-size Vec-to-Vec map/collect specialization reuses the
input `Vec`'s own buffer in place, at zero extra allocation cost.
`execute_located`'s equivalent step maps `&str -> LocatedValue` — a *larger*
element (adds a `usize`) — so that specialization cannot apply, and a new
buffer is always allocated. This is true whether that mapping happens via a
dedicated `resolve_field_values_located` (this feature's actual choice, kept
for a single direct call site) or via calling `resolve_field_values` and
mapping its output afterward — both cost the same total, confirmed
empirically (6 allocations either way, vs. `resolve_field_values`'s 5).

**Decision**: Accept the one-allocation-per-occurrence constant difference;
correct T017's own check (and this feature's SC-004 wording, already
message-size-focused) to verify what actually matters — allocation count is
independent of unrelated message/segment size, mirroring `execute`'s own
`execute_single_pass_allocation_count_independent_of_segment_count` test —
rather than bit-for-bit parity with `execute`'s incidental allocation
profile, which turns out to hinge on a standard-library optimization detail
neither function's contract ever promised.

**Rationale**: The extra allocation is a fixed, `O(1)`-per-occurrence
constant, not a repeated pass over the message or a cost that grows with
message size — the actual property Constitution Principle II and this
spec's SC-004 care about. Designing around a standard-library in-place-collect
specialization (which is not part of any stable API guarantee) to chase
exact parity would be optimizing for an implementation coincidence, not a
real architectural requirement.

**Alternatives considered**:
- *Restructure `select_by_field_index` to return an iterator/enum instead of
  an owned `Vec<&str>`, so the final collect is the only allocation
  regardless of target element size*: rejected — real HL7 fields have very
  few repetitions in practice (specs `004`/`009`'s corpus: typically 1, rarely
  more than a handful), so this micro-optimization has no measurable payoff,
  and it would touch `select_by_field_index`'s existing shape (5 of its own
  unit tests, plus `resolve_field_values`, spec `007`) for a feature-1000-local
  need.
