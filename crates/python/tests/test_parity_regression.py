"""Regression-detection test for parity_check.py (spec
6000-python-bindings-automation, User Story 3, SC-004).

The compiled `_hl7pet` extension can't be edited from a Python test, so
this deliberately corrupts one binding code path via monkeypatching a
single, narrowly-targeted call (`get_first_value("MSH-12", ...)`, the exact
call `fixtures/vectors/path/valid.json`'s `path-msh12` vector — the only
vector anywhere in the corpus using that path — exercises) and confirms the
parity check flags exactly that one vector as a mismatch, with every other
vector still reporting its prior status.
"""

from pathlib import Path

import hl7pet
import parity_check

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"
REPO_ROOT = FIXTURES.parent


def test_parity_check_catches_a_deliberately_corrupted_value(monkeypatch):
    baseline_statuses = {
        r["vector_id"]: r["status"] for r in parity_check.run(FIXTURES, REPO_ROOT)["results"]
    }
    assert baseline_statuses["path-msh12"] == "match"

    real_get_first_value = hl7pet.get_first_value

    def corrupted_get_first_value(message, path):
        value = real_get_first_value(message, path)
        if path == "MSH-12" and value is not None:
            return value[::-1]  # deliberately wrong -- "2.5.1" -> "1.5.2"
        return value

    monkeypatch.setattr(hl7pet, "get_first_value", corrupted_get_first_value)

    corrupted = parity_check.run(FIXTURES, REPO_ROOT)

    mismatches = {r["vector_id"] for r in corrupted["results"] if r["status"] == "mismatch"}
    assert mismatches == {"path-msh12"}

    for result in corrupted["results"]:
        if result["vector_id"] != "path-msh12":
            assert result["status"] == baseline_statuses[result["vector_id"]], result
