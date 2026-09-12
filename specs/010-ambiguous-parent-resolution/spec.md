# Feature Specification: Ambiguous-Position Parent Resolution for Hierarchy PATHs

**Feature Branch**: `010-ambiguous-parent-resolution`

**Created**: 2026-09-11

**Status**: Draft

**Input**: User description: "Fix hl7pet-core's hierarchy navigation (crates/core/src/hierarchy.rs, specs 002/008) so that a `->` PATH's parent-side segment type resolves correctly even when that segment type is legal at more than one position in the loaded hierarchy profile's tree -- e.g. OBX can legally be a direct child of OBR or of SPM, and NTE can legally be a direct child of OBR, OBX, or SPM. Today HierarchyProfile::node_for gives up (returns no match) whenever a segment type name maps to more than one tree position ... The fix must resolve each specific segment occurrence's true position in the profile tree using the message's actual structure/order (the same kind of contextual, document-order-driven resolution the engine already does correctly for descendant/child-side matching in direct_children_of_type), not just a static, ambiguity-blind name-to-node lookup. Scope this generally: any segment type that is a legal child under multiple different parent types or multiple different tree positions ... This is a fix to existing Rust Core hierarchy-navigation code (specs 002 hierarchy-semantics and 008 lazy-hierarchy-nav), not new tooling -- belongs in the Rust Core / Engine Migration module."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Correct results when a segment type has multiple legal parent positions (Priority: P1)

A developer has a hierarchy profile where a segment type — say `OBX` — is legally a
direct child of more than one other segment type or tree position (e.g. directly
under `OBR`, and separately under `OBR`'s `SPM` child). They write a `->` PATH using
that type as the parent (e.g. `OBX -> NTE` or `OBX[2] -> NTE`) and run it against a
real message whose actual segment order unambiguously places each `OBX` occurrence in
one specific position or the other. They expect the correct children for whichever
position each occurrence actually occupies — not silence.

**Why this priority**: This is the entire defect being fixed. Today, any `->` PATH
whose parent type is ambiguous in the profile always returns empty results,
regardless of index or message content — a currently-undiscoverable dead end for a
whole class of legitimate profile shapes and PATHs.

**Independent Test**: Load a profile where a segment type is a legal child at two
different tree positions, and a message where document order clearly distinguishes
which position each occurrence of that type occupies. Run a `->` PATH using that type
as the parent and confirm the results match the children of the correct occurrence(s)
— not an empty result.

**Acceptance Scenarios**:

1. **Given** a profile where `OBX` is legal both as a direct child of `OBR` and as a
   direct child of `SPM` (itself a child of `OBR`), and a message with one `OBR`
   containing an `SPM` with its own `OBX`, plus a separate direct `OBX` child of the
   same `OBR`, **When** a PATH addresses `OBX -> <field>` (no index), **Then** the
   results include the children of every `OBX` occurrence, each correctly matched to
   its own actual parent context, not silently dropped.
2. **Given** the same profile and message, **When** a PATH addresses a specific
   indexed occurrence (e.g. `OBX[2] -> <field>`), **Then** the result reflects that
   specific occurrence's real children, matching what a person manually tracing the
   message's structure would find for "the 2nd `OBX` in the message."
3. **Given** a profile where a segment type is legal under three or more different
   parent contexts (e.g. `NTE` under `OBR`, under `OBX`, and under `SPM`), **When** a
   `->` PATH uses `NTE` as the child of a specific, correctly-resolved parent
   occurrence, **Then** only the `NTE` occurrences actually nested under that specific
   parent (per the message's real structure) are returned.

---

### User Story 2 - No regression for already-unambiguous profiles (Priority: P2)

A developer with a profile where every segment type occupies exactly one position in
the tree (the common case today) continues to get exactly the same results as before
this fix.

**Why this priority**: This fix must not destabilize the hierarchy navigation
behavior every existing consumer already depends on; it only needs to additionally
handle the ambiguous case correctly.

**Independent Test**: Re-run the existing hierarchy conformance vectors (unambiguous
profiles) before and after the fix and confirm byte-for-byte identical results.

**Acceptance Scenarios**:

1. **Given** a profile where no segment type repeats at more than one tree position,
   **When** any existing `->` PATH is evaluated against any existing message, **Then**
   the result is identical to the pre-fix result.

---

### User Story 3 - Explicit absence when a message's real structure doesn't resolve an ambiguous occurrence (Priority: P3)

A developer runs a `->` PATH whose parent type is ambiguous in the profile, but the
specific selected occurrence doesn't actually sit under any of that type's legal
positions given the message's real preceding structure (e.g. it appears before any
segment that could establish it as a legal child of anything). They see an explicit
"no match," not an error, a crash, or a misleading guess.

**Why this priority**: Correctness under malformed or unexpected structure matters
less than the core fix (P1) but must not regress the engine's "never raise for
absent data" guarantee.

**Independent Test**: Construct a message where an ambiguous-type segment occurrence
appears in a position the profile doesn't sanction (e.g. before its would-be parent),
run a `->` PATH selecting it, and confirm the result is an explicit empty/no-match
outcome, never an error.

**Acceptance Scenarios**:

1. **Given** an ambiguous-type occurrence that doesn't correspond to any legal
   position given the message's actual structure, **When** a `->` PATH selects it as
   the parent, **Then** the system returns no children for that occurrence (absence),
   never an exception or a panic.

### Edge Cases

- A segment type ambiguous in the profile that never actually appears in a given
  message — the PATH simply has no matching parent occurrences (existing absence
  behavior, unchanged).
- Chains of ambiguity: a segment type ambiguous at one tree position whose correctly
  resolved position itself has a child type that is *also* ambiguous relative to
  other parts of the tree — resolution must work at any nesting depth, not just the
  two-position example above.
- A message whose structure is unusual enough that a segment occurrence could
  plausibly be read two different ways by a human — resolution follows the engine's
  existing nearest-enclosing-ancestor convention (already established for
  descendant/child-side matching, specs `002`/`008`) deterministically, so there is
  always exactly one engine answer even if it might occasionally surprise a reader of
  an unusual message.
- A `segmentDefinition` profile cannot legally list the same child name twice under
  one single parent (JSON object keys are unique) — true ambiguity always spans
  *different* parent chains, never a duplicate entry under one parent.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST resolve a `->` PATH's parent-side segment occurrence to
  its correct position in the hierarchy profile's tree using the message's actual
  segment order/document structure, for any segment type that is legal at more than
  one tree position.
- **FR-002**: This resolution MUST follow the same nearest-enclosing-ancestor matching
  convention the engine already uses for descendant/child-side resolution (specs
  `002`/`008`), so parent-side and child-side resolution remain consistent with each
  other and with already-documented hierarchy semantics.
- **FR-003**: Segment selection/indexing on the parent side of a `->` PATH (`[N]`,
  `$LAST`, `*`, filter clauses) MUST continue to select occurrences by raw segment
  name and document order, unaffected by this fix — only the resolution of *which
  tree position, and therefore which children,* a selected ambiguous-type occurrence
  has changes.
- **FR-004**: When a selected parent occurrence of an ambiguous-position segment type
  does not correspond to any legal position given the message's actual structure, the
  system MUST return no matching children for that occurrence — absence, never an
  error or a panic (Constitution Principle III).
- **FR-005**: The fix MUST correctly handle chains of ambiguity — a segment type
  ambiguous at one level whose resolved position then determines a second,
  also-ambiguous descendant type's legal children — without special-casing a fixed
  nesting depth or a fixed number of ambiguous positions.
- **FR-006**: Every hierarchy PATH that resolves unambiguously today (a segment type
  legal at exactly one profile position) MUST continue to produce identical results
  after this fix — no regression.
- **FR-007**: The fix MUST NOT change the existing 1-based, type-filtered,
  per-parent-occurrence child-index re-basing behavior already documented for spec
  `008` — only which parent occurrences are considered, and which tree position each
  resolves to, may change for ambiguous types.
- **FR-008**: The fix MUST apply uniformly to every place in the engine that maps a
  segment type name to a single hierarchy-profile tree position — not a special case
  for `OBX`/`SPM`/`NTE` specifically, but any profile shape where a type is legal at
  more than one position.

### Key Entities

- **Hierarchy Profile**: The declarative `segmentDefinition` tree of legal
  parent→child segment-type relationships a message is navigated against. A single
  type name may legally occupy more than one position in this tree.
- **Segment Occurrence**: One specific instance of a segment type at a specific point
  in a message's document order. Its *type name* alone may be ambiguous against the
  profile; its *real* tree position is determined by the segments that precede it in
  the actual message.
- **Ambiguous Segment Type**: A segment type that is legal at more than one position
  in a given hierarchy profile's tree (as a child of more than one parent type, or at
  more than one nesting depth).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a message and profile where document order unambiguously
  establishes each occurrence's structural position, a `->` PATH whose parent type is
  legal at multiple profile positions returns the correct matching children in 100%
  of cases — matching what a person manually tracing the message's structure would
  find — instead of today's unconditional empty result.
- **SC-002**: Every hierarchy conformance case that passes today continues to pass,
  byte-for-byte identical, after this fix.
- **SC-003**: A developer testing an ambiguous-position hierarchy PATH against a
  message whose real structure doesn't resolve it sees an explicit no-match outcome
  100% of the time — never an incorrect result, an exception, or a crash.
- **SC-004**: Evaluating a hierarchy PATH with an ambiguous-type parent does not
  measurably regress the throughput/latency already established for hierarchy
  navigation (spec `009`'s benchmarks) — the fix preserves the library's zero-copy,
  lazy-evaluation performance profile rather than trading correctness for a full
  eager tree build.

## Assumptions

- This fix is scoped to Rust Core hierarchy-navigation code (`crates/core/src/
  hierarchy.rs` and its fixtures/tests, owned by specs `002`/`008`) — no change to
  PATH grammar (spec `001`) and no new surface in the Python bindings (spec `6000`),
  which already call through to `execute_hierarchy`/`get_value_hierarchy` unchanged.
- Segment selection/indexing semantics for the *parent* side of `->` are unchanged
  (FR-003) — this fix is purely about correctly resolving which profile tree position
  a selected parent occupies, not about which occurrence gets selected.
- Because ambiguous-parent-type hierarchy PATHs always silently return no results
  today (a real, previously undocumented-as-fixable gap — discovered via this
  project's own playground tool, spec `9000`, using `fixtures/profiles/
  deep-nested.json`), making them resolve correctly closes an existing gap rather
  than changing any currently-meaningful result. No version bump or migration guide
  is expected to be required under the Backward-Compatible-Additions convention —
  nothing meaningful is taken away, previously-empty results become correct ones —
  but this should be confirmed during planning, not assumed final here.
- **Resolved during planning** (research.md #1): the real Scala engine
  (`gov.cdc:hl7-pet_2.13:1.2.11`) *does* have defined, correct behavior for this exact
  scenario — verified with a live run of `HL7HierarchyParser.parseMessageHierarchy`
  against `fixtures/messages/complex-hierarchy.hl7` + `fixtures/profiles/
  deep-nested.json`. It never encounters this ambiguity in the first place, because
  it builds its whole output via one top-down document-order walk rather than a
  name→position lookup. This confirms the fix is parity restoration, not a new
  capability beyond Scala.
- New hierarchy conformance vectors (messages + profiles specifically exercising
  ambiguous segment types, extending or sitting alongside `fixtures/profiles/
  deep-nested.json`) will be needed; these are a planning/implementation concern; not
  scoped further here.
