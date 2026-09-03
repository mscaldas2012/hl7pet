# Implementation Plan: Lazy Hierarchy Navigation

**Branch**: `008-lazy-hierarchy-nav` | **Date**: 2026-09-02 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/008-lazy-hierarchy-nav/spec.md`

## Summary

Implement `hl7pet-core`'s hierarchy executor: given a scanner's `ScanResult` (spec
`005`), a parser's `CompiledPath` whose `child` is `Some(_)` (spec `006`), and a new
`HierarchyProfile` (this spec's own deliverable, parsed from a `segmentDefinition`
JSON document), resolve the `->` operator's parent-scoped child lookup without ever
materializing a full segment tree over the message (spec `002` Section B.1,
Constitution Principle II). Parent-occurrence selection reuses spec `007`'s existing
`SEG_IDX` resolution unchanged (FR-002); child-line resolution is a bounded,
per-occurrence forward scan (FR-003) driven by a small profile-derived lookup
structure, not a per-message tree. Two decisions this spec's own Clarifications
already settled (not re-litigated here): multi-hop `->` chaining is **deferred**
(spec `006`'s `ChildPath`/`CompiledPath` types are unchanged), and the real Scala
engine's documented child-index bug (spec `002` Section A.4) is **fixed**, as a
documented Breaking Change, rather than reproduced — which this plan's research
(below) found has a concrete, previously-unnoticed consequence: two of the ten
existing `fixtures/vectors/hierarchy/` vectors (`hier-004`, `hier-008`) currently
encode the *buggy* Scala output and must have their `expected` values corrected as
part of this spec's own deliverable, not left to silently regress.

## Technical Context

**Language/Version**: Rust, stable channel, edition 2021 — same as specs `005`-`007`
(`crates/core`'s existing crate); no new MSRV decision.

**Primary Dependencies**: `serde` + `serde_json` (version `1`, already pinned in
`Cargo.lock` at `1.0.151` as a `hl7pet-core` **dev**-dependency since spec `007`,
used there only to deserialize test fixtures) are promoted to a **runtime**
dependency of `hl7pet-core`, for `HierarchyProfile`'s JSON parsing (spec.md FR-014).
Both are pure-Rust, pin-compatible, no system/C-library build step — satisfying
FR-014's cross-compilation constraint for the future Python (`PyO3`) and Java
(`JNI`/`JNA`) bindings. `serde`/`serde_json` types are used only inside
`hierarchy.rs`'s private profile-parsing code (a crate-private `RawProfile`
`#[derive(Deserialize)]` struct mirroring the JSON shape) — never appear in any
`pub` signature (FR-014's decoupling requirement; see contracts/hierarchy-api.md).

**Storage**: N/A — inputs are an in-memory `ScanResult<'a>`, a `CompiledPath<'_>`, and
a `HierarchyProfile` (owned, built once from a profile JSON string, independent of
any specific message); output borrows from the message only, same as spec `007`.

**Testing**: `cargo test` — unit tests colocated in `hierarchy.rs` for the bounded
child-scan algorithm (direct-child detection, unrecognized-segment drop, subtree-exit
boundary detection, the corrected type-filtered/per-parent/1-based indexing), profile
parsing (valid profile, malformed JSON, duplicate segment-type placement), plus a new
integration test (`crates/core/tests/hierarchy_vectors.rs`) running every vector in
`fixtures/vectors/hierarchy/{basic,complex}.json` end-to-end (scan → parse →
hierarchy-execute), mirroring `query_vectors.rs`'s structure but adding `profile_ref`
resolution and the `flags.buildHierarchy` static-mode-fallback case (`hier-002`).

**Target Platform**: Any platform Rust `stable` supports — same as specs `005`-`007`;
`serde_json` is itself cross-platform pure Rust with no target-specific behavior.

**Project Type**: Library. Extends the existing `hl7pet-core` crate (`crates/core/`);
no new workspace member.

**Performance Goals**: No numeric throughput target (full comparative benchmarking
against the Scala baseline remains spec `009`'s job, same deferral specs `005`-`007`
used). This spec's own structural claim (SC-002) is Big-O, not wall-clock: a
single-hop hierarchy query's cost is bounded by the matching parent occurrence(s)'
own scoped line ranges plus `O(profile size)` (a one-time, per-profile, message-size-
independent lookup precomputation, research.md #1) — never the whole message and
never a per-call full-tree build.

**Constraints**: MUST NOT construct a full segment hierarchy tree over the whole
message at any point (FR-003, Constitution Principle II) — not even once and reused,
since B.1 rejects eager-build-and-cache as a design, not just eager-build-per-call.
MUST NOT panic for any `ScanResult`/hierarchy-`CompiledPath`/`HierarchyProfile`
combination, valid or malformed (FR-012, SC-004). MUST NOT copy message substrings
where a borrowed slice suffices (Constitution Principle II, same as spec `007`).
`HierarchyProfile`'s public surface MUST NOT expose any `serde`/`serde_json` type
(FR-014).

**Scale/Scope**: Operates on one `(ScanResult, CompiledPath, HierarchyProfile)` tuple
per call. Validated against `fixtures/vectors/hierarchy/basic.json` (5 vectors,
including this spec's own `hier-011` addition) and `complex.json` (6 vectors) — 11
total; 9 are in scope for this spec, 2 of which (`hier-004`, `hier-008`) have their
`expected`/`expected_lines`/`known_limitation` fields corrected (research.md #4) to
reflect FR-007's fix rather than the original Scala bug, and one (`hier-011`) is a
new vector added specifically to prove parent-scoped isolation between two `OBR`
occurrences, which the original corpus had no vector for. The remaining 2
(`hier-009`, `hier-010`) use a two-hop PATH testing the multi-hop capability this
spec defers — discovered during implementation to be
unparseable under this spec's unchanged grammar, excluded from the validating test
suite with the exclusion documented (research.md #6), not silently miscounted.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Applies? | Assessment |
|---|---|---|
| I. Path Contract Stability | **Yes** | This is the first spec where `->` evaluation semantics become real, observable Rust behavior. FR-010's requirement to match the (corrected, per this spec's own research finding) `fixtures/vectors/hierarchy/` vectors is the enforcement mechanism. The child-index fix (FR-007) is itself a *deliberate, documented* Path Contract change — recorded in `ROADMAP.md`'s Documented Breaking Changes table with a MAJOR version bump, exactly as this principle requires for any intentional PATH-semantics deviation. |
| II. Zero-Copy & Lazy Evaluation | **Yes** | The central design constraint (FR-003, FR-013): no full-message tree, ever — not per-call, not built-once-and-cached. Every extracted value remains a borrowed `&'m str` slice, reusing spec `007`'s `resolve_field_values`. `HierarchyProfile` itself is the one exception worth naming: it is a small, `O(profile size)` structure (independent of message size) built once per profile and reused across calls — this does not violate "lazy hierarchy construction" (`buildHierarchy`), since the profile is not a per-message tree; it is closer to `CompiledPath` (spec `006`), a reusable compiled artifact of a *description*, not of a specific message. |
| III. Explicit, Exception-Free Data Absence | **Yes** | `execute_hierarchy` reuses `QueryError` unchanged — no new query-time error variant (research.md #3): every "no children," "parent selector matched nothing," or "no profile supplied" case is `Ok(vec![])` (FR-006, FR-009), matching spec `007`'s existing philosophy exactly. The one genuine error class this spec adds, `ProfileError`, is reserved for a malformed *profile* (a structural precondition on the profile itself, not on any specific query) — never returned from `execute_hierarchy`, only from `HierarchyProfile`'s own constructor (data-model.md). |
| IV. Multi-Language Interoperability | Not yet applicable, but directly protected | No FFI boundary exists yet in `crates/core` (bindings are Migration Plan Phase 5) — same assessment specs `005`-`007` made. FR-014 exists specifically to keep this spec from *foreclosing* that future work: `HierarchyProfile`'s public shape is plain Rust, chosen so a future `PyO3`/`JNI` binding never has to reckon with `serde_json` types. |
| V. Conformance Through Declarative Profiles & Documented Limitations | **Yes** | `HierarchyProfile` *is* the declarative profile this principle calls for (no hard-coded per-message-type nesting logic). A segment type placed at more than one position in `segmentDefinition` is fully supported, not a limitation — `deep-nested.json` already does this for `OBX`/`NTE` (research.md #2, corrected during implementation). The one real limitation this spec's design has is narrower: a segment type used as a `->` expression's own *parent* that is itself ambiguously placed cannot be resolved (folds into "no qualifying children," `node_for` returning `None`) — documented in contracts/hierarchy-api.md, not silently guessed, and unreachable by any existing vector's parent side. |
| Performance & Portability Standards | **Yes** | Phase 1 deliverables and specs `005`-`007` are `Complete`/`Implemented` per `ROADMAP.md`. Full baseline-comparison benchmarking stays deferred to spec `009`, the same explicit deferral specs `005`-`007` used; this spec's own SC-002 is a structural (allocation/scan-bound) test, not a wall-clock benchmark, matching spec `005`'s precedent. |

**Result**: PASS. One noteworthy, deliberate deviation is already fully justified above
and in spec.md's own FR-007/ROADMAP.md entry (the child-index Breaking Change) —
Complexity Tracking is reserved for *unjustified* violations, so it stays empty.

## Project Structure

### Documentation (this feature)

```text
specs/008-lazy-hierarchy-nav/
├── plan.md                       # This file
├── research.md                   # Phase 0 output
├── data-model.md                 # Phase 1 output
├── quickstart.md                 # Phase 1 output
├── contracts/
│   └── hierarchy-api.md          # Phase 1 output — Rust public API contract
├── checklists/
│   └── requirements.md           # /speckit-specify output
└── tasks.md                      # /speckit-tasks output (not this command)
```

### Source Code (repository root)

```text
crates/
└── core/                         # EXISTING — hl7pet-core (specs 005-007)
    ├── Cargo.toml                 # updated: serde + serde_json move from
    │                               # [dev-dependencies] to [dependencies]
    ├── src/
    │   ├── lib.rs                 # updated: adds `pub mod hierarchy;` + re-exports
    │   ├── scanner.rs             # unchanged (spec 005)
    │   ├── parser.rs              # unchanged (spec 006) — multi-hop deferred,
    │   │                          # ChildPath stays non-recursive (Clarifications)
    │   ├── query.rs               # updated: `resolve_segment_candidates`,
    │   │                          # `resolve_field_values`, and `filter_matches`
    │   │                          # change from private to `pub(crate)` so
    │   │                          # hierarchy.rs can reuse them without
    │   │                          # duplicating navigation logic (research.md #3
    │   │                          # precedent: "one navigation path, not two") —
    │   │                          # no behavior change, visibility only
    │   └── hierarchy.rs           # NEW — this spec's deliverable (FR-001-FR-014):
    │                              # HierarchyProfile, ProfileError,
    │                              # execute_hierarchy()
    └── tests/
        ├── scanner_regression.rs # unchanged (spec 005)
        ├── scanner_vectors.rs    # unchanged (spec 005)
        ├── parser_vectors.rs     # unchanged (spec 006)
        ├── query_vectors.rs      # unchanged (spec 007)
        └── hierarchy_vectors.rs  # NEW — integration test over
                                   # fixtures/vectors/hierarchy/{basic,complex}.json

crates/
└── cli/
    └── src/main.rs                # updated (not a roadmap requirement, spec.md
                                    # Assumptions, but the natural follow-up this
                                    # spec unblocks): replace the "hierarchy PATH is
                                    # not evaluated yet (spec 008)" warning with a
                                    # real call to execute_hierarchy(), loading a
                                    # profile via a new `--profile <file>` flag

fixtures/
├── profiles/                     # EXISTING (spec 002) — no new profiles needed;
│                                  # basic-two-level.json and deep-nested.json
│                                  # already cover this spec's structural cases
└── vectors/
    └── hierarchy/                # EXISTING family (spec 002/003, no schema change)
        ├── basic.json             # `hier-004`'s `expected`/`expected_lines`/
        │                          # `known_limitation` corrected (research.md #4):
        │                          # FR-007's fix, not the original Scala bug
        └── complex.json           # `hier-008`'s `expected`/`expected_lines`/
                                    # `known_limitation` corrected the same way
```

**Structure Decision**: The hierarchy executor is added to the existing
`hl7pet-core` crate (`crates/core/`) as a new sibling module (`hierarchy.rs`) next to
`scanner.rs`/`parser.rs`/`query.rs` — not a new crate — for the same reason spec `007`
gave: no FFI boundary exists between these modules yet, and they are one cohesive
engine. `query.rs`'s three navigation helpers move from private to `pub(crate)`
rather than being copy-pasted into `hierarchy.rs`, keeping "one navigation path" for
field/component/subcomponent extraction and filter evaluation shared by both flat and
hierarchy queries. `HierarchyProfile` lives in the same module as the executor that
consumes it (not split into its own file) since the two are small and tightly
coupled — mirroring how `CompiledPath` and `parse()` share `parser.rs`. Conformance
vectors extend spec `002`'s existing `hierarchy` family in place — no new vector
family, no schema change — except for the two `expected`-value corrections research.md
#4 requires, which are a content fix to existing vectors, not new vectors.

## Complexity Tracking

*No unjustified Constitution Check violations — this section is intentionally left
without entries. The one deliberate deviation (child-index Breaking Change, FR-007)
is a spec-level decision already justified in spec.md and `ROADMAP.md`, not a
planning-phase complexity trade-off requiring separate tracking here.*
