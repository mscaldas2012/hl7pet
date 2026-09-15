"""Apache Arrow / PyArrow / PySpark integration for hl7pet-core (spec
6002-arrow-integration).

See specs/6002-arrow-integration/contracts/arrow-api.md for the full
contract. `hl7pet_arrow.spark` provides PySpark DataFrame column wiring
(Story 3) and is not imported here, since it requires `pyspark` to be
installed and standalone PyArrow usage should not. `hl7pet_arrow.simplify`
provides plain-column helpers built on top of `extract_value`/
`extract_values`'s output (no new native code -- pure `pyarrow` post-
processing) and is likewise not imported here, kept separate so the
compiled-extension surface stays minimal.

`extract_value_located`/`extract_values_located` are the located
counterparts of `extract_value`/`extract_values` -- same mechanics, but
each matched occurrence is paired with its 1-based source line (mirroring
the plain `hl7pet` binding's `get_value_located`/`LocatedValue`, specs
`1000`/`011`), via a `{values: [{value: [...], line: N}, ...], status}`
Result Struct instead of `{value: [[...]], status}`.
"""

from ._hl7pet_arrow import (
    extract_value,
    extract_value_located,
    extract_values,
    extract_values_located,
)

__all__ = [
    "extract_value",
    "extract_value_located",
    "extract_values",
    "extract_values_located",
]
