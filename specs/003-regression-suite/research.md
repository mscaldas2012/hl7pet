# Phase 0 Research: Shared Regression Suite (`fixtures/` Corpus)

All items below were genuinely open (no reasonable default existed in the spec) and
are resolved here so Phase 1 design has no outstanding unknowns.

## Decision 1: Language/tooling for the CI validation script

**Decision**: A single Python 3.11+ script (`fixtures/scripts/validate_corpus.py`)
using the `jsonschema` package, run directly by a GitHub Actions workflow — no new
framework, test runner, or package manifest introduced.

**Rationale**: No Rust workspace exists yet (`crates/core` doesn't land until spec
`005`, Phase 2), so writing the validator in Rust would mean standing up `Cargo.toml`
and a crate purely to check JSON files — premature for a Phase 1 documentation/data
gate. `jsonschema` is already the tool spec `001`'s own `quickstart.md` assumed for
schema-checking vectors (Scenario 3), so this introduces no new dependency family,
just formalizes an existing one into something CI actually runs.

**Alternatives considered**:
- Rust binary using `crates/core`'s eventual JSON tooling: rejected — `crates/core`
  doesn't exist yet; would invert Phase 1/Phase 2 ordering.
- Shell script + `ajv-cli` (Node): rejected — introduces a second language runtime
  (Node) into a repo that has neither Rust nor Node infrastructure yet, for no
  benefit over Python, which the prior spec already reached for.
- No script at all, JSON Schema validation done manually per PR: rejected — directly
  contradicts FR-005 ("MUST run automatically in CI") and User Story 2's entire
  premise (catch drift without a human manually cross-checking).

## Decision 2: How message/profile references survive consolidation

**Decision**: No path rewriting is needed. Inspection of the actual vector files
(`specs/001-path-grammar-spec/vectors/*.json`,
`specs/002-hierarchy-semantics/vectors/*.json`) confirms every `message_ref` and
`profile_ref` value (e.g. `"messages/baseline.hl7"`, `"profiles/basic-two-level.json"`)
is already relative to *that spec's own root directory* — which is exactly the shape
`fixtures/` uses (`fixtures/messages/`, `fixtures/profiles/`). Copying each vector
file into `fixtures/vectors/<family>/` verbatim, alongside `fixtures/messages/` and
`fixtures/profiles/` populated the same way, means every reference resolves
correctly with zero edits.

**Rationale**: Confirmed by direct inspection rather than assumed — this is stronger
grounds for FR-002's "MUST NOT alter content" than treating it as a constraint to
satisfy; here it's simply true by construction once the same relative folder names
are reused at the new root.

**Alternatives considered**:
- Rewrite every `message_ref`/`profile_ref` to an absolute-from-repo-root path (e.g.
  `"fixtures/messages/baseline.hl7"`): rejected — would touch every vector's content
  (violates the spirit of FR-002, even if not its letter), and provides no benefit
  since relative resolution already works.

## Decision 3: Cross-family id-uniqueness enforcement mechanism

**Decision**: The validation script loads every vector record from every file under
`fixtures/vectors/**/*.json` into one flat list before checking `id` uniqueness,
rather than checking uniqueness per-file or per-family. Spec `001`'s ids use a
`path-*`/descriptive-slug pattern (e.g. `path-msh12`, `invalid-seg-firstchar`) and
spec `002`'s use `hier-*` (e.g. `hier-001`); confirmed no collisions exist between
the two sets today.

**Rationale**: FR-003 explicitly requires corpus-wide uniqueness, not per-file — a
future spec adding a third vector family (e.g. spec `005`'s scanner vectors) could
otherwise collide silently with an existing id from a different family if only
checked in isolation.

**Alternatives considered**:
- Enforce a mandatory per-family id prefix at the schema level (e.g. regex requiring
  `^path-`/`^hier-`): rejected as unnecessary schema churn — both existing schemas
  are already spec `001`/`002` deliverables treated as stable; corpus-wide uniqueness
  is fully checkable by the validator without changing either schema.

## Decision 4: Coverage-report scope for vector families beyond `path`/`hierarchy`

**Decision**: The coverage report keys off two known dimensions today —
`grammar_productions` (spec `001`'s enum) and `semantic_rules` (spec `002`'s enum) —
and, per spec FR-007, treats any vector file under a `fixtures/vectors/<family>/`
directory other than `path`/`hierarchy` as an "unrecognized family" bucket: counted
and listed, but not matched against a coverage dimension, so the script doesn't need
to be modified every time a later spec (`005`-`009`) adds a new family.

**Rationale**: Directly implements FR-007 ("MUST treat... as its own coverage
dimension (reported, not rejected)") without requiring this spec to predict what
spec `005`'s scanner-vector coverage taxonomy will look like.

**Alternatives considered**:
- Hard-fail on any vector family the script doesn't recognize: rejected — directly
  contradicts FR-007 and would make this spec a blocker for every later vector-family
  addition, re-creating the exact per-spec-tooling-copy problem this spec exists to
  avoid.

## Decision 5: CI trigger scope

**Decision**: The GitHub Actions workflow (`fixtures-validation.yml`) triggers on
`pull_request` and `push` events where the diff touches `fixtures/**`, using GitHub's
built-in `paths:` filter — not on every push regardless of what changed.

**Rationale**: SC-002 requires the check to "never be the bottleneck" for unrelated
PRs; a `paths:` filter is the standard, zero-maintenance way to scope a check to the
directory it actually validates, consistent with this being the repo's first CI
workflow (no existing convention to follow or deviate from).

**Alternatives considered**:
- Run on every push/PR unconditionally: rejected — wastes CI minutes on changes
  (e.g. editing `SPEC.md`) that can't possibly affect corpus validity, and adds
  needless noise/latency to unrelated PRs.
