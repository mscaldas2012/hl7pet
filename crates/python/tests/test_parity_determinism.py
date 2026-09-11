"""Determinism test for parity_check.py (spec
6000-python-bindings-automation, User Story 3, SC-005): running it twice
against the same build, with no change to the core or the binding,
produces identical `results` content and ordering — the only field allowed
to differ is the informational `run_at` timestamp.
"""

from pathlib import Path

import parity_check

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"
REPO_ROOT = FIXTURES.parent


def test_two_runs_produce_identical_results():
    first = parity_check.run(FIXTURES, REPO_ROOT)
    second = parity_check.run(FIXTURES, REPO_ROOT)

    assert first["results"] == second["results"]
    assert first["totals"] == second["totals"]
    assert first["binding_commit"] == second["binding_commit"]
    assert [r["vector_id"] for r in first["results"]] == [
        r["vector_id"] for r in second["results"]
    ]
