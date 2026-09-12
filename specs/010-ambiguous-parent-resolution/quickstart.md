# Quickstart: Ambiguous-Position Parent Resolution for Hierarchy PATHs

Validates the fix end-to-end against spec.md's user stories. Run from the repo root,
on this feature's branch.

## Prerequisites

```bash
cargo build -p hl7pet-core
```

No new fixtures, profile, or message files are needed — this fix is verified against
fixtures already in the repo: `fixtures/messages/complex-hierarchy.hl7` +
`fixtures/profiles/deep-nested.json` (the exact pair where `OBX` is legal under both
`OBR` and `SPM`, and `NTE` is legal under both `OBR` and `OBX`).

## Story 1 (P1) — correct results for an ambiguous parent type

Using the `hl7pet` dev CLI (`crates/cli`, spec `008`'s `--profile` flag) or a direct
`hl7pet_core::execute_hierarchy` call in a test:

1. Run PATH `OBX -> NTE-3` against `complex-hierarchy.hl7` with `deep-nested.json`
   loaded.
   **Expect**: exactly one result — `"Note attached to OBX-C, not to OBR directly"`
   (line 9) — the `NTE` actually nested under the second `OBX` (line 8), not the two
   `NTE`s directly under `OBR` (lines 5-6) and not the `OBX` under `SPM` (line 11),
   which has no `NTE` child at all.
2. Run PATH `OBX[2] -> NTE-3` against the same pair.
   **Expect**: the same single result as step 1 — `OBX[2]` (the 2nd raw `OBX` in the
   message, i.e. line 8) resolves to its real position as a direct child of `OBR`,
   and its `NTE` child (line 9) is found.
3. Run PATH `SPM -> OBX-3` against the same pair.
   **Expect**: one result, `"OBX-UNDER-SPM-CODE^Nested Under SPM^LN"` (line 11) —
   confirms the *other* ambiguous position (`OBX` under `SPM`) also resolves
   correctly, and independently of step 1/2's `OBX`-under-`OBR` result.
4. Compare all three results against the real Scala engine's own tree for this exact
   message/profile pair (research.md #1's live-verified output) and confirm agreement.

## Story 2 (P2) — no regression for unambiguous profiles

1. Re-run every existing vector in `fixtures/vectors/hierarchy/complex.json` and
   `fixtures/vectors/hierarchy/basic.json` (or equivalent) — all of which use `OBR`
   (unambiguous in `deep-nested.json`/`basic-two-level.json`) as their parent type.
   **Expect**: byte-for-byte identical `expected`/`expected_lines` values to before
   this fix, including the two documented-limitation vectors `hier-009`/`hier-010`
   (multi-hop chaining — unrelated to this fix, unchanged).
2. Re-run `cargo bench -p hl7pet-core` and compare hierarchy-mode figures against
   spec `009`'s existing baseline.
   **Expect**: no measurable regression for these unambiguous-parent-type benchmarks
   (SC-004) — the fast path they exercise is untouched code.

## Story 3 (P3) — explicit absence for an unresolvable ambiguous occurrence

1. Construct (or reuse, if one exists) a message where an `OBX` occurrence appears
   somewhere `deep-nested.json` doesn't sanction as a legal position given the real
   preceding structure (e.g. before any `OBR` at all).
2. Run any `->` PATH selecting that occurrence as the parent.
   **Expect**: an explicit no-match result (`Ok(vec![])` at the Rust level; `None` at
   the Python binding level per spec `6000`'s existing `owned_rows` mapping) — never
   a panic, never an incorrect guess.

## Automated checks

```bash
cargo test -p hl7pet-core
cargo clippy --workspace --all-targets
```

Covers the new unit tests and the extended `fixtures/vectors/hierarchy/` entries via
spec `008`'s existing `hierarchy_vectors` integration test.
