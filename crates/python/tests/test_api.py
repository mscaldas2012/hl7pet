"""pytest coverage for contracts/python-api.md's behavioral contract
(spec 6000-python-bindings-automation, User Story 1).

Focuses on the API-shape guarantees the fixtures-corpus parity check
(parity_check.py) doesn't itself exercise: None-on-absence, which
exceptions raise for which structural failure, and get_values' batch
ordering / immediate-raise-on-bad-path behavior (research.md #4).
"""

from pathlib import Path

import pytest

import hl7pet

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"
BASELINE = (FIXTURES / "messages" / "baseline.hl7").read_text()
MULTI_OBX = (FIXTURES / "messages" / "multi-obx.hl7").read_text()


def test_get_value_returns_none_on_no_match():
    assert hl7pet.get_value(BASELINE, "XYZ-99") is None


def test_get_first_value_returns_none_on_no_match():
    assert hl7pet.get_first_value(BASELINE, "XYZ-99") is None


def test_get_value_returns_matched_rows():
    assert hl7pet.get_value(BASELINE, "MSH-12") == [["2.5.1"]]


def test_get_first_value_returns_first_match():
    assert hl7pet.get_first_value(BASELINE, "MSH-12") == "2.5.1"


def test_get_value_raises_hl7_path_error_on_invalid_path():
    with pytest.raises(hl7pet.Hl7PathError):
        hl7pet.get_value(BASELINE, "9BC-1")


def test_get_value_raises_hl7_scan_error_on_malformed_message():
    with pytest.raises(hl7pet.Hl7ScanError):
        hl7pet.get_value("not an hl7 message", "MSH-12")


def test_hl7_scan_error_and_path_error_are_distinct_and_share_a_root():
    assert issubclass(hl7pet.Hl7ScanError, hl7pet.Hl7PetError)
    assert issubclass(hl7pet.Hl7PathError, hl7pet.Hl7PetError)
    assert not issubclass(hl7pet.Hl7ScanError, hl7pet.Hl7PathError)
    assert not issubclass(hl7pet.Hl7PathError, hl7pet.Hl7ScanError)


def test_get_value_raises_hl7_query_error_on_nonnumeric_ordering_filter():
    with pytest.raises(hl7pet.Hl7QueryError):
        hl7pet.get_value(MULTI_OBX, "OBX[@5>'100']-5")


def test_get_value_raises_hl7_path_error_for_hierarchy_path():
    with pytest.raises(hl7pet.Hl7PathError):
        hl7pet.get_value(BASELINE, "OBR[1] -> OBX-5")


def test_located_value_pairs_value_with_source_line():
    located = hl7pet.get_first_value_located(BASELINE, "MSH-12")
    assert located is not None
    assert located.value == "2.5.1"
    assert located.line == 1


def test_located_returns_none_on_no_match():
    assert hl7pet.get_first_value_located(BASELINE, "XYZ-99") is None


def test_get_value_located_raises_hl7_path_error_for_hierarchy_path():
    with pytest.raises(hl7pet.Hl7PathError):
        hl7pet.get_value_located(BASELINE, "OBR[1] -> OBX-5")


def test_get_values_batches_in_input_order():
    result = hl7pet.get_values(BASELINE, ["MSH-12", "XYZ-99", "MSH-9.1"])
    assert result == [
        hl7pet.get_value(BASELINE, "MSH-12"),
        None,
        hl7pet.get_value(BASELINE, "MSH-9.1"),
    ]


def test_get_values_raises_immediately_on_bad_path_not_per_element():
    with pytest.raises(hl7pet.Hl7PathError):
        hl7pet.get_values(BASELINE, ["MSH-12", "9BC-1", "MSH-9.1"])


def test_get_value_hierarchy_with_build_hierarchy_false_returns_none():
    profile = {"segmentDefinition": {"OBR": {"children": {"OBX": {}}}}}
    result = hl7pet.get_value_hierarchy(
        MULTI_OBX, "OBR[1] -> OBX-5", profile, build_hierarchy=False
    )
    assert result is None


def test_get_value_hierarchy_raises_hl7_profile_error_on_invalid_profile_json():
    with pytest.raises(hl7pet.Hl7ProfileError):
        hl7pet.get_value_hierarchy(MULTI_OBX, "OBR[1] -> OBX-5", {"not": "a profile"})
