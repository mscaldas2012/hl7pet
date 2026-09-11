"""Python bindings for hl7pet-core: fast, zero-copy HL7 v2 PATH extraction.

Quickstart (FR-013)::

    import hl7pet

    message = open("message.hl7").read()

    version = hl7pet.get_first_value(message, "MSH-12")
    # -> "2.5.1" or None if MSH-12 has no value

    all_obx5 = hl7pet.get_value(message, "OBX-5")
    # -> [["Positive"], ["Negative"], ...] or None if OBX doesn't occur

    # Batched: one message scan, many paths, in one call.
    values = hl7pet.get_values(message, ["PID-5.1", "PID-5.2", "MSH-12"])

    # Located: pairs each value with its 1-based source line.
    located = hl7pet.get_first_value_located(message, "MSH-12")
    if located is not None:
        print(located.value, located.line)

    # Hierarchy: requires a segmentDefinition profile (see fixtures/profiles/).
    import json
    profile = json.load(open("profile.json"))
    obs = hl7pet.get_value_hierarchy(message, "OBR[1] -> OBX-5", profile)

Every extraction call returns ``None`` when nothing matches -- it never
raises for "no data". A malformed message, an invalid PATH expression, a
non-numeric filter comparison, or an invalid hierarchy profile each raise a
distinct exception instead (see the ``Hl7*Error`` hierarchy below), so "no
data" and "structurally invalid input" are never conflated.
"""

from ._hl7pet import (
    Hl7PathError,
    Hl7PetError,
    Hl7ProfileError,
    Hl7QueryError,
    Hl7ScanError,
    LocatedValue,
    get_first_value,
    get_first_value_located,
    get_value,
    get_value_hierarchy,
    get_value_located,
    get_values,
)

__all__ = [
    "Hl7PathError",
    "Hl7PetError",
    "Hl7ProfileError",
    "Hl7QueryError",
    "Hl7ScanError",
    "LocatedValue",
    "get_first_value",
    "get_first_value_located",
    "get_value",
    "get_value_hierarchy",
    "get_value_located",
    "get_values",
]
