# Phase 0 Research: Located Hierarchy API

No `[NEEDS CLARIFICATION]` markers remain in `plan.md`'s Technical Context —
this feature's scope, dependencies, and line-number conventions were already
settled by spec `1000`'s own precedent and the `expected_lines` fixture
metadata already present (but unused) in `fixtures/vectors/hierarchy/`. The
findings below are the concrete technical decisions made while confirming
that Technical Context against the real source, not resolutions of open
unknowns.

## 1. Where the hierarchy line number comes from, and how to thread it without touching existing behavior

**Decision**: A hierarchy-matched value's 1-based line number is the 1-based
position of its *child* segment occurrence within `ScanResult.segments` —
identical convention to spec `1000`'s non-hierarchy line numbers. It is
captured by giving `direct_children_of_type` and `apply_child_index`
(`crates/core/src/hierarchy.rs`) each an `_indexed` sibling that threads
`(usize, SegmentSpan)` pairs through instead of bare `SegmentSpan`s, with the
existing functions becoming thin wrappers that discard the index
(`.into_iter().map(|(_, span)| span).collect()`), exactly mirroring spec
`1000` research.md #1's `resolve_segment_candidates` /
`resolve_segment_candidates_indexed` split.

**Rationale**: Verified directly against `direct_children_of_type`
(`crates/core/src/hierarchy.rs:275`): its scan loop already iterates
`&scan.segments[parent_line + 1..]` by reference without an index, and
pushes a matched child into `result: Vec<SegmentSpan>` with no position
retained. Switching that loop to
`scan.segments[parent_line + 1..].iter().enumerate()` and computing each
child's absolute 1-based line as `parent_line + 2 + local_i` (i.e.
`(parent_line + 1 + local_i) + 1`, matching
`resolve_segment_candidates_indexed`'s own `i + 1` convention exactly)
requires no new data — the position is available for free at the exact
point a match is already recorded. `apply_child_index` similarly already
receives spans in an order that preserves the pairing; its `_indexed` sibling
carries the `usize` through `Numeric`/`Last`/`Filter` selection unchanged
(`Filter` still calls `query::filter_matches` against the span component of
each pair only, ignoring the line, exactly as before).

**Refinement found while checking call sites** (before any code was
written): both `direct_children_of_type` and `apply_child_index` are private
(`fn`, not `pub`/`pub(crate)`) and have exactly one production call site
each — inside `execute_hierarchy` itself — plus their own dedicated unit
tests in `hierarchy.rs`'s `#[cfg(test)] mod tests`. Unlike spec `1000`'s
`resolve_segment_candidates` (which had to stay untouched because
`hierarchy.rs` already called it directly), there is no third-party caller
here to protect — but the delegation split is still the right design,
because `hierarchy.rs`'s own existing unit tests
(`direct_children_of_type_records_direct_child`, etc., lines 426-507) call
`direct_children_of_type` directly and assert on `Vec<SegmentSpan>`; forcing
those to unwrap tuples for a return type they never asked about would be
churn unrelated to what those tests actually verify. The `_indexed`
delegation keeps every existing test's assertions byte-for-byte unchanged.

**Alternatives considered**:
- *Recomputing each result span's line via a second
  `scan.segments.iter().position(...)` lookup after `direct_children_of_type`
  returns* (the same technique `resolve_occurrence_node` already uses once,
  at hierarchy.rs:220-224, to locate its own `target`): rejected as the
  primary mechanism — it would be an `O(n)` scan per result span, and
  `direct_children_of_type`'s own loop already walks past every span exactly
  once, so the position is available at zero additional cost right there.
- *Changing `direct_children_of_type`/`apply_child_index`'s return types in
  place* (since they have no third-party callers to protect): rejected —
  `execute_hierarchy` itself must keep returning `Vec<Vec<Cow<'m, str>>>`
  unchanged (FR-007), so `execute_hierarchy` would need to immediately strip
  the index back out anyway; keeping the plain functions as the public
  (well, private) face and adding `_indexed` siblings is no more code and
  matches this codebase's own established convention (spec `1000`) rather
  than inventing a new one.
- *Adding a `line: usize` field directly to `SegmentSpan`* (spec `005`):
  rejected for the same reason spec `1000` research.md #1 already rejected
  it — `SegmentSpan`'s position in `ScanResult.segments` already *is* that
  information, and a redundant field risks the two disagreeing; this
  decision was already made once and applies unchanged here.

## 2. Reusing `query::resolve_field_values_located` directly

**Decision**: `execute_hierarchy_located`'s final field-resolution step calls
`query::resolve_field_values_located` (already `pub(crate)`,
`crates/core/src/query.rs:436`) directly, passing each selected child's
computed line — no new field-resolution function.

**Rationale**: Confirmed `resolve_field_values_located` is `pub(crate)`
(visible anywhere in `hl7pet-core`, including `hierarchy.rs`) and already
takes exactly `(line: usize, field_expr, segment_content, segment_name,
delimiters)` — precisely the shape `execute_hierarchy_located` needs to call
once per selected child occurrence, identical to how
`execute_hierarchy`'s existing final loop (hierarchy.rs:405-414) already
calls the non-located `query::resolve_field_values`. No visibility change,
no new function, no duplicated escape-decoding logic (spec `1001`) — the
exact same `decode_escapes` path both hierarchy and non-hierarchy extraction
already share.

**Alternatives considered**:
- *A hierarchy-local reimplementation of field/component/subcomponent
  resolution with location tagging*: rejected outright — would duplicate
  `resolve_field_values_located`'s existing logic (including spec `1001`'s
  escape decoding) for no benefit, and risks the two ever silently
  diverging.

## 3. Grouping across multiple matching parent occurrences

**Decision**: `execute_hierarchy_located` accumulates `(usize, SegmentSpan)`
pairs across every matching parent occurrence into one flat
`Vec<(usize, SegmentSpan)>` (`selected_children`), then produces one result
group per selected child — identical grouping structure to
`execute_hierarchy`'s own `selected_children: Vec<SegmentSpan>` (line 381).

**Rationale**: Verified against `execute_hierarchy`'s existing body
(hierarchy.rs:359-417): it already flattens children across all matching
parent occurrences into a single `Vec<SegmentSpan>` before producing one
result group per span (per FR-002's requirement that value content and
grouping stay identical between the located and non-located hierarchy
entry points) — `hier-006`'s existing vector (`OBR -> OBX-3`, two matching
`OBR` parents) already proves this flattened-across-parents grouping is
`execute_hierarchy`'s real, tested behavior (`expected: [["...A"], ["...C"]]`,
one group per child regardless of which parent it came from). This feature
changes nothing about that grouping — it only carries a `usize` line
alongside each span through the exact same flattening.

**Alternatives considered**:
- *Grouping located results by parent occurrence rather than flattening*:
  rejected — would produce a different shape than `execute_hierarchy`'s
  existing output, violating FR-002's "identical, value-for-value and
  group-for-group" requirement and disagreeing with `hier-006`'s own
  documented expected shape.

## 4. Python binding surface footprint

**Decision**: One new `#[pyfunction]`, `get_value_hierarchy_located`
(`crates/python/src/lib.rs`), mirroring `get_value_hierarchy`'s existing
profile-handling (JSON re-serialization via the stdlib `json` module,
`build_hierarchy` flag) and `get_value_located`'s existing
`LocatedValue::from` wrapping — no new PyO3 type, no new Cargo dependency on
either side of the FFI boundary.

**Rationale**: Confirmed both precedents already exist verbatim in
`crates/python/src/lib.rs`: `get_value_hierarchy` (lines 71-95) already does
exactly the profile JSON re-serialization and `build_hierarchy` toggle this
feature needs unchanged, and `get_value_located` (lines 110-125) already
does exactly the `Option<Vec<Vec<LocatedValue>>>` empty-to-`None` collapse
and per-value `LocatedValue::from` wrapping this feature needs unchanged.
`get_value_hierarchy_located` combines both precedents mechanically — no new
pattern is introduced. `located_value::LocatedValue`'s existing
`From<hl7pet_core::LocatedValue<'_>>` impl (already generic over the
`LocatedValue`'s lifetime) requires no change to accept values originating
from `execute_hierarchy_located` instead of `execute_located`.

**Alternatives considered**:
- *A combined `get_value_hierarchy(..., located: bool = false)` flag instead
  of a new function*: rejected — would change `get_value_hierarchy`'s return
  type conditionally on a runtime flag (`list[list[str]]` vs.
  `list[list[LocatedValue]]`), which is both a poor Python API shape (no
  static return-type guarantee) and a direct violation of the
  Backward-Compatible-Additions convention's "new capability -> new method,
  never change an existing method's return shape" rule (`ROADMAP.md`), the
  same reasoning spec `1000` already applied when choosing
  `execute_located` as a sibling of `execute` rather than a flag on it.

## 5. Benchmark scope

**Decision**: No dedicated JMH-style comparative benchmark (unlike specs
`004`/`009`); correctness is validated via `expected_lines`-based
conformance vectors already present in `fixtures/vectors/hierarchy/`, and
the "no extra pass" performance claim (SC-004) is validated via a
counting-allocator unit test, following spec `1000` research.md #4's
identical precedent for the analogous non-hierarchy feature.

**Rationale**: Per `ROADMAP.md`, this is a new capability with no current
Scala equivalent — the real `HL7HierarchyParser` never returned location
data for hierarchy navigation, so there is no existing baseline to compare
against. The allocation-counting pattern is already established in this
exact module for an analogous "no regression for the existing path" claim
(spec `010`'s `unambiguous_parent_resolution_allocation_count_is_unaffected_by_unrelated_ambiguity`
test, hierarchy.rs:701-730) and is reused here to prove
`execute_hierarchy_located` does not add a second pass over the message
relative to `execute_hierarchy`, beyond the same fixed per-occurrence
constant spec `1000` research.md #5 already found and accepted for
`execute_located` vs. `execute` (a differently-sized `LocatedValue` element
cannot reuse the standard library's same-size in-place collect
optimization).

**Alternatives considered**:
- *Extending spec `009`'s/spec `6001`'s `cargo bench`/Python benchmark
  harnesses with a `hierarchy_located` target now*: deferred, not rejected —
  reasonable future work, not required to validate this spec's own success
  criteria (correctness and no added scanning, not absolute throughput).

## 6. SC-004's allocation-count test padding must not inflate the *parent* candidate list (found while writing T022)

**Finding**: An initial version of the SC-004 counting-allocator test padded
a "large message" with 2000 additional `OBR`/`OBX` segment pairs while
querying `"OBR[1] -> OBX-1"`, expecting identical allocation counts to a
small message. It failed (small: 12 allocations, large: 21). Root-caused
with a throwaway diagnostic comparing the *existing, unmodified*
`execute_hierarchy` (not this feature's new function) against the same two
messages: it scaled identically (14 → 23, the same +9 delta) — proving the
cause was not this feature's own code at all. The actual cause:
`query::resolve_segment_candidates` (spec `007`, unchanged, called
identically by both the old and new hierarchy entry points) collects *every*
segment named `"OBR"` into a `Vec` before applying the `SegIndex::Numeric(1)`
selector — so padding with 2000 additional `OBR` occurrences inflates that
intermediate `Vec`'s reallocation count, entirely independent of anything
`direct_children_of_type_indexed`/`apply_child_index_indexed` do. Spec
`1000`'s own analogous test (`execute_single_pass_allocation_count_independent_of_segment_count`,
`query.rs`) avoids this exact trap by padding with a *different* segment
type (`OBR`) than the one being queried (`OBX-5`) — a distinction this
feature's first test draft missed by padding with the same types being
queried on both sides of the `->`.

**Decision**: Corrected the test to pad *past `OBR[1]`'s own sibling
boundary* instead — a second `OBR` immediately after `OBR[1]`'s one real
`OBX` child, followed by a large tail of unrelated `OBX` lines that belong
to that *second* `OBR`, not the first. This mirrors spec `008`'s own
`direct_children_of_type_ignores_lines_past_the_boundary_regardless_of_tail_size`
test shape exactly (already proven, at the time-complexity level, to trigger
`direct_children_of_type`'s early-exit at the sibling boundary). Under this
padding, `resolve_segment_candidates`'s own `OBR`-matching list stays at
exactly 2 regardless of tail size, isolating and correctly proving *this
feature's* bounded-scan claim (SC-004) without also asserting something
about `resolve_segment_candidates`'s pre-existing behavior that this spec
never set out to change or verify.

**Rationale**: The test's purpose (per SC-004) is to prove
`execute_hierarchy_located` adds no *additional* scaling cost beyond what
`execute_hierarchy` already has — not to prove `execute_hierarchy`'s own
parent-selection step is unconditionally size-independent, which it
demonstrably isn't for a message containing many occurrences of the parent's
own segment type, and never claimed to be by any prior spec (`008`'s
Constitution-Principle-II claims are about the *child*-side bounded scan
specifically, per its own doc comments).

**Alternatives considered**:
- *Fixing `resolve_segment_candidates` to avoid materializing every matching
  candidate before applying a numeric index*: rejected as out of scope — it
  is spec `007`'s pre-existing, unmodified code, shared by `query::execute`
  and both hierarchy entry points alike; changing it is a distinct
  performance investigation unrelated to adding location tracking, and
  would require its own spec and benchmarking per the Constitution's
  Performance & Portability Standards.
