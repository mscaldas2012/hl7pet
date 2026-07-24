# Implementation Plan: Hierarchy-Mode Semantics Specification

**Branch**: `002-hierarchy-semantics` | **Date**: 2026-07-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-hierarchy-semantics/spec.md`

## Summary

Produce the formal hierarchy-mode semantics document (parent-occurrence scoping,
`segmentDefinition`-driven containment, single-hop `->` evaluation) and a hierarchy
conformance vector suite, extending spec `001`'s grammar and vector schema exactly as
spec `001` anticipated. This is a documentation/data deliverable — no Rust, Python, or
Java code is written by this feature. Unlike spec `001`, this plan also resolves two
decisions spec.md deliberately left open (FR-004: profile requirement; FR-005:
multi-level chaining), using direct inspection of the real Scala source
(`HL7HierarchyParser.scala`, `HL7ParseUtils.scala`) already available in a local
checkout at `/Users/m/projects/personal/HL7-PET`, rather than `SPEC.md`'s prose alone
— which turns out to under-specify several real behaviors (see `research.md`).

## Technical Context

**Language/Version**: N/A — this feature produces Markdown + JSON artifacts, no source
code. (The semantics it documents will later be implemented in Rust by spec `008`.)

**Primary Dependencies**: None required. A JSON Schema validator (e.g. `ajv`, Python
`jsonschema`) MAY be used to lint conformance vector files against
`contracts/hierarchy-conformance-vector.schema.json`, matching spec `001`'s approach.

**Storage**: N/A — flat files committed under this feature's own directory (FR-013).

**Testing**: Manual verification (SC-004) — each conformance vector's profile, message,
and PATH string are run against the real Scala library and the actual output compared
to the vector's `expected` field. A local checkout of the parity-target repo already
exists at `/Users/m/projects/personal/HL7-PET` (outside this repo, not a submodule,
consistent with `ROADMAP.md`'s "no hard dependency" convention) and was used directly
for this plan's research; the same checkout can be used for vector verification instead
of the fresh scratch clone spec `001`'s `research.md` describes, if still present when
`/speckit-tasks` executes.

**Target Platform**: N/A (documentation/data deliverable).

**Project Type**: Single feature directory, documentation/data artifacts only (no
`src/`, no application code).

**Performance Goals**: N/A directly — no code executes in this feature. However, FR-005
requires this feature's *output* (the multi-level navigation decision) to carry a
specific, falsifiable performance claim, since Constitution Principle II (zero-copy,
lazy hierarchy) and the feature request ("performance is always top priority") both
gate that decision. See `research.md` Decision 4.

**Constraints**: All conformance-vector source messages and profiles MUST be synthetic
(FR-012, no real PHI). Conformance vectors MUST conform to
`contracts/hierarchy-conformance-vector.schema.json`. No hard dependency on the external
Scala repo is committed into this repo.

**Scale/Scope**: One semantics document; ≥8 conformance vectors per spec SC-003 (one per
documented semantic rule: parent-scoping, zero-children case, missing-required-child
case, cross-parent child-indexing behavior (research.md Decision 3), static-mode `->`
fallback, plus multi-level vectors if FR-005's decision is "include").

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|---|---|
| I. Path Contract Stability (NON-NEGOTIABLE) | **Satisfied, with one flagged decision.** This document documents existing single-hop `->` semantics as the compatibility floor. The candidate multi-level extension (FR-005) is designed as a strict grammar *addition* to `CHILD_PATH` (spec `001`) — existing single-hop paths remain valid and keep their current meaning — so it does not itself require a MAJOR bump. It only becomes a Documented Breaking Change if a future spec changes single-hop behavior, which is explicitly out of scope here. |
| II. Zero-Copy & Lazy Evaluation | **Directly shapes FR-004 and FR-005.** The current Scala engine builds a full in-memory tree eagerly at construction (`HL7HierarchyParser.parseMessageHierarchy`), which is exactly what Migration Plan Phase 3 says the Rust core should *not* do ("contextual navigation without building a full tree"). `research.md` Decision 2 recommends the Rust core preserve profile-driven containment semantics without eager full-tree materialization. |
| III. Explicit, Exception-Free Data Absence | **Reinforced** — FR-001's "zero results for an occurrence with no matching children" and FR-008's static-mode fallback are both no-exception, empty-result outcomes, consistent with the real Scala behavior traced in `research.md` (no exception is thrown anywhere in `getChildrenValues`/`recursiveAction` for absent children). |
| IV. Multi-Language Interoperability | **Reinforced** — the semantics document and vectors are language-agnostic (Markdown prose, JSON Schema), consumed identically by future Rust/Python/Java work, matching spec `001`'s approach. |
| V. Conformance Through Declarative Profiles & Documented Limitations | **Directly shapes FR-004.** Weighed directly against Principle II in `research.md` Decision 2. Also: a previously undocumented real behavior — a numeric child index is compared against a position in the *combined, still mixed-segment-type* list of children across all selected parent occurrences, with no `-1` adjustment, and appears untested (`research.md` Decision 3) — is now made an explicit Documented Limitation rather than left for someone to discover by surprise. |
| Performance & Portability Standards | **Directly invoked by FR-005/SC-005** — the multi-level navigation decision must carry a benchmarkable claim, not an unquantified assertion, per this section's "every performance-sensitive change... MUST be benchmarked" standard (benchmarking itself happens in spec `009`, since no Rust core exists yet; this spec only commits to the claim being falsifiable then). |
| Development Workflow — Phased Migration Discipline | **This feature is the named Phase 1 gate deliverable** for hierarchy semantics ("Document hierarchy semantics (`OBR[1]->OBX-3`)" per the Migration Plan, and the constitution's explicit Phase 1 completion requirement). |

No violations at Phase 0. **Post-Phase-1 re-evaluation**: design work in
`contracts/hierarchy-semantics.md` surfaced a behavior that is arguably a defect in the
current Scala engine rather than an intentional design choice — a numeric child index
(e.g. `OBX[2]`) is compared against a 0-based position in the *combined, still
mixed-segment-type* list of children across every matched parent occurrence, with no
`-1` adjustment for the 1-based convention used everywhere else in the engine, and
appears to be untested (`research.md` Decision 3). This plan does **not** propose
fixing it (out of scope: this spec documents current behavior as the compatibility
floor per Principle I), but flags it explicitly as a Documented Limitation carried into
`contracts/hierarchy-semantics.md` Section A.4 rather than silently reproduced without
comment. No Complexity Tracking entry is needed — this is a documented-as-is
limitation, not new complexity this feature adds.

## Project Structure

### Documentation (this feature)

```text
specs/002-hierarchy-semantics/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md         # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

This feature produces no application source code. Its actual deliverables (semantics
document, conformance vectors, synthetic messages and profiles) live entirely under its
own feature directory, per FR-013, mirroring spec `001`'s layout and extending its
conformance-vector concept rather than duplicating it:

```text
specs/002-hierarchy-semantics/
├── contracts/
│   ├── hierarchy-semantics.md                       # Formal semantics document
│   │                                                 # (FR-001–FR-005, FR-008, FR-009)
│   └── hierarchy-conformance-vector.schema.json      # JSON Schema extending spec 001's
│                                                      # conformance-vector.schema.json
├── vectors/
│   └── *.json                                        # Conformance vector files
│                                                      # (FR-006, FR-007), authored during
│                                                      # /speckit-tasks + /speckit-implement,
│                                                      # not this plan
├── messages/
│   └── *.hl7                                         # Synthetic source HL7 messages
│                                                      # (FR-012), authored later
├── profiles/
│   └── *.json                                        # Synthetic segmentDefinition
│                                                      # profiles used by vectors, new
│                                                      # relative to spec 001 (which
│                                                      # needed no profile), authored later
├── data-model.md
├── quickstart.md
└── research.md
```

**Structure Decision**: Everything stays inside `specs/002-hierarchy-semantics/` — no
`src/`, `lib/`, or `tests/` tree, no code. `profiles/` is a new subdirectory (not present
in spec `001`, which needed no profile) added here because hierarchy vectors need a
`segmentDefinition` to evaluate against, in addition to `vectors/` and `messages/`
(carried over from spec `001`'s layout). `contracts/` holds the two artifacts other
specs treat as a stable interface: the semantics document itself (consumed by spec
`008`) and the vector schema (consumed by spec `003`). Per Phase 0/1 scope, this plan
authors `contracts/` and the four other Phase-1 documents now; `vectors/`, `messages/`,
and `profiles/` are populated by `/speckit-tasks` + `/speckit-implement`, exactly as
spec `001`'s were.

## Complexity Tracking

*No entries — Constitution Check reported no violations requiring justification.*
