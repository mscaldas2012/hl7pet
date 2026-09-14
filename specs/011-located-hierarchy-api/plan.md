# Implementation Plan: Located Hierarchy API

**Branch**: `011-located-hierarchy-api` | **Date**: 2026-09-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/011-located-hierarchy-api/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Add a new `hl7pet-core` hierarchy entry point — `execute_hierarchy_located`
(the location-aware counterpart to `execute_hierarchy`) — that returns every
`->`-matched value paired with the 1-based line number of the child segment
occurrence it came from, using spec `1000`'s existing `LocatedValue` type.
Additive alongside the unchanged `execute_hierarchy` in
`crates/core/src/hierarchy.rs`. Following spec `1000`'s own established
pattern (`resolve_segment_candidates` delegating to
`resolve_segment_candidates_indexed`), `direct_children_of_type` and
`apply_child_index` are each refactored into a thin wrapper around a new
`_indexed` sibling that threads each child span's absolute position through
unchanged — so the existing functions, their existing unit tests, and
`execute_hierarchy` itself see zero behavior change. The 1-based line number
is derived the same way `resolve_segment_candidates_indexed` already does
(position among all of `scan.segments`), reusing `query::resolve_field_values_located`
for the final field-resolution step. Surfaced to Python as a new
`get_value_hierarchy_located` (`crates/python/src/lib.rs`), wrapping results
in the existing `LocatedValue` PyO3 class (spec `6000`) alongside the
unchanged `get_value_hierarchy`. Closes the gap spec `9000` (playground
webapp) recorded as a deferred scope decision (FR-005a). Conformance vectors
reuse `expected_lines` metadata already present but previously unused in
`fixtures/vectors/hierarchy/complex.json`/`basic.json` (specs `002`/`008`/`010`).

## Technical Context

**Language/Version**: Rust, stable toolchain, edition 2021 (matches `crates/core/Cargo.toml`, unchanged); Python binding via PyO3 `abi3-py39` (matches `crates/python/Cargo.toml`, unchanged, spec `6000`)

**Primary Dependencies**: None new on either side. Rust: pure addition to `crates/core/src/hierarchy.rs`, reusing `query::LocatedValue`/`query::resolve_field_values_located` (already `pub(crate)`, no visibility change needed) exactly as spec `1000` established. Python: pure addition to `crates/python/src/lib.rs`, reusing the existing `located_value::LocatedValue` PyO3 class (spec `6000`) — no new PyO3 type

**Storage**: N/A

**Testing**: `cargo test` — new unit tests alongside `hierarchy.rs`'s existing ones (mirroring spec `010`'s own precedent of testing new hierarchy behavior in-module), plus a new integration test `crates/core/tests/located_hierarchy_vectors.rs` mirroring `hierarchy_vectors.rs`'s scan→parse→profile→execute pattern, verified against the `expected_lines` metadata already present on `fixtures/vectors/hierarchy/complex.json`'s vectors (`hier-005`, `hier-006`, `hier-008`). Python: `pytest crates/python/tests/` gains coverage for `get_value_hierarchy_located` mirroring the existing `get_value_located` tests in `test_api.py`

**Target Platform**: Same as `hl7pet-core`/`hl7pet-python` generally — no OS/platform dependency

**Project Type**: Library (Rust crate `hl7pet-core`, extended; PyO3 extension crate `hl7pet-python`, extended) — no new crate. The `hl7pet` dev CLI (`crates/cli`) is not required to gain a new flag by this spec (no existing `--hierarchy`+`--located` combination flag today, and spec.md does not call for one); left out of scope unless review determines otherwise during implementation

**Performance Goals**: Per spec.md SC-004, resolving a hierarchy-matched value's line number MUST NOT add a second pass over the message or the segment list beyond what `execute_hierarchy` already performs — the line number must come from the same bounded per-parent-occurrence forward scan `direct_children_of_type` already runs today, not a new lookup

**Constraints**: Zero-copy (Constitution Principle II) — `LocatedValue.value` borrows `&'m str`/`Cow<'m, str>` from `scan.message` exactly as `execute_hierarchy`'s own output already does; no new panics or exception-style failures beyond `execute_hierarchy`'s existing single `QueryError::NonNumericComparison` case and `ProfileError` (Constitution Principle III) — both entirely unchanged by this feature

**Scale/Scope**: Hierarchy-mode (`->`) PATHs only, single-hop (multi-hop chaining remains out of scope, spec `008`'s unchanged Non-Goal). This spec does not touch `query.rs`'s non-hierarchy `execute`/`execute_located`/`first_located` at all

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Path Contract Stability** — PASS. No PATH grammar or evaluation-semantics
  change; this feature adds a new hierarchy entry point, it does not touch
  `crates/core/src/parser.rs` or the PATH grammar (`contracts/path-grammar.md`)
  at all. The `->` operator's existing semantics (spec `002`/`008`/`010`) are
  read, never redefined.
- **II. Zero-Copy & Lazy Evaluation** — PASS. `execute_hierarchy_located`
  performs the identical bounded, per-parent-occurrence forward scan
  `execute_hierarchy` already performs (research.md #1) — no full-message
  tree, no eager materialization beyond what spec `008` already established.
  The line number is captured from data the scan already produces (each
  child span's position within `scan.segments`, computed while walking
  `direct_children_of_type`'s existing loop), not a second pass.
- **III. Explicit, Exception-Free Data Absence** — PASS.
  `execute_hierarchy_located` returns `Result<_, QueryError>` reusing
  `execute_hierarchy`'s existing error type unchanged (no new variant); "no
  match" is represented as an empty result, never a fabricated line number,
  never a panic — mirroring FR-006 exactly, and identical to
  `execute_hierarchy`'s own existing behavior for the same inputs.
- **IV. Multi-Language Interoperability** — PASS, addressed directly by this
  spec (unlike spec `1000`, which deferred Python parity since no binding
  existed yet at the time). `get_value_hierarchy_located` ships in the same
  spec as the core capability, not as a tracked follow-up, closing the exact
  parity gap spec `9000` recorded (FR-008/FR-009).
- **V. Conformance Through Declarative Profiles & Documented Limitations** —
  PASS. Hierarchy navigation remains driven by the same declarative
  `segmentDefinition` profile (spec `008`); this feature adds no new
  profile-driven behavior of its own. The one relevant limitation —
  multi-hop `->` chaining stays unsupported — is unchanged from spec `008`'s
  own documented Non-Goal, not newly introduced here.
- **Performance & Portability Standards** — No Scala baseline exists to
  regress against: like spec `1000`, this is a new capability with no
  current Scala equivalent (the real engine's `HL7HierarchyParser` never
  returned location data). Correctness against the `expected_lines` fixture
  metadata already present in `fixtures/vectors/hierarchy/` is this
  feature's primary validation; a counting-allocator unit test (reusing the
  pattern from specs `1000`/`010`) confirms the "no extra pass" performance
  claim (SC-004) directly.
- **Development Workflow — Phased Migration Discipline** — N/A for the
  phase-order rule (this extends Phase 3 hierarchy work, already complete
  through spec `010`; it is not blocked on any earlier phase). The
  Python-binding portion is not itself a `6000`-range spec, but is delivered
  as this spec's secondary module per `ROADMAP.md`'s cross-module
  convention (primary module owns the spec) rather than being deferred.

No violations requiring Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/011-located-hierarchy-api/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── located-hierarchy-api.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
crates/core/                                # hl7pet-core (existing crate, no new crate)
├── src/
│   ├── hierarchy.rs                        # spec 008/010 — extended: execute_hierarchy_located,
│   │                                        #   direct_children_of_type_indexed, apply_child_index_indexed;
│   │                                        #   execute_hierarchy/direct_children_of_type/apply_child_index
│   │                                        #   delegate, unchanged behavior
│   ├── query.rs                            # spec 1000/1001 — untouched (resolve_field_values_located,
│   │                                        #   LocatedValue reused as-is, already pub(crate)/pub)
│   └── lib.rs                              # add execute_hierarchy_located to the existing
│                                            #   `pub use hierarchy::{...}` re-export list
└── tests/
    ├── hierarchy_vectors.rs                # spec 008/010 — unchanged, existing precedent this
    │                                        #   feature's test mirrors
    └── located_hierarchy_vectors.rs        # NEW — verifies execute_hierarchy_located against
                                             #   expected_lines in fixtures/vectors/hierarchy/

crates/python/
├── src/
│   └── lib.rs                              # add get_value_hierarchy_located, mirroring
│                                            #   get_value_hierarchy's profile-handling and
│                                            #   get_value_located's LocatedValue wrapping
├── python/hl7pet/
│   ├── __init__.py                         # add get_value_hierarchy_located import + __all__ entry
│   └── _hl7pet.pyi                         # add get_value_hierarchy_located stub signature
└── tests/
    └── test_api.py                         # extend with get_value_hierarchy_located coverage
```

**Structure Decision**: No new crate or module file on either side. This
feature is a small, additive extension of the existing `hl7pet-core`
hierarchy module (`crates/core/src/hierarchy.rs`, spec `008`'s home) and the
existing `hl7pet-python` PyO3 module (`crates/python/src/lib.rs`, spec
`6000`'s home), following the exact `_indexed`-delegation precedent spec
`1000` already established in `query.rs` (research.md #1) rather than
introducing a parallel module or duplicating the bounded-scan logic.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations — this section is not applicable to this feature.
