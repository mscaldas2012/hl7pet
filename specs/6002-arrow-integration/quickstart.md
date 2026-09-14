# Quickstart: Arrow Integration for PySpark & PyArrow

Validation guide for spec `6002`. Assumes a Rust toolchain (stable) and
Python 3.9+ are already installed, and that `hl7pet-core`/`fixtures/` are
unchanged from the current repo state. See plan.md's Project Structure for
where everything lives.

## 1. Build and install `hl7pet_arrow` locally (Stories 1-2)

```bash
python3 -m venv .venv-arrow && source .venv-arrow/bin/activate
pip install maturin pyarrow>=18 pytest

cd crates/arrow
maturin develop           # builds hl7pet-core + hl7pet-arrow, installs
                           # the `hl7pet_arrow` package into the active venv
```

**Expected outcome**:

```python
import pyarrow as pa, hl7pet_arrow as ha

msg = open("../../fixtures/messages/baseline.hl7").read()
result = ha.extract_value(pa.array([msg]), "MSH-12")
print(result[0])   # StructScalar: {'value': [['2.5.1']], 'status': 'ok'}
```

matches `fixtures/vectors/path/valid.json`'s `path-msh12` vector's expected
value, now wrapped in the Result Struct shape (data-model.md).

## 2. Run the fixtures-corpus parity check (SC-003)

```bash
cd crates/arrow
pytest tests/test_parity.py
```

**Expected outcome**: every `fixtures/vectors/{path,hierarchy}/` vector,
run through `extract_value` on a single-row `messages` array, produces a
`status`/`value` pair matching what the existing plain `hl7pet.get_value`/
`hl7pet.get_value_hierarchy` return for that same vector (an `Hl7PetError`
raised by the plain binding maps to this test asserting `status ==
"scan_error"`, not to a mismatch). Zero mismatches is the pass bar, mirroring
spec `6000`'s `parity_check.py` precedent.

## 3. Confirm one scan per message regardless of PATH count (SC-002)

```bash
cd crates/arrow
pytest tests/test_scan_count.py
```

**Expected outcome**: `extract_values` with N paths against the same
message column reports the same per-message scan count as `extract_value`
with 1 path (a `cfg(test)`-only instrumented counting build, mirroring the
counting-allocator pattern already used by specs `009`/`1000`/`011`) — the
test fails if requesting more PATHs ever re-scans a message.

## 4. Run the PySpark wiring smoke test (Story 3)

```bash
pip install pyspark==4.2.0
cd crates/arrow
pytest tests/test_spark.py
```

**Expected outcome**: a local (`local[1]`) PySpark session applies
`hl7pet_arrow.spark.extract_value_udf("MSH-12")` and
`hl7pet_arrow.spark.extract_values_udf(["MSH-12", "PID-5.1"])` to a small
DataFrame built from `fixtures/messages/`, and `.collect()`'s results match
step 2's non-Spark results for the same messages/PATHs (spec Story 1
Acceptance Scenario 4 / Story 3).

## 5. Run the demo notebook end-to-end (Story 4, FR-011, SC-005)

```bash
pip install jupyter ipykernel
jupyter nbconvert --to notebook --execute notebooks/arrow_pyspark_demo.ipynb \
  --output /tmp/arrow_pyspark_demo.out.ipynb
```

**Expected outcome**: exits `0`, every cell executes without error. The
notebook's final comparison cell prints the row-by-row plain-`hl7pet`
result and the columnar `hl7pet_arrow` result for the same sample messages
side by side, agreeing exactly (SC-005 Acceptance Scenario 2).
