"""Tests for `hl7pet_arrow.simplify` -- plain-column helpers over
`extract_value`/`extract_values` output."""

from __future__ import annotations

import pyarrow as pa

import hl7pet_arrow as ha
import hl7pet_arrow.simplify as hs

from test_parity import _read_message as _read


def test_values_drops_status_keeps_null_for_no_match_and_error():
    messages = pa.array([_read("messages/baseline.hl7"), None, "garbled"])
    result = ha.extract_value(messages, "MSH-12")

    assert hs.values(result).to_pylist() == [[["2.5.1"]], None, None]


def test_first_value_flattens_to_first_repetition_of_first_occurrence():
    messages = pa.array([_read("messages/baseline.hl7"), None, "garbled"])
    result = ha.extract_value(messages, "MSH-12")

    first = hs.first_value(result)
    assert first.to_pylist() == ["2.5.1", None, None]
    assert first.type == pa.string()


def test_first_value_on_a_repeating_field_returns_only_the_first_repetition():
    message = _read("messages/multi-repetition.hl7")
    result = ha.extract_value(pa.array([message]), "OBX-5")

    assert hs.values(result).to_pylist() == [[["IgG", "IgM"]]]
    assert hs.first_value(result).to_pylist() == ["IgG"]


def test_simplify_works_on_a_field_of_extract_values_output():
    messages = pa.array([_read("messages/baseline.hl7")])
    multi = ha.extract_values(messages, ["MSH-12", "PID-5.1"])

    assert hs.first_value(multi.field("MSH-12")).to_pylist() == ["2.5.1"]
    assert hs.first_value(multi.field("PID-5.1")).to_pylist() == ["SYNTHETIC"]
