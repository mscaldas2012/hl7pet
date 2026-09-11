# Phase 1 Data Model: Python Bindings & Core-Sync Tooling

Source: spec.md Key Entities, resolved against research.md decisions 3, 5,
6, 8. This feature has no runtime database — every entity below is either a
Python-side value returned across the FFI boundary, or a small
version-controlled JSON file the maintainer tooling reads/writes.

## Python Binding Package (runtime, in-memory)

The compiled `hl7pet` package's public value/exception types, as seen from
Python. No persistent state; every value is produced fresh per call from
`hl7pet-core`'s `ScanResult`/`CompiledPath`.

### Return shapes

| Python type | Mirrors (Rust) | Shape |
|---|---|---|
| `str \| None` | `first_located`/Scala `getFirstValue` | `None` when no match |
| `list[list[str]] \| None` | `execute`/`execute_hierarchy`/Scala `getValue` | outer = matched segment occurrences (message order), inner = field repetitions; `None` when the outer list would be empty (Python-idiomatic absence, FR-005 — not an empty list, so "no match" and "matched but yielded no field values" stay distinguishable the way `hl7pet-core`'s own `Vec::is_empty()` collapse rule already treats them, per `query.rs`'s doc comments) |
| `LocatedValue` (a small `@dataclass`-like PyO3 class) | `hl7pet_core::LocatedValue` | `.value: str`, `.line: int` (1-based) |
| `list[list[LocatedValue]] \| None` | `execute_located` | same outer/inner shape as `get_value`, elements are `LocatedValue` |
| `list[list[str]] \| LocatedValue-shape \| None` per element | `get_values` batched (FR-004) | one entry per input path, same per-path shape as the corresponding single-call method |

### Exceptions (research.md #3)

| Class | Base | Raised for |
|---|---|---|
| `Hl7PetError` | `Exception` | root; never raised directly |
| `Hl7ScanError` | `Hl7PetError` | message fails to scan |
| `Hl7PathError` | `Hl7PetError` | PATH expression fails to parse |
| `Hl7QueryError` | `Hl7PetError` | non-numeric operand on an ordering filter |
| `Hl7ProfileError` | `Hl7PetError` | invalid hierarchy `segmentDefinition` profile JSON |

**Validation rule (FR-005)**: absence of data is never one of the above —
only these four structural-precondition cases raise; every other outcome is
a value or `None`.

## Sync Baseline (persistent, `crates/xtask/surface-baseline.json`)

The recorded `hl7pet-core` state the Python binding was last brought to full
parity with (spec Key Entities; research.md #6).

```json
{
  "commit": "<git sha>",
  "captured_at": "<ISO-8601 timestamp, informational only>",
  "surface": {
    "<module path, e.g. \"scanner\">": {
      "functions": { "<name>": "<normalized signature string>" },
      "structs": { "<name>": { "fields": { "<field>": "<type string>" } } },
      "enums": { "<name>": { "variants": ["<variant>", ...] } },
      "types": { "<name>": "<aliased type string>" }
    }
  }
}
```

**Validation rules**:
- `commit` MUST be a valid full-length git SHA present in this repo's history
  (written by `xtask sync-baseline`, which reads `git rev-parse HEAD`).
- `surface` MUST contain every `pub` item under `crates/core/src/**/*.rs`
  that is not excluded per FR-010's marker convention (research.md #5) —
  `xtask surface-diff` fails loudly (non-zero exit) rather than silently
  producing a partial snapshot if a source file fails to parse.
- Exactly one `surface-baseline.json` exists; there is no per-spec or
  per-commit historical copy (research.md #6 — git history is the record).

## Surface Change Report (`xtask surface-diff` output)

Every public `hl7pet-core` item added, changed, or removed since the Sync
Baseline, each classified per `ROADMAP.md`'s existing convention (spec Key
Entities; FR-007/FR-008).

```json
{
  "baseline_commit": "<git sha the diff started from>",
  "current_commit": "<git sha diffed against, usually HEAD>",
  "up_to_date": false,
  "changes": [
    {
      "kind": "added | changed | removed",
      "classification": "backward_compatible_addition | documented_breaking_change",
      "item_path": "<module::name, e.g. \"query::execute_located\">",
      "before": "<normalized signature string, or null for 'added'>",
      "after": "<normalized signature string, or null for 'removed'>",
      "requires_version_bump": false,
      "requires_migration_note": false
    }
  ]
}
```

**Validation / derivation rules**:
- `kind: "added"` is always `classification: "backward_compatible_addition"`
  (a wholly new item cannot break an existing caller) — FR-008.
- `kind: "removed"` or `kind: "changed"` is always `classification:
  "documented_breaking_change"`, with `requires_version_bump` /
  `requires_migration_note` both `true` — FR-012. This matches
  `ROADMAP.md`'s own Backward-Compatible-Additions convention exactly: a
  new capability is *always* added as a new item alongside the existing
  ones, never by changing an existing item's signature or return shape
  (e.g. spec `1001`'s `&str` → `Cow<str>` return-type change was accepted
  only as an explicit, documented exception to that rule) — there is no
  partial/superset middle ground in this project's convention, so the
  classifier does not attempt one.
- `changes` is `[]` and `up_to_date` is `true` when the current surface
  equals the baseline's exactly (Edge Case: "no changes since last sync" —
  FR spec Acceptance Scenario 3) — the report is still emitted, never
  omitted, so "ran and found nothing" stays distinguishable from "didn't
  run."
- An item excluded per FR-010's internal-marker convention never appears in
  `changes`, regardless of `kind`.

## Parity Report (`parity_check.py` output)

The result of running the Python binding against the shared fixtures corpus
(spec Key Entities; FR-009; research.md #7-8).

```json
{
  "run_at": "<ISO-8601 timestamp>",
  "binding_commit": "<git sha of the working tree the wheel was built from>",
  "totals": { "match": 0, "mismatch": 0, "not_implemented": 0 },
  "results": [
    {
      "vector_id": "<id from the fixtures vector, e.g. \"path-msh12\">",
      "family": "path | hierarchy | scanner | escapes",
      "status": "match | mismatch | not_implemented",
      "expected": "<the vector's own expected value, echoed>",
      "actual": "<Python call's actual return, or null for not_implemented>"
    }
  ]
}
```

**Validation / derivation rules**:
- Every vector under `fixtures/vectors/{path,hierarchy,scanner,escapes}/`
  produces exactly one `results` entry — none skipped, none duplicated
  (Edge Case: a scanner-level fix touching multiple families must surface
  each affected vector individually, never masked behind the first).
- `status: "not_implemented"` only for `AttributeError`/`NotImplementedError`
  from the binding itself (the entry point doesn't exist yet); any other
  exception during a call that the vector didn't expect is `"mismatch"`
  with `actual` describing the raised exception's type/message.
- Deterministic (SC-005): given the same binding build and the same fixtures
  corpus, two runs produce byte-identical `results` (order = vector
  iteration order = sorted `(family, vector_id)`, no wall-clock-dependent
  content besides the informational `run_at` field).
