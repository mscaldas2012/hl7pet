//! Loads `fixtures/messages/perf/corpus-manifest.json` (spec 009 research.md
//! #1) — the corpus both the extended Scala harness and this Rust harness
//! read identically, satisfying spec.md FR-001's "exact same named corpus
//! messages" by construction. Uses `serde`/`serde_json`, already a
//! `hl7pet-core` runtime dependency since spec 008 — no new dependency is
//! added for this bench-only concern.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    message_id: String,
    message_type: String,
    size_category: String,
    file_path: String,
    #[serde(default)]
    profile_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    corpus_id: String,
    messages: Vec<ManifestEntry>,
}

/// One corpus message, with its content (and, for hierarchy-eligible
/// entries, its paired profile's raw JSON) already loaded.
#[derive(Debug, Clone)]
pub struct CorpusMessage {
    pub message_id: String,
    pub message_type: String,
    pub size_category: String,
    pub content: String,
    /// `Some` only for entries the manifest tags with a `profileRef`
    /// (currently just `large_hierarchy_028`) — raw profile JSON, ready for
    /// `HierarchyProfile::from_json`.
    pub profile_json: Option<String>,
}

pub struct Corpus {
    pub corpus_id: String,
    pub messages: Vec<CorpusMessage>,
}

impl Corpus {
    pub fn hierarchy_eligible(&self) -> impl Iterator<Item = &CorpusMessage> {
        self.messages.iter().filter(|m| m.profile_json.is_some())
    }

    /// One message per distinct `message_type` among `sizeCategory ==
    /// "typical"` entries — the first in manifest order, mirroring spec
    /// 004's `TypicalMessageState.setUp()` (`Corpus.byType(type).stream()
    /// .filter(typical).findFirst()`) exactly. Several message types have
    /// multiple "typical" entries (research.md #1 addendum: the manifest
    /// keeps 5 per type from the promoted interim-v1 corpus); benchmarking
    /// every one of them on the Rust side while Scala's own parameterized
    /// benchmark only ever measures one would produce Rust-only results
    /// with no Scala counterpart to compare against, breaking FR-001's
    /// same-corpus guarantee for no real benefit.
    pub fn representative_typical_per_type(&self) -> Vec<&CorpusMessage> {
        let mut seen = std::collections::HashSet::new();
        self.messages
            .iter()
            .filter(|m| m.size_category == "typical" && seen.insert(m.message_type.clone()))
            .collect()
    }

    /// The one message tagged with `sizeCategory == category` — mirrors
    /// spec 004's `LargeMessageState`/`MinimalMessageState` `.findFirst()`.
    /// Panics if none or more than one exists, since both indicate a
    /// corpus/manifest inconsistency a bench run should surface loudly
    /// rather than silently pick an arbitrary one of (research.md #1
    /// addendum: this is exactly the ambiguity the `large-hierarchy`
    /// category rename avoided for `large-high-repetition`).
    pub fn unique_by_size_category(&self, category: &str) -> &CorpusMessage {
        let mut matches = self.messages.iter().filter(|m| m.size_category == category);
        let first = matches.next().unwrap_or_else(|| panic!("no message with sizeCategory {category:?}"));
        assert!(
            matches.next().is_none(),
            "more than one message with sizeCategory {category:?} -- ambiguous, per spec 004's own .findFirst() precedent"
        );
        first
    }
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Reads `fixtures/messages/perf/corpus-manifest.json` and every message
/// (and paired profile) it lists. Panics with a clear message on any I/O or
/// parse failure — a bench binary with a broken corpus has nothing useful to
/// measure, so failing fast beats limping on with partial data.
pub fn load() -> Corpus {
    let perf_dir = fixtures_root().join("messages/perf");
    let manifest_path = perf_dir.join("corpus-manifest.json");

    let manifest_json = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", manifest_path.display()));
    let manifest: Manifest = serde_json::from_str(&manifest_json)
        .unwrap_or_else(|e| panic!("parsing {}: {e}", manifest_path.display()));

    let messages = manifest
        .messages
        .into_iter()
        .map(|entry| {
            let message_path = perf_dir.join(&entry.file_path);
            let content = fs::read_to_string(&message_path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", message_path.display()));

            let profile_json = entry.profile_ref.map(|rel| {
                let profile_path = perf_dir.join(&rel);
                fs::read_to_string(&profile_path)
                    .unwrap_or_else(|e| panic!("reading {}: {e}", profile_path.display()))
            });

            CorpusMessage {
                message_id: entry.message_id,
                message_type: entry.message_type,
                size_category: entry.size_category,
                content,
                profile_json,
            }
        })
        .collect();

    Corpus { corpus_id: manifest.corpus_id, messages }
}
