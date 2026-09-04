//! Writes each bench target's own partial results file
//! (`specs/009-core-perf-validation/comparison/<run-date>/rust-results-<target>.json`,
//! contracts/comparison-artifact-schema.md's `rust-results.json` shape, split
//! per target). Three separate `cargo bench` binaries (`parsing`/`extraction`/
//! `hierarchy`) have no reliable way to coordinate a single shared-file write
//! without a race — writing one file per target and letting
//! `compare_results.py` merge them (it already has to read multiple JSON
//! files) avoids that race entirely, a refinement over plan.md's original
//! "coordinate a shared write" sketch.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MetricValue {
    pub value: f64,
    pub unit: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultRow {
    pub feature: &'static str,
    pub message_id: String,
    pub path_expression: Option<String>,
    pub throughput: MetricValue,
    pub latency_p50: MetricValue,
    pub latency_p95: MetricValue,
    pub allocation_bytes_per_op: MetricValue,
    pub memory_alloc_rate_bytes_per_sec: MetricValue,
    pub allocation_call_count: MetricValue,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineFailure {
    pub engine: &'static str,
    pub message_id: String,
    pub feature: &'static str,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HostEnvironment {
    os: &'static str,
    arch: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PartialResults {
    corpus_id: String,
    rust_engine_version: &'static str,
    host_environment: HostEnvironment,
    results: Vec<ResultRow>,
    engine_failures: Vec<EngineFailure>,
}

/// Resolves `specs/009-core-perf-validation/comparison/<run-date>/`, creating
/// it if absent. `run-date` comes from `PERF_RUN_DATE` if set (so a human or
/// `quickstart.md`'s own commands can pin both engines' runs to the same
/// directory); otherwise today's date, shelling out to `date +%F` rather than
/// adding a date/time crate dependency — the same format `quickstart.md`
/// already uses for this directory name.
fn run_dir() -> PathBuf {
    let run_date = std::env::var("PERF_RUN_DATE").unwrap_or_else(|_| {
        let output = Command::new("date")
            .arg("+%F")
            .output()
            .expect("running `date +%F`");
        String::from_utf8(output.stdout).expect("date output is valid UTF-8").trim().to_string()
    });

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/009-core-perf-validation/comparison")
        .join(run_date);
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("creating {}: {e}", dir.display()));
    dir
}

/// Writes `target_name`'s results (e.g. `"parsing"`) to its own JSON file in
/// the run directory.
pub fn write_partial_results(
    target_name: &str,
    corpus_id: String,
    results: Vec<ResultRow>,
    engine_failures: Vec<EngineFailure>,
) {
    let partial = PartialResults {
        corpus_id,
        rust_engine_version: env!("CARGO_PKG_VERSION"),
        host_environment: HostEnvironment { os: std::env::consts::OS, arch: std::env::consts::ARCH },
        results,
        engine_failures,
    };

    let path = run_dir().join(format!("rust-results-{target_name}.json"));
    let json = serde_json::to_string_pretty(&partial).expect("serializing partial results");
    fs::write(&path, json).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
    eprintln!("wrote {}", path.display());
}
