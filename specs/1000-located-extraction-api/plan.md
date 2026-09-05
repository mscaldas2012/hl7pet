# Implementation Plan: Located Extraction API

**Branch**: `1000-located-extraction-api` | **Date**: 2026-09-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1000-located-extraction-api/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Add two new `hl7pet-core` query-execution entry points — `execute_located` (the
location-aware counterpart to `execute`/`getValue`) and `first_located` (the
counterpart to `getFirstValue`) — that return each extracted value paired with the
1-based line number of the segment occurrence it came from. Both are additive
siblings of the existing `execute()` in `crates/core/src/query.rs`; neither changes
`execute()`'s behavior, signature, or output. The line number is obtained for free
from data `resolve_segment_candidates` already has in hand (each matched segment's
position in `ScanResult.segments`, which is already in document order) — no new
scan, no new dependency, and allocation count independent of message size (though
one small, fixed allocation more than `execute()` per matched occurrence, research.md
#5). Scope is limited to non-hierarchy PATHs,
matching spec `007`'s existing scope; hierarchy PATHs (`->`, spec `008`) are
explicitly out of scope per spec.md's Assumptions.

## Technical Context

**Language/Version**: Rust, stable toolchain, edition 2021 (matches `crates/core/Cargo.toml`, unchanged by this feature)

**Primary Dependencies**: None new. This feature adds zero Cargo dependencies — it is a pure addition to `crates/core/src/query.rs` using only data the scanner (spec `005`) and existing query-execution code (spec `007`) already produce, consistent with the project's dependency policy (pure-Rust, nothing new leaking through the public API, since the eventual goal is Python/Java bindings, module `6000`-`6999`)

**Storage**: N/A

**Testing**: `cargo test` — new unit tests alongside `query.rs`'s existing ones, plus a new integration test `crates/core/tests/located_vectors.rs` mirroring `query_vectors.rs`'s scan→parse→execute pattern, verified against the `expected_lines` metadata already present in `fixtures/vectors/path/valid.json` (spec `001` FR-008)

**Target Platform**: Same as `hl7pet-core` generally — no OS/platform dependency, builds anywhere stable Rust targets

**Project Type**: Library (Rust crate `hl7pet-core`) — extends `crates/core/src/query.rs`; the `hl7pet` dev CLI (`crates/cli`, not a tracked roadmap feature) gets a small `--located` flag to exercise the new entry points manually, mirroring its existing `--first`/`--profile` flags

**Performance Goals**: Per spec.md SC-004, resolving a value's line number MUST NOT add a second pass over the message or the segment list — it must be produced within the same single filtering pass `resolve_segment_candidates` already performs today

**Constraints**: Zero-copy (Constitution Principle II) — the new `LocatedValue` type borrows `&'m str` directly from `scan.message` exactly as `execute()`'s output already does; it owns nothing and copies nothing. No new panics or exception-style failures beyond `execute()`'s existing single `QueryError::NonNumericComparison` case (Constitution Principle III)

**Scale/Scope**: Non-hierarchy PATHs only — `crates/core/src/hierarchy.rs` (spec `008`) is untouched by this feature; a future spec may extend location-awareness to `->` PATHs

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Path Contract Stability** — PASS. No PATH grammar or evaluation-semantics
  change; this feature adds new query-execution entry points, it does not touch
  `crates/core/src/parser.rs` or the PATH grammar (`contracts/path-grammar.md`) at
  all.
- **II. Zero-Copy & Lazy Evaluation** — PASS. `LocatedValue<'m>` borrows from
  `scan.message` exactly like `execute()`'s existing `&'m str` output; the line
  number comes from the matched span's already-known position in
  `ScanResult.segments` (document order), captured during the one filtering pass
  `resolve_segment_candidates` already performs — no second pass over the message
  or the segment list, and allocation count does not scale with unrelated message
  size (one small, fixed allocation more than `execute()` per matched occurrence,
  traced to a standard-library same-size-element collect optimization that a
  differently-sized `LocatedValue` cannot use — research.md #5).
- **III. Explicit, Exception-Free Data Absence** — PASS. `execute_located`/
  `first_located` return `Result<_, QueryError>` reusing `execute()`'s existing
  error type unchanged (no new variant); "no match" is represented as an empty
  result (or `None` for `first_located`), never a fabricated line number, never a
  panic — mirroring FR-007's requirement exactly.
- **IV. Multi-Language Interoperability** — Tracked, not violated. This spec is
  Rust-core-only work, consistent with how specs `005`-`009` also shipped
  core-only ahead of any language binding — module `6000`-`6999` (Python/JNI) has
  not started yet, so there is no existing binding for this feature to diverge
  from. Parity work for `getValueLocated`/`getFirstValueLocated` belongs to
  whichever `6000`-range spec first wires up PyO3/JNI bindings; noting it here so
  that spec does not silently drop it.
- **V. Conformance Through Declarative Profiles & Documented Limitations** —
  PASS (not applicable). This feature is not profile-driven. Its one real
  limitation — hierarchy PATHs are out of scope — is explicitly documented in
  spec.md's Assumptions and Edge Cases, per this principle's requirement.
- **Performance & Portability Standards** — No Scala baseline exists to regress
  against: per `ROADMAP.md`, spec `1000` is "a new capability (no current Scala
  equivalent)," so there is nothing to benchmark comparatively (unlike specs
  `005`-`009`, which replaced or matched existing Scala behavior). Correctness
  against the `expected_lines` fixture metadata is this feature's primary
  validation; a counting-allocator unit test (reusing the pattern from specs `005`/
  `008`) confirms the "no extra pass" performance claim (SC-004) directly, in lieu
  of a JMH-style comparative benchmark.
- **Development Workflow — Phased Migration Discipline** — N/A. This is
  Parsing & Extraction module (`1000`-`1999`) work, not `0`-`999` Rust Core
  Migration-Plan-phase work; the phase-order rule does not gate it.

No violations requiring Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/1000-located-extraction-api/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── located-extraction-api.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
crates/core/                       # hl7pet-core (existing crate, no new crate)
├── src/
│   ├── scanner.rs                 # spec 005 — unchanged; ScanResult.segments already document-ordered
│   ├── parser.rs                  # spec 006 — unchanged
│   ├── query.rs                   # spec 007 — extended: execute_located, first_located, LocatedValue
│   ├── hierarchy.rs                # spec 008 — untouched (out of scope, see Assumptions)
│   └── lib.rs                     # add pub use query::{execute_located, first_located, LocatedValue}
└── tests/
    ├── query_vectors.rs           # spec 007 — unchanged, existing precedent this feature's test mirrors
    └── located_vectors.rs         # NEW — verifies execute_located/first_located against expected_lines

crates/cli/
└── src/main.rs                    # add --located flag alongside existing --first/--profile
```

**Structure Decision**: No new crate or module file. This feature is a small,
additive extension of the existing `hl7pet-core` query-execution module
(`crates/core/src/query.rs`, spec `007`'s home), following the same
sibling-module precedent specs `008` (`hierarchy.rs`) and `009` (`benches/`)
already established of extending `crates/core` in place rather than
introducing new crates. Its dev-CLI surface (`crates/cli`) picks up one new
flag, matching the existing `--first`/`--profile` pattern exactly.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations — this section is not applicable to this feature.
