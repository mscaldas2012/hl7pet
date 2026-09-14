# Implementation Plan: Arrow Integration for PySpark & PyArrow

**Branch**: `6002-arrow-integration` | **Date**: 2026-09-14 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/6002-arrow-integration/spec.md`

## Summary

Add a new Rust workspace crate, `hl7pet-arrow` (`crates/arrow`), that builds
Apache Arrow arrays directly from `hl7pet-core` — zero-copy across the PyO3
boundary via the Arrow C Data Interface — and exposes two extraction
mechanisms to Python: single-PATH extraction over a column of messages
(`extract_value`) and multi-PATH extraction computed from a single pass per
message (`extract_values`). A thin pure-Python layer wraps both as PySpark
`arrow_udf`-based column functions, and a demo Jupyter notebook exercises
all of it against both a standalone PyArrow Table and a local PySpark
session. JSON-template/profile-based whole-message transform stays out of
scope (spec FR-013).

## Technical Context

**Language/Version**: Rust 1.98 (stable; matches the installed toolchain and
every existing workspace crate's `edition = "2021"`) for `hl7pet-arrow`.
Python >=3.9 for the binding surface (matches `crates/python/pyproject.toml`'s
existing `requires-python` and `abi3-py39` target, which this feature
mirrors).

**Primary Dependencies**:
- Rust (`crates/arrow/Cargo.toml`): `hl7pet-core` (path dep, unchanged),
  `arrow` `^59` (arrow-rs; exposes the C Data Interface via its `ffi` module
  — `FFI_ArrowArray`/`FFI_ArrowSchema`, `to_ffi`/`from_ffi` — for zero-copy
  handoff), `pyo3` `0.29` with `abi3-py39` (matches `crates/python`'s existing
  pin exactly), `pyo3-arrow` `^0.19` (candidate for the PyO3<->Arrow
  boundary; see research.md #2 for the validation spike this pin depends on).
- Python (consumer/demo side only — `hl7pet-core` itself gains no new
  dependency, preserving the Dependency policy): `pyarrow>=18` (PySpark
  4.2's own stated minimum), `pyspark==4.2.0`, `maturin>=1.7,<2.0` (already
  used by `crates/python`), `jupyter`/`ipykernel` for the demo notebook.

**Storage**: N/A — no persistence; this feature operates entirely on
in-memory Arrow data passed through a function call.

**Testing**: `cargo test -p hl7pet-arrow` for Rust-level unit tests; a new
fixtures-parity test (mirrors `crates/python/tests/parity_check.py`) proving
`hl7pet-arrow`'s columnar output matches the existing plain Python binding's
row-by-row output across the shared `fixtures/` corpus (spec SC-003);
`pytest` for the Python-facing surface (`crates/arrow/tests/`); notebook
execution via `jupyter nbconvert --execute` as the FR-011/SC-005 acceptance
check, run in CI the same way `playground/tests/` already runs against a
built wheel.

**Target Platform**: Same as the existing Python binding — Linux/macOS/
Windows via `maturin`-built `abi3` wheels. No new target platform.

**Project Type**: Library — a new Cargo workspace member plus a
`maturin`-packaged Python extension, following the exact precedent
`crates/python` (spec `6000`) already established.

**Performance Goals**: SC-002 (multi-PATH extraction costs no more than a
small constant factor over single-PATH extraction on the same messages,
i.e. genuinely one scan per message regardless of PATH count) is this
spec's only hard performance claim, verified by a counting test analogous
to spec `009`/`1000`'s allocation-count tests, adapted to "PATH count doesn't
change scan count." This is a new capability with no prior Scala or Python
baseline to regress against (unlike spec `009`), so any wall-clock
comparison in the notebook (Story 4) is diagnostic only, following spec
`6001`'s precedent rather than spec `009`'s Constitution-mandated
non-regression bar.

**Constraints**: `hl7pet-arrow` MUST depend on `hl7pet-core` only (never the
reverse, and `hl7pet-core` itself gains zero new dependencies) per the
Dependency policy. Both extraction mechanisms MUST reproduce the existing
plain Python binding's values exactly (spec FR-004) — this crate changes
*how* extraction is invoked, not the PATH/hierarchy evaluation semantics
themselves, which stay owned by `hl7pet-core`.

**Scale/Scope**: Covers spec Stories 1-4 (single-PATH, multi-PATH, PySpark
wiring, demo notebook). Excludes the Java binding (no Arrow-for-Java ask
exists yet; tracked the same way spec `6000` tracked it — as a future spec
in the same Roadmap module if/when needed) and the JSON-template/profile
transform (spec FR-013, deferred).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Path Contract Stability** — PASS. This feature adds no PATH grammar
  and changes no evaluation semantics; it only adds a new columnar
  invocation surface over the same `hl7pet-core` query execution the plain
  Python binding already uses. FR-004 makes exact-parity with the existing
  binding a hard requirement, not just an intention.
- **II. Zero-Copy & Lazy Evaluation** — PASS, and directly engaged: the
  entire point of the native-crate architecture (vs. a Python-side
  list-packing wrapper, rejected during spec discussion) is to avoid an
  intermediate copy at the PyO3/Arrow boundary via the C Data Interface
  (research.md #1-#2). Hierarchy building stays opt-in per call exactly as
  in the existing binding (`build_hierarchy`/profile-required), not built
  eagerly for a whole column.
- **III. Explicit, Exception-Free Data Absence** — PASS, and this is the
  spec's one real new design problem: FR-010 requires a per-row
  "scan failed" signal that can't be a raised exception (a batched call
  can't raise per-row without aborting the whole call). Resolved in
  data-model.md via a `{value, status}` result-struct shape, extending the
  existing "no data vs. structural error" distinction into the columnar
  case rather than inventing a new one.
- **IV. Multi-Language Interoperability** — PASS. This spec targets Python/
  PySpark/PyArrow only, the exact language surface named by the migration
  plan's Phase 4. Arrow support for the Java binding is not a currently
  tracked need; if one emerges it becomes its own future spec in the same
  Roadmap module, matching spec `6000`'s own precedent for scoping the Java
  binding out without violating this principle.
- **V. Conformance Through Declarative Profiles & Documented Limitations** —
  PASS. Hierarchy profiles are the same unchanged `segmentDefinition` JSON
  the existing binding already consumes (Assumptions, spec.md). The one
  open technical uncertainty this plan carries (whether `pyspark.sql.
  functions.arrow_udf`'s struct-output path is fully supported in PySpark
  4.2 today) is treated as a documented research/validation item
  (research.md #4), not silently assumed.
- **Performance & Portability Standards** — PASS, directly engaged: "Apache
  Arrow output (migration Phase 4+) MUST avoid unnecessary intermediate
  conversions for downstream consumers" is this constitution clause's exact
  named scenario, and is what the native-crate + `pyo3-arrow` architecture
  exists to satisfy.
- **Development Workflow — Phased Migration Discipline** — Note, not a
  gate failure: this spec is Phase 4 (Arrow Integration), executed after
  Phase 5's Python binding (specs `6000`/`6001`) rather than before it, per
  the plan's documented phase order. That ordering was already established
  by those earlier specs (not introduced here) and is not re-litigated by
  this plan; this spec's own scope is unaffected either way since it only
  depends on `hl7pet-core`, not on the Python binding crate.

No violations requiring Complexity Tracking.

### Post-Phase-1 Re-check

Re-evaluated after `research.md`/`data-model.md`/`contracts/arrow-api.md`
were written: still PASS on every gate above. The two items flagged as
open during Phase 0 (`pyo3-arrow`'s abi3 support, `arrow_udf`'s
struct-output support) are implementation-time verification tasks with
already-designed fallbacks (hand-rolled `arrow::ffi`; `mapInArrow`), not
unresolved design questions — neither changes this feature's external
contract (`contracts/arrow-api.md`) if the fallback is needed, so neither
blocks proceeding to `/speckit-tasks`. Principle III's gate is now
concretely satisfied, not just asserted: data-model.md's `{value, status}`
Result Struct is the actual mechanism, not a placeholder.

## Project Structure

### Documentation (this feature)

```text
specs/6002-arrow-integration/
├── plan.md              # This file
├── research.md           # Phase 0 output
├── data-model.md         # Phase 1 output
├── quickstart.md         # Phase 1 output
├── contracts/
│   └── arrow-api.md      # Phase 1 output
└── tasks.md              # Phase 2 output (/speckit-tasks — not created here)
```

### Source Code (repository root)

```text
crates/
├── core/                          # unchanged (hl7pet-core)
├── python/                        # unchanged (existing plain Python binding, spec 6000)
├── cli/                           # unchanged
├── xtask/                         # unchanged
└── arrow/                         # NEW — hl7pet-arrow
    ├── Cargo.toml                 # depends on hl7pet-core (path) + arrow + pyo3 + pyo3-arrow
    ├── pyproject.toml             # maturin mixed layout, mirrors crates/python/pyproject.toml
    ├── src/
    │   ├── lib.rs                 # #[pymodule]: extract_value, extract_values
    │   ├── convert.rs             # hl7pet-core LocatedValue/Vec<Vec<Cow<str>>> -> Arrow builders
    │   ├── result_schema.rs       # the {value, status} / multi-field struct Arrow schema (data-model.md)
    │   └── errors.rs              # PyO3 exception types for up-front invalid-input rejection (FR-008/FR-009)
    ├── python/hl7pet_arrow/
    │   ├── __init__.py            # re-exports extract_value/extract_values from the compiled extension
    │   └── spark.py               # pure Python: arrow_udf-wrapped factories (Story 3), no new Rust
    └── tests/
        ├── test_parity.py         # fixtures-corpus parity vs. the plain hl7pet binding (SC-003)
        ├── test_scan_count.py     # SC-002: multi-PATH does one scan per message regardless of PATH count
        └── test_spark.py          # Story 3 acceptance, local PySpark session

notebooks/
└── arrow_pyspark_demo.ipynb       # NEW — FR-011/SC-005, Story 4
```

**Structure Decision**: New Cargo workspace member `crates/arrow`, packaged
as its **own** `maturin` root (own `pyproject.toml`/wheel, importable as
`hl7pet_arrow`) rather than folded into the existing `crates/python`/`hl7pet`
wheel. Rationale: `hl7pet-arrow` links against `hl7pet-core` directly in
Rust and needs nothing from the already-compiled `_hl7pet` extension module,
so there is no functional reason to co-package them, and `maturin`'s mixed
layout ties one Cargo crate to one Python distribution — trying to fuse two
`#[pymodule]`s into one wheel would add packaging complexity for no benefit.
This exactly mirrors how `crates/python` itself is already its own
independent `maturin` root alongside `crates/cli`/`crates/xtask`. A
top-level `notebooks/` directory is new (this repo currently has no runnable
notebooks); `playground/` is not reused since it's a Flask demo app for a
different spec (`9000`), not a notebook host.

## Complexity Tracking

*No Constitution Check violations — table intentionally empty.*
