# Feature Specification: Python Bindings & Core-Sync Tooling

**Feature Branch**: `6000-python-bindings-automation`

**Created**: 2026-09-06

**Status**: Draft

**Input**: User description: "Start porting hl7pet-core to Python. Since the Rust core will keep gaining new capabilities spec by spec, include as part of this plan the creation of scripts/tooling that automate the repetitive work of bringing the Python binding back to parity each time hl7pet-core changes, rather than requiring a full manual re-port every time."

## User Scenarios & Testing *(mandatory)*

This is a Language Bindings deliverable (Roadmap module 6000-6999, spec `6000`)
— the first spec in this module, and the first language binding built on top
of `hl7pet-core`. Its "users" are two distinct groups: (1) Python developers
who want to call HL7-PET's extraction capabilities from Python, and (2) the
maintainer(s) who will keep porting the binding forward every time a future
core spec (1002+, 2000+, etc.) adds or changes capability, per Constitution
Principle IV (Multi-Language Interoperability) and the migration plan's
Phase 5 (Language Bindings). The second group's recurring, repetitive work is
this spec's second half — the sync tooling — and is explicitly in scope, not
a follow-up.

### User Story 1 - Python developer extracts HL7 values with parity to the Rust core (Priority: P1)

A Python developer installs the hl7pet Python package and calls its
extraction API against an HL7 message, getting the same values (including
located extraction, hierarchy navigation, and decoded escape sequences) that
`hl7pet-core` already produces today, without touching Rust.

**Why this priority**: This is the feature's entire reason to exist — a
Python port with no working extraction capability is not a port. Every other
story in this spec exists to keep this one true over time.

**Independent Test**: Install the built package into a clean Python
environment and, for every vector in every family of the shared fixtures
corpus (`fixtures/vectors/{path,hierarchy,scanner,escapes}/`), call the
Python API with that vector's `path`/`message_ref` and confirm the returned
value(s) match the vector's `expected` output exactly.

**Acceptance Scenarios**:

1. **Given** an HL7 message and a PATH expression, **When** a Python caller
   requests a value, **Then** the returned value(s) match what
   `hl7pet-core`'s own extraction produces for the same message and PATH,
   including decoded escape sequences.
2. **Given** a PATH using the `->` hierarchy operator, **When** a Python
   caller requests a value, **Then** the result matches `hl7pet-core`'s
   hierarchy-navigated extraction exactly.
3. **Given** a PATH that matches nothing, **When** a Python caller requests a
   value, **Then** the call returns Python's idiomatic "no value" result
   (never raises), mirroring Constitution Principle III.
4. **Given** a malformed message that fails to scan, **When** a Python caller
   attempts extraction, **Then** the call raises, surfacing the structural
   failure distinctly from "no value found."

---

### User Story 2 - Maintainer identifies exactly what a new core spec changed for Python (Priority: P2)

After a future spec adds or changes `hl7pet-core`'s public surface (a new
function, a changed return shape, a documented breaking change), the
maintainer runs a script that reports precisely what changed since the
Python binding was last brought to parity — instead of manually re-reading
the entire core source to find what's new.

**Why this priority**: The roadmap plans hundreds of numbered specs across
several modules, each a candidate to touch `hl7pet-core`'s public surface. A
full manual re-audit of the whole core API on every single one of those specs
does not scale and is exactly the repetitive work this feature was asked to
automate; it is the second-most important capability after the port itself
existing at all.

**Independent Test**: Starting from a Python binding already synced to a
known `hl7pet-core` state, add one new `pub fn` to the core (simulating a
future spec) and run the sync script. Confirm its report lists exactly that
one addition — no false positives from unrelated, already-ported surface, no
false negatives.

**Acceptance Scenarios**:

1. **Given** a `hl7pet-core` state with one new public function since the
   last recorded sync baseline, **When** the sync script runs, **Then** its
   report lists that function as needing a Python counterpart and nothing
   else.
2. **Given** a core change already recorded as a Documented Breaking Change
   in `ROADMAP.md` (e.g., spec `1001`'s unconditional escape decoding),
   **When** the sync script runs, **Then** the report classifies it
   distinctly from a plain addition, flagging that it requires a version
   bump and migration-guide update on the Python side too (Constitution
   Principle I).
3. **Given** no changes to `hl7pet-core`'s public surface since the last sync,
   **When** the sync script runs, **Then** it reports the binding as already
   up to date, with no fabricated changes.
4. **Given** a Rust item never intended to cross the FFI boundary (e.g., an
   internal helper), **When** the sync script runs, **Then** that item is not
   flagged as pending Python work.

---

### User Story 3 - Maintainer verifies the Python binding still matches the Rust core after every change (Priority: P3)

After porting a flagged change (User Story 2) or making any other edit to the
Python binding, the maintainer runs a repeatable check that re-verifies the
binding's output against the full shared fixtures corpus, catching any
mismatch with the Rust core's behavior before it reaches a released package.

**Why this priority**: Automation that finds *what* changed (User Story 2)
does not guarantee the human-written Python-side change was implemented
correctly. This closes the loop, but only matters once there is a port (User
Story 1) and a way to know what to re-port (User Story 2), so it is
appropriately third.

**Independent Test**: Deliberately introduce one incorrect behavior into the
Python binding (e.g., an off-by-one in a single code path) and run the parity
check. Confirm it fails specifically on the vector(s) that exercise that code
path and passes on every other vector.

**Acceptance Scenarios**:

1. **Given** a Python binding whose output matches `hl7pet-core` for every
   fixture vector, **When** the parity check runs, **Then** it reports full
   parity with zero mismatches.
2. **Given** a Python binding with one incorrect behavior, **When** the
   parity check runs, **Then** it reports exactly the affected vector(s) as
   mismatches, identifying expected vs. actual output.
3. **Given** the parity check has just run, **When** it is run again with no
   changes to either the core or the binding, **Then** it produces the same
   result (deterministic, not flaky).

---

### Edge Cases

- What happens when a future core spec changes a function's return shape
  (e.g., a new field on an existing result type) rather than adding a wholly
  new function? The sync script MUST still detect and report it as a change
  needing Python-side attention, not just detect wholly-new items.
- What happens when a core capability's natural Rust representation (e.g., a
  zero-copy borrowed slice, Constitution Principle II) has no direct Python
  equivalent? The sync tooling flags that a human decision is needed; it does
  not silently guess a lossy or incorrect mapping.
- What happens if the maintainer never runs the sync script for several
  consecutive core specs? The next run MUST still report the full, correct
  set of accumulated changes since the last recorded baseline — not just the
  most recent one.
- What happens when the parity check is run before any Python port of a given
  vector family exists yet (e.g., mid-way through initial porting)? It MUST
  report those vectors as not-yet-implemented, distinctly from "implemented
  but wrong," so partial progress isn't misreported as failure.
- What happens when the same underlying core change affects multiple vector
  families at once (e.g., a scanner-level fix affecting `path`, `hierarchy`,
  and `escapes` vectors together)? The parity check must report each
  affected vector individually rather than masking later failures behind the
  first one found.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a Python-importable package exposing every
  extraction capability in `hl7pet-core`'s public surface as of this spec —
  scanning, PATH-based query execution, located extraction, hierarchy
  navigation, and escape-sequence decoding — with equivalent semantics, per
  Constitution Principle IV.
- **FR-002**: The Python package MUST be installable via a standard Python
  packaging mechanism (a pip-installable wheel), per the migration plan's
  stated Python deliverable.
- **FR-003**: The Python API's primary extraction methods MUST be one-call-
  per-field, named and shaped as counterparts to the current Scala API (e.g.
  `getValue`/`getFirstValue`-style), per the migration plan's Phase 5 API-
  shape decision, so callers migrating from the existing Scala library or
  Java binding don't need to restructure their calling code.
- **FR-004**: System MUST also provide a batched extraction entry point that
  accepts multiple PATH expressions in a single call and returns all
  requested results together, per the migration plan's Phase 5 decision to
  amortize per-call FFI overhead for callers with hot extraction loops.
- **FR-005**: Python-side "value not found" and "no value at this
  index/path" outcomes MUST be reported via Python's idiomatic absent-value
  mechanism (e.g., returning `None`), never by raising — mirroring
  Constitution Principle III. Structural failures (e.g., a message that
  fails to scan) MUST raise distinctly from an absent value.
- **FR-006**: The Python binding's output MUST be verified against the full
  shared fixtures corpus (`fixtures/vectors/{path,hierarchy,scanner,
  escapes}/`) with zero discrepancies from each vector's documented expected
  output, before the initial port is considered complete.
- **FR-007**: System MUST provide a script that, given `hl7pet-core`'s
  source at the last-recorded sync baseline and its current state,
  identifies every public-API-surface change relevant to the Python binding
  — added items, changed signatures or return shapes, and removed items.
- **FR-008**: The sync script's report MUST classify each identified change
  as either a Backward-Compatible Addition or a Documented Breaking Change,
  matching the classification convention already tracked in `ROADMAP.md`,
  so the porting work required (a parallel addition vs. a version-bumped
  replacement with a migration note) is clear without manually re-reading
  the roadmap's full history.
- **FR-009**: System MUST provide a repeatable parity check that runs the
  currently-shipped Python binding against the full shared fixtures corpus
  and reports any vector whose Python output diverges from the documented
  expected output, runnable on demand after any core or binding change (not
  only once, at initial port time).
- **FR-010**: The sync script MUST support explicitly excluding Rust items
  that are not intended to cross the FFI boundary, so internal-only surface
  is never flagged as pending Python work.
- **FR-011**: After each successful sync (all flagged changes ported and the
  parity check passing), the tooling MUST record the `hl7pet-core`
  version/commit the Python binding is now synced to, so the next sync run
  diffs from that point rather than re-flagging already-ported changes.
- **FR-012**: When the sync script identifies a change classified as a
  Documented Breaking Change, its report MUST call out that a Python-side
  version bump and migration-guide update are required, per Constitution
  Principle I.
- **FR-013**: New Python-binding capability MUST ship with an example added
  to the binding's own API reference/quickstart documentation, per the
  Constitution's Development Workflow documentation obligation.

### Key Entities

- **Python Binding Package**: The distributable artifact exposing
  `hl7pet-core`'s capabilities to Python callers with equivalent semantics;
  the thing User Story 1 delivers and User Stories 2-3 keep current.
- **Sync Baseline**: The recorded `hl7pet-core` version/commit the Python
  binding was last brought to full parity with; the reference point every
  sync script run diffs against.
- **Surface Change Report**: The sync script's output — every public
  `hl7pet-core` item added, changed, or removed since the Sync Baseline,
  each classified as a Backward-Compatible Addition or a Documented Breaking
  Change.
- **Parity Report**: The output of running the Python binding against the
  shared fixtures corpus — every vector, and whether its Python output
  matched, mismatched, or is not yet implemented.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A Python developer can install the package and reproduce the
  documented expected value for 100% of vectors across every family in the
  shared fixtures corpus (`path`, `hierarchy`, `scanner`, `escapes`).
- **SC-002**: When a future core spec changes `hl7pet-core`'s public surface,
  the maintainer can identify every item needing a Python-side counterpart
  by reading only the sync script's report — with no need to manually
  re-review the rest of the core source that didn't change.
- **SC-003**: Across the next three specs that change `hl7pet-core`'s public
  surface after this one ships, the sync script correctly reports 100% of
  the actual changes relevant to Python, with zero missed items and zero
  items flagged that were never actually part of the public surface.
- **SC-004**: Running the parity check after a deliberately introduced
  defect in the Python binding catches the defect every time — zero
  undetected regressions across repeated trials.
- **SC-005**: Re-running the sync script or the parity check with no
  underlying change produces identical results each time — no flaky or
  order-dependent output.

## Assumptions

- The initial port's scope is `hl7pet-core`'s full public surface as of this
  spec — scanning (`005`), PATH parsing (`006`), query execution (`007`),
  lazy hierarchy navigation (`008`), located extraction (`1000`), and
  escape-sequence decoding (`1001`) — establishing complete parity before any
  later core spec lands, rather than porting an interim subset first.
- Per the migration plan's already-decided architecture (`HL7-PET-Rust-
  Migration-Plan.md`), the Python binding wraps `hl7pet-core` via PyO3 and is
  packaged for pip installation via maturin. This spec does not reopen that
  architectural choice; it scopes the capability delivered and the tooling
  that keeps it current.
- Apache Arrow output (migration plan Phase 4) is out of scope for this
  spec. The Python binding returns plain Python types (`str`, `None`,
  `list`), not Arrow arrays/tables; a future spec may add Arrow-based return
  types once Phase 4 lands.
- The Java binding (also tracked under Roadmap module 6000-6999) is out of
  scope for this spec, which addresses only the Python side of Language
  Bindings. A future spec in this same module will bring Java to parity
  using whatever pattern this spec establishes, if applicable.
- "Automated" sync tooling means scripts a maintainer runs on demand (e.g.,
  once a new core spec lands) to surface what changed and to verify parity —
  not an unattended process that ports and merges Python-side changes
  without human review. The scripts remove the need to manually diff the
  entire core source by hand each time; a human still decides how to
  implement each flagged change.
- The Sync Baseline is tracked per completed Python-binding sync (i.e., the
  last point full parity was confirmed), not per individual `hl7pet-core`
  commit — intermediate, not-yet-synced-to commits are not separate
  baselines.
