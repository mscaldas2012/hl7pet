# Quickstart: Located Extraction API

Validates spec.md's user stories end-to-end: a single-match PATH returns its value
paired with the correct line (US1), a multi-occurrence PATH returns one line per
occurrence rather than one line for the whole result (US2), and the first-value
convenience returns exactly one value/line pair (US3) — all validated against the
`expected_lines` metadata already present in the shared fixtures corpus (spec `001`
FR-008), reused rather than re-derived (spec.md FR-009).

## Prerequisites

- Rust `stable` toolchain — same as specs `005`-`009`, no pinned MSRV.
- No JVM, Scala, or Maven required — this feature has no Scala baseline to verify
  against (research.md #4); the existing `fixtures/` corpus is the only input needed.
- Specs `005` (scanner) and `007` (query executor) already implemented — this feature
  extends `query.rs` in place (plan.md Project Structure), it does not touch the
  scanner or hierarchy modules.

## 1. Build with the new located-extraction functions

```bash
cargo build --workspace
```

**Expected outcome**: `crates/core` compiles cleanly with zero warnings, now also
exporting `hl7pet_core::query::{execute_located, first_located, LocatedValue}`
alongside the existing `execute`/`QueryError`.

## 2. Run unit tests

```bash
cargo test -p hl7pet-core --lib query
```

**Expected outcome**: new unit tests colocated in `query.rs` pass, covering:
`execute_located`'s output matching `execute()`'s value content exactly for a sample
of existing cases; single- and multi-occurrence line assignment, including filtered
occurrences; `first_located`'s first-value/`None` behavior; and a counting-allocator
test confirming allocation count does not scale with unrelated message size
(research.md #5, SC-004) — `execute_located` costs one small, constant allocation
more than `execute()` per matched occurrence (research.md #5's finding), not an
identical count, but that constant never grows with message size.

## 3. Run the conformance vector suite against `expected_lines` (US1, US2)

```bash
cargo test -p hl7pet-core --test located_vectors
```

**Expected outcome**: every vector in `fixtures/vectors/path/valid.json` that carries
`expected_lines` passes with `execute_located`'s returned lines matching exactly. For
example, vector `path-obx5-occurrences` (`OBX-5` against
`fixtures/messages/multi-obx.hl7`) MUST return:

```text
[[LocatedValue { value: "Positive", line: 4 }],
 [LocatedValue { value: "Negative", line: 5 }],
 [LocatedValue { value: "Equivocal", line: 6 }]]
```

— matching that vector's `expected: [["Positive"], ["Negative"], ["Equivocal"]]` and
`expected_lines: [[4], [5], [6]]` fields exactly (contracts/located-extraction-api.md's
`execute_located`-to-`execute` equivalence).

## 4. Exercise `first_located` (US3)

```bash
cargo test -p hl7pet-core --lib -- first_located
```

**Expected outcome**: two unit tests pass —
`first_located_returns_only_the_first_group_first_value` (against a three-occurrence
`OBX-5` message, `first_located` returns exactly `execute_located`'s `[0][0]` entry —
the first group's first value only, never the full set) and
`first_located_returns_none_when_nothing_matches` (an absent segment type, an
out-of-range segment index, and a filter matching nothing all return `Ok(None)`,
never a fabricated value/line and never an error for this case).

## 5. Manual check via the dev CLI

```bash
cargo run -p hl7pet-cli -- fixtures/messages/multi-obx.hl7 "OBX-5" --located
```

**Expected outcome**: prints each matched value together with its source line, e.g.:

```text
line 4: Positive
line 5: Negative
line 6: Equivocal
```

Compare against the existing non-located form to confirm identical values:

```bash
cargo run -p hl7pet-cli -- fixtures/messages/multi-obx.hl7 "OBX-5"
```

## 6. Full regression check

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
```

**Expected outcome**: the full pre-existing suite (specs `005`-`009`) continues to
pass unmodified alongside this feature's new tests — confirming `execute()`'s
existing behavior is untouched (spec.md FR-008, SC-003) — and `clippy` is clean.
