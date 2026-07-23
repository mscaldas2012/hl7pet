<!--
Sync Impact Report
- Version change: 1.0.0 → 1.0.1
- Modified principles:
  - III. Explicit, Exception-Free Data Absence — reworded to drop Scala-specific
    `Option`/`Result` naming in favor of language-agnostic "idiomatic absence" phrasing,
    so it applies cleanly to the planned Rust core and its bindings. No change in
    requirement or scope (PATCH).
- Added sections: none (this amendment)
- Removed sections: none
- Templates requiring updates:
  - .specify/templates/plan-template.md ✅ no change needed (generic Constitution Check gate, still valid)
  - .specify/templates/spec-template.md ✅ no change needed (no constitution-specific references)
  - .specify/templates/tasks-template.md ✅ no change needed (no constitution-specific references)
  - .claude/skills/speckit-*/SKILL.md ✅ no change needed (agent-agnostic, no hardcoded principle names)
- Follow-up TODOs:
  - TODO(RATIFICATION_DATE): original adoption date unknown; using constitution creation date as
    placeholder ratification date pending confirmation from the project owner.
-->

# HL7-PET Constitution

## Core Principles

### I. Path Contract Stability (NON-NEGOTIABLE)

The PATH expression syntax — segment/field/component/subcomponent addressing, index
selectors (`$LAST`, `*`), filters (`@field=value`), and the hierarchy `->` operator —
is HL7-PET's public contract. It MUST remain backward compatible across implementations
and language ports (the current Scala engine, the planned Rust core, and the Python/Java
bindings built on top of it). Any breaking change to PATH grammar or evaluation semantics
MUST be accompanied by a MAJOR version bump and a written migration guide.

Rationale: consumers address HL7 data by path string, often stored in external configs
and rules files. A silent grammar or semantics change breaks downstream extraction logic
in ways unit tests on the library itself will not catch.

### II. Zero-Copy & Lazy Evaluation

Extraction MUST NOT require materializing a full object model of the message.
Implementations MUST index only what is necessary to answer the query being run, defer
hierarchy construction until it is actually requested (`buildHierarchy`), and prefer
borrowed/sliced views over copies wherever the host language and FFI boundary allow it.

Rationale: HL7-PET's value over a full DOM-style parser is throughput on high-volume
message streams. This principle is carried forward directly from the Rust migration's
guiding principles (zero-copy, no full object model, index only what's necessary, lazy
hierarchy) and applies equally to the current Scala engine.

### III. Explicit, Exception-Free Data Absence

Methods that extract data MUST signal missing, empty, or out-of-range data through the
host language's idiomatic "value may be absent" mechanism — they MUST NOT throw or raise
to communicate absence. Exceptions/errors are reserved exclusively for violated
structural preconditions: a segment expected to be unique that repeats, a request for a
segment that does not exist via `retrieveFirstSegmentOf`, an out-of-range field index in
`splitFields`, or an invalid cardinality string in a profile.

Rationale: callers process large volumes of messages in tight loops; "no data present"
and "message is malformed" are distinct outcomes and must not require a try/catch around
every field access.

### IV. Multi-Language Interoperability

Every capability exposed by the core engine MUST be reachable, with equivalent semantics,
from all supported host languages — today the JVM (Scala/Java), and per the migration
plan, Python via PyO3 and a dedicated Java layer via JNI/JNA. New functionality MUST NOT
land in one binding without a tracked plan to bring the others to parity before the
feature is considered done.

Rationale: the migration's explicit vision is to "become a first-class Python library"
while maintaining JVM interoperability. Divergent bindings silently defeat that goal and
create a support burden across ecosystems (Pandas, Polars, PySpark, Databricks, DuckDB).

### V. Conformance Through Declarative Profiles & Documented Limitations

Structural validation, predicate rules, and conformance rules MUST be driven by
declarative, versionable JSON profiles and rules files (`segmentDefinition`,
`segmentFields`, predicate/conformance rule files) rather than hard-coded per-message-type
logic. Every known limitation of the parsing or validation engine (e.g. unsupported HL7
escape sequences, the fixed `MSH-1`/`MSH-2` delimiter assumption, single-clause filter
syntax) MUST be explicitly documented rather than silently mishandled.

Rationale: HL7 profiles vary by jurisdiction, program, and message type; hard-coded rules
prevent reuse across profiles. HL7-PET is used for public-health data — undocumented
limitations erode trust in a library whose output feeds conformance-adjacent decisions.

## Performance & Portability Standards

- Current engine runtime target: JVM 11+, Scala 2.13.x. The Rust core MUST build on
  stable Rust and MUST NOT regress parsing throughput, extraction throughput, memory
  usage, allocation count, or latency versus the Scala baseline it replaces.
- Every performance-sensitive change (parser, path evaluator, hierarchy builder) MUST be
  benchmarked against the existing baseline before merge; regressions MUST be justified
  in the PR/plan or reverted.
- Apache Arrow output (migration Phase 4+) MUST avoid unnecessary intermediate
  conversions for downstream consumers (Pandas, Polars, PySpark, Databricks, DuckDB).

## Development Workflow — Phased Migration Discipline

- Rust migration work MUST follow the phase order defined in
  `HL7-PET-Rust-Migration-Plan.md` (Document Current Behavior → Rust Core → Hierarchy →
  Arrow Integration → Language Bindings → Performance) unless a deviation is explicitly
  justified in the feature's plan.md.
- Phase 1 deliverables — PATH grammar documentation, hierarchy-semantics documentation,
  a comprehensive regression suite, and a Scala baseline benchmark — MUST be complete, or
  explicitly scoped into the current feature's plan, before any Rust core implementation
  work begins.
- New segment/field/validation behavior MUST ship with: (a) an example added to the
  API reference / quick-start docs, (b) a profile or rules-file fixture when the behavior
  is profile-driven, and (c) coverage in the regression suite.

## Governance

This constitution supersedes ad hoc practice for this repository. Amendments are made by
editing this file directly: a proposed change MUST include an updated Sync Impact Report
(as an HTML comment at the top of this file) and MUST update any dependent template or
command file found to reference an outdated principle in the same change.

Versioning policy (semantic versioning applied to this document):
- MAJOR — a principle is removed or redefined in a backward-incompatible way.
- MINOR — a new principle or section is added, or existing guidance is materially expanded.
- PATCH — wording, typo, or clarification changes with no semantic effect.

Compliance review: every `/speckit-plan` run MUST evaluate its Constitution Check gate
against the Core Principles above before implementation proceeds. Any unavoidable
deviation MUST be recorded in that plan's Complexity Tracking section with a stated
justification and a simpler alternative that was rejected and why.

**Version**: 1.0.1 | **Ratified**: TODO(RATIFICATION_DATE): confirm original adoption date | **Last Amended**: 2026-07-22
