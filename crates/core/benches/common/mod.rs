//! Shared support for the `parsing`/`extraction`/`hierarchy` bench targets
//! (spec 009-core-perf-validation). Not a bench target itself — included via
//! `#[path = "common/mod.rs"] mod common;` in each target, since Cargo only
//! auto-discovers top-level `benches/*.rs` files as targets, not
//! subdirectories.
//!
//! `#[path]`-included this way, each bench binary gets its own independent
//! compilation of this whole module tree — so each binary's dead-code
//! analysis only sees *that binary's* usage of it (e.g. `parsing.rs` never
//! touches `CorpusMessage::profile_json`, `extraction.rs` never calls
//! `hierarchy_eligible`). `allow(dead_code)` here is deliberate, not a sign
//! of actually-unused code across the harness as a whole.
#![allow(dead_code)]

pub mod alloc;
pub mod corpus;
pub mod output;
pub mod timing;
