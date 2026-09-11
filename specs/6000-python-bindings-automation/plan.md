# Implementation Plan: Python Bindings & Core-Sync Tooling

**Branch**: `6000-python-bindings-automation` | **Date**: 2026-09-08 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/6000-python-bindings-automation/spec.md`

## Summary

Wrap `hl7pet-core`'s full public surface (scanning `005`, PATH parsing `006`,
query execution `007`, lazy hierarchy navigation `008`, located extraction
`1000`, escape-sequence decoding `1001`) in a PyO3 extension module, packaged
as a pip-installable wheel via maturin, exposing a Scala-API-shaped surface
(`get_value`/`get_first_value`/`get_values` batched) that returns `None` for
absence and raises typed exceptions only for structural failures. Alongside
the binding, ship two maintainer tools as a new pure-Rust `xtask` binary: a
`surface-diff` command that snapshots `hl7pet-core`'s public API via `syn`
and classifies changes since a committed baseline (Backward-Compatible
Addition vs. Documented Breaking Change), and a `sync-baseline` command that
records the current commit as the new baseline once a sync is verified. A
Python `parity_check.py` script (also wired into `pytest`) verifies the
built wheel's output against every vector in the shared fixtures corpus,
runnable standalone at any time, not just at initial-port time.

## Technical Context

**Language/Version**: Rust 1.75+ (workspace-pinned; matches existing
`crates/core`/`crates/cli`) for the binding crate and `xtask`; Python 3.9+
for the binding's target runtime and the maintainer-facing scripts (`pytest`
tests, `parity_check.py`).

**Primary Dependencies**: `pyo3` (`abi3-py39` feature, so one wheel per
platform covers Python 3.9+) for the extension module; `maturin` as the
PyO3/pip build backend; `syn` + `proc-macro2` (dev-only, `xtask` crate) for
parsing `crates/core/src/**/*.rs` into a public-surface snapshot — no new
runtime dependency of `hl7pet-core` itself, preserving its pure-Rust,
nothing-leaks-through-the-public-API dependency policy. No new Python
runtime dependency for the shipped package (stdlib only); `pytest` is a
dev-only dependency for the binding's own test suite.

**Storage**: N/A for the binding itself. The Sync Baseline (data-model.md)
is a single committed JSON file, `crates/xtask/surface-baseline.json`
(`{"commit": "<git sha>", "surface": {...}}`), read and rewritten in place by
`xtask sync-baseline` — not a database, just version-controlled state.

**Testing**: `cargo test` (unchanged) for `hl7pet-core`/`crates/cli`; new
`cargo test -p xtask` unit tests for the surface parser/differ against small
fixed Rust source snippets; `pytest` (via `maturin develop` + `pytest
crates/python/tests/`) for the Python binding's own API tests and for
`parity_check.py`'s fixtures-corpus run (User Story 1's Independent Test and
FR-006/FR-009).

**Target Platform**: Wheels built locally via `maturin build`/`develop` for
the maintainer's own platform (macOS arm64/x86_64, Linux x86_64/aarch64) —
this spec scopes the binding and its build config, not a CI release matrix.
Multi-platform wheel publishing (`cibuildwheel` or a GitHub Actions matrix)
is explicitly deferred; noted as a follow-up under Assumptions, not a gate
this spec must clear, since Success Criteria (SC-001 through SC-005) are all
correctness/parity outcomes, not distribution outcomes.

**Project Type**: Library extension (Rust core + language binding), within
the existing Cargo workspace.

**Performance Goals**: None newly introduced by this spec. The binding calls
straight into `hl7pet-core`'s already-benchmarked (`009`) `scan`/`execute`/
`execute_located`/`execute_hierarchy` with a single conversion at the outer
FFI boundary; per the migration plan's own phase order (Phase 5 — Language
Bindings — precedes Phase 6 — Performance), Python-side throughput
benchmarking is out of scope here and belongs to a future spec.

**Constraints**: Every extraction method's Python return shape MUST be
constructible from `hl7pet-core`'s existing borrowed/`Cow` output with
exactly one copy into a Python-owned object per returned value (never a
second intermediate Rust-side copy) — the FFI-boundary exception Principle
II already anticipates. Absence MUST surface as `None`; only `ScanError`/
`ParseError`/`QueryError`/profile errors (structural preconditions) raise,
each as a distinct Python exception type (FR-005).

**Scale/Scope**: Matches the existing shared fixtures corpus size (dozens of
messages, ~50-80 vectors across `path`/`hierarchy`/`scanner`/`escapes`) —
no new corpus, no distributed system, no new persistent service.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Path Contract Stability** — PASS. The binding exposes PATH evaluation
  unchanged; it adds no new grammar and changes no evaluation semantics. Any
  future PATH-grammar change is `hl7pet-core`'s concern (specs `001`/`006`
  territory), not this binding's.
- **II. Zero-Copy & Lazy Evaluation** — PASS, with the FFI-boundary exception
  the principle already carves out ("prefer borrowed/sliced views ... wherever
  the host language and FFI boundary allow it"). Rust-side calls into
  `hl7pet-core` stay fully zero-copy/lazy (no eager hierarchy build unless
  requested, same `scan`→`parse`→`execute*` pipeline as the CLI); exactly one
  conversion into a Python-owned `str`/`list`/`None` happens per returned
  value, at the outermost boundary, not before.
- **III. Explicit, Exception-Free Data Absence** — PASS by design (FR-005):
  every extraction entry point returns `None` for "no match," and raises only
  for the four structural-precondition cases the core itself already treats
  as errors (`ScanError`, `ParseError`, `QueryError::NonNumericComparison`,
  `ProfileError`) — each mapped to its own typed Python exception so
  "no data" and "malformed input" stay distinguishable without a blanket
  try/except.
- **IV. Multi-Language Interoperability** — PASS. This spec is exactly the
  tracked step that brings Python to parity with the JVM engine for the core
  surface through spec `1001`; the Java binding is explicitly out of scope
  here but already tracked as a separate future spec in the same Roadmap
  module (`ROADMAP.md` 6000-6999), satisfying the principle's "tracked plan
  to bring the others to parity" requirement rather than violating it.
- **V. Conformance Through Declarative Profiles & Documented Limitations** —
  PASS / not materially engaged. No new segment/field/validation behavior is
  introduced; hierarchy profiles remain the same `segmentDefinition` JSON
  consumed unchanged by `hl7pet_core::hierarchy::HierarchyProfile`. FR-013's
  documentation obligation is satisfied by `quickstart.md` plus the package's
  own README/API reference carrying a runnable example per capability.
- **Performance & Portability Standards** — PASS / not applicable to this
  spec's scope: no parser/path-evaluator/hierarchy-builder change, so no new
  benchmark is required before merge; existing spec `009` numbers are
  unaffected since `hl7pet-core` itself is untouched.

No violations to record in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/6000-python-bindings-automation/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   ├── python-api.md
│   ├── surface-snapshot.schema.json
│   └── parity-report.schema.json
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
Cargo.toml                        # workspace members gains "crates/python", "crates/xtask"

crates/
├── core/                         # UNCHANGED — hl7pet-core, wrapped not modified
├── cli/                          # UNCHANGED — existing dev CLI
├── python/                       # NEW — PyO3 extension crate (maturin mixed layout)
│   ├── Cargo.toml                # cdylib, depends on hl7pet-core (path dep) + pyo3
│   ├── pyproject.toml            # maturin build-backend config, abi3-py39
│   ├── src/
│   │   └── lib.rs                # #[pymodule]: get_value/get_first_value/get_values,
│   │                              # hierarchy + located variants, exception types
│   ├── python/
│   │   └── hl7pet/
│   │       ├── __init__.py       # re-exports the compiled extension's public names
│   │       └── _hl7pet.pyi       # type stubs (FR-013 doc-adjacent contract)
│   └── tests/
│       ├── test_api.py           # pytest: one-call and batched API behavior
│       └── parity_check.py       # FR-009: runnable standalone AND via pytest;
│                                  # walks fixtures/vectors/{path,hierarchy,scanner,escapes}/
└── xtask/                        # NEW — pure-Rust maintainer tooling, dev-only
    ├── Cargo.toml                # depends on syn, proc-macro2, serde_json (dev/tool-only,
    │                              # never a hl7pet-core dependency)
    ├── surface-baseline.json     # Sync Baseline (data-model.md), committed, updated by
    │                              # `cargo run -p xtask -- sync-baseline`
    └── src/
        ├── main.rs               # subcommands: surface-diff, sync-baseline
        ├── surface.rs            # syn-based public-surface extraction from crates/core/src
        └── classify.rs           # Backward-Compatible Addition vs. Documented Breaking
                                   # Change classification (FR-008), matching ROADMAP.md's
                                   # existing convention
```

**Structure Decision**: Two new Cargo workspace members alongside the
existing `crates/core`/`crates/cli`. `crates/python` follows maturin's
standard "mixed Rust/Python" project layout (`pyproject.toml` + `src/` +
`python/<package>/` all under one directory) so `maturin develop`/`build`
work with zero extra configuration, and depends on `hl7pet-core` as a plain
path dependency — no FFI-specific fork of the core logic. `crates/xtask`
is the conventional Rust "cargo xtask" pattern for repo-maintenance tooling
that needs real Rust-source parsing (`syn`); keeping it a separate workspace
member (rather than a script under `fixtures/scripts/`, which is Python and
fixtures-only) keeps `hl7pet-core`'s own `Cargo.toml` free of any
`syn`/tooling dependency, consistent with its pure-Rust, nothing-leaks
policy — `xtask` depends on `hl7pet-core`'s *source tree* only via file
parsing, not via a compiled dependency edge.

## Complexity Tracking

*No Constitution Check violations — table intentionally empty.*
