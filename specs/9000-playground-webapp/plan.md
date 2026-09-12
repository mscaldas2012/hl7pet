# Implementation Plan: HL7-PET Playground Web App

**Branch**: `9000-playground-webapp` | **Date**: 2026-09-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/9000-playground-webapp/spec.md`

## Summary

A small, self-contained local web app that lets a developer paste a raw HL7 v2
message, optionally upload a hierarchy profile, type a PATH expression, and see
every matching value — annotated with 1-based source line numbers for non-hierarchy
PATHs (hierarchy PATHs show values only, per the documented gap in FR-005a). The
technical approach is to build the thinnest possible layer over the already-shipped
`hl7pet` Python package (spec `6000`): a single-page vanilla HTML/CSS/JS frontend
talking to one JSON endpoint on a small Flask backend, which does nothing but
validate input shape, dispatch to the right `hl7pet` function, and translate its
typed exceptions into distinct, user-readable error states. No new Rust or Python
binding code is introduced — this spec is pure consumption of the existing public
surface.

## Technical Context

**Language/Version**: Python 3.11 for the backend (the existing `hl7pet` wheel
targets `abi3-py39`, so 3.9+ all work; 3.11 is the concrete dev/CI target for this
feature). No new Rust code.

**Primary Dependencies**: Flask (backend routing + templating), the local `hl7pet`
package (built from `crates/python` via `maturin develop`, per that crate's existing
README — not published to PyPI). Frontend: vanilla HTML/CSS/JS, no framework, no
build step.

**Storage**: N/A — fully stateless; nothing submitted is persisted (FR-013).

**Testing**: `pytest` + Flask's test client for backend route/error-mapping
behavior (one test per `hl7pet` exception type, one for the no-profile/hierarchy-PATH
case, one for the "no results" case). No UI test framework introduced; end-to-end UI
behavior is verified manually via `quickstart.md`.

**Target Platform**: Local developer machine only, run with `flask --app
playground.app run` and opened in any modern browser at `http://localhost:5000`. Not
containerized, not deployed, no production target.

**Project Type**: web (single backend + static frontend, no separate frontend
build/toolchain).

**Performance Goals**: Interactive devtool feel — a typical single message/single
PATH round trip completes in a couple of seconds (SC-005), dominated by the browser
request round trip, not `hl7pet-core`'s own (already benchmarked, spec `009`)
extraction cost.

**Constraints**: No server-side persistence (FR-013); pasted message + uploaded
profile capped at 5 MB combined per request (research.md #6) so an oversized paste
fails fast with a clear message instead of hanging the page; hierarchy PATH results
never include line numbers in this version (FR-005a, a documented limitation, not a
bug).

**Scale/Scope**: Single-user, single-machine local tool. No auth, no multi-tenancy,
no concurrency target beyond "doesn't block on one slow request."

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|---|---|
| I. Path Contract Stability | **Pass.** This feature is a pure consumer of the existing PATH grammar/evaluation semantics via the `hl7pet` package; it introduces no new PATH syntax or evaluation behavior. |
| II. Zero-Copy & Lazy Evaluation | **Pass (not applicable at this layer).** This principle governs `hl7pet-core`'s own implementation. The playground calls the existing `get_value_located`/`get_value_hierarchy` entry points as-is and does not reimplement or duplicate extraction/scanning logic. |
| III. Explicit, Exception-Free Data Absence | **Pass.** The web app's own JSON contract preserves this distinction one layer up: "no match" (`hl7pet` returns `None`) maps to an explicit `no_results` response state (FR-007), while each of `hl7pet`'s four typed exceptions maps to its own distinct `errorType` (research.md #4) — never conflated with "no data," and never a raw 500/crash. |
| IV. Multi-Language Interoperability | **Pass (not applicable).** This feature adds no new `hl7pet-core` capability, so no cross-binding parity work is triggered. It happens to consume the Python binding specifically because that's the binding available in this repo; that is an implementation choice, not a new capability. |
| V. Declarative Profiles & Documented Limitations | **Pass.** The app never hardcodes per-message-type or per-profile logic — it forwards whatever profile JSON the user supplies straight to `hl7pet`. The one known limitation this feature introduces (no line numbers for hierarchy results) is explicitly surfaced in the UI and recorded in spec.md's Assumptions (FR-005a), not silently mishandled. |

**Result**: No violations. Complexity Tracking is not needed.

## Project Structure

### Documentation (this feature)

```text
specs/9000-playground-webapp/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── playground-api.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

New top-level `playground/` directory, a sibling of `crates/`, `fixtures/`, and
`specs/` — it is tooling that consumes the workspace, not a Cargo workspace member
itself, so it stays outside `crates/`.

```text
playground/
├── README.md                  # how to install hl7pet locally + run this app
├── requirements.txt            # Flask (+ pytest for dev)
├── app.py                      # Flask app factory / entrypoint (`flask --app playground.app run`)
├── hl7_playground/
│   ├── __init__.py
│   ├── routes.py               # the single POST /api/extract view
│   └── extraction.py           # dispatch to hl7pet + exception -> errorType mapping (research.md #3, #4)
├── static/
│   ├── style.css
│   └── app.js                  # fetch() call to /api/extract, renders results/errors
├── templates/
│   └── index.html              # the single page: message textarea, profile file input, PATH input, results panel
└── tests/
    ├── test_extraction.py      # unit tests over hl7_playground/extraction.py's dispatch + mapping
    └── test_routes.py          # Flask test-client tests over the /api/extract contract
```

**Structure Decision**: A single small Flask package (`playground/`) at the repo
root, not nested under `crates/` (it's not a Rust workspace member and has no Cargo
manifest) and not split into separate `backend/`/`frontend/` projects (the frontend
is a handful of static files with no build step, so a `static/`+`templates/` split
inside one small app is simpler than a second project). This mirrors the "Option 1:
single project" shape in the planning template, adapted for a Python web app instead
of a CLI/library.
