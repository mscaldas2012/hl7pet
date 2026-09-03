# Quickstart: Lazy Hierarchy Navigation

Validates spec.md's user stories end-to-end: a single-hop `->` PATH resolves against
a scanned message and a parsed profile without a full-message tree (US1), the 9
single-hop `fixtures/vectors/hierarchy/` vectors pass with FR-007's corrected
child-index behavior (US2 — 2 of the 11 total vectors use a two-hop PATH and are out
of scope, spec.md FR-010/research.md #6), and multi-hop chaining is confirmed still
rejected at parse time, not silently attempted (US3). Includes `hier-011`, added to
prove parent-scoped isolation between two `OBR` occurrences directly.

## Prerequisites

- Rust `stable` toolchain — same as specs `005`-`007`, no pinned MSRV.
- No JVM, Scala, or Maven required to *run* this spec's own tests. The Scala source
  trace in research.md #1 was one-time verification during planning, already done.
- Specs `005` (scanner), `006` (parser), and `007` (query execution) already
  implemented — this spec is a new sibling module, not a rewrite of any of them
  (plan.md Project Structure).

## 1. Build with the new hierarchy module and promoted dependency

```bash
cargo build --workspace
```

**Expected outcome**: `crates/core` compiles cleanly with zero warnings, now
exporting `hl7pet_core::hierarchy::{HierarchyProfile, ProfileError,
execute_hierarchy}` alongside the existing `scanner`/`parser`/`query` modules.
`serde`/`serde_json` now appear under `hl7pet-core`'s `[dependencies]` in
`Cargo.toml`, not only `[dev-dependencies]` (research.md #5).

## 2. Run unit tests (bounded scan algorithm, profile parsing)

```bash
cargo test -p hl7pet-core --lib hierarchy
```

**Expected outcome**: unit tests colocated in `hierarchy.rs` pass — covering direct-
child detection, the unrecognized-segment-drop vs. subtree-exit-boundary
distinction (research.md #1), the corrected type-filtered/per-parent/1-based
`csegIdx` resolution (data-model.md's table), profile parsing (valid profile,
invalid JSON rejected, a segment type repeated at multiple tree positions
correctly accepted — research.md #2), and the "no profile supplied" empty-result
case (FR-009).

## 3. Run the conformance vector suite against `fixtures/vectors/hierarchy/` (US1, US2)

```bash
cargo test -p hl7pet-core --test hierarchy_vectors
```

**Expected outcome**: 9 of the 11 vectors across `basic.json` (5) and `complex.json`
(6) pass, including the two corrected in this spec (`hier-004`, `hier-008` —
research.md #4's new `expected` values), `hier-011` (this spec's own addition,
proving two `OBR` occurrences' children stay isolated — `messages/basic-hierarchy.hl7`
was extended with a second `OBR` and its own `OBX` children specifically to make
this checkable), and the static-mode-fallback vector (`hier-002`,
`flags.buildHierarchy: false` → `profile: None` at the call site, expecting
`Ok(vec![])`). `hier-009`/`hier-010` (a two-hop PATH) are skipped by the
harness's `is_multi_hop` filter, documented in the test file — they are out of
scope, not silently passing (research.md #6). This is the single command that
proves SC-001.

## 4. Confirm no full-message tree is ever built (SC-002)

```bash
cargo test -p hl7pet-core --lib -- bounded_scan no_full_tree
```

**Expected outcome**: a dedicated test (mirroring spec `005`'s allocation-counting
precedent) confirms that querying a large synthetic message for one parent
occurrence's children performs work bounded by that occurrence's own scoped line
range plus `O(profile size)` — not `O(message size)` — by asserting the scan visits
no line past the computed boundary.

## 5. Confirm a malformed profile never panics, and a repeated segment type is accepted (SC-004)

```bash
cargo test -p hl7pet-core --lib -- rejects_invalid_json accepts_a_segment_type
```

**Expected outcome**: invalid JSON returns `Err(ProfileError::InvalidJson)` from
`HierarchyProfile::from_json` — never a panic — and a profile with a segment type
repeated at two positions (`deep-nested.json`'s real shape) is accepted, not
rejected (research.md #2's corrected design).

## 6. Confirm multi-hop chaining is still rejected, unchanged (US3)

```bash
cargo test -p hl7pet-core --lib parser -- multiple_hierarchy_hops
```

**Expected outcome**: `hl7pet_core::parse("ORC[1] -> OBR[1] -> OBX-5")` still returns
`Err(ParseErrorKind::MultipleHierarchyHops)`, exactly as spec `006` already
guarantees — this spec adds no grammar or parser change (spec.md Clarifications,
User Story 3).

## 7. Corpus validation still passes with the corrected `hierarchy` family

```bash
python3 fixtures/scripts/validate_corpus.py
```

**Expected outcome**: spec `003`'s existing validation script accepts
`hier-004`/`hier-008`'s corrected `expected`/`expected_lines` values and the removed
`known_limitation` field without any script or schema change — the shape is
unchanged, only the content of two existing entries differs (plan.md Structure
Decision).

## 8. (Optional) Try it from the `hl7pet` dev CLI

```bash
cargo run -q -p hl7pet-cli -- fixtures/messages/basic-hierarchy.hl7 'OBR[1] -> OBX-3' \
  --profile fixtures/profiles/basic-two-level.json
```

**Expected outcome**: prints both `OBX` values scoped under the first (and only)
`OBR` occurrence — the CLI's current "hierarchy PATH is not evaluated yet" warning
is gone, replaced by a real call to `execute_hierarchy` (plan.md Project Structure;
not a roadmap requirement in its own right, per spec.md Assumptions, but the natural
follow-up this spec unblocks).
