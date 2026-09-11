"""Wires parity_check.py's fixtures-corpus run into pytest (spec
6000-python-bindings-automation, research.md #7, User Story 3) —
`pytest crates/python/tests/` now covers full-corpus parity alongside
test_api.py's unit-level API tests, not just the standalone
`python parity_check.py` invocation.
"""

from pathlib import Path

import parity_check

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"
REPO_ROOT = FIXTURES.parent

# Pre-existing, documented exclusions (see parity_check.py's module
# docstring): multi-hop "->" hierarchy chaining, which hl7pet-core's own
# parser already rejects and crates/core/tests/{query,hierarchy}_vectors.rs
# already skip with the same rationale — not a binding gap. Pinned by id
# here (not just a count) so any *other*, unexpected not_implemented vector
# — a real gap — still fails this test.
KNOWN_NOT_IMPLEMENTED = {"hier-009", "hier-010", "path-childpath-hierarchy"}


def test_binding_matches_the_full_fixtures_corpus():
    report = parity_check.run(FIXTURES, REPO_ROOT)

    mismatches = [r for r in report["results"] if r["status"] == "mismatch"]
    assert mismatches == [], mismatches

    not_implemented = {
        r["vector_id"] for r in report["results"] if r["status"] == "not_implemented"
    }
    assert not_implemented == KNOWN_NOT_IMPLEMENTED

    totals = report["totals"]
    assert totals["mismatch"] == 0
    assert totals["not_implemented"] == len(KNOWN_NOT_IMPLEMENTED)
