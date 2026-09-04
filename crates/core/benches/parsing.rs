//! Raw structural scanning benchmarks (`hl7pet_core::scan`) — the Rust
//! equivalent of spec 004's `ParsingBenchmarks` (`retrieveFirstSegmentOf`/
//! `retrieveMultipleSegments`), run against the same promoted corpus (spec
//! 009 research.md #1).
//!
//! Benchmarks exactly the messages Scala's `ParsingBenchmarks` does — one
//! representative "typical" message per type (`retrieveFirstSegment`,
//! `TypicalMessageState`), plus the dedicated large and minimal messages
//! (`retrieveFirstSegmentLarge`/`retrieveMultipleSegmentsLarge`,
//! `retrieveFirstSegmentMinimal`) — not every corpus message, since several
//! types have more than one "typical" entry and Scala's own parameterized
//! benchmark only ever measures one of them (research.md #1 addendum).

#[path = "common/mod.rs"]
mod common;

use common::output::{write_partial_results, MetricValue, ResultRow};
use common::timing::{measure_operation, DEFAULT_MEASURED_ITERS, DEFAULT_WARMUP_ITERS};

fn run(results: &mut Vec<ResultRow>, message: &common::corpus::CorpusMessage) {
    let (timing, alloc) =
        measure_operation(DEFAULT_WARMUP_ITERS, DEFAULT_MEASURED_ITERS, || hl7pet_core::scan(&message.content));

    results.push(ResultRow {
        feature: "parsing",
        message_id: message.message_id.clone(),
        path_expression: None,
        throughput: MetricValue { value: timing.throughput_ops_per_us, unit: "ops/us" },
        latency_p50: MetricValue { value: timing.p50_us, unit: "us" },
        latency_p95: MetricValue { value: timing.p95_us, unit: "us" },
        allocation_bytes_per_op: MetricValue { value: alloc.bytes_per_op, unit: "bytes" },
        memory_alloc_rate_bytes_per_sec: MetricValue {
            value: alloc.bytes_per_op * timing.throughput_ops_per_us * 1_000_000.0,
            unit: "bytes/sec",
        },
        allocation_call_count: MetricValue { value: alloc.call_count_per_op, unit: "count" },
    });
}

fn main() {
    let corpus = common::corpus::load();
    let mut results = Vec::new();

    for message in corpus.representative_typical_per_type() {
        run(&mut results, message);
    }
    run(&mut results, corpus.unique_by_size_category("large-high-repetition"));
    run(&mut results, corpus.unique_by_size_category("minimal"));

    write_partial_results("parsing", corpus.corpus_id.clone(), results, Vec::new());
}
