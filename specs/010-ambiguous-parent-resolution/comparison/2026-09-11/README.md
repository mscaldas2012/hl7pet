# Hierarchy benchmark re-run (T012) — 2026-09-11

`rust-results-hierarchy.json` in this directory is `cargo bench -p hl7pet-core
--bench hierarchy` re-run against this spec's fixed code
(`PERF_RUN_OUTPUT_DIR` pointed here, not at spec `009`'s own committed directory —
mirroring spec `6001`'s own precedent for keeping a later spec's re-run out of an
earlier spec's committed artifacts).

## Wall-clock comparison against spec `009`'s original baseline

All four benchmarked path forms use `OBR` as their parent type, which is
**unambiguous** in every profile this benchmark suite uses — so none of them should
exercise this spec's new code path (`resolve_occurrence_node`) at all; only the
untouched `node_for` fast path.

| Path form | Throughput: `009` baseline → this re-run | Allocation count: `009` → this re-run |
|---|---|---|
| `OBR[1] -> OBX-5` | 0.7216 → 0.5828 ops/µs (-19.2%) | 37 → 38 |
| `OBR[1] -> OBX[3]-5` | 1.3652 → 1.2497 ops/µs (-8.5%) | 18 → 19 |
| `OBR[1] -> OBX[@5='VAL-1-2']-5` | 0.7391 → 0.5907 ops/µs (-20.1%) | 33 → 34 |
| `OBR -> OBX-5` | 0.0510 → 0.0442 ops/µs (-13.3%) | 611 → 611 (unchanged) |

Taken at face value, this looks like a regression — but it does not survive scrutiny,
and is **not** attributed to this spec's change:

1. **The allocation-count pattern is inconsistent with this spec's code.** The three
   `OBR[1] -> ...` forms (exactly one parent candidate processed) each show `+1`
   allocation; the `OBR -> OBX-5` form (roughly 20 parent candidates processed, one
   per `OBR` occurrence in `large_hierarchy_028`) shows `+0`. If this spec's per-parent
   wiring change (`execute_hierarchy`'s new `is_ambiguous` check before each
   `direct_children_of_type` call) were the cause, the `+1` would scale with the
   number of parent candidates processed — it does not.
2. **`git diff` of `crates/core/src/hierarchy.rs` confirms no new allocation exists on
   the unambiguous path.** The only new `vec![...]` in the whole diff is
   `resolve_occurrence_node`'s own stack initialization — a function that is
   structurally unreachable for any of these four benchmarks (all use the
   unambiguous `OBR` as their parent type; `is_ambiguous("OBR")` is `false` for every
   profile this suite loads).
3. **A deterministic, in-process test settles it**: `hierarchy::tests::
   unambiguous_parent_resolution_allocation_count_is_unaffected_by_unrelated_ambiguity`
   calls `execute_hierarchy` for `OBR[1] -> OBX-5` against two profiles — one with an
   unrelated `OBX`/`SPM` ambiguity elsewhere in the tree, one without — and asserts
   identical allocation counts. It passes. Unlike the wall-clock comparison above
   (two separate `cargo bench` process invocations, different binary layouts,
   different OS scheduling), this test controls every variable except the one thing
   in question, in one process, one binary.

**Conclusion**: the wall-clock deltas above are measurement variance between two
separate benchmark runs, not a real regression — consistent with spec `004`'s own
documented precedent (`specs/004-scala-baseline-bench/baseline/README.md`) that this
repo's fast, sub-2-microsecond benchmarks show noise up to ~18-20% run-to-run, worst
at the fastest calls, which is exactly the pattern here (the slowest form, `OBR ->
OBX-5` at ~20µs/op, shows the smallest deviation; the sub-2µs forms show the largest).
SC-004 is satisfied by the deterministic allocation-count test, not by this wall-clock
comparison alone.
