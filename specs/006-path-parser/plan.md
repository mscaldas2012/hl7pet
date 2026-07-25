# Implementation Plan: PATH Parser

**Branch**: `006-path-parser` | **Date**: 2026-07-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/006-path-parser/spec.md`

## Summary

Implement `hl7pet-core`'s PATH parser: a hand-written recursive-descent parser that
turns a raw PATH string into a structured, reusable `CompiledPath` (segment name,
segment index selector, optional field/component/subcomponent expression, optional
filter clause, optional single-hop hierarchy child) strictly per the grammar spec `001`
already documented (`contracts/path-grammar.md`), or into a located `ParseError` that
names the violated grammar rule — never a panic, never a partial result. Parsing is a
pure function of the PATH string alone (no message, no scanner offsets, no hierarchy
profile), so a `CompiledPath` can be produced once and reused for any number of later
evaluations. This closes the exact defect spec `001`'s grammar tightened but nothing
yet enforces: today's Scala engine's regex accepts syntax like `PID[ABC]-1` or
`OBX[@3=='9945-3']-5` and only fails with an uncaught exception once evaluation runs
(`contracts/path-grammar.md` Notes #2/#3) — this parser rejects both at parse time
instead. The grammar's `CHILD_PATH` production remains single-hop only per spec `001`'s
own Non-Goals; multi-hop chaining is explicitly out of scope here.

## Technical Context

**Language/Version**: Rust, stable channel, edition 2021 — same as spec `005`
(`crates/core`'s existing crate); no new MSRV decision needed (spec `005` research.md
#1 already settled "no pin," not re-litigated here).

**Primary Dependencies**: None at runtime — the parser lives in the existing
dependency-free `hl7pet-core` crate. `serde`/`serde_json` (already a dev-dependency)
are reused by the new integration test to deserialize `fixtures/vectors/path/*.json`.
No parser-combinator crate (`nom`, `pest`) is introduced (research.md #1).

**Storage**: N/A — input is an in-memory `&str` (the PATH string), output is an
in-memory `CompiledPath` borrowing from it; both ephemeral.

**Testing**: `cargo test` — unit tests colocated in `parser.rs` for individual grammar
productions (segment index forms, field expressions, filter clauses, hierarchy hop),
plus a new integration test (`crates/core/tests/parser_vectors.rs`) that loads every
entry in `fixtures/vectors/path/valid.json` and `invalid.json` and asserts `parse()`
succeeds/fails exactly as each vector's `expected` field implies (research.md #2).

**Target Platform**: Any platform Rust `stable` supports — pure computation over `&str`
with no OS-specific behavior, same as spec `005`.

**Project Type**: Library. Extends the existing `hl7pet-core` crate (`crates/core/`);
no new workspace member.

**Performance Goals**: No numeric throughput target in this spec — full comparative
benchmarking against the Scala baseline (spec `004`) remains deferred to spec `009`,
once specs `006`-`008` together produce an operation comparable to the Scala baseline's
measured `getValue`/`getFirstValue` calls (spec `005`'s plan.md set this precedent).
This spec's own performance claim (SC-004) is structural, not throughput-based: a
`CompiledPath` is parsed exactly once and carries no method that triggers re-parsing on
reuse (research.md #5).

**Constraints**: Parser MUST NOT panic for any input, well-formed or not (FR-006,
Constitution Principle III); a parse call MUST produce exactly a `CompiledPath` or a
`ParseError`, never both, never neither (FR-007); parsing MUST NOT depend on any HL7
message, scanner output, or hierarchy profile (FR-009); MUST NOT copy PATH substrings
where a borrowed slice suffices (FR-011, Constitution Principle II); accepts/rejects
exactly per `contracts/path-grammar.md` — no broader, no narrower (FR-001).

**Scale/Scope**: Operates on one PATH string at a time. Validated against
`fixtures/vectors/path/`'s existing 17 vectors (11 valid, 6 invalid, spec `001`) plus 4
new vectors this spec adds (FR-012): OR'd filter values, a filter with a subcomponent,
whitespace-tolerant filter operator, and rejection of a multi-hop hierarchy chain.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Applies? | Assessment |
|---|---|---|
| I. Path Contract Stability | **Yes — this spec is the primary code-level embodiment of it** | This parser is the first place spec `001`'s tightened grammar (`SEG` alpha-first, `SEG_IDX`/`FIELD_IDX` parse-time rejection, six-token `OPERATOR` set — `ROADMAP.md`'s Documented Breaking Changes table) is actually enforced rather than just documented. FR-001 requires accepting/rejecting *exactly* what `contracts/path-grammar.md` defines, no more, no less — any drift here would silently break the contract Principle I protects. |
| II. Zero-Copy & Lazy Evaluation | **Yes** | `CompiledPath` borrows segment names and filter values from the original PATH string rather than copying them (FR-011, data-model.md); no field/component text is duplicated, and no evaluation against message data happens here at all (that stays spec `007`'s job). |
| III. Explicit, Exception-Free Data Absence | **Yes** | A malformed PATH is a violated structural precondition on the *string itself* (not "no data present"), so it correctly surfaces as `Result<CompiledPath, ParseError>` — never a panic (FR-006) and never a partial/default result alongside an error (FR-007). |
| IV. Multi-Language Interoperability | Not yet applicable | No FFI boundary exists yet in `crates/core` (Python/Java bindings are Migration Plan Phase 5); nothing to keep in parity at this stage — same assessment spec `005`'s plan.md made. |
| V. Conformance Through Declarative Profiles & Documented Limitations | Partially | The parser is not profile-driven (no `segmentDefinition` involved — Validation module, 2000-2999, is unrelated). It does actively document, rather than silently carry forward, a real scope boundary: multi-hop `->` chaining is explicitly rejected with a specific error rather than silently mis-parsed or partially accepted (spec.md Edge Cases), consistent with this principle's spirit. |
| Performance & Portability Standards | **Yes** | Phase 1 deliverables (specs `001`-`004`) and spec `005` are all `Complete`/`Implemented` per `ROADMAP.md`'s Status table, satisfying the Development Workflow gate. Full baseline-comparison benchmarking is deferred to spec `009` with the same explicit, already-documented reason spec `005` used — not an unexamined omission. |

**Result**: PASS. No violations requiring justification; Complexity Tracking is
intentionally empty.

## Project Structure

### Documentation (this feature)

```text
specs/006-path-parser/
├── plan.md                       # This file
├── research.md                   # Phase 0 output
├── data-model.md                 # Phase 1 output
├── quickstart.md                 # Phase 1 output
├── contracts/
│   └── path-parser-api.md        # Phase 1 output — Rust public API contract
├── checklists/
│   └── requirements.md           # /speckit-specify output
└── tasks.md                      # /speckit-tasks output (not this command)
```

### Source Code (repository root)

```text
crates/
└── core/                         # EXISTING — hl7pet-core (spec 005)
    ├── src/
    │   ├── lib.rs                # updated: adds `pub mod parser;` + re-exports
    │   ├── scanner.rs            # unchanged (spec 005)
    │   └── parser.rs             # NEW — this spec's deliverable (FR-001-FR-011)
    └── tests/
        ├── scanner_regression.rs # unchanged (spec 005)
        ├── scanner_vectors.rs    # unchanged (spec 005)
        └── parser_vectors.rs     # NEW — integration test over fixtures/vectors/path/

fixtures/
└── vectors/
    └── path/                     # EXISTING family (spec 001, no new schema needed)
        ├── valid.json            # + 3 new entries (FR-012): OR'd filter values,
        │                         #   filter subcomponent, whitespace-tolerant operator
        └── invalid.json          # + 1 new entry (FR-012): multi-hop hierarchy chain
```

**Structure Decision**: The parser is added to the existing `hl7pet-core` crate
(`crates/core/`) that spec `005` created, as a new sibling module (`parser.rs`) next to
`scanner.rs` — not a new crate — since both are pure, dependency-free components of the
same engine with no FFI boundary between them yet. Conformance vectors extend spec
`001`'s existing `path` family in place (`fixtures/vectors/path/valid.json`/
`invalid.json`) rather than introducing a new family or schema: this spec's vectors are
still PATH-shaped (`id`/`path`/`message_ref`/`expected`/`grammar_productions`) and
`fixtures/scripts/validate_corpus.py` already recognizes the `path` family against
`conformance-vector.schema.json` — no script or schema change required, only new
entries in the two existing files.

## Complexity Tracking

*No Constitution Check violations — this section is intentionally left without entries.*
