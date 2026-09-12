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
//!
//! Spec 010-ambiguous-parent-resolution extends this module: a `->`
//! expression's *parent*-side segment type that is legal at more than one
//! profile tree position (e.g. `OBX` under both `OBR` and `SPM` in
//! `fixtures/profiles/deep-nested.json`) is resolved per-occurrence via
//! [`resolve_occurrence_node`], using the message's real document order —
//! rather than `HierarchyProfile::node_for`'s O(1) global lookup giving up.
//! Verified against the real Scala engine's own output for this exact
//! scenario (spec 010 research.md #1). The O(1) `node_for` path is
//! completely unchanged for the (common, and today the only exercised)
//! unambiguous case — [`resolve_occurrence_node`] is only consulted when
//! [`HierarchyProfile::is_ambiguous`] says the type actually needs it.

use std::borrow::Cow;
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
    /// shape, not malformed data (spec 002/008 research.md #2). Descendant
    /// matching during the bounded scan (`direct_children_of_type`) never
    /// consults this map — it walks each node's own `children` map
    /// directly, which already disambiguates correctly by construction.
    /// This map serves two purposes: `node_for`'s O(1) lookup for a
    /// *parent*-side type that occupies exactly one position, and
    /// `is_ambiguous`'s check for when a type occupies more than one
    /// (spec 010), in which case the caller must use
    /// [`resolve_occurrence_node`] instead of `node_for`.
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
    /// *parent* side of a `->` expression only, when that type is
    /// unambiguous (occupies exactly one profile position) — an O(1)
    /// lookup, completely unchanged since spec 008. `None` both when the
    /// type is absent from the profile and when it is ambiguous (occupies
    /// more than one position); callers MUST check [`Self::is_ambiguous`]
    /// first and use [`resolve_occurrence_node`] for the ambiguous case
    /// (spec 010) rather than treating this `None` as "no children" —
    /// `node_for` itself still cannot and does not attempt history-aware
    /// disambiguation.
    fn node_for(&self, name: &str) -> Option<usize> {
        match self.by_name.get(name) {
            Some(indices) if indices.len() == 1 => Some(indices[0]),
            _ => None,
        }
    }

    /// True when `name` occupies more than one position in this profile's
    /// tree — the signal a caller uses to choose between `node_for`'s O(1)
    /// lookup (unambiguous) and [`resolve_occurrence_node`]'s per-occurrence,
    /// document-order-driven resolution (ambiguous, spec 010). A name
    /// entirely absent from the profile is not ambiguous (zero positions,
    /// not multiple) — `node_for` already handles that case correctly.
    fn is_ambiguous(&self, name: &str) -> bool {
        matches!(self.by_name.get(name), Some(indices) if indices.len() > 1)
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

/// Resolves `target`'s *actual* profile tree position using the full,
/// history-dependent nearest-enclosing-ancestor walk from the top of the
/// message — mirroring `HL7HierarchyParser.scala`'s single top-down pass
/// exactly (spec 010 research.md #1/#2), rather than `HierarchyProfile`'s
/// static, ambiguity-blind `node_for` lookup. Only ever needed (and only
/// ever called) when `target`'s segment type is ambiguous
/// (`HierarchyProfile::is_ambiguous`) — the unambiguous case keeps using
/// `node_for`'s O(1) lookup, completely untouched by this function
/// (spec 010 FR-006, no regression).
///
/// Unlike `direct_children_of_type`'s bounded scan (which only ever needs
/// to track depth relative to one already-known parent, and so can safely
/// discard a popped level for good once it's been superseded), this walk
/// must never permanently lose a popped level: an unrecognized-anywhere
/// segment restores every level it popped before moving to the next
/// segment, exactly like `HL7HierarchyParser.scala`'s own backup/restore
/// stacks (spec 010 research.md #6) — losing a level here could make a
/// later, legitimately-deeper occurrence unresolvable.
///
/// Returns the resolved node index, or `None` if `target` itself is never
/// recognized as a legal child of anything along the way (FR-004 — the
/// caller treats this identically to a segment type entirely absent from
/// the profile: no children, never an error or a panic).
fn resolve_occurrence_node(scan: &ScanResult<'_>, profile: &HierarchyProfile, target: SegmentSpan) -> Option<usize> {
    let target_line = scan
        .segments
        .iter()
        .position(|s| s.start == target.start)
        .expect("target must come from this ScanResult's segments");

    // Index 0 is the synthetic root (HierarchyProfile::from_json); its
    // children are the top-level segmentDefinition entries (MSH included),
    // matching where HL7HierarchyParser.scala's own walk effectively begins.
    let mut stack: Vec<usize> = vec![0];

    for (i, span) in scan.segments.iter().enumerate() {
        let seg_type = scan.segment_name(span);
        let mut backup: Vec<usize> = Vec::new();
        let mut matched_node: Option<usize> = None;

        loop {
            let top = *stack.last().expect("stack always has at least the synthetic root");
            if let Some(&child_idx) = profile.nodes[top].children.get(seg_type) {
                stack.push(child_idx);
                matched_node = Some(child_idx);
                break;
            } else if stack.len() > 1 {
                backup.push(stack.pop().expect("stack.len() > 1 was just checked"));
            } else {
                // Unrecognized anywhere: restore every level popped while
                // searching, so the next segment sees the same context this
                // one started with (research.md #6) -- never drop this
                // segment's failed search permanently into the live stack.
                while let Some(n) = backup.pop() {
                    stack.push(n);
                }
                break;
            }
        }

        if i == target_line {
            return matched_node;
        }
    }

    unreachable!("target_line must be within scan.segments, per its own precondition")
}

/// Resolves one matching parent occurrence's *direct* children of type
/// `cseg`, via a single bounded forward scan from the line immediately
/// after `parent_span` — never a full-message tree (research.md #1,
/// FR-003). Returns an already type-filtered list, in document order.
///
/// `parent_node` is the caller's already-resolved seed node for this
/// specific `parent_span` occurrence — via `HierarchyProfile::node_for`
/// when its type is unambiguous, or via [`resolve_occurrence_node`] when
/// it isn't (spec 010). This function itself is unchanged either way: it
/// has no opinion on *how* `parent_node` was resolved, only on what its
/// direct children are.
fn direct_children_of_type<'m>(
    scan: &ScanResult<'m>,
    profile: &HierarchyProfile,
    parent_span: SegmentSpan,
    parent_node: usize,
    cseg: &str,
) -> Vec<SegmentSpan> {
    let mut result = Vec::new();
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
) -> Result<Vec<Vec<Cow<'m, str>>>, QueryError> {
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
        let parent_type = scan.segment_name(parent_span);
        // spec 010: an ambiguous parent type needs the message's real
        // document order to resolve which position this specific occurrence
        // occupies; an unambiguous one keeps today's O(1) lookup, byte-for-
        // byte unchanged (FR-006).
        let parent_node = if profile.is_ambiguous(parent_type) {
            resolve_occurrence_node(scan, profile, *parent_span)
        } else {
            profile.node_for(parent_type)
        };
        let Some(parent_node) = parent_node else {
            // Absent from the profile entirely, or (spec 010 FR-004) an
            // ambiguous-type occurrence that doesn't correspond to any
            // legal position given the real message structure -- no
            // children, never an error.
            continue;
        };
        let direct = direct_children_of_type(scan, profile, *parent_span, parent_node, child.segment.name);
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
        let obr_node = profile.node_for("OBR").unwrap();
        let direct = direct_children_of_type(&scan_result, &profile, obr1, obr_node, "OBX");

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
        let obr_node = profile.node_for("OBR").unwrap();
        // NTE is only a child of OBX, not a direct child of OBR.
        let direct_nte = direct_children_of_type(&scan_result, &profile, obr1, obr_node, "NTE");
        assert!(direct_nte.is_empty(), "NTE is a grandchild of OBR, not a direct child");

        let direct_obx = direct_children_of_type(&scan_result, &profile, obr1, obr_node, "OBX");
        assert_eq!(direct_obx.len(), 1, "OBX is still recognized as OBR's direct child");
    }

    #[test]
    fn direct_children_of_type_stops_at_sibling_boundary() {
        let message = "MSH|^~\\&|A|B\nOBR|1\nOBX|1\nOBR|2\nOBX|2\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let profile =
            HierarchyProfile::from_json(r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {}}}}}"#).unwrap();

        let obr1 = scan_result.segments[1];
        let obr_node = profile.node_for("OBR").unwrap();
        let direct = direct_children_of_type(&scan_result, &profile, obr1, obr_node, "OBX");

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
        let obr_node = profile.node_for("OBR").unwrap();
        let direct = direct_children_of_type(&scan_result, &profile, obr1, obr_node, "OBX");

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
        let obr_node = profile.node_for("OBR").unwrap();
        let direct = direct_children_of_type(&scan_result, &profile, obr1, obr_node, "OBX");

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

    // --- spec 010 (T005): resolve_occurrence_node resolves ambiguous
    // parent-side types using the message's real document order ---

    // The same profile shape as fixtures/profiles/deep-nested.json: OBX legal
    // both directly under OBR and under OBR's SPM child; NTE legal both
    // directly under OBR and under OBR's OBX child.
    const DEEP_NESTED_PROFILE: &str = r#"{
        "segmentDefinition": {
            "MSH": {
                "children": {
                    "PID": {},
                    "ORC": {},
                    "OBR": {
                        "children": {
                            "NTE": {},
                            "OBX": {"children": {"NTE": {}}},
                            "SPM": {"children": {"OBX": {}}}
                        }
                    }
                }
            }
        }
    }"#;

    // The same message as fixtures/messages/complex-hierarchy.hl7, verified
    // live against the real gov.cdc:hl7-pet_2.13:1.2.11 engine (spec 010
    // research.md #1).
    const COMPLEX_HIERARCHY_MESSAGE: &str = "MSH|^~\\&|SENDAPP|SENDFAC|RECVAPP|RECVFAC|20260101120500||ORU^R01|MSG00002|P|2.5.1\nPID|1||9990002^^^MRN^MR||SMITH^ALEX^R||19750615|M\nORC|RE||ORDER0002\nOBR|1|ORDER0002||OBR-COMPLEX-TEST^Complex Panel^LN|||20260101120500\nNTE|1|L|Note attached directly to OBR (first)\nNTE|2|L|Note attached directly to OBR (second)\nOBX|1|CE|OBX-A-CODE^Direct Child A^LN||POS||||||F\nOBX|2|CE|OBX-C-CODE^Direct Child C^LN||NEG||||||F\nNTE|1|L|Note attached to OBX-C, not to OBR directly\nSPM|1|SPECIMEN0001||SPM-TYPE^Nasal Swab^LN\nOBX|1|CE|OBX-UNDER-SPM-CODE^Nested Under SPM^LN||POS||||||F\nOBR|2|ORDER0002B||OBR-SECOND-TEST^Second Panel With No Children^LN|||20260101120600\n";

    #[test]
    fn resolve_occurrence_node_distinguishes_obx_under_obr_from_obx_under_spm() {
        let scan_result = crate::scanner::scan(COMPLEX_HIERARCHY_MESSAGE).unwrap();
        let profile = HierarchyProfile::from_json(DEEP_NESTED_PROFILE).unwrap();
        assert!(profile.is_ambiguous("OBX"));

        let obx_line7 = scan_result.segments[6]; // OBX-A-CODE, direct child of OBR
        let obx_line8 = scan_result.segments[7]; // OBX-C-CODE, direct child of OBR
        let obx_line11 = scan_result.segments[10]; // OBX-UNDER-SPM-CODE, child of SPM

        let node7 = resolve_occurrence_node(&scan_result, &profile, obx_line7).unwrap();
        let node8 = resolve_occurrence_node(&scan_result, &profile, obx_line8).unwrap();
        let node11 = resolve_occurrence_node(&scan_result, &profile, obx_line11).unwrap();

        assert_eq!(node7, node8, "both direct-child OBX occurrences resolve to the same OBR-child node");
        assert_ne!(node7, node11, "the SPM-nested OBX occurrence must resolve to a different node");
    }

    #[test]
    fn resolve_occurrence_node_distinguishes_nte_under_obr_from_nte_under_obx() {
        let scan_result = crate::scanner::scan(COMPLEX_HIERARCHY_MESSAGE).unwrap();
        let profile = HierarchyProfile::from_json(DEEP_NESTED_PROFILE).unwrap();
        assert!(profile.is_ambiguous("NTE"));

        let nte_line5 = scan_result.segments[4]; // direct child of OBR
        let nte_line6 = scan_result.segments[5]; // direct child of OBR
        let nte_line9 = scan_result.segments[8]; // child of the second OBX

        let node5 = resolve_occurrence_node(&scan_result, &profile, nte_line5).unwrap();
        let node6 = resolve_occurrence_node(&scan_result, &profile, nte_line6).unwrap();
        let node9 = resolve_occurrence_node(&scan_result, &profile, nte_line9).unwrap();

        assert_eq!(node5, node6, "both direct-child NTE occurrences resolve to the same OBR-child node");
        assert_ne!(node5, node9, "the OBX-nested NTE occurrence must resolve to a different node");
    }

    // research.md #6: an unrecognized-anywhere segment must not permanently
    // discard the levels popped while searching for it -- the next segment
    // must still resolve against the same context as if the unrecognized
    // segment had never been there.
    #[test]
    fn resolve_occurrence_node_restores_context_after_an_unrecognized_segment() {
        let message = "MSH|^~\\&|A|B\nOBR|1\nOBX|1\nZZZ|unrecognized\nNTE|1\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let profile = HierarchyProfile::from_json(
            r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {"children": {"NTE": {}}}, "SPM": {"children": {"OBX": {}}}}}}}"#,
        )
        .unwrap();
        assert!(profile.is_ambiguous("OBX"));

        let nte = scan_result.segments[4];
        assert!(
            resolve_occurrence_node(&scan_result, &profile, nte).is_some(),
            "NTE must still resolve as OBX's child after the unrecognized ZZZ segment, not be lost"
        );
    }

    #[test]
    fn execute_hierarchy_resolves_ambiguous_obx_parent_correctly() {
        let scan_result = crate::scanner::scan(COMPLEX_HIERARCHY_MESSAGE).unwrap();
        let profile = HierarchyProfile::from_json(DEEP_NESTED_PROFILE).unwrap();

        // OBX -> NTE-3: only line 9's NTE is a child of an OBX (the second one).
        let compiled = crate::parser::parse("OBX -> NTE-3").unwrap();
        let result = execute_hierarchy(&scan_result, &compiled, Some(&profile)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0][0].as_ref(), "Note attached to OBX-C, not to OBR directly");

        // OBX[2] -> NTE-3: same result, addressed by index.
        let compiled = crate::parser::parse("OBX[2] -> NTE-3").unwrap();
        let result = execute_hierarchy(&scan_result, &compiled, Some(&profile)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0][0].as_ref(), "Note attached to OBX-C, not to OBR directly");

        // SPM -> OBX-3: the SPM-nested OBX, a completely different position.
        let compiled = crate::parser::parse("SPM -> OBX-3").unwrap();
        let result = execute_hierarchy(&scan_result, &compiled, Some(&profile)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0][0].as_ref(), "OBX-UNDER-SPM-CODE^Nested Under SPM^LN");
    }

    // --- spec 010 (T013/T014): an ambiguous-type occurrence with no legal
    // position given the real message structure yields absence, never a
    // panic (FR-004, Constitution Principle III) ---

    #[test]
    fn resolve_occurrence_node_returns_none_when_no_legal_position_exists() {
        // OBX appears before any OBR/SPM -- the profile has no legal
        // position for it at that point, even though OBX is a recognized
        // (ambiguous) type elsewhere in the same profile.
        let message = "MSH|^~\\&|A|B\nOBX|1\nOBR|1\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let profile = HierarchyProfile::from_json(DEEP_NESTED_PROFILE).unwrap();
        assert!(profile.is_ambiguous("OBX"));

        let misplaced_obx = scan_result.segments[1];
        assert!(resolve_occurrence_node(&scan_result, &profile, misplaced_obx).is_none());
    }

    #[test]
    fn execute_hierarchy_never_panics_for_a_misplaced_ambiguous_parent() {
        let message = "MSH|^~\\&|A|B\nOBX|1\nOBR|1\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let profile = HierarchyProfile::from_json(DEEP_NESTED_PROFILE).unwrap();
        let compiled = crate::parser::parse("OBX -> NTE-3").unwrap();

        let result = execute_hierarchy(&scan_result, &compiled, Some(&profile)).unwrap();
        assert!(result.is_empty(), "a misplaced ambiguous-type occurrence must yield no children, never an error");
    }

    // --- spec 010 (T010): no regression for the unambiguous case (FR-006) ---

    #[test]
    fn is_ambiguous_distinguishes_unambiguous_from_ambiguous_types() {
        let profile = HierarchyProfile::from_json(
            r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {}, "SPM": {"children": {"OBX": {}}}}}}}"#,
        )
        .unwrap();

        assert!(!profile.is_ambiguous("OBR"), "OBR occupies exactly one position");
        assert!(profile.is_ambiguous("OBX"), "OBX is legal both directly under OBR and under SPM");
        assert!(!profile.is_ambiguous("ZZZ"), "an absent type is not ambiguous -- zero positions, not multiple");
    }

    // FR-006: resolving an unambiguous parent type must cost identically
    // whether or not an unrelated ambiguous type exists elsewhere in the
    // same profile -- proves the O(1) node_for path, not the classifier, is
    // what actually runs for OBR here, without relying on noisy wall-clock
    // benchmarking (spec 010 research.md).
    #[test]
    fn unambiguous_parent_resolution_allocation_count_is_unaffected_by_unrelated_ambiguity() {
        use crate::test_alloc::count_allocs;

        let message = "MSH|^~\\&|A|B\nOBR|1\nOBX|1\n";
        let scan_result = crate::scanner::scan(message).unwrap();
        let compiled = crate::parser::parse("OBR[1] -> OBX-5").unwrap();

        let ambiguous_elsewhere = HierarchyProfile::from_json(
            r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {}, "SPM": {"children": {"OBX": {}}}}}}}"#,
        )
        .unwrap();
        let plain = HierarchyProfile::from_json(r#"{"segmentDefinition": {"OBR": {"children": {"OBX": {}}}}}"#)
            .unwrap();
        assert!(ambiguous_elsewhere.is_ambiguous("OBX"));
        assert!(!ambiguous_elsewhere.is_ambiguous("OBR"));

        let allocs_with_unrelated_ambiguity = count_allocs(|| {
            execute_hierarchy(&scan_result, &compiled, Some(&ambiguous_elsewhere)).unwrap();
        });
        let allocs_plain = count_allocs(|| {
            execute_hierarchy(&scan_result, &compiled, Some(&plain)).unwrap();
        });

        assert_eq!(
            allocs_with_unrelated_ambiguity, allocs_plain,
            "resolving an unambiguous parent type (OBR) must cost identically whether or not an unrelated \
             ambiguous type (OBX) exists elsewhere in the same profile"
        );
    }
}
