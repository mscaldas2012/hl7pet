"""Fixtures-corpus parity check for `hl7pet_arrow.extract_value_located`
(mirrors test_parity.py's structure/scope notes exactly, for the located
variant) -- confirms each matched occurrence's `line` matches what the
existing plain `hl7pet.get_value_located`/`get_value_hierarchy_located`
already report (specs `1000`/`011`), not just that a line number is
present.

Same six documented, pinned-by-id `not_implemented` exclusions as
test_parity.py (see that file's module docstring for why): the located
mechanism has the identical scope boundary, since it's the same PATH/
profile handling, just a different per-occurrence payload.
"""

from __future__ import annotations

import json
from typing import Any

import pyarrow as pa

import hl7pet
import hl7pet_arrow as ha
from test_parity import FIXTURES, KNOWN_NOT_IMPLEMENTED, _load_vectors, _read_message


def _located_row(messages: pa.Array, path: str, profile: dict[str, Any] | None = None) -> dict[str, Any]:
    result = ha.extract_value_located(messages, path, profile)
    return result[0].as_py()


def _expected_occurrences(plain_located: list[list[Any]]) -> list[dict[str, Any]]:
    """Converts the plain binding's `list[list[LocatedValue]]` into this
    crate's `{value: [...], line: N}` per-occurrence shape, taking the
    first repetition's line as the occurrence's line -- the same fold
    `convert::located_row_outcome` performs, relying on the same
    same-occurrence-shares-one-line guarantee `crates/core/src/query.rs`
    already tests directly."""
    return [
        {"value": [lv.value for lv in occurrence], "line": occurrence[0].line}
        for occurrence in plain_located
    ]


def _check_path_vector_located(vector: dict[str, Any]) -> tuple[str, Any]:
    path = vector["path"]
    expected = vector["expected"]
    method = vector["method"]

    if " -> " in path and expected != "INVALID":
        return "not_implemented", None
    if method != "getValue":
        return "not_implemented", None

    message = _read_message(vector["message_ref"])
    messages = pa.array([message])

    if expected == "INVALID":
        try:
            ha.extract_value_located(messages, path)
        except hl7pet.Hl7PathError:
            return "match", "Hl7PathError (as expected)"
        except Exception as e:  # noqa: BLE001
            return "mismatch", f"{type(e).__name__}: {e}"
        return "mismatch", "no exception raised"

    if expected == "ERROR:NonNumericComparison":
        row = _located_row(messages, path)
        if row["status"] == "error":
            return "match", row
        return "mismatch", row

    plain = hl7pet.get_value_located(message, path)
    row = _located_row(messages, path)

    if plain is None:
        ok = row["status"] == "no_match" and row["values"] is None
        return ("match" if ok else "mismatch"), row

    ok = row["status"] == "ok" and row["values"] == _expected_occurrences(plain)
    return ("match" if ok else "mismatch"), row


def _check_hierarchy_vector_located(vector: dict[str, Any]) -> tuple[str, Any]:
    path = vector["path"]
    method = vector.get("method")
    flags = vector.get("flags") or {}

    if path.count(" -> ") > 1:
        return "not_implemented", None
    if method != "getValue":
        return "not_implemented", None
    if flags.get("buildHierarchy", True) is False:
        return "not_implemented", None

    message = _read_message(vector["message_ref"])
    profile = json.loads((FIXTURES / vector["profile_ref"]).read_text())
    messages = pa.array([message])

    plain = hl7pet.get_value_hierarchy_located(message, path, profile)
    row = _located_row(messages, path, profile)

    if plain is None:
        ok = row["status"] == "no_match" and row["values"] is None
        return ("match" if ok else "mismatch"), row

    ok = row["status"] == "ok" and row["values"] == _expected_occurrences(plain)
    return ("match" if ok else "mismatch"), row


def run() -> dict[str, Any]:
    results: list[dict[str, Any]] = []
    for vector in _load_vectors("path"):
        status, actual = _check_path_vector_located(vector)
        results.append({"vector_id": vector["id"], "status": status, "actual": actual})
    for vector in _load_vectors("hierarchy"):
        status, actual = _check_hierarchy_vector_located(vector)
        results.append({"vector_id": vector["id"], "status": status, "actual": actual})
    return {"results": results}


def test_extract_value_located_matches_the_full_fixtures_corpus():
    report = run()

    mismatches = [r for r in report["results"] if r["status"] == "mismatch"]
    assert mismatches == [], mismatches

    not_implemented = {r["vector_id"] for r in report["results"] if r["status"] == "not_implemented"}
    assert not_implemented == KNOWN_NOT_IMPLEMENTED


def test_located_repeating_field_shares_one_line_across_repetitions():
    """A field with multiple repetitions in one segment occurrence: all
    repetitions in `value`, one shared `line` -- not one `{value, line}`
    pair per repetition."""
    message = _read_message("messages/multi-repetition.hl7")
    row = _located_row(pa.array([message]), "OBX-5")
    assert row["status"] == "ok"
    assert row["values"] == [{"value": ["IgG", "IgM"], "line": 4}]


def test_extract_values_located_agrees_with_extract_value_located():
    message = _read_message("messages/baseline.hl7")
    messages = pa.array([message])
    paths = ["MSH-12", "PID-5.1"]

    multi = ha.extract_values_located(messages, paths)
    for i, path in enumerate(paths):
        single = _located_row(messages, path)
        combined = multi.field(i)[0].as_py()
        assert combined == single, (path, combined, single)


def test_extract_values_located_rejects_empty_paths_list():
    message = _read_message("messages/baseline.hl7")
    try:
        ha.extract_values_located(pa.array([message]), [])
    except ValueError as e:
        assert "paths must not be empty" in str(e)
    else:
        raise AssertionError("expected ValueError for an empty paths list")
