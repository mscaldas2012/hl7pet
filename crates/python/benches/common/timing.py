"""Warmup-then-measure sampling loop (spec 6001-python-ffi-benchmark,
research.md #4) — mirrors `crates/core/benches/common/timing.rs` exactly
(50 warmup / 500 measured, nearest-rank p50/p95) so latency/throughput
figures are directly comparable, not merely superficially similar.
"""

from __future__ import annotations

import time
from dataclasses import dataclass
from typing import Callable, TypeVar

DEFAULT_WARMUP_ITERS = 50
DEFAULT_MEASURED_ITERS = 500

T = TypeVar("T")


@dataclass(frozen=True)
class TimingStats:
    throughput_ops_per_us: float
    p50_us: float
    p95_us: float


def _percentile_us(sorted_durations_s: list[float], pct: float) -> float:
    if not sorted_durations_s:
        return 0.0
    idx = round((len(sorted_durations_s) - 1) * pct)
    return sorted_durations_s[idx] * 1_000_000.0


def stats_from_sorted(sorted_durations_s: list[float]) -> TimingStats:
    total_us = sum(sorted_durations_s) * 1_000_000.0
    n = len(sorted_durations_s)
    return TimingStats(
        throughput_ops_per_us=(n / total_us) if total_us > 0.0 else 0.0,
        p50_us=_percentile_us(sorted_durations_s, 0.50),
        p95_us=_percentile_us(sorted_durations_s, 0.95),
    )


def sample(
    f: Callable[[], T],
    warmup_iters: int = DEFAULT_WARMUP_ITERS,
    measured_iters: int = DEFAULT_MEASURED_ITERS,
) -> TimingStats:
    """Runs `f` `warmup_iters` times (discarded), then `measured_iters`
    times, timing each measured call individually via
    `time.perf_counter()` — the direct analog of Rust's `Instant::now()`.
    """
    for _ in range(warmup_iters):
        f()

    durations_s: list[float] = []
    for _ in range(measured_iters):
        start = time.perf_counter()
        f()
        durations_s.append(time.perf_counter() - start)

    durations_s.sort()
    return stats_from_sorted(durations_s)
