# Quickstart: Core Performance Validation

Validates spec.md's user stories end-to-end: both engines run against the exact
same corpus (US1), the resulting report carries an explicit per-metric
meets/beats/regresses verdict against the Constitution's non-regression
requirement (US2), and Rust's allocation behavior is confirmed independent of
message size across the whole corpus, not just one hand-picked message per prior
spec (US3).

## Prerequisites

- Everything specs `004`-`008` already require: JDK 17+, Maven, Rust `stable`, no
  local Scala `hl7-pet` checkout needed (Maven-resolved dependency only).
- `fixtures/messages/perf/` and `fixtures/profiles/large-hierarchy.json` exist
  (this spec's own corpus promotion + addition, research.md #1) — a one-time setup
  task, not run per comparison.
- Both benchmark runs (Scala and Rust) MUST happen on the same, otherwise-idle
  machine, back-to-back, for the comparison to be meaningful (spec.md Technical
  Context) — this is a human/CI discipline this quickstart can't enforce
  mechanically, only document.

## 1. Run the extended Scala harness (now including hierarchy mode)

```bash
cd specs/004-scala-baseline-bench/harness
mvn compile exec:java -Dexec.mainClass=gov.cdc.hl7.bench.BenchmarkRunner \
  -- ../../009-core-perf-validation/comparison/$(date +%F)
```

**Expected outcome**: `scala-results.json` is written under
`specs/009-core-perf-validation/comparison/<today>/`, containing `thrpt` and
`sample` mode entries for `ParsingBenchmarks`, `ExtractionBenchmarks`, **and**
the new `HierarchyBenchmarks` — confirmed by checking that at least one entry's
`benchmark` field contains `HierarchyBenchmarks`.

## 2. Run the new Rust harness

```bash
cargo bench -p hl7pet-core
```

**Expected outcome**: `rust-results.json` is written to the same
`comparison/<today>/` directory (the bench binaries' `fn main()` writes there
directly, per contracts/comparison-artifact-schema.md's shape), covering
`parsing`, `getValue`, `getFirstValue`, and `hierarchy` features against every
message in `fixtures/messages/perf/corpus-manifest.json`.

## 3. Run the comparison

```bash
python3 specs/009-core-perf-validation/scripts/compare_results.py \
  specs/009-core-perf-validation/comparison/$(date +%F)
```

**Expected outcome**: `comparison-report.json` is written to the same directory.
Exits non-zero and prints an explanatory error if `corpusId` doesn't match
between the two input files (contracts/comparison-artifact-schema.md's
precondition) — confirming FR-001's "exact same corpus" requirement is checked,
not assumed.

## 4. Confirm every metric carries a verdict (SC-002)

```bash
python3 -c "
import json, sys
report = json.load(open(sys.argv[1]))
missing = [r for r in report['results'] if 'verdict' not in r or not r['verdict']]
print(f\"{len(report['results'])} results, {len(missing)} missing a verdict\")
sys.exit(1 if missing else 0)
" specs/009-core-perf-validation/comparison/$(date +%F)/comparison-report.json
```

**Expected outcome**: `0 missing a verdict` — proving SC-002 (every metric has an
explicit meets/beats/regresses/not-comparable verdict, never a bare number).

## 5. Confirm no silent exclusions (SC-003)

```bash
python3 -c "
import json, sys
report = json.load(open(sys.argv[1]))
print(f\"engineFailures: {len(report['engineFailures'])}\")
for f in report['engineFailures']:
    print(f\"  {f['engine']} / {f['feature']} / {f['messageId']}: {f['description']}\")
" specs/009-core-perf-validation/comparison/$(date +%F)/comparison-report.json
```

**Expected outcome**: any engine failure is printed with full detail — confirming
SC-003's "zero silent exclusions" by making every exclusion visible and
attributed, not absent from the report.

## 6. Read the regression verdict (US2, SC-005)

```bash
python3 -c "
import json, sys
report = json.load(open(sys.argv[1]))
regressions = [r for r in report['results'] if r['verdict'] == 'regresses']
print(f\"{len(regressions)} regression(s) out of {len(report['results'])} metrics\")
for r in regressions:
    print(f\"  {r['feature']}/{r['messageId']}/{r['metric']}: scala={r['scalaValue']}{r['unit']} rust={r['rustValue']}{r['unit']}\")
" specs/009-core-perf-validation/comparison/$(date +%F)/comparison-report.json
```

**Expected outcome**: a reader can determine, from this one command's output
alone, whether the Rust core currently satisfies the Constitution's "MUST NOT
regress" requirement in full (`0 regression(s)`) or exactly which metrics don't
(SC-005) — without re-running either benchmark or reading either harness's
source.

## 7. Confirm allocation-count independence at corpus scale (US3, SC-004)

```bash
python3 -c "
import json, sys
report = json.load(open(sys.argv[1]))
by_feature = {}
for r in report['results']:
    if r['metric'] == 'allocationCallCount':
        by_feature.setdefault(r['feature'], set()).add(r['rustValue'])
for feature, counts in by_feature.items():
    print(f\"{feature}: distinct allocation counts across corpus = {sorted(counts)}\")
" specs/009-core-perf-validation/comparison/$(date +%F)/comparison-report.json
```

**Expected outcome**: for `parsing`/`getValue`/`getFirstValue`/`hierarchy`, the
distinct allocation-count values observed across every corpus message stay
small and bounded (matching specs `005`/`007`/`008`'s own "independent of
message size" claims) — a message-size-dependent trend here would falsify those
claims at corpus scale, not just fail to confirm them.
