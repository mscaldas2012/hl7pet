"""Unit tests for hl7_playground.extraction.extract()'s dispatch and error
mapping (specs/9000-playground-webapp/research.md #3, #4, #5)."""

import io
import json

import hl7pet
import pytest

from hl7_playground.extraction import extract


def test_non_hierarchy_path_dispatches_to_get_value_located(monkeypatch, multi_obx_message):
    calls = []
    monkeypatch.setattr(
        hl7pet,
        "get_value_located",
        lambda message, path: calls.append(("located", message, path)) or None,
    )
    monkeypatch.setattr(
        hl7pet,
        "get_value_hierarchy",
        lambda *a, **k: pytest.fail("get_value_hierarchy should not be called"),
    )

    extract(multi_obx_message, "OBX-5", None)

    assert calls == [("located", multi_obx_message, "OBX-5")]


def test_hierarchy_path_dispatches_to_get_value_hierarchy(monkeypatch, basic_hierarchy_message, basic_two_level_profile_bytes):
    calls = []
    monkeypatch.setattr(
        hl7pet,
        "get_value_located",
        lambda *a, **k: pytest.fail("get_value_located should not be called"),
    )
    monkeypatch.setattr(
        hl7pet,
        "get_value_hierarchy",
        lambda message, path, profile: calls.append(("hierarchy", message, path, profile)) or None,
    )

    profile_file = io.BytesIO(basic_two_level_profile_bytes)
    extract(basic_hierarchy_message, "OBR -> OBX-5", profile_file)

    assert len(calls) == 1
    assert calls[0][:3] == ("hierarchy", basic_hierarchy_message, "OBR -> OBX-5")
    assert calls[0][3] == json.loads(basic_two_level_profile_bytes)


@pytest.mark.parametrize(
    "exc_type,status",
    [
        (hl7pet.Hl7ScanError, "scan_error"),
        (hl7pet.Hl7PathError, "path_error"),
        (hl7pet.Hl7QueryError, "path_error"),
        (hl7pet.Hl7ProfileError, "profile_error"),
    ],
)
def test_exception_mapping_for_non_hierarchy_path(monkeypatch, multi_obx_message, exc_type, status):
    def raise_it(message, path):
        raise exc_type("boom")

    monkeypatch.setattr(hl7pet, "get_value_located", raise_it)

    result = extract(multi_obx_message, "OBX-5", None)

    assert result["status"] == status
    assert "boom" in result["message"]


def test_profile_json_decode_error_maps_to_profile_error(basic_hierarchy_message):
    bad_profile = io.BytesIO(b"not json")

    result = extract(basic_hierarchy_message, "OBR -> OBX-5", bad_profile)

    assert result["status"] == "profile_error"
