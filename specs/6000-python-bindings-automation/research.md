# Phase 0 Research: Python Bindings & Core-Sync Tooling

All "NEEDS CLARIFICATION" items from Technical Context are resolved below.
Migration-plan Phase 5 (`HL7-PET-Rust-Migration-Plan.md`) already fixes the
architecture (PyO3 + maturin, one-call-per-field API shape matching Scala,
plus a batched variant) — this research resolves the decisions Phase 5
leaves open: crate layout, ABI target, exception mapping, and the two
sync-tooling implementations.

## 1. PyO3 ABI target: `abi3-py39` vs. per-version wheels

**Decision**: Build against PyO3's stable `abi3` ABI, floor `py39` (Python
3.9+).

**Rationale**: `abi3` produces a single wheel per platform that loads on any
Python ≥ the floor version, instead of one wheel per Python minor version.
This spec has no CI release matrix yet (Target Platform, plan.md) — a single
build command per platform keeps the maintainer's local `maturin build` loop
simple, and the choice is forward-compatible with adding a real
`cibuildwheel` matrix later without an ABI-target migration. `py39` matches
the oldest Python still commonly deployed with current `pandas`/`pyspark`
stacks (the constitution's own stated audience, Principle IV rationale) as
of 2026.

**Alternatives considered**: Per-version wheels (`cp39`, `cp310`, ... one
build per version) — rejected for this spec: more build matrix complexity
than a from-scratch port needs, and nothing in FR-001–FR-013 requires
version-specific behavior that would force it.

## 2. Crate layout: single mixed Rust/Python directory vs. split `bindings/`

**Decision**: `crates/python/`, maturin's standard "mixed layout" (Cargo
crate + `pyproject.toml` + `python/<package>/` all siblings in one
directory), as a new Cargo workspace member.

**Rationale**: Keeps the binding inside the existing `crates/` convention
this repo already uses for `core`/`cli`, so `cargo build --workspace` and
`cargo test --workspace` keep covering it without a second top-level
directory tree to remember. Maturin's mixed layout is its documented default
for "wrap a Rust crate as an importable Python package with some pure-Python
glue" — exactly this case (`__init__.py` re-exporting the compiled
extension, `.pyi` stubs alongside).

**Alternatives considered**: A separate top-level `bindings/python/` (outside
`crates/`) — rejected, no other part of this repo uses a `bindings/` root,
and it would need its own `Cargo.toml`-workspace-membership wiring anyway
with no offsetting benefit.

## 3. Exception mapping (FR-005)

**Decision**: One Python exception hierarchy rooted at `Hl7PetError`
(itself a plain `Exception` subclass), with one subclass per `hl7pet-core`
error enum crossed at the FFI boundary:

| Rust error | Python exception | Raised when |
|---|---|---|
| `ScanError` | `Hl7ScanError` | message fails to scan (structural) |
| `ParseError` | `Hl7PathError` | PATH expression is syntactically invalid |
| `QueryError::NonNumericComparison` | `Hl7QueryError` | ordering filter operand isn't numeric |
| `ProfileError` | `Hl7ProfileError` | hierarchy profile JSON is invalid |

**Rationale**: Directly satisfies FR-005 and Principle III: every one of
these is a genuine structural precondition failure in `hl7pet-core` today
(confirmed by reading `query.rs`'s and `scanner.rs`'s own doc comments —
"the one genuine error is..."), never a "no data" outcome, which is why each
gets a raise instead of a `None`. A shared root class lets a caller
`except Hl7PetError` broadly while still allowing `except Hl7PathError`
narrowly, mirroring how the Scala library's own exception surface is used in
practice (broad catch around a batch, narrow catch for a specific known
failure mode).

**Alternatives considered**: A single generic `Hl7PetError` for everything —
rejected, FR-005 explicitly requires structural failures to "surface
distinctly," and losing the ability to distinguish "bad PATH syntax" from
"unparseable message" from Python would be a regression versus what
`hl7pet-core`'s own `Result` types already distinguish.

## 4. Batched extraction call shape (FR-004)

**Decision**: `get_values(paths: list[str]) -> list[list[list[str]] | None]`
— one Rust `scan`/parse pass per message, then one `execute` per compiled
path, returned in input order; a path that fails to parse raises immediately
(consistent with the single-call `get_value` raising for the same case)
rather than substituting a placeholder in the list.

**Rationale**: Matches the migration plan's own Phase 5 language verbatim
("batched variant ... returns all requested paths in a single Rust
crossing ... amortizes PyO3 ... per-call overhead"). Raising immediately on
a bad PATH inside a batch (rather than embedding an error marker per
element) keeps the single- and batched-call error contracts identical, so
User Story 2/3 tooling and callers don't need two different absence/error
conventions to reason about.

**Alternatives considered**: Returning a per-element `(value, error)` tuple
so one bad PATH doesn't abort the whole batch — rejected as unnecessary
scope: nothing in the spec's Acceptance Scenarios or FR-004 asks for
partial-batch-failure tolerance, and it would introduce a second absence/
error shape alongside `get_value`'s plain-raise contract for no
spec-mandated benefit.

## 5. Surface-diff tool implementation (FR-007/FR-008/FR-010)

**Decision**: A new pure-Rust `xtask` binary crate using `syn` to parse
`crates/core/src/**/*.rs` into a serializable "surface snapshot" (every
top-level `pub fn`/`pub struct`/`pub enum`/`pub type`, with signatures,
field/variant lists, and doc-comment first line), diffed against a committed
baseline JSON file.

**Rationale**: `syn` is the standard pure-Rust source-parsing crate (no
nightly toolchain, no external tool install, no network fetch — consistent
with this repo's existing "no hard dependency on external repos, everything
committed as static files" convention already used for `fixtures/`). It is
a dev-only dependency of the new `xtask` crate, never of `hl7pet-core`
itself, so the pure-Rust/nothing-leaks-through-the-public-API dependency
policy for the core crate is untouched. Parsing source directly (rather than
introspecting compiled output) also makes "item never intended to cross the
FFI boundary" (FR-010) straightforward: `xtask` recognizes a
`#[doc(hidden)]` attribute or a leading `#[allow(hl7pet::internal)]`-style
marker comment convention on the pub item and excludes it from the snapshot
by construction — no separate exclusion list to keep in sync by hand.

**Alternatives considered**:
- `cargo public-api` (the existing community crate built exactly for this
  purpose) — rejected: as of current releases it depends on rustdoc's JSON
  output, which requires a nightly `rustc` toolchain to generate. This repo
  targets stable Rust only (constitution's Performance & Portability
  Standards: "MUST build on stable Rust"); adding a nightly-only dev
  dependency for tooling would be a first exception to that with no
  corresponding benefit `syn` doesn't already provide for this repo's small,
  single-crate surface.
- A hand-maintained changelog the maintainer updates manually per spec —
  rejected: this is exactly the manual, doesn't-scale process User Story 2
  exists to replace (spec's own "Why this priority" reasoning).

## 6. Sync Baseline storage & versioning (FR-011, Key Entities)

**Decision**: `crates/xtask/surface-baseline.json`, committed to git,
containing `{"commit": "<git sha the baseline was captured at>", "surface":
{<the full snapshot from research item 5>}}`. `xtask sync-baseline` (run by
the maintainer only after porting every flagged change and passing the
parity check, FR-011) overwrites this file and the maintainer commits it as
part of that sync's own PR — no separate database, no timestamp-based
staleness logic.

**Rationale**: A git-committed file gets free history (`git log
crates/xtask/surface-baseline.json` shows every past sync point), free
diffing (`git diff` on the JSON shows exactly what changed at a glance even
without running the tool), and requires no new infrastructure. Per the
spec's own Assumptions section, the baseline is "tracked per completed
Python-binding sync ... not per individual `hl7pet-core` commit" — a single
current-state file matches that exactly; there is no need to retain
every historical baseline as separate rows, since git history already is
that record.

**Alternatives considered**: A running log/append-only file of every sync
event — rejected as unneeded: nothing in the spec's Edge Cases or FR list
asks for a queryable history beyond "the next run reports the full
accumulated diff since last baseline" (Edge Cases, "never runs the sync
script for several consecutive specs"), which a single current-baseline file
plus a full re-diff already satisfies.

## 7. Parity-check implementation language (FR-006/FR-009)

**Decision**: Python (`crates/python/tests/parity_check.py`), runnable both
standalone (`python parity_check.py`) and as a `pytest` test module
(`pytest crates/python/tests/`), reading every vector under
`fixtures/vectors/{path,hierarchy,scanner,escapes}/*.json` and calling the
built `hl7pet` package directly.

**Rationale**: The parity check's whole job is exercising the *Python*
package's behavior end-to-end (User Story 3), so it must run in Python
against the installed wheel, not in Rust against `hl7pet-core` directly —
that would test the wrong layer and miss exactly the PyO3-boundary bugs this
check exists to catch. Reusing the fixtures corpus's existing vector/schema
format (already read by `fixtures/scripts/validate_corpus.py`) means no new
data format to maintain. Wiring it into `pytest` as well as standalone
satisfies both FR-009's "runnable on demand" wording and gives the binding a
normal `pytest`-based CI-ready test entry point for User Story 1's own
Independent Test.

**Alternatives considered**: A Rust integration test that calls into the
compiled `.so`/`.pyd` via `pyo3::Python::with_gil` from within `cargo test`
— rejected: adds a second, more complex way to invoke Python from Rust for
no benefit over just running Python directly, and would still need a
Python interpreter present, so it doesn't actually reduce the toolchain
footprint.

## 8. Not-yet-implemented vs. wrong (Edge Cases)

**Decision**: `parity_check.py` classifies each vector into exactly one of
three states — `match`, `mismatch`, `not_implemented` — where
`not_implemented` fires when calling the corresponding Python entry point
raises `AttributeError`/`NotImplementedError` (the function doesn't exist
yet on the current binding) rather than any `Hl7*Error` or a wrong value.

**Rationale**: Directly satisfies the Edge Case requirement that partial
progress during initial porting isn't misreported as failure — a vector
family with no Python counterpart yet is distinguishable at a glance from
one that's implemented but returns the wrong value, per SC's parity
reporting intent.

**Alternatives considered**: Treating any exception (including
`AttributeError`) as a plain `mismatch` — rejected, collapses "not built
yet" and "built wrong" into one bucket, which is the exact ambiguity the
Edge Case calls out as something the check MUST avoid.
