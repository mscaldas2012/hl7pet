# Quickstart: Python FFI Overhead Benchmark

Validates spec.md's user stories end-to-end: a same-corpus Python-vs-Rust
comparison with an explicit penalty ratio per metric (US1), a check for
whether that penalty stays constant or grows with message size (US2), and
confirmation the numbers were measured fairly — same session, allocation
metrics honestly excluded (US3).

## Prerequisites

- Rust `stable` (workspace, unchanged) and a working `hl7pet` Python
  install built in **release mode**: `cd crates/python && maturin develop
  --release` into an active venv. **Do not use plain `maturin develop`**
  (spec `6000`'s own quickstart step 1, correct for day-to-day development
  but not for this spec) — it builds a debug extension, and `cargo bench`
  always builds Rust in release mode; comparing debug-Python against
  release-Rust inflates every penalty ratio 3-5x and is not a fair
  measurement (discovered during this spec's own real benchmarking run —
  see research.md #9).
- `fixtures/messages/perf/` already exists (spec `009`'s corpus) — nothing
  new to set up.
- Both benchmark runs (Rust and Python) MUST happen on the same,
  otherwise-idle machine, back-to-back, for the comparison to be
  meaningful (spec.md FR-005/Technical Context) — a human/CI discipline
  this quickstart can't enforce mechanically, only document, same as spec
  `009`'s own equivalent note.

## 1. Run the Rust harness into this spec's own directory

```bash
export PERF_RUN_DATE=$(date +%F)
export PERF_RUN_OUTPUT_DIR="$(pwd)/specs/6001-python-ffi-benchmark/comparison/${PERF_RUN_DATE}"
cargo bench -p hl7pet-core
```

**Expected outcome**: `rust-results-{parsing,extraction,hierarchy}.json`
are written under `specs/6001-python-ffi-benchmark/comparison/<today>/`
(research.md #1's opt-in `PERF_RUN_OUTPUT_DIR` override) — not under spec
`009`'s directory. Confirm spec `009`'s own workflow is untouched:
`PERF_RUN_OUTPUT_DIR= cargo bench -p hl7pet-core` (unset) still writes to
`specs/009-core-perf-validation/comparison/<today>/` as before.

## 2. Run the new Python harness, same session

```bash
source .venv/bin/activate   # the venv `maturin develop` installed into
python crates/python/benches/run_all.py \
  --out "specs/6001-python-ffi-benchmark/comparison/${PERF_RUN_DATE}"
```

**Expected outcome**: `python-results.json` is written to the same
directory step 1 used, covering `parsing` (the `get_first_value(msg,
"MSH-1")` proxy, research.md #3), `getValue`, `getFirstValue`, and
`hierarchy` — the same features, messages, and PATH forms
`rust-results-*.json` already covers (FR-001/FR-002).

## 3. Run the comparison

```bash
python3 specs/6001-python-ffi-benchmark/scripts/compare_penalty.py \
  "specs/6001-python-ffi-benchmark/comparison/${PERF_RUN_DATE}"
```

**Expected outcome**: `penalty-report.json` is written to the same
directory. Exits non-zero with an explanatory error if `corpusId` doesn't
match across the four input files (contracts/comparison-artifact-
schema.md's precondition) — confirming FR-001 is checked, not assumed.

## 4. Read the per-feature penalty (US1, SC-001/SC-002)

```bash
python3 -c "
import json, sys
report = json.load(open(sys.argv[1]))
for r in report['results']:
    if r['metric'] == 'throughput':
        print(f\"{r['feature']:12s} {r['pathExpression']:28s} {r['messageId']:20s} penalty={r['penaltyRatio']:.1f}x\")
" specs/6001-python-ffi-benchmark/comparison/${PERF_RUN_DATE}/penalty-report.json
```

**Expected outcome**: one line per benchmarked (feature, message, PATH)
combination, each with a computed throughput penalty ratio — confirming
SC-001/SC-002 (every metric carries a ratio, broken out by feature, not
one aggregate number).

## 5. Check whether the penalty is constant or scales with size (US2, SC-003)

```bash
python3 -c "
import json, sys
report = json.load(open(sys.argv[1]))
c = report['scalingCheck']
print(f\"{c['feature']} / {c['pathExpression']}:\")
print(f\"  typical ({c['typicalMessageId']}): {c['typicalPenaltyRatio']:.2f}x\")
print(f\"  large   ({c['largeMessageId']}):   {c['largePenaltyRatio']:.2f}x\")
" specs/6001-python-ffi-benchmark/comparison/${PERF_RUN_DATE}/penalty-report.json
```

**Expected outcome**: two penalty ratios for the same feature/path at two
different message scales, letting a reader judge directly whether the
overhead is roughly stable (a fixed per-call FFI tax) or grew
substantially (a scaling cost worth investigating) — no fabricated
threshold decides this for them (spec.md Assumptions).

## 6. Confirm no silent exclusions (FR-010)

```bash
python3 -c "
import json, sys
report = json.load(open(sys.argv[1]))
print(f\"engineFailures: {len(report['engineFailures'])}\")
for f in report['engineFailures']:
    print(f\"  {f['engine']} / {f['feature']} / {f['messageId']}: {f['description']}\")
" specs/6001-python-ffi-benchmark/comparison/${PERF_RUN_DATE}/penalty-report.json
```

**Expected outcome**: any failure on either side is printed with full
detail — confirming every exclusion is visible and attributed, never
silently absent from the report.

## 7. Confirm allocation/memory is honestly excluded, not approximated (US3, SC-004)

```bash
python3 -c "
import json, sys
report = json.load(open(sys.argv[1]))
print('notComparableMetrics:', report['notComparableMetrics'])
assert all(m not in r for r in report['results'] for m in ('allocationBytesPerOp', 'allocationCallCount', 'memoryAllocRateBytesPerSec'))
print('confirmed: no allocation/memory field appears in any Comparison Result row')
" specs/6001-python-ffi-benchmark/comparison/${PERF_RUN_DATE}/penalty-report.json
```

**Expected outcome**: the three allocation/memory metric names are listed
once, at the run level, and never appear inside any individual result row
— confirming FR-008/SC-004 (never presented as directly comparable,
never silently faked).
