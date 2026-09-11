#!/usr/bin/env python3
"""Fixtures-corpus parity check for the `hl7pet` Python binding (spec
6000-python-bindings-automation, FR-006/FR-009).

Runnable standalone::

    python parity_check.py [--fixtures PATH] [--out PATH]

...and importable (`run()`) for pytest, per research.md #7 (test_parity.py,
User Story 3). Walks every vector under
fixtures/vectors/{path,hierarchy,scanner,escapes}/*.json, calls the `hl7pet`
function matching that vector's family/method, and classifies each vector
as `match`, `mismatch`, or `not_implemented` (research.md #8) — never
conflating "no Python counterpart yet" with "implemented but wrong". Output
shape: contracts/parity-report.schema.json.

Two deliberate scope notes:

- Scanner-family vectors exercise the *scan* step only, indirectly, through
  `get_value(message, "MSH-1")` — the Python API has no standalone `scan()`
  entry point (FR-001 only requires extraction, not raw scanner internals),
  so a scan failure is observed via the exception it causes get_value to
  raise, and a scan success is observed via the absence of one.
- A hierarchy-shaped PATH (containing " -> ") in the plain `path` family,
  and a two-hop ("A -> B -> C") PATH in the `hierarchy` family, are both
  reported `not_implemented` rather than attempted — mirroring the exact
  skips `crates/core/tests/query_vectors.rs` (`is_hierarchy`) and
  `crates/core/tests/hierarchy_vectors.rs` (`is_multi_hop`) already apply
  to hl7pet-core's own conformance suite for these same vector ids
  (`path-childpath-hierarchy`, `hier-009`, `hier-010`): the first belongs to
  `get_value_hierarchy` instead, and the second is deferred to a future
  multi-hop-chaining spec, not achievable by any binding today.
"""

from __future__ import annotations

import argparse
import datetime
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import hl7pet

FAMILIES = ("escapes", "hierarchy", "path", "scanner")

# hl7pet_core::scanner::ScanError's Display text embeds these phrases
# (crates/core/src/scanner.rs) -- used to confirm *which* structural
# failure was raised without a second, structured error-introspection API.
SCAN_ERROR_PHRASES = {
    "MissingMsh": "does not start with a valid MSH segment",
    "TruncatedMsh": "truncated before MSH-2",
    "UnrecognizedSegment": "unrecognized name",
}


def _git_head(repo_root: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=repo_root, text=True
        ).strip()
    except Exception:
        return "unknown"


def _load_vectors(fixtures_dir: Path, family: str) -> list[dict[str, Any]]:
    vectors: list[dict[str, Any]] = []
    for path in sorted((fixtures_dir / "vectors" / family).glob("*.json")):
        vectors.extend(json.loads(path.read_text()))
    return vectors


def _read_message(fixtures_dir: Path, message_ref: str) -> str:
    return (fixtures_dir / message_ref).read_text()


def _is_empty_expected(expected: Any) -> bool:
    return expected is None or expected == []


def _is_hierarchy_path(path: str) -> bool:
    return " -> " in path


def _is_multi_hop_path(path: str) -> bool:
    return path.count(" -> ") > 1


def _check_path_or_escapes_vector(fixtures_dir: Path, vector: dict[str, Any]) -> tuple[str, Any]:
    path = vector["path"]
    expected = vector["expected"]

    if _is_hierarchy_path(path) and expected != "INVALID":
        # Out of scope for get_value/get_first_value by design (no profile
        # to navigate with) -- crates/core/tests/query_vectors.rs's own
        # `is_hierarchy` skip established this precedent for the plain
        # `path` vector family; hl7pet.get_value_hierarchy is the right
        # entry point for a PATH shaped like this, exercised by the
        # `hierarchy` vector family instead. A vector expecting "INVALID"
        # (e.g. a two-hop PATH the parser itself rejects) still falls
        # through below -- get_value naturally reproduces that parse
        # failure without needing a profile.
        return "not_implemented", None

    message = _read_message(fixtures_dir, vector["message_ref"])
    method = vector["method"]
    call = hl7pet.get_value if method == "getValue" else hl7pet.get_first_value

    if expected == "INVALID":
        try:
            call(message, path)
        except hl7pet.Hl7PathError:
            return "match", "Hl7PathError (as expected)"
        except AttributeError:
            return "not_implemented", None
        except Exception as e:
            return "mismatch", f"{type(e).__name__}: {e}"
        return "mismatch", "no exception raised"

    if expected == "ERROR:NonNumericComparison":
        try:
            call(message, path)
        except hl7pet.Hl7QueryError:
            return "match", "Hl7QueryError (as expected)"
        except AttributeError:
            return "not_implemented", None
        except Exception as e:
            return "mismatch", f"{type(e).__name__}: {e}"
        return "mismatch", "no exception raised"

    try:
        actual = call(message, path)
    except AttributeError:
        return "not_implemented", None
    except Exception as e:
        return "mismatch", f"{type(e).__name__}: {e}"

    if _is_empty_expected(expected):
        return ("match" if actual is None else "mismatch"), actual
    return ("match" if actual == expected else "mismatch"), actual


def _check_hierarchy_vector(fixtures_dir: Path, vector: dict[str, Any]) -> tuple[str, Any]:
    path = vector["path"]

    if _is_multi_hop_path(path):
        # Deferred to a future spec -- hl7pet-core's own parser rejects a
        # second " -> " hop outright (crates/core/src/parser.rs), and
        # crates/core/tests/hierarchy_vectors.rs's own `is_multi_hop` skip
        # already documents these two vectors (hier-009/hier-010) as
        # anticipating a future multi-hop spec, not exercisable today.
        return "not_implemented", None

    method = vector["method"]
    if method != "getValue":
        # hl7pet has no dedicated hierarchy first-value entry point; no
        # current vector exercises this, but report honestly if one ever does.
        return "not_implemented", None

    message = _read_message(fixtures_dir, vector["message_ref"])
    profile = json.loads((fixtures_dir / vector["profile_ref"]).read_text())
    expected = vector["expected"]
    build_hierarchy = (vector.get("flags") or {}).get("buildHierarchy", True)

    try:
        actual = hl7pet.get_value_hierarchy(message, path, profile, build_hierarchy)
    except AttributeError:
        return "not_implemented", None
    except Exception as e:
        return "mismatch", f"{type(e).__name__}: {e}"

    if _is_empty_expected(expected):
        return ("match" if actual is None else "mismatch"), actual
    return ("match" if actual == expected else "mismatch"), actual


def _check_scanner_vector(fixtures_dir: Path, vector: dict[str, Any]) -> tuple[str, Any]:
    message = _read_message(fixtures_dir, vector["message_ref"])
    expected_error = vector.get("expected_error")

    try:
        hl7pet.get_value(message, "MSH-1")
    except hl7pet.Hl7ScanError as e:
        actual = f"Hl7ScanError: {e}"
        if expected_error is None:
            return "mismatch", actual
        phrase = SCAN_ERROR_PHRASES.get(expected_error["kind"], "")
        matches = phrase in str(e) and str(expected_error["offset"]) in str(e)
        return ("match" if matches else "mismatch"), actual
    except AttributeError:
        return "not_implemented", None
    except Exception as e:
        return "mismatch", f"{type(e).__name__}: {e}"

    return ("match" if expected_error is None else "mismatch"), "no scan error"


def run(fixtures_dir: Path, repo_root: Path) -> dict[str, Any]:
    totals = {"match": 0, "mismatch": 0, "not_implemented": 0}
    results: list[dict[str, Any]] = []

    checkers = {
        "path": _check_path_or_escapes_vector,
        "escapes": _check_path_or_escapes_vector,
        "hierarchy": _check_hierarchy_vector,
        "scanner": _check_scanner_vector,
    }

    for family in FAMILIES:
        vectors = sorted(_load_vectors(fixtures_dir, family), key=lambda v: v["id"])
        for vector in vectors:
            status, actual = checkers[family](fixtures_dir, vector)
            totals[status] += 1
            results.append(
                {
                    "vector_id": vector["id"],
                    "family": family,
                    "status": status,
                    "expected": vector.get("expected", vector.get("expected_error", "no-scan-error")),
                    "actual": actual,
                }
            )

    return {
        "run_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "binding_commit": _git_head(repo_root),
        "totals": totals,
        "results": results,
    }


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixtures", type=Path, default=repo_root / "fixtures")
    parser.add_argument("--out", type=Path, default=None)
    args = parser.parse_args()

    report = run(args.fixtures.resolve(), repo_root)
    output = json.dumps(report, indent=2)

    if args.out:
        args.out.write_text(output)
        print(f"wrote {args.out}", file=sys.stderr)
    else:
        print(output)

    t = report["totals"]
    print(
        f"match={t['match']} mismatch={t['mismatch']} not_implemented={t['not_implemented']}",
        file=sys.stderr,
    )
    return 0 if t["mismatch"] == 0 and t["not_implemented"] == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
