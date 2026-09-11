# Quickstart: Python Bindings & Core-Sync Tooling

Validation guide for spec `6000`. Assumes a Rust toolchain (stable, matching
the workspace) and Python 3.9+ are already installed. See plan.md's Project
Structure for where everything lives.

## 1. Build and install the binding locally (User Story 1)

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install maturin pytest

cd crates/python
maturin develop          # builds hl7pet-core + the extension, installs
                          # the `hl7pet` package into the active venv
```

**Expected outcome**: `python -c "import hl7pet; print(hl7pet.get_first_value(open('../../fixtures/messages/baseline.hl7').read(), 'MSH-12'))"` prints `2.5.1` — the same value `fixtures/vectors/path/valid.json`'s `path-msh12` vector documents for `hl7pet-core`.

## 2. Run the full parity check (User Story 1's Independent Test, FR-006)

```bash
cd crates/python
python tests/parity_check.py --fixtures ../../fixtures --out /tmp/parity-report.json
```

**Expected outcome**: `/tmp/parity-report.json` (schema:
`contracts/parity-report.schema.json`) shows `totals.mismatch == 0` and
`totals.match == 49`. `totals.not_implemented == 3` — `hier-009`,
`hier-010`, `path-childpath-hierarchy` — is the correct, stable result, not
a gap: all three exercise multi-hop `->` hierarchy chaining, which
`hl7pet-core`'s own parser already rejects and its own conformance suite
(`crates/core/tests/{query,hierarchy}_vectors.rs`) already skips with the
identical documented rationale (parity_check.py's own module docstring
explains this). The script's exit code is non-zero in this state (it flags
`not_implemented` conservatively, per FR-009), so treat `mismatch == 0` as
the pass bar for this step, not the raw exit code. This is also runnable as
`pytest crates/python/tests/` for CI/editor integration (research.md #7) —
`test_parity.py` encodes this exact expectation (pinned by vector id, so a
*new*, unexpected `not_implemented` still fails the test).

## 3. Simulate a future core change and run the surface-diff (User Story 2)

```bash
# Simulate spec N+1 adding one new pub fn to hl7pet-core:
echo 'pub fn example_new_fn() {}' >> crates/core/src/query.rs

cargo run -p xtask -- surface-diff
```

**Expected outcome**: report (schema:
`contracts/surface-snapshot.schema.json`, "Surface Change Report" variant)
lists exactly one `changes` entry — `kind: "added"`,
`item_path: "query::example_new_fn"`,
`classification: "backward_compatible_addition"` — and nothing else.
Revert the simulated change afterward:
`git checkout -- crates/core/src/query.rs`.

## 4. Verify a Documented Breaking Change classifies distinctly (User Story 2, Acceptance Scenario 2)

```bash
cp crates/xtask/surface-baseline.json /tmp/real-baseline.json
cargo run -p xtask -- sync-baseline --against abe47e4   # commit before spec 1001 merged
cargo run -p xtask -- surface-diff
cp /tmp/real-baseline.json crates/xtask/surface-baseline.json   # restore afterward
```

**Expected outcome**: the entry for `query::execute`'s signature change
(`&str` → `Cow<str>` return type, spec `1001`) shows
`classification: "documented_breaking_change"`,
`requires_version_bump: true`, `requires_migration_note: true` — distinct
from the `added` entries in the same report.

## 5. Confirm "no changes" reports cleanly (Acceptance Scenario 3)

```bash
cargo run -p xtask -- surface-diff
```

**Expected outcome** (on an unmodified tree, baseline already current):
`up_to_date: true`, `changes: []`.

## 6. Catch a deliberate regression (User Story 3, SC-004)

```bash
# Introduce a one-line defect in the binding, e.g. flip a >= to > in
# crates/python/src/lib.rs's index handling, then:
cd crates/python
maturin develop
python tests/parity_check.py --fixtures ../../fixtures --out /tmp/parity-report.json
```

**Expected outcome**: non-zero exit; `/tmp/parity-report.json` lists exactly
the vector(s) exercising that code path as `status: "mismatch"` with
`expected`/`actual` both populated for inspection, and every unaffected
vector still `status: "match"`. Revert the defect afterward.

## 7. Record a new Sync Baseline after a real sync (FR-011)

```bash
cargo run -p xtask -- sync-baseline
git diff crates/xtask/surface-baseline.json   # review before committing
```

**Expected outcome**: only performed after step 2's parity check passes
with zero mismatches/not-implemented; `surface-baseline.json`'s `commit`
field updates to the current `HEAD` SHA and its `surface` field reflects
the current `hl7pet-core` public API.
