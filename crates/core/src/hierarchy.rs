//! Lazy hierarchy navigation (spec 008-lazy-hierarchy-nav).
//!
//! Resolves a compiled PATH's `child` (the `->` hop, spec 006) against a
//! scanner's `ScanResult` (spec 005) and a declarative `HierarchyProfile`
//! (this spec's own deliverable), without ever materializing a full segment
//! tree over the message — a bounded, per-parent-occurrence forward scan
//! instead (contracts/hierarchy-api.md, research.md #1).
//!
//! Deliberately out of scope, per spec.md's Clarifications: multi-hop `->`
//! chaining (deferred to a future spec — `CompiledPath.child`'s type stays
//! non-recursive) and reproducing the real Scala engine's documented
//! child-index bug (spec 002 Section A.4) — this module fixes it instead,
//! as a documented Breaking Change (`ROADMAP.md`).

use std::collections::HashMap;

use serde::Deserialize;

use crate::parser::{CompiledPath, SegIndex};
use crate::query::{self, QueryError};
use crate::scanner::{ScanResult, SegmentSpan};

/// The executor's non-panic failure output for a malformed *profile* —
/// never returned by [`execute_hierarchy`] itself (research.md #3); that
/// function reuses [`QueryError`] exclusively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// The input is not well-formed JSON, or does not match
    /// `segmentDefinition`'s expected shape. Wraps `serde_json::Error`'s
    /// `Display` output as an owned `String` — the `serde_json::Error` type
    /// itself never crosses this module's public boundary (FR-014).
    InvalidJson { message: String },
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileError::InvalidJson { message } => {
                write!(f, "invalid hierarchy profile JSON: {message}")
            }
        }
    }
}

impl std::error::Error for ProfileError {}

/// Crate-private mirror of `segmentDefinition`'s recursive JSON shape
/// (`fixtures/profiles/{basic-two-level,deep-nested}.json`). Never `pub` —
/// `HierarchyProfile::from_json` converts into the plain `HierarchyProfile`
/// arena before returning, so no `serde`/`serde_json` type crosses this
/// module's public boundary (FR-014).
#[derive(Debug, Deserialize)]
struct RawProfile {
    #[serde(rename = "segmentDefinition")]
    segment_definition: HashMap<String, RawSegmentDef>,
}

#[derive(Debug, Deserialize)]
struct RawSegmentDef {
    /// Read but never consulted — navigation never enforces cardinality
    /// (spec 002 Section A.3); that is the Validation module's job
    /// (Roadmap 2000-2999), not this spec's (research.md #2).
    #[allow(dead_code)]
    cardinality: Option<String>,
    #[serde(default)]
    children: HashMap<String, RawSegmentDef>,
}

#[derive(Debug, Clone)]
struct ProfileNode {
    children: HashMap<String, usize>,
    parent: Option<usize>,
}

/// The Rust representation of a `segmentDefinition` map — a small node
/// arena, independent of any specific message, used purely as a
/// legal-child lookup table (FR-004). Opaque to callers: no field is
/// public; construct via [`HierarchyProfile::from_json`].
#[derive(Debug, Clone)]
pub struct HierarchyProfile {
    /// Index `0` is always the synthetic root.
    nodes: Vec<ProfileNode>,
    /// Every node's position(s) by name. A name maps to more than one index
    /// when that segment type occupies more than one place in the tree
    /// (e.g. `deep-nested.json`'s `OBX`, legal both directly under `OBR`
    /// and under `OBR`'s `SPM` child, and `NTE`, legal both directly under
    /// `OBR` and under `OBR`'s `OBX` child) — a normal, common profile
    /// shape, not malformed data (research.md #2). Descendant matching
    /// during the bounded scan (`direct_children_of_type`) never consults
    /// this map — it walks each node's own `children` map directly, which
    /// already disambiguates correctly by construction. This map exists
    /// only to resolve a `->` expression's *parent*-side type into its one
    /// starting node.
    by_name: HashMap<String, Vec<usize>>,
}

impl HierarchyProfile {
    /// Parses `json`'s `segmentDefinition` object into a node arena, or a
    /// located [`ProfileError`] — never a panic. Construction is
    /// all-or-nothing: never a partially built profile.
    pub fn from_json(json: &str) -> Result<Self, ProfileError> {
        let raw: RawProfile = serde_json::from_str(json)
            .map_err(|e| ProfileError::InvalidJson { message: e.to_string() })?;

        let mut nodes = vec![ProfileNode { children: HashMap::new(), parent: None }];
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();

        for (name, def) in &raw.segment_definition {
            let idx = build_node(&mut nodes, &mut by_name, 0, name, def);
            nodes[0].children.insert(name.clone(), idx);
        }

        Ok(HierarchyProfile { nodes, by_name })
    }

    /// Resolves a segment type to its one starting node, for use as the
    /// *parent* side of a `->` expression only. `None` both when the type
    /// is absent from the profile and when it occupies more than one
    /// position (ambiguous parent placement, unresolvable by this spec's
    /// lazy design without a full-history replay, research.md #1) — both
    /// cases correctly fold into FR-006's "matching parent occurrence has
    /// zero qualifying children" outcome at the call site, never a panic
    /// and never a guess. No existing conformance vector exercises an
    /// ambiguous *parent* type — only descendant types repeat in practice.
    fn node_for(&self, name: &str) -> Option<usize> {
        match self.by_name.get(name) {
            Some(indices) if indices.len() == 1 => Some(indices[0]),
            _ => None,
        }
    }

    /// Strict ancestors of `node` — its parent, grandparent, ..., up to and
    /// including the synthetic root. Does not include `node` itself.
    /// `O(profile depth)`, computed on demand (data-model.md).
    fn ancestor_chain(&self, node: usize) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut current = self.nodes[node].parent;
        while let Some(idx) = current {
            chain.push(idx);
            current = self.nodes[idx].parent;
        }
        chain
    }
}

/// Recursively builds `raw`'s subtree into `nodes`, rooted at a new child of
/// `parent`. A name recurring elsewhere in the tree is not an error
/// (research.md #2) — `by_name` simply records every position a name
/// occupies.
fn build_node(
    nodes: &mut Vec<ProfileNode>,
    by_name: &mut HashMap<String, Vec<usize>>,
    parent: usize,
    name: &str,
    raw: &RawSegmentDef,
) -> usize {
    let idx = nodes.len();
    nodes.push(ProfileNode { children: HashMap::new(), parent: Some(parent) });
    by_name.entry(name.to_string()).or_default().push(idx);

    for (child_name, child_raw) in &raw.children {
        let child_idx = build_node(nodes, by_name, idx, child_name, child_raw);
        nodes[idx].children.insert(child_name.clone(), child_idx);
    }

    idx
}

/// Resolves one matching parent occurrence's *direct* children of type
/// `cseg`, via a single bounded forward scan from the line immediately
/// after `parent_span` — never a full-message tree (research.md #1,
/// FR-003). Returns an already type-filtered list, in document order.
fn direct_children_of_type<'m>(
    scan: &ScanResult<'m>,
    profile: &HierarchyProfile,
    parent_span: SegmentSpan,
    cseg: &str,
) -> Vec<SegmentSpan> {
    let mut result = Vec::new();

    let parent_type = scan.segment_name(&parent_span);
    let Some(parent_node) = profile.node_for(parent_type) else {
        // The parent segment type isn't in the profile at all -- it has no
        // tree position, so it cannot have any recognized children.
        return result;
    };
    let ancestors = profile.ancestor_chain(parent_node);

    let parent_line = scan
        .segments
        .iter()
        .position(|s| s.start == parent_span.start)
        .expect("parent_span must come from this ScanResult's segments");

    let mut stack = vec![parent_node];

    for span in &scan.segments[parent_line + 1..] {
        let seg_type = scan.segment_name(span);
        loop {
            let top = *stack.last().expect("stack always has at least parent_node");
            if let Some(&child_idx) = profile.nodes[top].children.get(seg_type) {
                stack.push(child_idx);
                if stack.len() == 2 && seg_type == cseg {
                    result.push(*span);
                }
                break;
            } else if stack.len() > 1 {
                stack.pop();
                continue;
            } else {
                // Local floor reached (stack is back down to [parent_node])
                // and it still doesn't match. Distinguish "exited the
                // parent's subtree" from "unrecognized everywhere" using
                // the static ancestor chain (research.md #1).
                let exits_subtree =
                    ancestors.iter().any(|&a| profile.nodes[a].children.contains_key(seg_type));
                if exits_subtree {
                    return result;
                }
                // Unrecognized anywhere (spec 002 Section A.1 case 4(b)):
                // silently drop this line, stack unchanged, keep scanning.
                break;
            }
        }
    }

    result
}

/// Applies a child-side `SEG_IDX` (`csegIdx`) to one parent occurrence's
/// already type-filtered `direct_children_of_type` output — corrected per
/// FR-007 (type-filtered and re-based *before* this call, per-parent, never
/// combined across parents; 1-based here, matching every other `SEG_IDX` in
/// the engine).
fn apply_child_index<'m>(
    scan: &ScanResult<'m>,
    candidates: Vec<SegmentSpan>,
    index: Option<&SegIndex<'_>>,
) -> Result<Vec<SegmentSpan>, QueryError> {
    match index {
        None | Some(SegIndex::Star) => Ok(candidates),
        Some(SegIndex::Numeric(n)) => {
            let idx = *n as usize;
            Ok(if idx >= 1 && idx <= candidates.len() {
                vec![candidates[idx - 1]]
            } else {
                vec![]
            })
        }
        Some(SegIndex::Last) => Ok(candidates.last().copied().into_iter().collect()),
        Some(SegIndex::Filter(clause)) => {
            let mut selected = Vec::new();
            for span in candidates {
                if query::filter_matches(scan, &span, clause)? {
                    selected.push(span);
                }
            }
            Ok(selected)
        }
    }
}

/// Executes `path` against `scan`, resolving `path.child` (the `->` hop)
/// when present, using `profile` as the legal-child lookup table
/// (contracts/hierarchy-api.md). A flat `path` (`child: None`) delegates to
/// [`query::execute`] unchanged, `profile` ignored. A hierarchy `path` with
/// `profile: None` yields `Ok(vec![])` (FR-009, spec 002 Section A.5's
/// static-mode fallback) — the parent and child sides are never
/// independently evaluated as flat paths.
pub fn execute_hierarchy<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
    profile: Option<&HierarchyProfile>,
) -> Result<Vec<Vec<&'m str>>, QueryError> {
    let Some(child) = path.child.as_ref() else {
        return query::execute(scan, path);
    };
    let Some(profile) = profile else {
        return Ok(vec![]);
    };

    let parent_candidates =
        query::resolve_segment_candidates(scan, path.segment.name, path.segment.index.as_ref())?;

    let mut selected_children: Vec<SegmentSpan> = Vec::new();
    for parent_span in &parent_candidates {
        let direct = direct_children_of_type(scan, profile, *parent_span, child.segment.name);
        let chosen = apply_child_index(scan, direct, child.segment.index.as_ref())?;
        selected_children.extend(chosen);
    }

    let mut result = Vec::with_capacity(selected_children.len());
    for span in &selected_children {
        let segment_content = &scan.message[span.start..span.end];
        let segment_name = scan.segment_name(span);
        let values =
            query::resolve_field_values(child.field.as_ref(), segment_content, segment_name, &scan.delimiters);
        if !values.is_empty() {
            result.push(values);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- US1 (T015): direct_children_of_type's core rules ---

    #[test]
    fn direct_children_of_type_records_direct_child() {
        let message = "MSH|^~\\&|A|B\nOBR|1\nOBX|1\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let profile =
            HierarchyProfile::from_json(r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {}}}}}"#).unwrap();

        let obr1 = scan_result.segments[1];
        let direct = direct_children_of_type(&scan_result, &profile, obr1, "OBX");

        assert_eq!(direct.len(), 1);
        assert_eq!(scan_result.segment_name(&direct[0]), "OBX");
    }

    #[test]
    fn direct_children_of_type_excludes_grandchild() {
        let message = "MSH|^~\\&|A|B\nOBR|1\nOBX|1\nNTE|1\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let profile = HierarchyProfile::from_json(
            r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {"children": {"NTE": {}}}}}}}"#,
        )
        .unwrap();

        let obr1 = scan_result.segments[1];
        // NTE is only a child of OBX, not a direct child of OBR.
        let direct_nte = direct_children_of_type(&scan_result, &profile, obr1, "NTE");
        assert!(direct_nte.is_empty(), "NTE is a grandchild of OBR, not a direct child");

        let direct_obx = direct_children_of_type(&scan_result, &profile, obr1, "OBX");
        assert_eq!(direct_obx.len(), 1, "OBX is still recognized as OBR's direct child");
    }

    #[test]
    fn direct_children_of_type_stops_at_sibling_boundary() {
        let message = "MSH|^~\\&|A|B\nOBR|1\nOBX|1\nOBR|2\nOBX|2\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let profile =
            HierarchyProfile::from_json(r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {}}}}}"#).unwrap();

        let obr1 = scan_result.segments[1];
        let direct = direct_children_of_type(&scan_result, &profile, obr1, "OBX");

        assert_eq!(direct.len(), 1, "the second OBR's own OBX child must not be included");
        assert_eq!(direct[0].start, scan_result.segments[2].start, "must be the first OBR's OBX, not the second's");
    }

    #[test]
    fn direct_children_of_type_drops_unrecognized_segment_and_continues() {
        let message = "MSH|^~\\&|A|B\nOBR|1\nZZZ|unrecognized\nOBX|1\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let profile =
            HierarchyProfile::from_json(r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {}}}}}"#).unwrap();

        let obr1 = scan_result.segments[1];
        let direct = direct_children_of_type(&scan_result, &profile, obr1, "OBX");

        assert_eq!(direct.len(), 1, "ZZZ (unrecognized anywhere) must be silently dropped, not end the scan");
    }

    // SC-002: a large tail of segments after the boundary must never affect
    // the result -- the scan stops the moment the boundary is reached,
    // never visiting the tail.
    #[test]
    fn direct_children_of_type_ignores_lines_past_the_boundary_regardless_of_tail_size() {
        let profile =
            HierarchyProfile::from_json(r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {}}}}}"#).unwrap();

        let mut message = String::from("MSH|^~\\&|A|B\nOBR|1\nOBX|1\nOBR|2\n");
        for i in 0..2000 {
            message.push_str(&format!("OBX|{i}\n"));
        }

        let scan_result = crate::scanner::scan(&message).unwrap();
        let obr1 = scan_result.segments[1];
        let direct = direct_children_of_type(&scan_result, &profile, obr1, "OBX");

        assert_eq!(direct.len(), 1, "only OBR[1]'s own OBX child, none of the 2000 OBX lines after OBR[2]");
    }

    // FR-009: `->` with no profile supplied yields no match, without
    // evaluating either side as an independent flat path.
    #[test]
    fn execute_hierarchy_without_profile_is_empty() {
        let message = "MSH|^~\\&|A|B\nOBR|1\nOBX|1\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let compiled = crate::parser::parse("OBR[1] -> OBX-1").unwrap();

        let result = execute_hierarchy(&scan_result, &compiled, None).unwrap();
        assert!(result.is_empty());
    }

    // --- US2 (T021): malformed profile never panics ---

    #[test]
    fn from_json_rejects_invalid_json_without_panicking() {
        assert!(matches!(HierarchyProfile::from_json("not json"), Err(ProfileError::InvalidJson { .. })));
        assert!(matches!(
            HierarchyProfile::from_json(r#"{"segmentDefinition": "not an object"}"#),
            Err(ProfileError::InvalidJson { .. })
        ));
        assert!(matches!(HierarchyProfile::from_json(""), Err(ProfileError::InvalidJson { .. })));
    }

    // A segment type repeated at multiple positions (deep-nested.json's real
    // shape: OBX legal both directly under OBR and under OBR's SPM child) is
    // valid, common profile data -- not malformed (research.md #2's
    // corrected design).
    #[test]
    fn from_json_accepts_a_segment_type_repeated_at_multiple_positions() {
        let json = r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {}, "SPM": {"children": {"OBX": {}}}}}}}"#;
        assert!(HierarchyProfile::from_json(json).is_ok());
    }
}
