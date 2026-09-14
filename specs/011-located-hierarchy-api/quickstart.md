# Quickstart: Located Hierarchy API

Validates spec.md's user stories end-to-end: a single-match `->` PATH returns
its value paired with the correct line (US1), a `->` PATH matching children
across multiple parent occurrences returns one line per child rather than one
line for the whole result (US2), and every existing hierarchy "no match" edge
case still returns an empty result with no fabricated line (US3) — all
validated against the `expected_lines` metadata already present in
`fixtures/vectors/hierarchy/complex.json`/`basic.json` (specs `002`/`008`/
`010`), reused rather than re-derived (spec.md FR-010).

## Prerequisites

- Rust `stable` toolchain — same as specs `005`-`010`, no pinned MSRV.
- No JVM, Scala, or Maven required — this feature has no Scala baseline to
  verify against (research.md #5); the existing `fixtures/` corpus is the
  only input needed.
- Specs `008` (lazy hierarchy nav) and `1000` (located extraction) already
  implemented — this feature extends `hierarchy.rs` in place (plan.md
  Project Structure), reusing `query.rs`'s existing `LocatedValue`/
  `resolve_field_values_located` unchanged.
- For the Python-side steps: a maturin-built `hl7pet` wheel, per spec
  `6000`'s existing quickstart (`maturin develop --release` from
  `crates/python/`).

## 1. Build with the new located-hierarchy function

```bash
cargo build --workspace
```

**Expected outcome**: `crates/core` compiles cleanly with zero warnings, now
also exporting `hl7pet_core::execute_hierarchy_located` alongside the
existing `execute_hierarchy`/`HierarchyProfile`/`ProfileError`.

## 2. Run unit tests

```bash
cargo test -p hl7pet-core --lib hierarchy
```

**Expected outcome**: all of spec `008`/`010`'s existing `hierarchy.rs` unit
tests still pass unmodified (confirming `direct_children_of_type`/
`apply_child_index`'s `_indexed`-delegation refactor changed nothing about
their existing behavior, research.md #1), alongside new tests covering:
`execute_hierarchy_located`'s output matching `execute_hierarchy`'s value
content and grouping exactly for a sample of existing cases; correct line
assignment for a single matched child, for children across multiple matching
parents, and for a child-side `SEG_IDX`; every existing "no match" case
(zero children, no profile, ambiguous-parent misplacement) still returning
an empty result; and a counting-allocator test confirming allocation count
does not scale with unrelated message size (research.md #5, SC-004).

## 3. Run the conformance vector suite against `expected_lines` (US1, US2, US3)

```bash
cargo test -p hl7pet-core --test located_hierarchy_vectors
```

**Expected outcome**: every vector in `fixtures/vectors/hierarchy/` that
carries `expected_lines` passes with `execute_hierarchy_located`'s returned
lines matching exactly. For example, vector `hier-005`
(`OBR[1] -> OBX-3` against `fixtures/messages/complex-hierarchy.hl7`) MUST
return:

```text
[[LocatedValue { value: "OBX-A-CODE^Direct Child A^LN", line: 7 }],
 [LocatedValue { value: "OBX-C-CODE^Direct Child C^LN", line: 8 }]]
```

— matching that vector's `expected: [["OBX-A-CODE^Direct Child A^LN"],
["OBX-C-CODE^Direct Child C^LN"]]` and `expected_lines: [[7], [8]]` fields
exactly (contracts/located-hierarchy-api.md's `execute_hierarchy_located`-to-
`execute_hierarchy` equivalence). Vector `hier-006` (`OBR -> OBX-3`,
children across two matching `OBR` occurrences) MUST return the same two
`LocatedValue`s, confirming grouping stays flattened-across-parents rather
than nested by parent (research.md #3). Vector `hier-007` (`OBR[2] ->
OBX-3`, zero children) MUST return an empty result — no fabricated
`LocatedValue`.

## 4. Exercise the Python binding (Multi-Language Interoperability)

```bash
cd crates/python && maturin develop --release
python3 -c "
import hl7pet, json

message = open('../../fixtures/messages/complex-hierarchy.hl7').read()
profile = json.load(open('../../fixtures/profiles/deep-nested.json'))

located = hl7pet.get_value_hierarchy_located(message, 'OBR[1] -> OBX-3', profile)
for group in located:
    for lv in group:
        print(lv.value, lv.line)
"
```

**Expected outcome**: prints each matched value together with its source
line, e.g.:

```text
OBX-A-CODE^Direct Child A^LN 7
OBX-C-CODE^Direct Child C^LN 8
```

Compare against the existing non-located form to confirm identical values:

```bash
python3 -c "
import hl7pet, json
message = open('../../fixtures/messages/complex-hierarchy.hl7').read()
profile = json.load(open('../../fixtures/profiles/deep-nested.json'))
print(hl7pet.get_value_hierarchy(message, 'OBR[1] -> OBX-3', profile))
"
```

## 5. Full regression check

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cd crates/python && pytest tests/
```

**Expected outcome**: the full pre-existing suite (specs `005`-`010`,
`1000`-`1001`, `6000`-`6001`) continues to pass unmodified alongside this
feature's new tests — confirming `execute_hierarchy`'s and
`get_value_hierarchy`'s existing behavior is untouched (spec.md FR-007,
SC-003) — and `clippy`/`pytest` are both clean.
