# Quickstart: Validating This Spec's Deliverables

This is a documentation/data feature — there's no app to run. "Validation" here means
proving the semantics document and conformance vectors actually satisfy the spec's
Success Criteria (SC-001 through SC-005). Each scenario below is runnable and maps to
one SC.

## Prerequisites

- A local checkout of the parity-target Scala repo. One already exists at
  `/Users/m/projects/personal/HL7-PET` (remote `github.com/mscaldas2012/HL7-PET`) and
  was used directly for this plan's research — reuse it if still present; otherwise
  fall back to spec `001`'s scratch-clone approach (`research.md` there, Decision 3),
  including its three documented local-only `sbt`/JDK fixes.
- `sbt` + a JVM (for SC-004).
- `python3` (for the JSON Schema check below; any JSON Schema validator works).

None of these are project dependencies — they're one-time tools for producing and
verifying this spec's deliverables.

## Scenario 1 — Semantics document covers `SPEC.md`'s hierarchy prose (SC-001)

Every hierarchy-mode behavior described in `SPEC.md` §3.3 (Hierarchy Mode vs. Static
Mode) and §4.1 (constructors, `->`) must be traceable to a section in
`contracts/hierarchy-semantics.md`.

**Manual check**: for each hierarchy-related sentence in `SPEC.md` §3.3/§4.1 (e.g.
"Hierarchy mode... builds a tree... using the profile's `segmentDefinition`", "Parent→
child path expressions become available and correctly scope child segments to their
parent occurrence"), find the corresponding subsection in
`contracts/hierarchy-semantics.md` Section A. Record any sentence with no
corresponding subsection — SC-001 fails until every one is covered.

## Scenario 2 — Profile and multi-level decisions are both settled (SC-002)

```bash
grep -n "^### B\." specs/002-hierarchy-semantics/contracts/hierarchy-semantics.md
```

**Expected outcome**: both `B.1 Profile Requirement` and `B.2 Multi-Level Navigation`
print, each followed by a `**Decision**:` line with no `NEEDS CLARIFICATION` or
"TBD" language. If either section is missing a definite decision statement, SC-002
fails.

## Scenario 3 — Conformance vectors are schema-valid

Every file under `vectors/` must validate against
`contracts/hierarchy-conformance-vector.schema.json`.

```bash
python3 -c "
import json, glob
try:
    import jsonschema
except ImportError:
    raise SystemExit('pip install jsonschema, or substitute any other JSON Schema validator')

schema = json.load(open('specs/002-hierarchy-semantics/contracts/hierarchy-conformance-vector.schema.json'))
validator = jsonschema.Draft202012Validator(schema)

for f in glob.glob('specs/002-hierarchy-semantics/vectors/*.json'):
    for record in json.load(open(f)):
        errors = list(validator.iter_errors(record))
        if errors:
            print(f'{f}: {record.get(\"id\", \"?\")}: {[e.message for e in errors]}')
print('done')
"
```

**Expected outcome**: no errors printed. Every vector's `id` is unique across all
files, and does not collide with spec `001`'s `path-*` ids.

## Scenario 4 — Every semantic rule has a vector (SC-003)

```bash
python3 -c "
import json, glob

expected_rules = {
    'A.1-nearest-enclosing-ancestor', 'A.1-unrecognized-segment-dropped',
    'A.2-single-hop-basic', 'A.2-zero-children', 'A.2-multi-parent-combined-children',
    'A.3-cardinality-not-enforced-by-navigation', 'A.4-cross-parent-child-indexing',
    'A.5-static-mode-fallback', 'A.6-chained-arrow-silently-empty',
}
# B.2-multi-level-navigation only required if contracts/hierarchy-semantics.md
# Section B.2 recommends inclusion (it does, as of this plan) — see below.

covered = set()
for f in glob.glob('specs/002-hierarchy-semantics/vectors/*.json'):
    for record in json.load(open(f)):
        covered.update(record.get('semantic_rules', []))

missing = expected_rules - covered
print('missing:', missing if missing else 'none')
"
```

**Expected outcome**: `missing: none`. Since `contracts/hierarchy-semantics.md` Section
B.2 recommends including multi-level navigation, at least one vector tagged
`B.2-multi-level-navigation` is also required.

## Scenario 5 — Vectors verified against the real Scala library (SC-004)

For each conformance vector:

1. Point at the local checkout (or a fresh clone if unavailable):
   ```bash
   cd /Users/m/projects/personal/HL7-PET
   ```
2. Using the vector's `profile_ref` and `message_ref` content, call the real library
   in hierarchy mode (`new HL7ParseUtils(message, profile, true)`) with the vector's
   `path` and `method`, passing `flags` if present.
3. Compare the actual output to the vector's `expected` field.
   - **Match**: mark the vector `Verified` (`data-model.md`'s lifecycle).
   - **Mismatch**: do not silently fix the vector. Open a `[NEEDS CLARIFICATION]` per
     spec FR-011, resolve it, then re-verify.

**Expected outcome**: 100% of vectors reach `Verified` before this spec is considered
complete (SC-004). Note: any vector tagged `A.6-chained-arrow-silently-empty` or
`B.2-multi-level-navigation` is inherently testing *current* (pre-Rust) Scala
behavior only for the former — the latter (multi-level) has no real implementation to
verify against yet (spec `008` doesn't exist), so those vectors are recorded as
"designed expected behavior for the future Rust core" rather than "verified against
Scala," and MUST be labeled as such rather than silently mixed in with SC-004's
Scala-verified set.

## Scenario 6 — Child-indexing limitation is actually reproducible (A.4)

Confirms `research.md` Decision 3 / `contracts/hierarchy-semantics.md` Section A.4 is
real, not a documentation error — this is the highest-value scenario to actually run
against the real library, since A.4 describes behavior with no existing test coverage.

**Manual check, part 1 (type mixing)**: construct a synthetic message with one `OBR`
occurrence whose children (per profile) interleave types, e.g. `NTE`, `OBX`, `NTE`,
`OBX` in that document order. Run `OBR -> OBX[1]` in hierarchy mode. Per Section A.4,
this should NOT reliably return the 2nd `OBX` (position 3 in document order); it
compares `1` against raw position `1` in the mixed list, which is the *first* `NTE`,
not an `OBX` at all — so the expected outcome per A.4 is an empty result. If the real
library instead returns the 2nd `OBX`, Section A.4's point 2 is wrong and must be
corrected.

**Manual check, part 2 (off-by-one)**: construct a message with one `OBR` occurrence
whose children are all `OBX` (no type mixing), three of them. Run `OBR -> OBX[1]`. Per
Section A.4 point 3 (no `-1` adjustment), this should return the *2nd* `OBX`
(0-based position 1), not the 1st `OBX` a 1-based reading of `OBX[1]` would suggest. If
the real library returns the 1st `OBX` instead, Section A.4's point 3 is wrong and must
be corrected — in that case, re-check whether some upstream layer normalizes `csegIdx`
before it reaches `getChildrenValues` that this research missed.
