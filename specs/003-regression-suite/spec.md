# Feature Specification: Shared Regression Suite (`fixtures/` Corpus)

**Feature Branch**: `003-regression-suite`

**Created**: 2026-07-24

**Status**: Draft

**Input**: User description: "Shared golden-message corpus (fixtures/) + expected outputs exported from the Scala baseline (Roadmap module 0-999 Rust Core, Migration Plan Phase 1)"

## User Scenarios & Testing *(mandatory)*

This is a documentation/data/tooling deliverable (Migration Plan Phase 1,
Roadmap module 0-999 "Rust Core", spec `003`), not a runtime feature of the
engine itself. Its "users" are the specs and test suites that consume the
corpus it produces.

### User Story 1 - Rust core implementer gets one canonical corpus to test against (Priority: P1)

An engineer implementing the Rust message scanner, PATH parser, or hierarchy
navigator (Roadmap specs `005`-`009`) needs a single, canonical, already
Scala-verified set of golden messages and expected results to write tests
against, instead of hunting through each prior spec's own directory for its
own copy of vectors.

**Why this priority**: This is the entire point of the spec — Constitution
Principle I (Path Contract Stability) and the Development Workflow section
both require a comprehensive regression suite before any Rust core
implementation work begins. Without one canonical location, "does the Rust
core agree with Scala" has no single place to check.

**Independent Test**: Point a new, empty test harness at `fixtures/` alone
(no access to `specs/001-path-grammar-spec/` or
`specs/002-hierarchy-semantics/`) and confirm every message, vector, and
profile needed to validate PATH grammar and hierarchy semantics is present
and loadable.

**Acceptance Scenarios**:

1. **Given** the `fixtures/` directory, **When** a test harness loads every
   vector file under it, **Then** every vector's `message_ref` resolves to a
   message file that exists under `fixtures/` (no broken references).
2. **Given** the `fixtures/` directory, **When** a reader compares its vector
   content against the vectors previously authored in
   `specs/001-path-grammar-spec/vectors/` and
   `specs/002-hierarchy-semantics/vectors/`, **Then** every PATH string and
   expected result is identical — consolidation MUST NOT alter
   already-Scala-verified data.

---

### User Story 2 - CI catches corpus drift automatically (Priority: P1)

A contributor adding a new vector to `fixtures/` (in this spec or a later
one, e.g. `005`-`009`) needs immediate, specific feedback if they violate the
vector schema, duplicate an existing vector id, or reference a message file
that doesn't exist — without another person having to manually cross-check
the whole corpus during review.

**Why this priority**: The corpus is only useful as a regression suite if it
stays internally consistent as it grows across many future specs. Catching
drift at PR time, not at Rust-implementation time, is what makes this a
Phase 1 gate deliverable rather than a one-off snapshot.

**Independent Test**: Introduce a deliberately broken vector (duplicate id,
schema violation, dangling `message_ref`) on a branch and confirm the CI
check fails with an error identifying the specific file and problem, before
any other test suite runs.

**Acceptance Scenarios**:

1. **Given** a new vector file added under `fixtures/vectors/`, **When** its
   `id` collides with an existing vector's `id` anywhere else in the corpus,
   **Then** the CI check fails and names both colliding file locations.
2. **Given** a change that touches `fixtures/**`, **When** the pull request
   is opened, **Then** the corpus validation check runs automatically as
   part of CI, with no manual step required.

---

### User Story 3 - Coverage gaps are visible without manual tallying (Priority: P2)

A contributor planning the next Rust core spec (e.g. `005`, the message
scanner) needs to know which PATH grammar productions and hierarchy-semantics
rules already have a covering conformance vector and which don't, so new
vectors can be targeted at real gaps instead of duplicating coverage.

**Why this priority**: Prevents the corpus from silently over-covering easy
cases while leaving edge cases (e.g. a specific Known Limitation) unverified,
which would only surface once Rust implementation work is already underway.

**Independent Test**: Run the coverage report against the corpus as
consolidated from specs `001` and `002` and confirm it lists 100% of
`001`'s grammar productions and 100% of `002`'s hierarchy-semantics rules as
covered, with zero gaps (since both source specs already required this before
being marked complete).

**Acceptance Scenarios**:

1. **Given** the coverage report, **When** it is generated against the
   current corpus, **Then** it lists, per grammar production and per
   hierarchy rule, at least one covering vector id.
2. **Given** a future spec adds a new vector family (e.g. scanner-level
   vectors in spec `005`) without registering its coverage dimension,
   **Then** the report flags the new vectors as present but uncategorized
   rather than silently ignoring them.

---

### Edge Cases

- What happens when a future spec (`005`-`009`) needs to add vectors that
  don't fit either the PATH-grammar or hierarchy-semantics coverage
  dimensions (e.g. scanner offset correctness)? The corpus MUST accept
  additive vector families under `fixtures/vectors/<family>/` without
  requiring changes to existing families, and the coverage report MUST treat
  unrecognized families as their own dimension rather than erroring.
- What happens when two vectors from different origin specs reference
  functionally identical messages saved under different file names? No
  automatic de-duplication is performed — this spec preserves the exact
  content it consolidates (see User Story 1's acceptance scenario 2);
  byte-for-byte duplicate messages MAY be flagged for manual cleanup in a
  later spec but MUST NOT block this one.
- What happens when a vector's expected result is later found to be wrong
  (e.g. a discrepancy resolved after this spec is complete)? Corrections are
  made in place under `fixtures/`; the corpus does not need its own versioning
  scheme beyond standard git history.
- What happens when a vector file conforms to neither `001`'s
  `conformance-vector.schema.json` nor `002`'s
  `hierarchy-conformance-vector.schema.json`? The CI validation check MUST
  reject it rather than silently skipping schema validation for unrecognized
  files.
- What happens when the corpus is queried by a language binding that doesn't
  exist yet (Python, Java — Roadmap module 6000-6999)? Out of scope for this
  spec; `fixtures/` only needs to be structured so those future consumers can
  read it the same way the Rust core tests will, per the Migration Plan's
  "single shared corpus" rationale.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: This spec MUST create a `fixtures/` directory at the repository
  root, structured as:
  - `fixtures/messages/` — every synthetic HL7 message file currently under
    `specs/001-path-grammar-spec/messages/` and
    `specs/002-hierarchy-semantics/messages/`, consolidated without content
    changes.
  - `fixtures/vectors/path/` — the contents of
    `specs/001-path-grammar-spec/vectors/` (`valid.json`, `invalid.json`).
  - `fixtures/vectors/hierarchy/` — the contents of
    `specs/002-hierarchy-semantics/vectors/` (`basic.json`, `complex.json`).
  - `fixtures/profiles/` — every profile JSON file currently under
    `specs/002-hierarchy-semantics/profiles/`.
  - `fixtures/schemas/` — a copy of both vector JSON schemas
    (`conformance-vector.schema.json`,
    `hierarchy-conformance-vector.schema.json`) so validation tooling has a
    single place to resolve them from.
- **FR-002**: Consolidation MUST NOT alter any PATH string, expected result,
  `message_ref` content, or other field value already authored and
  Scala-verified in specs `001`/`002` — only file location and, where
  necessary, relative-path references MAY change.
- **FR-003**: Every vector `id` MUST be unique across the entire `fixtures/`
  corpus, not merely within its own file or origin spec. This spec MUST
  confirm no collisions exist between the `001` (`path-*`) and `002`
  (`hier-*`) vector sets at consolidation time.
- **FR-004**: A CI-runnable validation check MUST be added that, for every
  vector file under `fixtures/vectors/`:
  1. Validates it against the matching JSON schema in `fixtures/schemas/`.
  2. Confirms every `message_ref` (or message-reference field, per FR-011 of
     spec `001`/`002`'s equivalents) resolves to an existing file under
     `fixtures/messages/`.
  3. Confirms every vector `id` is unique corpus-wide (FR-003).
- **FR-005**: The validation check MUST run automatically in CI on every push
  or pull request that touches `fixtures/**`, failing the build and
  reporting the specific file, vector id, and rule violated on any failure.
- **FR-006**: A coverage report MUST be produced (as part of, or alongside,
  the validation check) that lists, for every grammar production defined in
  `specs/001-path-grammar-spec/contracts/path-grammar.md` and every rule
  documented in
  `specs/002-hierarchy-semantics/contracts/hierarchy-semantics.md`, at least
  one corpus vector id that exercises it. Productions/rules with zero
  covering vectors MUST be listed as gaps.
- **FR-007**: The coverage report MUST treat any vector family added under
  `fixtures/vectors/<family>/` that isn't `path` or `hierarchy` as its own
  coverage dimension (reported, not rejected), so later specs (`005`-`009`)
  can extend the corpus without modifying this spec's tooling.
- **FR-008**: `fixtures/` becomes the canonical corpus going forward — this
  spec's own deliverables (any new vectors or messages it authors, if any)
  and every subsequent spec that adds regression coverage MUST write
  directly under `fixtures/`, not into a new `specs/NNN/vectors/` copy.
- **FR-009**: The original vector/message files under
  `specs/001-path-grammar-spec/` and `specs/002-hierarchy-semantics/` MUST be
  left in place unmodified, as the historical record of those completed
  specs' deliverables; this spec copies rather than moves them.
- **FR-010**: All content under `fixtures/` MUST continue to meet spec
  `001`'s FR-009 constraint: synthetic/fabricated test data only, no real or
  de-identified patient data, inherited unchanged by everything this spec
  consolidates.

### Key Entities

- **Fixture Corpus (`fixtures/`)**: The single canonical directory tree of
  golden messages, conformance vectors, profiles, and schemas, consumed
  identically by the Rust core test suite and, later, the Python and Java
  binding test suites.
- **Golden Message**: A synthetic HL7 message file under
  `fixtures/messages/`, potentially referenced by many vectors across
  multiple vector families.
- **Conformance Vector**: A record (PATH-grammar or hierarchy-semantics
  family, per specs `001`/`002`) tying a PATH expression and source message
  to an expected result, now stored under `fixtures/vectors/<family>/`.
- **Coverage Report**: A derived artifact listing, per grammar production or
  hierarchy rule, which vector id(s) exercise it, and flagging any with zero
  coverage.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of vectors currently under
  `specs/001-path-grammar-spec/vectors/` and
  `specs/002-hierarchy-semantics/vectors/` exist under `fixtures/` after
  consolidation, with identical `path`/expected-result content — zero
  vectors lost, dropped, or altered in meaning.
- **SC-002**: The CI validation check runs on 100% of pull requests that
  touch `fixtures/**` and completes in well under a typical CI job timeout
  (under 1 minute for the current corpus size), so it is never the
  bottleneck in a PR's checks.
- **SC-003**: A contributor who introduces a duplicate vector id, a schema
  violation, or a dangling message reference gets a failing, specific CI
  error (naming the file and problem) on their first push — zero cases
  requiring a human reviewer to manually cross-check the corpus by hand.
- **SC-004**: The coverage report shows zero gaps for the `path` and
  `hierarchy` vector families immediately after consolidation (both source
  specs already required 100% production/rule coverage before being marked
  complete in `ROADMAP.md`).

## Assumptions

- No Rust, Python, or Java implementation code exists yet at this point in
  the migration (Phase 1); "executable, CI-wired regression suite" in this
  spec's scope therefore means schema/reference/uniqueness validation and
  coverage reporting over the corpus itself, not running vectors against an
  actual engine. Specs `005`-`009` are what will add engine code that
  consumes this corpus and asserts `actual == expected` per vector.
- The "expected outputs exported from the Scala baseline" described in
  `ROADMAP.md` for this spec refers to the Scala-verified `expected` values
  already produced by specs `001` (SC-004) and `002` during their own vector
  authoring — this spec consolidates that already-verified data rather than
  re-running verification against the external Scala library.
- Vector JSON schemas remain as two distinct families (`path`,
  `hierarchy`) rather than being unified into one schema; unifying them is
  out of scope here since both source specs are already complete and their
  schemas are an established, working contract consumed by other specs.
- Choice of validation-check implementation language/tooling (e.g. a small
  Rust, Python, or shell script wired into CI) is an implementation detail
  left to this spec's `/speckit-plan`, not decided here, as long as it meets
  FR-004/FR-005/FR-006.
- `specs/001-path-grammar-spec/` and `specs/002-hierarchy-semantics/` are not
  deleted or redirected to `fixtures/` via symlink; their original files
  remain as-is per FR-009, accepting the resulting duplication between
  `specs/00{1,2}/.../vectors/` and `fixtures/vectors/` as the cost of keeping
  each completed spec's own deliverable directory intact.
