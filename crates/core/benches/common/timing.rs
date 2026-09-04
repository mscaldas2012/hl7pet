//! Warmup-then-measure sampling loop (spec 009 research.md #2) — a custom,
//! dependency-free harness rather than `criterion`, so this project keeps
//! full, precise control over matching JMH's own `sample`-mode percentile
//! definition (verified directly against real `jmh-results.json` output:
//! p50/p95 come from per-invocation sampling, not from a separate
//! throughput-mode run).

use std::time::{Duration, Instant};

use super::alloc;

/// Default warmup/measurement iteration counts. The JVM benefits heavily
/// from a real warmup phase (JIT tiering); Rust has no equivalent transition,
/// but a small warmup still helps settle CPU frequency scaling and cache
/// state, so one is still run for fairness.
pub const DEFAULT_WARMUP_ITERS: usize = 50;
pub const DEFAULT_MEASURED_ITERS: usize = 500;

/// One operation's timing/throughput summary, in the same units JMH's own
/// output already uses (`ops/us`, `us`) so `compare_results.py` needs no
/// unit conversion beyond what it already does for the Scala side.
#[derive(Debug, Clone, Copy)]
pub struct TimingStats {
    pub throughput_ops_per_us: f64,
    pub p50_us: f64,
    pub p95_us: f64,
}

/// Runs `f` `warmup_iters` times (discarded), then `measured_iters` times,
/// timing each measured call individually via [`Instant`]. Returns the
/// sorted per-call durations alongside the derived [`TimingStats`] — callers
/// needing allocation stats measure those in the same loop themselves
/// (`common::alloc::measure`), since interleaving two measurements per call
/// keeps everything over the exact same set of invocations.
pub fn sample<T>(
    warmup_iters: usize,
    measured_iters: usize,
    mut f: impl FnMut() -> T,
) -> (Vec<Duration>, TimingStats) {
    for _ in 0..warmup_iters {
        std::hint::black_box(f());
    }

    let mut durations = Vec::with_capacity(measured_iters);
    for _ in 0..measured_iters {
        let start = Instant::now();
        let result = f();
        let elapsed = start.elapsed();
        std::hint::black_box(result);
        durations.push(elapsed);
    }

    durations.sort();
    let stats = stats_from_sorted(&durations);
    (durations, stats)
}

/// Computes [`TimingStats`] from an already-sorted sample set. Exposed
/// separately so a caller that also wants allocation stats per iteration can
/// sort once and derive both from the same data.
pub fn stats_from_sorted(sorted_durations: &[Duration]) -> TimingStats {
    let total: Duration = sorted_durations.iter().sum();
    let total_us = total.as_secs_f64() * 1_000_000.0;
    let n = sorted_durations.len();

    TimingStats {
        throughput_ops_per_us: if total_us > 0.0 { n as f64 / total_us } else { 0.0 },
        p50_us: percentile_us(sorted_durations, 0.50),
        p95_us: percentile_us(sorted_durations, 0.95),
    }
}

/// Nearest-rank percentile over an already-sorted sample set, in
/// microseconds.
fn percentile_us(sorted_durations: &[Duration], pct: f64) -> f64 {
    if sorted_durations.is_empty() {
        return 0.0;
    }
    let idx = ((sorted_durations.len() as f64 - 1.0) * pct).round() as usize;
    sorted_durations[idx].as_secs_f64() * 1_000_000.0
}

/// Average allocation activity per measured call — the per-op figures
/// `common::output::ResultRow` needs (research.md #3).
#[derive(Debug, Clone, Copy)]
pub struct AllocPerOp {
    pub bytes_per_op: f64,
    pub call_count_per_op: f64,
}

/// Combines timing and allocation measurement into one loop over the same
/// calls, so both figures describe literally the same set of invocations
/// rather than two independently-sampled runs. Each measured call is timed
/// via [`Instant`] and passed through `common::alloc::measure` in the same
/// iteration.
pub fn measure_operation<T>(
    warmup_iters: usize,
    measured_iters: usize,
    mut f: impl FnMut() -> T,
) -> (TimingStats, AllocPerOp) {
    for _ in 0..warmup_iters {
        std::hint::black_box(f());
    }

    let mut durations = Vec::with_capacity(measured_iters);
    let mut total_alloc_count: u64 = 0;
    let mut total_alloc_bytes: u64 = 0;

    for _ in 0..measured_iters {
        let start = Instant::now();
        let (result, stats) = alloc::measure(&mut f);
        let elapsed = start.elapsed();
        std::hint::black_box(result);
        durations.push(elapsed);
        total_alloc_count += stats.call_count;
        total_alloc_bytes += stats.bytes;
    }

    durations.sort();
    let timing_stats = stats_from_sorted(&durations);
    let n = measured_iters as f64;
    let bytes_per_op = total_alloc_bytes as f64 / n;
    let alloc_per_op = AllocPerOp {
        bytes_per_op,
        call_count_per_op: total_alloc_count as f64 / n,
    };

    (timing_stats, alloc_per_op)
}
