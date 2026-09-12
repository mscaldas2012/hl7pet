# Implementation Plan: Ambiguous-Position Parent Resolution for Hierarchy PATHs

**Branch**: `010-ambiguous-parent-resolution` | **Date**: 2026-09-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/010-ambiguous-parent-resolution/spec.md`

## Summary

`hl7pet-core`'s hierarchy navigation (`crates/core/src/hierarchy.rs`, specs
`002`/`008`) currently returns an unconditional empty result for any `->` PATH whose
*parent*-side segment type is legal at more than one position in the loaded profile
(e.g. `OBX` under both `OBR` and `SPM`) — `HierarchyProfile::node_for` gives up rather
than guess. A live run against the real published Scala engine
(`gov.cdc:hl7-pet_2.13:1.2.11`) confirms this is a genuine regression, not a missing
nice-to-have: the real engine has never had this problem, because it builds its
output by a single top-down document-order walk rather than a name→position lookup.
The fix generalizes the *same* nearest-enclosing-ancestor rule `hl7pet-core` already
uses correctly for child-side matching (`direct_children_of_type`) to also resolve
which tree position an ambiguous-type *parent* occurrence actually occupies, by
replaying that rule from the top of the message specifically when (and only when) the
query's parent type is ambiguous — leaving the existing fast, purely-local path for
every unambiguous query (100% of today's conformance vectors) untouched.

## Technical Context

**Language/Version**: Rust (stable toolchain already pinned by the workspace) — no
new language or toolchain.

**Primary Dependencies**: None new. The fix lives entirely inside `crates/core`;
`serde`/`serde_json` (already a runtime dependency since spec `008`, for
`HierarchyProfile::from_json`) is unchanged and sufficient.

**Storage**: N/A.

**Testing**: `cargo test -p hl7pet-core` — new unit tests alongside
`crates/core/src/hierarchy.rs`'s existing `#[cfg(test)] mod tests`, plus new entries
in `fixtures/vectors/hierarchy/complex.json` exercised through spec `008`'s existing
`hierarchy_vectors` integration test. `cargo bench` (`crates/core/benches/`, spec
`009`'s harness) re-run to confirm the unambiguous-case benchmarks are unaffected.

**Target Platform**: Unchanged — any Rust-stable target `hl7pet-core` already
supports; no new platform constraint.

**Project Type**: Library (a correctness fix inside an existing Rust crate, not a new
project, service, or binary).

**Performance Goals**: Zero measurable regression on every existing hierarchy
benchmark (spec `009`'s `HierarchyBenchmarks`, unambiguous profiles) — SC-004. A new
ambiguous-type query pays a new, previously-nonfunctional cost (a walk over the
message's segment list, not its field content), which is the price of a capability
that returns nothing today, not a regression of anything existing.

**Constraints**: Constitution Principle II (zero-copy, lazy, index only what a query
needs) is the live design tension this fix must respect — see Constitution Check
below. Constitution Principle III (never panic; absence over error) governs the case
where an ambiguous-type occurrence resolves to no legal position at all (FR-004).

**Scale/Scope**: Single crate, single module (`crates/core/src/hierarchy.rs`) and its
existing fixtures family (`fixtures/vectors/hierarchy/`,
`fixtures/profiles/deep-nested.json`, `fixtures/messages/complex-hierarchy.hl7` — all
pre-existing, no new fixture files needed). No Python binding code change (spec
`6000`'s `get_value_hierarchy` already forwards to `execute_hierarchy` unchanged, so
it benefits automatically with no signature change). Also updates spec `008`'s own
contract file (`contracts/hierarchy-api.md`) to retire the limitation this spec
closes, so that document doesn't go stale.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|---|---|
| I. Path Contract Stability | **Pass.** No PATH grammar or `->` syntax change. Every existing PATH string continues to mean exactly what it means today; ambiguous-parent-type PATHs simply stop being unconditionally empty. |
| II. Zero-Copy & Lazy Evaluation | **Pass, with a documented tension resolved by scoping.** A full top-down walk is inherently more work than today's bounded local scan — but research.md #2/#3 establishes it is triggered *only* when the query's parent type is actually ambiguous (a cheap, already-existing check), never for the common unambiguous case, which keeps today's exact fast path and is provably equivalent to it (research.md #2's equivalence argument). The walk itself still touches only already-scanned segment metadata (names/offsets from spec `005`'s scanner), never re-parses fields, and is bounded by message segment count, not message byte size squared or a full object-model build. |
| III. Explicit, Exception-Free Data Absence | **Pass.** An ambiguous-type occurrence with no legal position given real message structure yields `Ok(vec![])` (FR-004) — absence, matching the real Scala engine's own "unrecognized anywhere, silently skip" behavior (`HL7HierarchyParser.scala:66-71`), never a panic or new error variant. |
| IV. Multi-Language Interoperability | **Pass.** Entirely inside `hl7pet-core`; the Python binding (spec `6000`) needs no change and benefits automatically, since it already calls `execute_hierarchy`/`get_value_hierarchy` unchanged — no binding falls behind another. |
| V. Declarative Profiles & Documented Limitations | **Pass, and this spec actively closes a documented limitation.** Spec `008`'s own `contracts/hierarchy-api.md` explicitly named "resolving an ambiguous parent-side type ... via history-dependent disambiguation" as deferred. This plan's implementation MUST update that file (not just add a new one) so the limitation registry stays accurate rather than describing a gap that no longer exists. |

**Result**: No violations requiring Complexity Tracking. Principle II's tension is
resolved by design (scoping the expensive path to only when needed), not by accepting
a deviation.

## Project Structure

### Documentation (this feature)

```text
specs/010-ambiguous-parent-resolution/
├── plan.md               # This file (/speckit-plan command output)
├── research.md           # Phase 0 output (/speckit-plan command)
├── data-model.md         # Phase 1 output (/speckit-plan command)
├── quickstart.md         # Phase 1 output (/speckit-plan command)
├── contracts/            # Phase 1 output (/speckit-plan command)
│   └── ambiguous-parent-resolution.md
└── tasks.md              # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

No new project or directory — this is a correctness fix confined to the existing
Rust workspace member `crates/core`, plus one fixtures update and one cross-spec
documentation update:

```text
crates/core/
├── src/
│   └── hierarchy.rs          # MODIFIED: new occurrence-resolution logic
│                              #   alongside HierarchyProfile/direct_children_of_type
│                              #   (specs 002/008's existing home for this concern)
└── benches/                  # RE-RUN (unchanged code): confirms SC-004

fixtures/
└── vectors/hierarchy/
    └── complex.json           # MODIFIED: new vectors for OBX/NTE as -> parents,
                                #   reusing existing complex-hierarchy.hl7 + deep-nested.json

specs/008-lazy-hierarchy-nav/
└── contracts/
    └── hierarchy-api.md       # MODIFIED: retires the "ambiguous parent-side type"
                                #   deferred-limitation bullet this spec closes
```

**Structure Decision**: No new source tree. This is the "Option 1: single project"
shape already in place (`crates/core` as the library, `fixtures/` as its shared
conformance data) — there is nothing to choose here beyond confirming the fix stays
inside the module that already owns this concern, per spec.md's own scoping.
