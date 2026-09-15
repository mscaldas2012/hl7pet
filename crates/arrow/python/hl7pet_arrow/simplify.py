"""Plain-column helpers built on top of `hl7pet_arrow.extract_value`/
`extract_values`'s Result Struct output -- pure `pyarrow` post-processing,
no new native code. The Result Struct (`{value, status}`) is the one
canonical, full-fidelity source of truth (it's the only shape that can
distinguish "no match" from "row failed to scan" without raising mid-batch,
per FR-010); these helpers exist for callers who don't need that
distinction and just want a plain column to drop into a DataFrame.

Both functions accept any Result Struct array -- the direct output of
`extract_value(...)`, or one field of `extract_values(...)`'s
struct-of-structs (`multi_result.field(path_or_index)`):

    values(ha.extract_value(messages, "OBX-5"))
    values(ha.extract_values(messages, ["OBX-5", "MSH-12"]).field("OBX-5"))

Rows where nothing matched, or the row failed to scan/evaluate, both
collapse to `null` in either helper's output -- exactly like the plain
`hl7pet.get_value`/`get_first_value`'s own `None`-for-absence convention,
just without a raised exception for the "row failed" case (there is no
per-row raise in a batched call to raise from).
"""

from __future__ import annotations

import pyarrow as pa
import pyarrow.compute as pc


def values(result: pa.StructArray) -> pa.Array:
    """The plain `List<List<Utf8>>` value column -- `result.field("value")`,
    dropping `status`. Null for both `"no_match"` and `"error"` rows."""
    return result.field("value")


def first_value(result: pa.StructArray) -> pa.Array:
    """Flat `Utf8` column: the first field repetition of the first matched
    segment occurrence. Null for `"no_match"`/`"error"` rows, or for a row
    whose match happens to have zero repetitions (shouldn't occur in
    practice, but handled null-safely rather than assumed impossible)."""
    return pc.list_element(pc.list_element(result.field("value"), 0), 0)
