# Contract: `hl7pet` Python API

The public surface of the `hl7pet` Python package (spec `6000`). Every
function here is a thin PyO3 wrapper around the matching `hl7pet-core`
entry point (`crates/python/src/lib.rs`) — see data-model.md for exact
return/exception shapes and research.md #3-4 for the rationale.

Naming is `snake_case` (PEP 8), matching the parenthesized Scala names for
migration familiarity per FR-003 / the migration plan's Phase 5 API-shape
decision.

```python
import hl7pet

# --- One-call-per-field API (FR-003) ---

def get_value(message: str, path: str) -> list[list[str]] | None:
    """Scala getValue(msg, path) counterpart. Non-hierarchy PATHs only —
    a hierarchy PATH (the '->' operator) raises Hl7PathError, since this
    entry point has no profile to navigate with; use get_value_hierarchy
    for that case.
    Returns None when nothing matches. Never raises for "no match" —
    only for a structural failure (see Exceptions below).
    """

def get_value_hierarchy(
    message: str, path: str, profile: dict, build_hierarchy: bool = True
) -> list[list[str]] | None:
    """Hierarchy counterpart (spec 008's execute_hierarchy). `profile` is
    the same segmentDefinition JSON shape hl7pet-core's HierarchyProfile
    already parses (fixtures/profiles/*.json), passed as a Python dict.
    Raises Hl7ProfileError if `profile` doesn't parse as a valid profile.
    `build_hierarchy=False` mirrors the Scala engine's static-mode toggle
    (fixtures `flags.buildHierarchy`) — `profile` is not consulted at all
    and the call always returns None, matching hl7pet-core's own
    execute_hierarchy(scan, path, profile=None) behavior.
    """

def get_first_value(message: str, path: str) -> str | None:
    """Scala getFirstValue(msg, path) counterpart. None when no match.
    Non-hierarchy PATHs only, same restriction as get_value."""

def get_value_located(
    message: str, path: str
) -> list[list[LocatedValue]] | None:
    """Spec 1000 execute_located counterpart. Non-hierarchy PATHs only —
    a hierarchy PATH raises Hl7PathError (same non-hierarchy-only scope
    hl7pet-core itself enforces, spec 1000 FR-009).
    """

def get_first_value_located(message: str, path: str) -> "LocatedValue | None":
    """Spec 1000 first_located counterpart."""

# --- Batched API (FR-004) ---

def get_values(
    message: str, paths: list[str]
) -> list[list[list[str]] | None]:
    """One scan of `message`, then one execute() per path in `paths`, in
    order. A path that fails to parse, or is a hierarchy PATH, raises
    Hl7PathError immediately (aborts the whole batch) — same error
    contract as get_value, not a per-element error marker (research.md #4).
    """

# --- Types ---

class LocatedValue:
    value: str
    line: int  # 1-based

# --- Exceptions (data-model.md) ---

class Hl7PetError(Exception): ...
class Hl7ScanError(Hl7PetError): ...
class Hl7PathError(Hl7PetError): ...
class Hl7QueryError(Hl7PetError): ...
class Hl7ProfileError(Hl7PetError): ...
```

## Behavioral contract (verified by `parity_check.py`, FR-006/FR-009)

1. For every vector in `fixtures/vectors/path/valid.json` and
   `fixtures/vectors/escapes/*.json`: calling the method named by the
   vector's `method` field (`getValue` → `get_value`, `getFirstValue` →
   `get_first_value`) with the vector's `path` against the message at
   `message_ref` returns exactly the vector's `expected` value.
2. For every vector in `fixtures/vectors/path/invalid.json`: calling
   `get_value`/`get_first_value` with the vector's `path` raises
   `Hl7PathError` (never returns `"INVALID"` as a string — that literal is
   the fixtures schema's own placeholder for the Rust-side conformance
   table, not a Python return value).
3. For every vector in `fixtures/vectors/hierarchy/*.json`: calling
   `get_value_hierarchy` with the vector's `path`, the message at
   `message_ref`, the profile at `profile_ref`, and
   `build_hierarchy=vector.flags.buildHierarchy` (default `True` when the
   vector has no such flag) returns exactly the vector's `expected` value,
   including `expected: null` cases (`flags.buildHierarchy: false`'s
   static-mode-fallback vectors). A two-hop (multi-`->`) vector is out of
   scope here, same as `crates/core/tests/hierarchy_vectors.rs`'s own
   documented skip — multi-hop chaining is deferred to a future spec.
4. For every vector in `fixtures/vectors/scanner/*.json`: the underlying
   scan step (exercised indirectly through any `get_*` call against that
   vector's message) raises `Hl7ScanError` exactly when the vector's own
   expected outcome is a scan failure, and succeeds otherwise.
5. `get_value_located`/`get_first_value_located`, applied to every
   `path`-family vector that also carries `expected_lines`, reproduce
   `expected` in `.value` and `expected_lines` in `.line`, matching shape
   for shape.
