# Phase 0 Research: Arrow Integration for PySpark & PyArrow

All version numbers below were checked live (crates.io API, PyPI, and the
official Apache Spark docs) on 2026-09-14, not recalled from training data,
per this repo's established practice of verifying against real sources
rather than assuming.

## 1. `arrow` (arrow-rs) crate version and C Data Interface support

**Decision**: Pin `arrow = "59"` (latest stable, `59.3.0`, released
2026-09-01 on crates.io).

**Rationale**: `arrow-rs` still ships its `ffi` module implementing the
Arrow C Data Interface (`FFI_ArrowArray`/`FFI_ArrowSchema`, `to_ffi`/
`from_ffi`), which is what makes zero-copy handoff to `pyarrow.Array`/
`pyarrow.Table` possible without a serialization round-trip — directly
satisfying Constitution Principle II and the Performance & Portability
Standards' "avoid unnecessary intermediate conversions" clause for Arrow
output.

**Alternatives considered**: Pinning an older `arrow` major (e.g. the 5x
series still referenced in some 2025-era tutorials) — rejected, no reason
to start a new crate on a stale major when 59.x is current and the FFI
module's shape has been stable across recent majors. A 60.0.0 major was
reportedly scheduled for August 2026; if it has landed by implementation
time, `cargo update` picks it up under the same `"59"` → re-check caret
range at that point — not re-litigated here.

## 2. Zero-copy PyO3<->Arrow boundary: `pyo3-arrow` vs. hand-rolled FFI

**Decision**: Use `pyo3-arrow = "0.19"` (current as of 2026-06-15, part of
the `kylebarron/arro3` ecosystem) for the PyO3-facing function signatures,
rather than hand-rolling `arrow::ffi::FFI_ArrowArray`/`FFI_ArrowSchema`
export/import in `hl7pet-arrow` itself.

**Rationale**: `pyo3-arrow` provides `PyArray`/`PyRecordBatch` (via the
`__arrow_c_array__` PyCapsule protocol) and `PyTable`/`PyChunkedArray`/
`PyRecordBatchReader` (via `__arrow_c_stream__`) wrapper types with
`FromPyObject` implementations, so a `#[pyfunction]` can simply accept
`PyArray` / return `PyArray` and get zero-copy conversion to/from *any*
Python object implementing the Arrow PyCapsule interface — which
`pyarrow.Array`/`pyarrow.Table` already do — without `hl7pet-arrow` writing
its own FFI marshaling code. Hand-rolling the same thing directly against
`arrow::ffi` is strictly more code for the same zero-copy result and is
the kind of low-level plumbing a maintained crate exists precisely to
absorb.

**Open item carried into implementation** (not a blocker for this plan,
but explicitly flagged rather than assumed): `pyo3-arrow`'s `abi3` feature
compatibility with `pyo3 = "0.29"`'s `abi3-py39` (used by every other
binding in this repo, `crates/python` included) was not confirmed via
search. **Task for Phase 2/implementation**: a small spike — add the
dependency, build with `abi3-py39` enabled, and confirm it compiles and
the resulting wheel is a true `abi3` wheel (one binary for all Python
>=3.9, matching `crates/python`'s existing distribution model) before
committing to it project-wide. If `pyo3-arrow`'s abi3 support turns out to
be incomplete, the fallback is hand-rolled `arrow::ffi` conversion (the
rejected alternative above), still zero-copy, just more code — not a
change to this feature's external contract either way.

**Alternatives considered**: Hand-rolled `arrow::ffi` (rejected above,
kept as the abi3-spike fallback). `minarrow`/`minarrow-pyo3` (found during
search) — rejected: a newer, less-established alternative Arrow
implementation rather than the canonical `arrow-rs`/PyArrow ecosystem this
repo's consumers (Pandas, Polars, PySpark, Databricks, DuckDB — per the
migration plan's own Phase 4 goals) actually standardize on.

## 3. PyO3 version

**Decision**: Keep `pyo3 = "0.29"` (current stable is `0.29.2`) with
`abi3-py39`, matching `crates/python`'s existing pin exactly.

**Rationale**: Already the pin used elsewhere in this workspace; no reason
for `hl7pet-arrow` to diverge to a different PyO3 line, and 0.29.x is
current (not stale) as of this research date.

**Alternatives considered**: A newer PyO3 major, if one exists by
implementation time — deferred; matching the existing binding's exact pin
is more valuable here than chasing the newest minor, since a workspace-wide
PyO3 bump is a separate, cross-cutting concern outside this spec's scope.

## 4. PySpark Arrow-native UDF API

**Decision**: Target **PySpark `4.2.0`** (current stable, released
2026-07-14) and use `pyspark.sql.functions.arrow_udf` — the vectorized,
Arrow-native UDF decorator (functions operate directly on `pyarrow.Array`,
introduced in Spark 4.1) — for Story 1's single-PATH column function,
rather than `pandas_udf` or a plain `@udf(useArrow=True)`.

**Rationale**: `arrow_udf` operates on `pa.Array` directly with no pandas
conversion step, which is the leanest path between "Arrow array of
messages" and "Arrow array of results" — the same zero-copy goal driving
the Rust-side design, carried through to the Spark-side call site. Plain
`@udf(useArrow=True)` is still scalar-per-row Python logic under the hood
(rejected: no vectorization benefit). PySpark 4.2 also raised its own
minimum PyArrow requirement to `18.0.0` (up from 15.0.0), which is already
satisfied by targeting current PyArrow (`25.0.1`) in the notebook/test
environment.

**Open item carried into implementation**: `arrow_udf`'s support for a
struct-typed return (needed for Story 2's multi-PATH, one named field per
requested PATH — mirroring the well-documented `pandas_udf` struct-output
pattern) was not confirmed against official examples during this research
pass; only scalar `pa.Array`-to-`pa.Array` examples were found documented.
**Task for Phase 2/implementation**: verify `@arrow_udf("struct<...>")`
returning a `pa.StructArray` against a real, locally installed PySpark
4.2.0 session as an early implementation task, before building Story 2's
Spark wiring on top of it. If unsupported, `DataFrame.mapInArrow`
(RecordBatch iterator in/out, stable since Spark 3.3, well-documented) is
the fallback for the multi-PATH Spark entry point specifically — Story 2's
standalone-PyArrow path (`hl7pet_arrow.extract_values`, spec FR-005) is
unaffected either way since it doesn't depend on Spark's UDF machinery at
all.

**Alternatives considered**: `pandas_udf` with `useArrow=True` — rejected
as the primary mechanism (adds a pandas conversion `arrow_udf` skips
entirely) but noted as a widely-documented, safe fallback pattern for the
struct-output case if both `arrow_udf` and `mapInArrow` prove awkward for
it. `mapInArrow` as the *primary* mechanism for everything — rejected for
Story 1 specifically: it's a whole-DataFrame RecordBatch transform, more
machinery than a simple scalar column-in/column-out call needs.

## 5. Result shape for FR-010 (per-row scan-failure signal)

**Decision**: Both mechanisms return a **result struct** per requested
PATH — `{value: List<List<Utf8>>, status: Utf8}` — rather than a bare
value array plus a side-channel, and rather than raising. `status` is one
of `"ok"` / `"no_match"` / `"scan_error"`. Single-PATH extraction returns
one such struct (itself a single `pa.Array`, satisfying spec FR-001 — a
`StructArray` is one Arrow array); multi-PATH extraction returns a
struct-of-structs, one outer field per requested PATH, each an inner
`{value, status}` struct (satisfying FR-002 and mapping directly onto
Spark's struct-output column convention, `.select("result.<path>.*")`).

**Rationale**: FR-010 requires the "this row failed to scan" signal to be
readable without the caller catching an exception, and to stay distinct
from "no match" (Constitution Principle III's absence-vs-error
distinction, extended to the per-row case where raising isn't an option
for a batched call). A 2-state plain value (null-for-both-cases) would
silently conflate "no data" with "malformed input" for that row, which the
plain Python binding already treats as never acceptable — this spec
doesn't get to relax that guarantee just because the call is now
columnar. Full design in data-model.md.

**Alternatives considered**: Raise-and-abort-whole-batch (matches the
plain binding's per-message semantics exactly, but rejected per spec
FR-010's explicit requirement that other rows keep processing — a
deliberate, discussed departure for the batched case, not an oversight).
A separate parallel "errors" array returned alongside the values array —
rejected: forces every caller to zip two arrays back together by
position to reconstruct per-row state, more error-prone than one
self-contained struct per row.
