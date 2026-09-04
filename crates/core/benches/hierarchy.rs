//! Hierarchy-mode (`->`) benchmarks (`hl7pet_core::execute_hierarchy`) — the
//! Rust equivalent of spec 004's new `HierarchyBenchmarks.java` (spec 009
//! research.md #4), run against the same corpus entry that Scala side
//! benchmarks.
//!
//! Deliberately scoped to exactly the corpus's hierarchy-eligible entries
//! (currently just `large_hierarchy_028`) — NOT also the existing
//! `basic-hierarchy.hl7`/`complex-hierarchy.hl7` correctness fixtures an
//! earlier task draft considered reusing here. Scala's `HierarchyBenchmarks`
//! only benchmarks `large-hierarchy.hl7`; adding extra Rust-only messages
//! would produce Comparison Result rows with no Scala counterpart, breaking
//! FR-001's "exact same named corpus messages" guarantee for no real
//! benefit (those two fixtures are tiny — 9 and 12 lines — and add no scale
//! signal `large-hierarchy.hl7` doesn't already provide).

#[path = "common/mod.rs"]
mod common;

use common::output::{write_partial_results, MetricValue, ResultRow};
use common::timing::{measure_operation, DEFAULT_MEASURED_ITERS, DEFAULT_WARMUP_ITERS};
use hl7pet_core::HierarchyProfile;

/// Mirrors `HierarchyBenchmarks.java`'s four representative PATH forms
/// exactly, for a true row-for-row comparison.
const PATH_FORMS: &[&str] =
    &["OBR[1] -> OBX-5", "OBR[1] -> OBX[3]-5", "OBR[1] -> OBX[@5='VAL-1-2']-5", "OBR -> OBX-5"];

fn main() {
    let corpus = common::corpus::load();
    let mut results = Vec::new();
    let mut engine_failures = Vec::new();

    for message in corpus.hierarchy_eligible() {
        let profile_json = message.profile_json.as_deref().expect("hierarchy_eligible guarantees Some");

        let scan_result = match hl7pet_core::scan(&message.content) {
            Ok(s) => s,
            Err(e) => {
                engine_failures.push(common::output::EngineFailure {
                    engine: "rust",
                    message_id: message.message_id.clone(),
                    feature: "hierarchy",
                    description: format!("scan() failed: {e}"),
                });
                continue;
            }
        };
        let profile = match HierarchyProfile::from_json(profile_json) {
            Ok(p) => p,
            Err(e) => {
                engine_failures.push(common::output::EngineFailure {
                    engine: "rust",
                    message_id: message.message_id.clone(),
                    feature: "hierarchy",
                    description: format!("HierarchyProfile::from_json() failed: {e}"),
                });
                continue;
            }
        };

        for path_expr in PATH_FORMS {
            let compiled = match hl7pet_core::parse(path_expr) {
                Ok(c) => c,
                Err(e) => {
                    engine_failures.push(common::output::EngineFailure {
                        engine: "rust",
                        message_id: message.message_id.clone(),
                        feature: "hierarchy",
                        description: format!("parse({path_expr:?}) failed: {e}"),
                    });
                    continue;
                }
            };

            let (timing, alloc) = measure_operation(DEFAULT_WARMUP_ITERS, DEFAULT_MEASURED_ITERS, || {
                hl7pet_core::execute_hierarchy(&scan_result, &compiled, Some(&profile))
            });

            results.push(ResultRow {
                feature: "hierarchy",
                message_id: message.message_id.clone(),
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
    }

    write_partial_results("hierarchy", corpus.corpus_id, results, engine_failures);
}
