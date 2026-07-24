# Implementation Plan: Shared Regression Suite (`fixtures/` Corpus)

**Branch**: `003-regression-suite` | **Date**: 2026-07-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-regression-suite/spec.md`

## Summary

Consolidate the already Scala-verified PATH-grammar vectors (spec `001`) and
hierarchy-semantics vectors (spec `002`) — plus their synthetic messages and
profiles — into one canonical `fixtures/` directory at the repository root,
exactly as `HL7-PET-Rust-Migration-Plan.md`'s repository layout prescribes.
Add a small, dependency-light Python validation script (schema conformance +
corpus-wide id uniqueness + no dangling `message_ref`/`profile_ref`) and a
coverage report (grammar productions from `001`, hierarchy rules from `002`),
both wired into a new GitHub Actions workflow so drift is caught at PR time.
No Rust/Python/Java engine code is written here — that begins at spec `005`.

## Technical Context

**Language/Version**: Python 3.11+ for the validation/coverage tooling. No
Rust workspace exists yet (`crates/core` is Phase 2, spec `005`), so this
spec intentionally doesn't introduce one just to validate JSON files.

**Primary Dependencies**: `jsonschema` (Draft 2020-12) — the same package
spec `001`'s `quickstart.md` already assumed for schema-checking vectors; no
new dependency family introduced.

**Storage**: N/A — flat files under `fixtures/` at the repo root (FR-001).

**Testing**: The validation script itself, run two ways: locally
(`python3 fixtures/scripts/validate_corpus.py`) and automatically in CI on
every push/PR touching `fixtures/**` (FR-005). No unit-test framework is
needed for a ~150-line validation script with no branching business logic to
unit-test in isolation — the corpus itself, plus the deliberately-broken
fixture in `quickstart.md` Scenario 2, is the test input.

**Target Platform**: GitHub Actions (`ubuntu-latest`) for CI; any contributor
machine with Python 3.11+ for local runs. This is the first CI workflow this
repository defines.

**Project Type**: Data consolidation + one small tooling script. No
application/library source tree (`src/`, `crates/`) is introduced — that
starts at spec `005`.

**Performance Goals**: SC-002 — validation (schema check + uniqueness +
reference check + coverage report) completes in well under a minute for the
current ~27-vector corpus (11 + 6 from spec `001`, 4 + 6 from spec `002`).

**Constraints**: FR-002 — consolidation MUST NOT alter vector/message/profile
content. FR-009 — original files under `specs/001-.../` and
`specs/002-.../` are copied, not moved. Synthetic-data-only inherited
unchanged from spec `001` FR-009 / spec `002` FR-012 (see FR-010).

**Scale/Scope**: 17 PATH vectors (spec `001`: `valid.json` 11 +
`invalid.json` 6) + 10 hierarchy vectors (spec `002`: `basic.json` 4 +
`complex.json` 6) = 27 vectors; 7 message files (4 from `001`, 3 from `002`);
2 profile files (from `002`); 2 vector JSON schemas.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|---|---|
| I. Path Contract Stability (NON-NEGOTIABLE) | N/A at this stage — no PATH grammar or hierarchy semantics changes are made here; this spec only relocates already-approved vectors verbatim (FR-002). |
| II. Zero-Copy & Lazy Evaluation | N/A — no engine code is written by this spec. Applies starting at spec `005`. |
| III. Explicit, Exception-Free Data Absence | N/A — no runtime extraction behavior is implemented here; the vectors already encode this distinction (spec `001` FR-004/FR-012) and are carried forward unchanged. |
| IV. Multi-Language Interoperability | **Directly served** — `fixtures/` at the repo root (not nested under a language-specific tree) is precisely what lets the future Rust core tests, Python `pytest` suite, and Java tests all read the identical corpus, per the Migration Plan's explicit rationale ("does Rust agree with Scala" and "does Python agree with Java" become the same question). |
| V. Conformance Through Declarative Profiles & Documented Limitations | **Reinforced** — the coverage report (FR-006) makes gaps in Known-Limitation vector coverage visible rather than silently missing; profiles remain declarative JSON (`fixtures/profiles/`), unchanged from spec `002`. |
| Performance & Portability Standards | N/A — no engine benchmarks at this stage (that's spec `004`/`009`). The validation script's own runtime is bounded by SC-002, not a Constitution-level performance standard. |
| Development Workflow — Phased Migration Discipline | **This spec is an explicit Phase 1 gate deliverable** — the constitution requires "a comprehensive regression suite" complete before Rust core implementation (Phase 2, spec `005`) begins. This plan satisfies that gate by making the corpus canonical, validated, and CI-enforced. |

No violations. **Post-Phase-1 re-evaluation**: design work in Phase 1 confirmed
that `message_ref`/`profile_ref` values in the existing vectors (e.g.
`"messages/baseline.hl7"`) are already relative to their *own spec's root
directory* — the same relative shape `fixtures/` uses (`fixtures/messages/`,
`fixtures/profiles/`) — so consolidation requires **zero path rewriting**
inside any vector file, only file copying. This is stronger evidence for
FR-002 ("MUST NOT alter... content") than assumed at Phase 0: not just "we
won't change it," but "there is nothing that needs changing." No new
Constitution concerns surfaced.

## Project Structure

### Documentation (this feature)

```text
specs/003-regression-suite/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

This spec's actual deliverable is **not** inside its own `specs/` directory —
per FR-001/FR-008, it lives at the repository root so every future language
binding can reach it identically:

```text
fixtures/                                  # NEW — canonical corpus (FR-001)
├── messages/                              # from specs/001 + specs/002 messages/, copied verbatim
│   ├── baseline.hl7                       # (spec 001)
│   ├── multi-repetition.hl7               # (spec 001)
│   ├── filter-example.hl7                 # (spec 001)
│   ├── multi-obx.hl7                      # (spec 001)
│   ├── complex-hierarchy.hl7              # (spec 002)
│   ├── unrecognized-segment.hl7           # (spec 002)
│   └── basic-hierarchy.hl7                # (spec 002)
├── profiles/                              # from specs/002 profiles/, copied verbatim
│   ├── basic-two-level.json
│   └── deep-nested.json
├── vectors/
│   ├── path/                              # from specs/001 vectors/, copied verbatim
│   │   ├── valid.json
│   │   └── invalid.json
│   └── hierarchy/                         # from specs/002 vectors/, copied verbatim
│       ├── basic.json
│       └── complex.json
├── schemas/                               # copies of both spec 001/002 vector schemas (FR-001)
│   ├── conformance-vector.schema.json
│   └── hierarchy-conformance-vector.schema.json
└── scripts/
    └── validate_corpus.py                 # FR-004/FR-005/FR-006: schema + uniqueness +
                                            # reference validation, plus coverage report

.github/workflows/
└── fixtures-validation.yml                # NEW — first CI workflow in this repo (FR-005)

specs/003-regression-suite/
├── contracts/
│   ├── fixture-corpus-layout.md           # Documents the fixtures/ tree above as a stable
│   │                                       # contract for specs 005-009 and future bindings
│   └── validation-script.md               # CLI contract for validate_corpus.py (inputs,
│                                           # exit codes, coverage report format)
├── data-model.md
├── quickstart.md
└── research.md
```

**Structure Decision**: The corpus and its validation tooling live at the
repository root (`fixtures/`, `.github/workflows/`), not inside
`specs/003-regression-suite/`, because FR-008 requires `fixtures/` to be the
place every subsequent spec (`005`-`009`) and future language binding writes
into directly — nesting it under one spec's own directory would recreate the
exact per-spec-copy problem (User Story 1) this spec exists to eliminate.
`specs/003-regression-suite/` holds only this spec's own planning artifacts
and the `contracts/` documents that describe the root-level layout as a
stable interface, matching the pattern `contracts/` already serves in specs
`001`/`002` (grammar/schema documents other specs build against).

## Complexity Tracking

*No entries — Constitution Check reported no violations requiring justification.*
