# hl7pet (Python)

PyO3 bindings for [`hl7pet-core`](../core), giving Python callers the same
fast, zero-copy HL7 v2 PATH extraction the Rust core provides — scanning,
PATH query execution, located extraction, hierarchy navigation, and
escape-sequence decoding (spec `6000-python-bindings-automation`).

## Install (development)

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install maturin
maturin develop   # builds hl7pet-core + this crate, installs `hl7pet` into the venv
```

## Quickstart

```python
import hl7pet

message = open("message.hl7").read()

# One-call-per-field, Scala getValue/getFirstValue-shaped API.
version = hl7pet.get_first_value(message, "MSH-12")   # -> "2.5.1" or None
all_obx5 = hl7pet.get_value(message, "OBX-5")          # -> [["Positive"], ...] or None

# Batched: one message scan, many paths, one call -- amortizes FFI overhead
# for hot extraction loops.
values = hl7pet.get_values(message, ["PID-5.1", "PID-5.2", "MSH-12"])

# Located: pairs each value with its 1-based source line (spec 1000).
located = hl7pet.get_first_value_located(message, "MSH-12")
if located is not None:
    print(located.value, located.line)

# Hierarchy: requires a segmentDefinition profile (see fixtures/profiles/).
import json
profile = json.load(open("profile.json"))
obs = hl7pet.get_value_hierarchy(message, "OBR[1] -> OBX-5", profile)
```

Every extraction call returns `None` when nothing matches — it never raises
for "no data". A malformed message, an invalid PATH expression, a
non-numeric filter comparison, or an invalid hierarchy profile each raise a
distinct exception instead, so "no data" and "structurally invalid input"
are never conflated (Constitution Principle III):

| Exception | Raised for |
|---|---|
| `Hl7ScanError` | message fails to scan |
| `Hl7PathError` | PATH fails to parse, or is a hierarchy PATH passed to a non-hierarchy method |
| `Hl7QueryError` | an ordering filter (`>`, `>=`, `<`, `<=`) compares a non-numeric operand |
| `Hl7ProfileError` | a hierarchy profile fails to parse |

All four share a common `Hl7PetError` base for a broad `except`.

See `specs/6000-python-bindings-automation/contracts/python-api.md` for the
full function-by-function contract.

## Testing

```bash
pytest tests/                       # API + parity tests
python tests/parity_check.py        # fixtures-corpus parity report, standalone
```

`parity_check.py` verifies this binding's output against every vector in
the shared `fixtures/vectors/{path,hierarchy,scanner,escapes}/` corpus. As
of this spec, 49/52 vectors match exactly; the remaining 3
(`hier-009`, `hier-010`, `path-childpath-hierarchy`) exercise multi-hop
hierarchy chaining (`A -> B -> C`), which `hl7pet-core`'s own parser
already rejects today and `crates/core/tests/hierarchy_vectors.rs` /
`query_vectors.rs` already skip with the same documented rationale — not a
binding gap, deferred to whichever future spec adds multi-hop chaining.

## Benchmarking (FFI overhead)

```bash
maturin develop --release   # IMPORTANT: plain `maturin develop` builds a
                             # debug extension; `cargo bench` always builds
                             # Rust in release mode, so a debug/release
                             # mismatch inflates every penalty ratio 3-5x
                             # (spec 6001-python-ffi-benchmark research.md #9)
export PERF_RUN_DATE=$(date +%F)
export PERF_RUN_OUTPUT_DIR="$(pwd)/../../specs/6001-python-ffi-benchmark/comparison/${PERF_RUN_DATE}"
(cd ../.. && cargo bench -p hl7pet-core)
python benches/run_all.py --out "$PERF_RUN_OUTPUT_DIR"
python ../../specs/6001-python-ffi-benchmark/scripts/compare_penalty.py "$PERF_RUN_OUTPUT_DIR"
```

`benches/` (mirroring `crates/core/benches/`'s own layout) measures how
much calling through this binding costs versus calling `hl7pet-core`
directly from Rust — the same corpus and PATH forms
`crates/core/benches/{parsing,extraction,hierarchy}.rs` already use, so
the two sides are directly comparable. See
`specs/6001-python-ffi-benchmark/quickstart.md` for the full walkthrough
and `contracts/comparison-artifact-schema.md` for the output shape.
