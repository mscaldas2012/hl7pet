//! Diffs two [`surface::Surface`] snapshots into a Surface Change Report
//! (spec 6000-python-bindings-automation, FR-007/FR-008/FR-012,
//! data-model.md, contracts/surface-snapshot.schema.json).
//!
//! Classification matches `ROADMAP.md`'s own established convention
//! exactly (Backward-Compatible Additions / Documented Breaking Changes):
//! a new capability is always a *new* item; changing or removing an
//! existing item is always a Documented Breaking Change. There is no
//! partial/superset middle ground — this project's convention doesn't
//! have one (data-model.md's Surface Change Report section).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::surface::Surface;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Changed,
    Removed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    BackwardCompatibleAddition,
    DocumentedBreakingChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Change {
    pub kind: ChangeKind,
    pub classification: Classification,
    pub item_path: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub requires_version_bump: bool,
    pub requires_migration_note: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeReport {
    pub baseline_commit: String,
    pub current_commit: String,
    pub up_to_date: bool,
    pub changes: Vec<Change>,
}

/// Flattens a [`Surface`] into `"module::item" -> canonical representation`
/// across every item category, so diffing is one uniform map comparison
/// regardless of whether an item is a function, struct, enum, or type
/// alias.
fn flatten(surface: &Surface) -> BTreeMap<String, String> {
    let mut flat = BTreeMap::new();
    for (module, ms) in surface {
        for (name, sig) in &ms.functions {
            flat.insert(format!("{module}::{name}"), sig.clone());
        }
        for (name, def) in &ms.structs {
            let fields: Vec<String> =
                def.fields.iter().map(|(f, t)| format!("{f}: {t}")).collect();
            flat.insert(format!("{module}::{name}"), format!("struct {{ {} }}", fields.join(", ")));
        }
        for (name, def) in &ms.enums {
            flat.insert(
                format!("{module}::{name}"),
                format!("enum {{ {} }}", def.variants.join(", ")),
            );
        }
        for (name, rhs) in &ms.types {
            flat.insert(format!("{module}::{name}"), format!("type = {rhs}"));
        }
    }
    flat
}

pub fn diff(baseline: &Surface, current: &Surface, baseline_commit: &str, current_commit: &str) -> ChangeReport {
    let before = flatten(baseline);
    let after = flatten(current);

    let mut changes = Vec::new();

    for (item_path, after_sig) in &after {
        match before.get(item_path) {
            None => changes.push(Change {
                kind: ChangeKind::Added,
                classification: Classification::BackwardCompatibleAddition,
                item_path: item_path.clone(),
                before: None,
                after: Some(after_sig.clone()),
                requires_version_bump: false,
                requires_migration_note: false,
            }),
            Some(before_sig) if before_sig != after_sig => changes.push(Change {
                kind: ChangeKind::Changed,
                classification: Classification::DocumentedBreakingChange,
                item_path: item_path.clone(),
                before: Some(before_sig.clone()),
                after: Some(after_sig.clone()),
                requires_version_bump: true,
                requires_migration_note: true,
            }),
            Some(_) => {}
        }
    }

    for (item_path, before_sig) in &before {
        if !after.contains_key(item_path) {
            changes.push(Change {
                kind: ChangeKind::Removed,
                classification: Classification::DocumentedBreakingChange,
                item_path: item_path.clone(),
                before: Some(before_sig.clone()),
                after: None,
                requires_version_bump: true,
                requires_migration_note: true,
            });
        }
    }

    changes.sort_by(|a, b| a.item_path.cmp(&b.item_path));

    ChangeReport {
        baseline_commit: baseline_commit.to_string(),
        current_commit: current_commit.to_string(),
        up_to_date: changes.is_empty(),
        changes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::ModuleSurface;

    fn surface_with_fn(module: &str, name: &str, sig: &str) -> Surface {
        let mut ms = ModuleSurface::default();
        ms.functions.insert(name.to_string(), sig.to_string());
        let mut surface = Surface::new();
        surface.insert(module.to_string(), ms);
        surface
    }

    #[test]
    fn detects_an_added_function_as_backward_compatible() {
        let baseline = Surface::new();
        let current = surface_with_fn("query", "example_new_fn", "fn example_new_fn ()");
        let report = diff(&baseline, &current, "aaa", "bbb");
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].kind, ChangeKind::Added);
        assert_eq!(report.changes[0].classification, Classification::BackwardCompatibleAddition);
        assert_eq!(report.changes[0].item_path, "query::example_new_fn");
        assert!(!report.up_to_date);
    }

    #[test]
    fn detects_a_changed_signature_as_documented_breaking_change() {
        let baseline = surface_with_fn("query", "execute", "fn execute () -> Vec < & str >");
        let current = surface_with_fn("query", "execute", "fn execute () -> Vec < Cow < str > >");
        let report = diff(&baseline, &current, "aaa", "bbb");
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].kind, ChangeKind::Changed);
        assert_eq!(report.changes[0].classification, Classification::DocumentedBreakingChange);
        assert!(report.changes[0].requires_version_bump);
        assert!(report.changes[0].requires_migration_note);
    }

    #[test]
    fn detects_a_removed_function_as_documented_breaking_change() {
        let baseline = surface_with_fn("query", "old_fn", "fn old_fn ()");
        let current = Surface::new();
        let report = diff(&baseline, &current, "aaa", "bbb");
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].kind, ChangeKind::Removed);
        assert_eq!(report.changes[0].classification, Classification::DocumentedBreakingChange);
    }

    #[test]
    fn reports_up_to_date_when_nothing_changed() {
        let baseline = surface_with_fn("query", "execute", "fn execute ()");
        let current = surface_with_fn("query", "execute", "fn execute ()");
        let report = diff(&baseline, &current, "aaa", "aaa");
        assert!(report.changes.is_empty());
        assert!(report.up_to_date);
    }
}
