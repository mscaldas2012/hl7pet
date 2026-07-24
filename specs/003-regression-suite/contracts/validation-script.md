# Contract: `fixtures/scripts/validate_corpus.py`

CLI contract for the validation tool CI (`fixtures-validation.yml`) and
contributors run locally. This is the concrete implementation of spec FR-004,
FR-005, and FR-006.

## Invocation

```bash
python3 fixtures/scripts/validate_corpus.py [--corpus-root fixtures] [--json]
```

- `--corpus-root` (optional): defaults to `fixtures/` relative to the repo root
  (auto-detected from the script's own location if run from elsewhere). Overridable
  for testing the script against a scratch corpus.
- `--json` (optional): emit the Coverage Report (`data-model.md`) as JSON on stdout
  instead of the default human-readable summary. CI uses the default (human-readable)
  form for job logs; `--json` is for future tooling (e.g. a badge generator) that
  isn't part of this spec's scope.

## Checks performed, in order

1. **Schema conformance** (FR-004.1): every record in every file under
   `fixtures/vectors/<family>/*.json` validates against
   `fixtures/schemas/<family>.schema.json` (Draft 2020-12, via `jsonschema`). A
   family directory with no matching schema file is itself an error, not a silent
   skip.
2. **Reference resolution** (FR-004.2): every `message_ref` resolves to an existing
   file under `fixtures/messages/`; every `profile_ref` (when present) resolves
   under `fixtures/profiles/`.
3. **Corpus-wide id uniqueness** (FR-004.3): every vector `id`, across every family
   and file, is unique. On collision, both file locations are reported (per spec
   User Story 2 Acceptance Scenario 1).
4. **Coverage report** (FR-006/FR-007): for the `path` family, every enum value of
   `grammar_productions` in `conformance-vector.schema.json` has ≥1 covering vector.
   For the `hierarchy` family, every enum value of `semantic_rules` in
   `hierarchy-conformance-vector.schema.json` has ≥1 covering vector. Any other
   family directory is counted and listed as "unrecognized" (informational only).

## Exit codes

| Code | Meaning |
|---|---|
| `0` | All checks (1)-(3) passed, and (4) reports zero gaps for `path`/`hierarchy`. |
| `1` | At least one check (1), (2), or (3) failed — corpus is structurally broken. |
| `2` | Checks (1)-(3) passed but (4) reports ≥1 coverage gap for `path` or `hierarchy` (SC-004). |

CI (`fixtures-validation.yml`) treats any non-zero exit code as a failed check.

## Output (human-readable mode, default)

```text
fixtures/ validation
  schema:      27/27 vectors valid
  references:  27/27 message_ref/profile_ref resolved
  uniqueness:  27/27 ids unique
  coverage:
    path       12/12 grammar_productions covered
    hierarchy  10/10 semantic_rules covered
    (no unrecognized vector families)
OK
```

On failure, each violation is printed as one line:
`<file>: <id or "?">: <problem>` — e.g.
`fixtures/vectors/path/valid.json: path-msh12: message_ref "messages/typo.hl7" not found`.

## Non-goals

- Does not execute any vector against a real parser/engine — there is none yet
  (Phase 1). It validates the corpus's own internal consistency only.
- Does not check HL7 message *content* validity (e.g. well-formed segments) beyond
  what's needed to resolve `message_ref` to an existing file — that's out of this
  spec's scope.
