"""Fixtures-corpus parity check for `hl7pet_arrow.extract_value` (spec
6002-arrow-integration, SC-003, User Story 1 Acceptance Scenarios 1-3).

Runs every `fixtures/vectors/{path,hierarchy}/*.json` vector through
`extract_value` on a one-row `pa.array([message])` and confirms the row's
`status`/`value` matches what the existing plain `hl7pet.get_value`/
`get_value_hierarchy` return for that same vector -- an `Hl7ScanError`/
`Hl7QueryError` the plain binding would raise maps to this test asserting
`status == "error"` instead (FR-010's per-row signal, quickstart.md step 2),
never a raised exception, since `extract_value` never raises mid-batch.

Extended by test_scan_count.py/multi-PATH coverage in this same module for
Story 2 (T020) -- this file covers single-PATH (`extract_value`) only.

Deliberate, documented scope exclusions (mirrors
`crates/python/tests/parity_check.py`'s own "never conflate no-counterpart-
yet with wrong" convention -- `not_implemented`, pinned by id below, not
silently passed):

- `path-msh12`/`path-fieldexpr-subcomp` (`method: getFirstValue`):
  `extract_value`'s Result Struct shape mirrors `get_value` (list-of-lists),
  not `get_first_value` -- no columnar "first value" mechanism exists in
  this spec.
- `path-childpath-hierarchy`: a hierarchy-shaped PATH in the plain `path`
  vector family, which carries no `profile_ref` -- there is no profile to
  pass `extract_value` here even though it *can* execute hierarchy PATHs
  given one (unlike `get_value`, which rejects them outright).
- `hier-002` (`flags.buildHierarchy: false`): `extract_value` has no
  `build_hierarchy` toggle (contracts/arrow-api.md) -- out of scope, not a
  bug.
- `hier-009`/`hier-010` (multi-hop `->` chaining): `hl7pet_core::parse`
  already rejects a second `" -> "` hop, the same pre-existing limitation
  `crates/core/tests/hierarchy_vectors.rs` and
  `crates/python/tests/parity_check.py` both already document.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pyarrow as pa

import hl7pet
import hl7pet_arrow as ha

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"

KNOWN_NOT_IMPLEMENTED = {
    "path-msh12",
    "path-fieldexpr-subcomp",
    "path-childpath-hierarchy",
    "hier-002",
    "hier-009",
    "hier-010",
}


def _load_vectors(family: str) -> list[dict[str, Any]]:
    vectors: list[dict[str, Any]] = []
    for path in sorted((FIXTURES / "vectors" / family).glob("*.json")):
        vectors.extend(json.loads(path.read_text()))
    return sorted(vectors, key=lambda v: v["id"])


def _read_message(message_ref: str) -> str:
    return (FIXTURES / message_ref).read_text()


def _is_empty_expected(expected: Any) -> bool:
    return expected is None or expected == []


def _row(messages: pa.Array, path: str, profile: dict[str, Any] | None = None) -> dict[str, Any]:
    result = ha.extract_value(messages, path, profile)
    return result[0].as_py()


def _check_path_vector(vector: dict[str, Any]) -> tuple[str, Any]:
    path = vector["path"]
    expected = vector["expected"]
    method = vector["method"]

    if " -> " in path and expected != "INVALID":
        return "not_implemented", None
    if method != "getValue":
        return "not_implemented", None

    messages = pa.array([_read_message(vector["message_ref"])])

    if expected == "INVALID":
        try:
            ha.extract_value(messages, path)
        except hl7pet.Hl7PathError:
            return "match", "Hl7PathError (as expected)"
        except Exception as e:  # noqa: BLE001 - report exact mismatch
            return "mismatch", f"{type(e).__name__}: {e}"
        return "mismatch", "no exception raised"

    if expected == "ERROR:NonNumericComparison":
        row = _row(messages, path)
        if row["status"] == "error":
            return "match", row
        return "mismatch", row

    row = _row(messages, path)
    if _is_empty_expected(expected):
        ok = row["status"] == "no_match" and row["value"] is None
        return ("match" if ok else "mismatch"), row
    ok = row["status"] == "ok" and row["value"] == expected
    return ("match" if ok else "mismatch"), row


def _check_hierarchy_vector(vector: dict[str, Any]) -> tuple[str, Any]:
    path = vector["path"]
    method = vector.get("method")
    flags = vector.get("flags") or {}

    if path.count(" -> ") > 1:
        return "not_implemented", None
    if method != "getValue":
        return "not_implemented", None
    if flags.get("buildHierarchy", True) is False:
        return "not_implemented", None

    messages = pa.array([_read_message(vector["message_ref"])])
    profile = json.loads((FIXTURES / vector["profile_ref"]).read_text())
    expected = vector["expected"]

    row = _row(messages, path, profile)
    if _is_empty_expected(expected):
        ok = row["status"] == "no_match" and row["value"] is None
        return ("match" if ok else "mismatch"), row
    ok = row["status"] == "ok" and row["value"] == expected
    return ("match" if ok else "mismatch"), row


def run() -> dict[str, Any]:
    results: list[dict[str, Any]] = []
    for vector in _load_vectors("path"):
        status, actual = _check_path_vector(vector)
        results.append({"vector_id": vector["id"], "status": status, "actual": actual})
    for vector in _load_vectors("hierarchy"):
        status, actual = _check_hierarchy_vector(vector)
        results.append({"vector_id": vector["id"], "status": status, "actual": actual})
    return {"results": results}


def test_extract_value_matches_the_full_fixtures_corpus():
    report = run()

    mismatches = [r for r in report["results"] if r["status"] == "mismatch"]
    assert mismatches == [], mismatches

    not_implemented = {r["vector_id"] for r in report["results"] if r["status"] == "not_implemented"}
    assert not_implemented == KNOWN_NOT_IMPLEMENTED


def _eligible_path_vectors() -> list[dict[str, Any]]:
    """Every `path` family vector `_check_path_vector` would score `match`
    on via a plain (non-INVALID, non-ERROR) comparison -- the vectors
    `extract_values` can be meaningfully compared against `extract_value`
    for."""
    eligible = []
    for vector in _load_vectors("path"):
        path = vector["path"]
        expected = vector["expected"]
        if " -> " in path or vector["method"] != "getValue":
            continue
        if expected in ("INVALID", "ERROR:NonNumericComparison"):
            continue
        eligible.append(vector)
    return eligible


def test_extract_values_agrees_with_extract_value_for_paths_sharing_a_message():
    """Story 2 Acceptance Scenario 1: every outer field of an
    `extract_values` call matches what `extract_value` alone returns for
    that same path -- not just that multi-PATH extraction runs."""
    by_message: dict[str, list[dict[str, Any]]] = {}
    for vector in _eligible_path_vectors():
        by_message.setdefault(vector["message_ref"], []).append(vector)

    groups = [vectors for vectors in by_message.values() if len(vectors) >= 2]
    assert groups, "expected at least one message_ref shared by 2+ eligible vectors"

    for vectors in groups:
        message = _read_message(vectors[0]["message_ref"])
        messages = pa.array([message])
        paths = [v["path"] for v in vectors]

        multi = ha.extract_values(messages, paths)
        for i, vector in enumerate(vectors):
            single = _row(messages, vector["path"])
            combined = multi.field(i)[0].as_py()
            assert combined == single, (vector["id"], combined, single)
            # Also matches the vector's own recorded expectation directly.
            assert combined["value"] == (vector["expected"] or None)


def test_extract_values_rejects_empty_paths_list():
    """FR-008 / Story 2 Acceptance Scenario 3: an empty paths list is
    rejected up front, no row processed -- a plain ValueError, not an
    Hl7*Error (contracts/arrow-api.md, no hl7pet-core equivalent to
    mirror)."""
    message = _read_message("messages/baseline.hl7")
    try:
        ha.extract_values(pa.array([message]), [])
    except ValueError as e:
        assert "paths must not be empty" in str(e)
    else:
        raise AssertionError("expected ValueError for an empty paths list")


def test_extract_values_computes_duplicate_paths_independently():
    """spec.md Edge Cases: the same PATH string twice in `paths` produces
    two independent outer fields, both computed -- accessed positionally
    (`.field(i)`), since PyArrow's own `to_pylist()`/dict conversion
    refuses a StructArray with duplicate field names (contracts/
    arrow-api.md's documented access pattern)."""
    message = _read_message("messages/baseline.hl7")
    result = ha.extract_values(pa.array([message]), ["MSH-12", "MSH-12"])
    assert result.type.num_fields == 2
    first = result.field(0)[0].as_py()
    second = result.field(1)[0].as_py()
    assert first == second == {"value": [["2.5.1"]], "status": "ok"}
