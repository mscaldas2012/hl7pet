//! `getValue`/`getFirstValue`-shaped extraction benchmarks
//! (`hl7pet_core::scan` + `parse` + `execute`) — the Rust equivalent of spec
//! 004's `ExtractionBenchmarks`, run against the same promoted corpus (spec
//! 009 research.md #1). Representative PATH expressions cover, per spec.md
//! FR-004: a plain field, an indexed segment selector, and (for `ORU^R01`-
//! shaped messages, which have `OBX` segments to filter on) a filter clause.
//!
//! `getFirstValue` is benchmarked as its own `feature` (matching spec 004's
//! `getFirstValuePatientLastName`/`getFirstValueMinimal`, distinct Scala
//! benchmark methods) using the same universal `"PID-5.1"` path those use —
//! `PID` is present in every message type in this corpus, so this is the one
//! path both engines' getFirstValue benchmarks apply uniformly, rather than
//! the type-varying representative field `getValue` uses.
//!
//! Benchmarks exactly the messages Scala's `ExtractionBenchmarks` does — one
//! representative "typical" message per type, plus the dedicated large and
//! minimal messages — not every corpus message (research.md #1 addendum,
//! same reasoning as `parsing.rs`).

#[path = "common/mod.rs"]
mod common;

use common::corpus::CorpusMessage;
use common::output::{write_partial_results, MetricValue, ResultRow};
use common::timing::{measure_operation, DEFAULT_MEASURED_ITERS, DEFAULT_WARMUP_ITERS};

const GET_FIRST_VALUE_PATH: &str = "PID-5.1";

/// Mirrors spec 004's `ExtractionBenchmarks.REPRESENTATIVE_PATH` — one
/// representative, non-trivial (repeating/nested) field per message type, so
/// this isn't limited to the trivial `PID-5.1` case for every type.
fn representative_field(message_type: &str) -> Option<&'static str> {
    match message_type {
        "ADT^A01" | "ADT^A08" => Some("PV1-3.1"),
        "ORU^R01" | "ORU^R01^HIERARCHY" => Some("OBX-5"),
        "VXU^V04" => Some("RXA-5.2"),
        "ORM^O01" => Some("OBR-4.2"),
        _ => None,
    }
}

fn run_get_value(results: &mut Vec<ResultRow>, message: &str, message_id: &str, path_expr: &str) {
    let scan_result = match hl7pet_core::scan(message) {
        Ok(s) => s,
        Err(_) => return, // not this bench's concern; parsing.rs already covers scan() itself
    };
    let compiled = match hl7pet_core::parse(path_expr) {
        Ok(c) => c,
        Err(_) => return,
    };

    let (timing, alloc) = measure_operation(DEFAULT_WARMUP_ITERS, DEFAULT_MEASURED_ITERS, || {
        hl7pet_core::execute(&scan_result, &compiled)
    });

    results.push(ResultRow {
        feature: "getValue",
        message_id: message_id.to_string(),
        path_expression: Some(path_expr.to_string()),
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

/// `getFirstValue` is a pure derivation over `execute()`'s result
/// (`.first().and_then(|r| r.first())`, spec 007's `contracts/query-api.md`)
/// — no separate Rust call path exists, so timing that derivation alongside
/// `execute()` itself is the honest "getFirstValue" cost.
fn run_get_first_value(results: &mut Vec<ResultRow>, message: &str, message_id: &str) {
    let scan_result = match hl7pet_core::scan(message) {
        Ok(s) => s,
        Err(_) => return,
    };
    let compiled = match hl7pet_core::parse(GET_FIRST_VALUE_PATH) {
        Ok(c) => c,
        Err(_) => return,
    };

    let (timing, alloc) = measure_operation(DEFAULT_WARMUP_ITERS, DEFAULT_MEASURED_ITERS, || {
        hl7pet_core::execute(&scan_result, &compiled)
            .ok()
            .and_then(|v| v.first().and_then(|reps| reps.first()).copied())
    });

    results.push(ResultRow {
        feature: "getFirstValue",
        message_id: message_id.to_string(),
        path_expression: Some(GET_FIRST_VALUE_PATH.to_string()),
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

fn run_representative(results: &mut Vec<ResultRow>, message: &CorpusMessage) {
    let Some(field) = representative_field(&message.message_type) else {
        return;
    };
    let segment = field.split(['-', '[']).next().unwrap_or(field);

    // Plain field (FR-004).
    run_get_value(results, &message.content, &message.message_id, field);

    // Indexed segment selector (FR-004): the field expression stays the
    // same, only the segment gets an explicit [1].
    let field_suffix = &field[segment.len()..];
    let indexed_path = format!("{segment}[1]{field_suffix}");
    run_get_value(results, &message.content, &message.message_id, &indexed_path);

    // Filter clause (FR-004): only for OBX-bearing (ORU^R01-shaped)
    // messages, where OBX-1 (the set id) reliably starts at "1".
    if segment == "OBX" {
        run_get_value(results, &message.content, &message.message_id, "OBX[@1='1']-5");
    }
}

fn main() {
    let corpus = common::corpus::load();
    let mut results = Vec::new();

    for message in corpus.representative_typical_per_type() {
        run_get_first_value(&mut results, &message.content, &message.message_id);
        run_representative(&mut results, message);
    }

    let large = corpus.unique_by_size_category("large-high-repetition");
    run_get_value(&mut results, &large.content, &large.message_id, "OBX-5");

    let minimal = corpus.unique_by_size_category("minimal");
    run_get_first_value(&mut results, &minimal.content, &minimal.message_id);

    write_partial_results("extraction", corpus.corpus_id.clone(), results, Vec::new());
}
