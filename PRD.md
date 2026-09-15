# HL7-PET Product Requirements Document

**Status:** Living document — reflects current intent, not a point-in-time snapshot.
**Owner:** Marcelo Caldas
**Related docs:** [README.md](README.md) (orientation) ·
[HL7-PET-Rust-Migration-Plan.md](HL7-PET-Rust-Migration-Plan.md) (architecture & phasing) ·
[SPEC.md](SPEC.md) (parity reference — the current Scala API) ·
[.specify/memory/constitution.md](.specify/memory/constitution.md) (non-negotiable engineering
principles) · [ROADMAP.md](ROADMAP.md) (live per-spec status)

This document says **what** HL7-PET needs to do and **why**, and for whom. It does not
restate architecture (migration plan), non-negotiable engineering constraints (constitution),
or current build status (ROADMAP) — it links to those instead.

---

## 1. Problem statement

HL7 v2 messages are consumed in high volume by public-health and clinical data pipelines,
but most tooling forces a choice between two bad options:

- **Full object-model parsers** — correct, but pay the cost of materializing an entire
  message into objects even when a caller only needs three fields out of it.
- **Ad hoc string/regex splitting** — fast, but brittle, and re-implemented per project with
  no shared grammar, no validation, and no de-identification story.

HL7-PET exists to give callers a **direct, path-based query surface** into a raw message
(`OBX[3]-5[2].1`) without either paying for a full parse or hand-rolling delimiter logic.
That query surface — the PATH grammar — is the product; everything else (validation,
de-identification, batch handling) is built on top of the same lazy-extraction model.

The Scala implementation (`gov.cdc.hl7`, [mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet))
has proven this model in production, but is JVM-only. This project is a clean-room Rust
rewrite of it, adding first-class Python and continued Java support, so the same engine and
the same PATH contract serve both ecosystems from one implementation instead of two
independently-maintained ones.

## 2. Target users & use cases

- **Public-health / clinical data engineers** (the primary user, including the author's own
  work at CDC) building pipelines that extract, validate, and de-identify HL7 v2 feeds at
  volume — today in Scala/Java, increasingly in Python-based data stacks.
- **Data scientists / analysts** who want HL7 fields as Arrow-native columns directly in
  Pandas, Polars, PySpark, Databricks, or DuckDB, without a hand-written parsing step.
- **Existing Scala/`gov.cdc.hl7` consumers** who need a migration path to Rust/JVM-native
  performance without rewriting the path expressions and rules files they already depend on.

Representative use cases, carried over from the Scala library and extended:
1. Pull specific fields (e.g. patient ID, observation value) out of millions of HL7
   messages per run, without building a full object model per message.
2. Validate message structure and field-level conformance rules against a declarative
   JSON profile, for a given message type/jurisdiction.
3. De-identify PII/PHI fields in a message before it's stored or forwarded.
4. Split batch files (FHS/BHS/BTS/FTS) into individual messages for downstream processing.
5. Load a large HL7 corpus directly into a dataframe (Pandas/Polars/PySpark) as Arrow
   columns for analysis, instead of message-by-message extraction in a loop.

## 3. Goals

- **G1 — Preserve the PATH contract.** Every path expression that works against the Scala
  engine today works, with identical semantics, against the Rust engine and every binding.
  (See [Constitution I](.specify/memory/constitution.md#i-path-contract-stability-non-negotiable).)
- **G2 — Match or beat Scala performance.** Lower memory usage, fewer allocations, higher
  extraction/parsing throughput than the JVM baseline, verified by benchmark, not assumed.
- **G3 — First-class Python.** A real PyPI wheel (PyO3/maturin), not a JVM-bridge wrapper —
  Python is a primary target, not an afterthought.
- **G4 — Maintain JVM interoperability.** A Java binding at parity with Python, so existing
  `gov.cdc.hl7` consumers have a migration path.
- **G5 — Native Arrow output.** Query results usable directly by Pandas/Polars/PySpark/
  Databricks/DuckDB without an intermediate conversion step.
- **G6 — Full functional parity with the Scala library**: field extraction, hierarchy
  navigation, structure/conformance validation, de-identification, and batch file handling
  — not just the query engine subset.

## 4. Non-goals

- **Not a general HL7 object model / full DOM parser.** Zero-copy, lazy, index-only-what's-
  needed extraction is a permanent design constraint, not a v1 shortcut
  (see [Constitution II](.specify/memory/constitution.md#ii-zero-copy--lazy-evaluation)).
- **Not HL7 v3 / FHIR.** Scope is HL7 v2.x only, matching the Scala library.
- **Not a message-generation or message-editing library.** Read/query/validate/de-identify/
  split — not construct or mutate messages.
- **Not vendoring or depending on the Scala repo at build time.** It's referenced only as a
  parity target via exported fixtures, never a submodule or runtime dependency.
- **Not accepting external contributions yet** — solo, early-stage rewrite; see README
  [Contributing](README.md#contributing).

## 5. Functional requirements

Each area below maps to a module range in [ROADMAP.md](ROADMAP.md), where per-spec detail
and current status live. Requirements here are the durable "what," not the fine-grained
"how" or "is it built yet."

| # | Requirement | Module (ROADMAP.md) |
|---|---|---|
| FR1 | Parse PATH expressions (segment/field/component/subcomponent, `[n]` repetition, `$LAST`, `*`, `@field=value` filters) into reusable compiled query objects. | Parsing & Extraction |
| FR2 | Execute a compiled query against a raw message and return the addressed value(s) without materializing a full object model. | Rust Core |
| FR3 | Support hierarchy-aware navigation (`OBR[1] -> OBX-5`) evaluated lazily, only when a hierarchy-mode path is actually used. | Rust Core |
| FR4 | Decode HL7 escape sequences correctly and document any sequences that remain unsupported. | Rust Core / Parsing |
| FR5 | Validate message structure and cardinality against declarative, versionable JSON profiles. | Validation |
| FR6 | Validate conditional/conformance predicate rules against declarative rules files. | Validation |
| FR7 | Validate batch file structure (FHS/BHS/BTS/FTS). | Validation |
| FR8 | De-identify/redact configured PII/PHI fields in a message. | De-identification |
| FR9 | Split batch files into individual messages and other file/batch utilities carried over from `HL7FileUtils`. | File & Batch Utilities |
| FR10 | Produce query results as Apache Arrow arrays/tables directly, for single messages and batches. | Cross-cutting (Arrow) |
| FR11 | Expose every core capability through a Python binding (PyO3), with a one-call-per-field API matching the current Scala shape (`getValue`, `getFirstValue`) plus a batched multi-path variant for hot loops. | Language Bindings |
| FR12 | Expose every core capability through a Java binding (JNI/JNA) at semantic parity with Python. | Language Bindings |
| FR13 | Signal missing/empty/out-of-range data through the host language's idiomatic absence mechanism (`Option`, `null`-safe return, etc.), never via exception, per [Constitution III](.specify/memory/constitution.md#iii-explicit-exception-free-data-absence). | Cross-cutting |

## 6. Success metrics

- **Parity:** 100% of the committed conformance-vector corpus (`fixtures/`) passes against
  both the Scala baseline and every language binding — this is the operational definition
  of "PATH contract preserved" (G1) and "functional parity" (G6).
- **Performance:** Rust core benchmarks (Criterion) show lower memory usage, fewer
  allocations, and equal-or-higher extraction/parsing throughput than the Scala baseline
  benchmark captured in Phase 1, for every workload class benchmarked.
- **Python adoption readiness:** a `pip install`-able wheel exists, published from
  `crates/python`, with install/quickstart docs — no source build required by end users.
- **Java parity:** every capability reachable from Python is reachable from the Java
  binding with equivalent semantics before that feature is considered done
  (per [Constitution IV](.specify/memory/constitution.md#iv-multi-language-interoperability)).
- **Arrow interoperability:** query results load into Pandas, Polars, PySpark, Databricks,
  and DuckDB without a hand-written conversion step, demonstrated by an example per target.

## 7. Constraints

- `hl7pet-core` must build on stable Rust with zero FFI dependencies, so it can be
  benchmarked honestly and reused unmodified by both bindings
  ([Constitution II](.specify/memory/constitution.md#ii-zero-copy--lazy-evaluation)).
- `hl7pet-core`'s dependencies must be pure-Rust and must never leak through the public API
  — required for clean Python/Java bindings.
- No breaking change to PATH grammar or evaluation semantics without a major version bump
  and a written migration guide
  ([Constitution I](.specify/memory/constitution.md#i-path-contract-stability-non-negotiable)).
- Structural validation and conformance rules must stay declarative/profile-driven, not
  hard-coded per message type — HL7 profiles vary by jurisdiction and program
  ([Constitution V](.specify/memory/constitution.md#v-conformance-through-declarative-profiles--documented-limitations)).
- This repo has no build-time dependency on the Scala repo; parity fixtures are exported
  once and committed statically, not fetched live.

## 8. Open questions

- Java binding mechanism: JNI/JNA vs. the Panama FFM API — undecided per the migration
  plan (Phase 5); revisit when that phase starts.
- Whether the Java binding gets its own `bindings/java` root or lives inside `crates/`
  alongside the Python crate, given the Python binding ended up diverging from the
  original planned `bindings/python` layout (see README [Repository layout](README.md#repository-layout)).
- Ratification date for the project constitution is still a placeholder
  (`TODO(RATIFICATION_DATE)` in constitution.md).

## 9. Milestones & current status

Not tracked here — this section would drift immediately. See:
- [ROADMAP.md](ROADMAP.md) for per-spec status by module.
- [README.md — Current Status](README.md#current-status) for a plain-language summary.
- [HL7-PET-Rust-Migration-Plan.md](HL7-PET-Rust-Migration-Plan.md) for the phase sequence
  (Document Current Behavior → Rust Core → Hierarchy → Arrow → Language Bindings →
  Performance) this PRD's requirements are delivered against.
