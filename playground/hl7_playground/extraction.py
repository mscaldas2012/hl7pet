"""Dispatch engine: message + PATH + optional profile -> a status-shaped dict.

Implements the contract in specs/9000-playground-webapp/contracts/playground-api.md
and the decisions in that spec's research.md (#3 hierarchy dispatch, #4 error
taxonomy, #5 profile parsing). Never lets an hl7pet exception escape as a 500 --
every outcome is one of the documented `status` values.
"""

from __future__ import annotations

import json

import hl7pet

_HIERARCHY_TOKEN = "->"


def _located_results(message: str, path: str) -> dict:
    """Non-hierarchy branch (research.md #3): line-numbered results (FR-005)."""
    result = hl7pet.get_value_located(message, path)
    if result is None:
        return {"status": "no_results"}
    return {
        "status": "results",
        "hierarchy": False,
        "results": [
            {"value": located.value, "line": located.line}
            for occurrence in result
            for located in occurrence
        ],
    }


def _hierarchy_results(message: str, path: str, profile_file) -> dict:
    """Hierarchy branch (research.md #3): values only, no line numbers (FR-005a)."""
    if profile_file is None:
        return {
            "status": "profile_required",
            "message": (
                "This PATH uses hierarchy navigation ('->') and needs a "
                "hierarchy profile. Upload one to continue."
            ),
        }

    raw = profile_file.read()
    try:
        profile = json.loads(raw)
    except json.JSONDecodeError as exc:
        return {"status": "profile_error", "message": f"Profile file is not valid JSON: {exc}"}

    result = hl7pet.get_value_hierarchy(message, path, profile)
    if result is None:
        return {"status": "no_results"}
    return {
        "status": "results",
        "hierarchy": True,
        "results": [value for occurrence in result for value in occurrence],
    }


def extract(message: str, path: str, profile_file) -> dict:
    """Runs `path` against `message`, using `profile_file` only if `path` is a
    hierarchy PATH. `profile_file` is a werkzeug `FileStorage` or `None`.

    Always returns a dict matching one of contracts/playground-api.md's
    `status` shapes -- never raises for any hl7pet-documented failure mode
    (FR-007 through FR-011).
    """
    try:
        if _HIERARCHY_TOKEN in path:
            return _hierarchy_results(message, path, profile_file)
        return _located_results(message, path)
    except hl7pet.Hl7ScanError as exc:
        return {"status": "scan_error", "message": str(exc)}
    except (hl7pet.Hl7PathError, hl7pet.Hl7QueryError) as exc:
        return {"status": "path_error", "message": str(exc)}
    except hl7pet.Hl7ProfileError as exc:
        return {"status": "profile_error", "message": str(exc)}
