# Implementation Plan: PATH Grammar Specification & Conformance Vectors

**Branch**: `001-path-grammar-spec` | **Date**: 2026-07-23 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-path-grammar-spec/spec.md`

## Summary

Produce the formal, language-agnostic PATH grammar (EBNF) and a JSON-Schema-conformant
set of conformance vectors (valid, zero-value, and syntactically-invalid cases) that
later specs (`002` hierarchy semantics, `003` regression suite, `006` Rust PATH parser)
build against. This is a documentation/data deliverable — no Rust, Python, or Java code
is written by this feature. The grammar and vectors are derived from `SPEC.md` §3.1 and
verified against the real behavior of the external Scala library
([mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet)), with any
documentation/reality discrepancy escalated per FR-010 rather than silently resolved.

## Technical Context

**Language/Version**: N/A — this feature produces Markdown + JSON artifacts, no source
code. (The grammar it documents will later be implemented in Rust by spec `006`.)

**Primary Dependencies**: None required. A JSON Schema validator (e.g. `ajv`,
Python `jsonschema`) MAY be used during Phase 1/implementation to lint conformance
vector files against `contracts/conformance-vector.schema.json`, but is not a hard
dependency of this spec's deliverables.

**Storage**: N/A — flat files committed under this feature's own directory (FR-007).

**Testing**: Manual verification (SC-004) — each conformance vector's PATH string is
run against the real Scala library and the actual output compared to the vector's
`expected` field. No automated test harness exists yet; wiring these vectors into an
automated/CI-run regression suite is spec `003`'s responsibility, not this one's.

**Target Platform**: N/A (documentation/data deliverable).

**Project Type**: Single feature directory, documentation/data artifacts only (no
`src/`, no application code).

**Performance Goals**: N/A — no code executes. (Performance validation of the eventual
Rust engine is spec `009`, downstream of this one.)

**Constraints**: All conformance-vector source messages MUST be synthetic (FR-009, no
real PHI). Conformance vectors MUST conform to the JSON Schema formalized from FR-011.
No hard dependency (submodule, build-time fetch) on the external Scala repo — anything
needed from it is fetched manually, once, and either discarded (verification runs) or
committed as static data (message fixtures, if any are adapted rather than authored
fresh).

**Scale/Scope**: One grammar document; ≥25 conformance vectors per SC-003 (one per
grammar production, one per syntax-level Known Limitation, one invalid-PATH vector per
production where a violation is structurally possible).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|---|---|
| I. Path Contract Stability (NON-NEGOTIABLE) | **Enforced, not just satisfied** — this feature *is* the artifact Principle I requires to exist. No violation. |
| II. Zero-Copy & Lazy Evaluation | N/A at this stage — no code is written. The grammar document doesn't prescribe an execution model, so it can't violate this. Applies to specs `005`-`007`. |
| III. Explicit, Exception-Free Data Absence | **Reinforced** — FR-004's "zero, one, or many values" and FR-012's `"INVALID"` marker vs. FR-004's zero-values result directly operationalize the "absent data ≠ malformed input" distinction this principle requires. |
| IV. Multi-Language Interoperability | **Reinforced** — grammar and vectors are language-agnostic by construction (JSON Schema, EBNF prose), consumed identically by future Rust/Python/Java work (FR-005). |
| V. Conformance Through Declarative Profiles & Documented Limitations | **Reinforced** — FR-003/FR-012 require every syntax-level Known Limitation to have an explicit vector; the grammar itself is declarative EBNF, not embedded logic. |
| Performance & Portability Standards | N/A — no code, no benchmarks at this stage. |
| Development Workflow — Phased Migration Discipline | **This feature is an explicit Phase 1 gate deliverable** named by the constitution ("PATH grammar documentation ... MUST be complete ... before any Rust core implementation work begins"). Completing this plan satisfies that gate for the grammar portion (hierarchy semantics and the regression suite remain separate gates owned by specs `002`/`003`). |

No violations at Phase 0. **Post-Phase-1 re-evaluation**: design work in
`contracts/path-grammar.md` tightened three productions (`SEG` now requires an
alphabetic first character; `SEG_IDX`/`FIELD_IDX` now reject non-numeric/non-`$LAST`/
non-`*` content at parse time instead of matching-then-crashing; `OPERATOR` is now
restricted to its six defined tokens) versus what `SPEC.md` previously documented and
the current regex previously accepted (see that document's Notes section for the
full rationale, derived from reading the actual `HL7StaticParser.scala` source, not
just `SPEC.md` prose).

These are breaking changes to the PATH grammar under Principle I. Principle I permits
breaking changes — it does not forbid them — but only when accompanied by a MAJOR
version bump and a written migration guide. This is the same accommodation already
used for spec `1001` (escape-sequence decoding) in `ROADMAP.md`'s Conventions section,
now extended to cover these three grammar-tightening decisions as well (recorded
there). No Complexity Tracking entry is needed — this isn't unjustified complexity,
it's a deliberate, documented, principle-compliant breaking change.

## Project Structure

### Documentation (this feature)

```text
specs/001-path-grammar-spec/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

This feature produces no application source code. Its actual deliverables (grammar
document, conformance vectors, synthetic messages) live entirely under its own feature
directory, per FR-007:

```text
specs/001-path-grammar-spec/
├── contracts/
│   ├── path-grammar.md                    # Formal EBNF grammar (FR-001, FR-002, FR-006)
│   └── conformance-vector.schema.json     # JSON Schema formalizing FR-011
├── vectors/
│   └── *.json                             # Conformance vector files (FR-003, FR-011, FR-012),
│                                           # one record per FR-011 schema, grouped however
│                                           # is convenient (e.g. one file per grammar production)
├── messages/
│   └── *.hl7                              # Synthetic source HL7 messages referenced by
│                                           # vectors' message_ref field (FR-009)
├── data-model.md
├── quickstart.md
└── research.md
```

**Structure Decision**: Everything this feature produces stays inside
`specs/001-path-grammar-spec/` — there is no `src/`, `lib/`, or `tests/` tree because no
code is written. `vectors/` and `messages/` are new subdirectories (not present in the
generic plan template) introduced here to hold the concrete files FR-003, FR-009, and
FR-011 describe abstractly. `contracts/` holds the two artifacts other specs treat as a
stable interface: the grammar itself (consumed by spec `006`) and the vector schema
(consumed by spec `003`).

## Complexity Tracking

*No entries — Constitution Check reported no violations requiring justification.*
