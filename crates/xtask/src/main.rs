//! Maintainer tooling for keeping the Python binding in sync with
//! `hl7pet-core`'s public surface (spec 6000-python-bindings-automation,
//! User Story 2). Two subcommands:
//!
//! - `xtask surface-diff [--against <commit>]` — diffs the current (or a
//!   given historical commit's) `hl7pet-core` public surface against the
//!   committed Sync Baseline (`surface-baseline.json`), printing a Surface
//!   Change Report (contracts/surface-snapshot.schema.json) and exiting
//!   non-zero when anything changed.
//! - `xtask sync-baseline` — records the current `hl7pet-core` surface and
//!   `HEAD` commit as the new baseline. Run only after every flagged
//!   change from `surface-diff` has been ported to the Python binding and
//!   the parity check passes (FR-011) — this tool does not enforce that
//!   itself (quickstart.md documents the workflow order).

mod classify;
mod surface;

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use surface::Surface;

#[derive(Debug, Serialize, Deserialize)]
struct Baseline {
    commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    captured_at: Option<String>,
    surface: Surface,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/xtask has a parent")
        .parent()
        .expect("crates/ has a parent")
        .to_path_buf()
}

fn core_src_dir(root: &Path) -> PathBuf {
    root.join("crates/core/src")
}

fn baseline_path(root: &Path) -> PathBuf {
    root.join("crates/xtask/surface-baseline.json")
}

fn run_git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap_or_else(|e| panic!("running git {args:?}: {e}"));
    if !out.status.success() {
        eprintln!("git {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
        std::process::exit(2);
    }
    String::from_utf8(out.stdout)
        .unwrap_or_else(|e| panic!("non-utf8 git output: {e}"))
        .trim()
        .to_string()
}

fn git_head(root: &Path) -> String {
    run_git(root, &["rev-parse", "HEAD"])
}

fn utc_now_iso8601() -> Option<String> {
    let out = Command::new("date").args(["-u", "+%Y-%m-%dT%H:%M:%SZ"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
}

/// Extracts `hl7pet-core`'s public surface as it existed at `commit`, via
/// `git show <commit>:<path>` rather than reading the working tree.
fn extract_surface_at_commit(root: &Path, commit: &str) -> Surface {
    let read = |rel: &str| -> String {
        let spec = format!("{commit}:{rel}");
        let out = Command::new("git")
            .args(["show", &spec])
            .current_dir(root)
            .output()
            .unwrap_or_else(|e| panic!("running git show {spec}: {e}"));
        if !out.status.success() {
            eprintln!("git show {spec} failed: {}", String::from_utf8_lossy(&out.stderr));
            std::process::exit(2);
        }
        String::from_utf8(out.stdout).unwrap_or_else(|e| panic!("non-utf8 output for {spec}: {e}"))
    };
    let lib_rs_content = read("crates/core/src/lib.rs");
    surface::extract_surface_from(&lib_rs_content, |module| {
        read(&format!("crates/core/src/{module}.rs"))
    })
}

fn load_baseline(root: &Path) -> Baseline {
    let path = baseline_path(root);
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "reading {}: {e}. Run `cargo run -p xtask -- sync-baseline` first to establish one.",
            path.display()
        )
    });
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

fn cmd_surface_diff(against: Option<&str>) {
    let root = repo_root();
    let baseline = load_baseline(&root);

    let (current_surface, current_commit) = match against {
        Some(commit) => (extract_surface_at_commit(&root, commit), commit.to_string()),
        None => (surface::extract_surface(&core_src_dir(&root)), git_head(&root)),
    };

    let report = classify::diff(&baseline.surface, &current_surface, &baseline.commit, &current_commit);
    println!("{}", serde_json::to_string_pretty(&report).unwrap());

    if !report.up_to_date {
        std::process::exit(1);
    }
}

fn cmd_sync_baseline(against: Option<&str>) {
    let root = repo_root();
    let (surface, commit) = match against {
        Some(commit) => (extract_surface_at_commit(&root, commit), commit.to_string()),
        None => (surface::extract_surface(&core_src_dir(&root)), git_head(&root)),
    };
    let baseline = Baseline { commit, captured_at: utc_now_iso8601(), surface };

    let path = baseline_path(&root);
    let json = serde_json::to_string_pretty(&baseline).unwrap();
    std::fs::write(&path, format!("{json}\n"))
        .unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
    println!("wrote {}", path.display());
}

fn print_usage() {
    eprintln!("usage: xtask <surface-diff | sync-baseline> [--against <commit>]");
}

fn against_arg(args: &[String]) -> Option<&str> {
    args.iter().position(|a| a == "--against").and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("surface-diff") => cmd_surface_diff(against_arg(&args)),
        Some("sync-baseline") => cmd_sync_baseline(against_arg(&args)),
        _ => {
            print_usage();
            std::process::exit(2);
        }
    }
}
