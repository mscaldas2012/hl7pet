# Implementation Plan: Escape-Sequence Decoding

**Branch**: `1001-escape-sequence-decoding` | **Date**: 2026-09-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1001-escape-sequence-decoding/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Make every `hl7pet-core` value-extraction entry point — `execute`, `execute_located`/
`first_located` (spec `1000`), and `execute_hierarchy` (spec `008`) — decode standard
HL7 v2 escape sequences (`\F\`, `\S\`, `\T\`, `\R\`, `\E\`, `\H\`/`\N\`, `\Xdddd\`,
`\Zxxx\`) unconditionally, with no opt-out (confirmed with the user during this
planning session, superseding this spec's own Input description). Since decoding
can shorten or rewrite text (removing highlighting markers, substituting delimiter
characters, decoding hex bytes), a decoded value cannot always be represented as a
slice of the original message — every one of those functions' return types changes
from `&'m str` to `std::borrow::Cow<'m, str>` (and `LocatedValue<'m>.value` likewise),
a real, deliberate breaking change to already-shipped Rust API surface. A single
presence-check fast path (no escape-delimiter byte anywhere in the raw value) keeps
the overwhelmingly common case fully zero-copy (`Cow::Borrowed`, no allocation), so
this stays consistent with Constitution Principle II and protects spec `009`'s
existing extraction benchmark baseline, whose corpus contains no escape sequences.
Because that same fact — no message in the entire pre-existing shared fixtures
corpus contains an escape sequence — means this change is behaviorally invisible to
every pre-existing test, no comparative "before/after" verification is needed beyond
the existing suite passing unmodified; new, purpose-built fixture data (kept
separate from the existing corpus, per spec.md's Assumptions) is what actually
exercises decoding.

## Technical Context

**Language/Version**: Rust, stable toolchain, edition 2021 (matches `crates/core/Cargo.toml`, unchanged by this feature)

**Primary Dependencies**: None new. Uses only `std::borrow::Cow`, already part of the standard library — consistent with the project's dependency policy (pure-Rust, nothing new leaking through the public API, since the eventual goal is Python/Java bindings, module `6000`-`6999`). `Cow<str>` marshals to a plain string at any future FFI boundary, so this doesn't create a binding-parity problem later (Constitution Principle IV)

**Storage**: N/A

**Testing**: `cargo test` — a brand-new fixture vector family, `fixtures/vectors/escapes/` (its own schema, `fixtures/schemas/escape-conformance-vector.schema.json`, registered in `fixtures/scripts/validate_corpus.py`), plus new fixture messages under a dedicated location containing every documented escape-sequence type — kept separate from the pre-existing corpus per spec.md's Assumptions, so it's unambiguous which fixtures exist specifically to validate decoding. A new integration test, `crates/core/tests/escape_vectors.rs`, mirrors `query_vectors.rs`'s pattern. Unit tests colocated in `query.rs` cover the decode algorithm's edge cases (malformed sequences, adjacent sequences, hex-to-invalid-UTF-8 fallback) directly

**Target Platform**: Same as `hl7pet-core` generally — no OS/platform dependency

**Project Type**: Library (Rust crate `hl7pet-core`) — modifies `crates/core/src/query.rs` and `crates/core/src/hierarchy.rs` in place (no new module); the `hl7pet` dev CLI (`crates/cli`) needs no code change at all (research.md #5)

**Performance Goals**: A value containing no escape-delimiter byte MUST decode via a single `O(value length)` presence check with zero allocation (`Cow::Borrowed` fast path) — required specifically so spec `009`'s existing extraction/parsing benchmarks (whose corpus has no escape sequences) see no measurable regression, per the Constitution's Performance & Portability Standards

**Constraints**: `execute`, `execute_located`, `first_located`, and `execute_hierarchy`'s return types change from `&'m str`/`LocatedValue<'m>{ value: &'m str, .. }` to `Cow<'m, str>`/`LocatedValue<'m>{ value: Cow<'m, str>, .. }` — a real breaking change to already-shipped Rust API surface (specs `007`, `1000`, `008`), confirmed acceptable with the user given the existing corpus is entirely unaffected behaviorally. `LocatedValue` drops its `#[derive(Copy)]` (`Cow<str>` is not `Copy`) — `Clone`/`Debug`/`PartialEq`/`Eq` are retained. No new `QueryError` variant: a malformed/unrecognized escape sequence is left unmodified, never an error (FR-006), so this feature introduces no new failure mode

**Scale/Scope**: Decoding applies to every value-extraction entry point uniformly (flat-PATH, location-aware, and hierarchy-navigated alike), per spec.md's Assumptions. Filter-clause (`@field=value`) internal comparisons remain against **raw**, undecoded segment content, unchanged — a filter's comparison value is an internal navigation mechanism, not a value "returned" to a caller (FR-001's scope), so this feature does not touch `filter_matches`

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Path Contract Stability** — PASS. No PATH grammar or evaluation-semantics
  change; this changes returned *value content*, not how PATHs are written,
  parsed, or navigated. `crates/core/src/parser.rs` is untouched.
- **II. Zero-Copy & Lazy Evaluation** — PASS, with a deliberate, narrow, and
  justified exception. A value containing at least one escape sequence requires
  an owned allocation — decoded text (markers removed, characters substituted,
  hex bytes decoded) cannot be represented as a slice of the original message.
  The common case — no escape sequence present — stays fully zero-copy via a
  single presence-check before any allocation (`Cow::Borrowed`, research.md #1),
  so this principle's intent (avoid unnecessary copies, index only what's
  necessary) is preserved for the overwhelmingly common case, with real
  allocation only where decoding is genuinely unavoidable.
- **III. Explicit, Exception-Free Data Absence** — PASS. No new `QueryError`
  variant. A malformed or unrecognized escape sequence is left completely
  unmodified in the output (FR-006) — the same "never throw to communicate an
  edge case, just represent it plainly" spirit this principle already requires
  for missing/out-of-range data, extended here to "can't decode this part."
- **IV. Multi-Language Interoperability** — Tracked, not violated. Rust-core-only
  work; module `6000`-`6999` (Python/JNI bindings) hasn't started. `Cow<str>`
  marshals to a plain string at any future FFI boundary exactly like `&str`
  would have, so this decision creates no binding-parity problem to resolve
  later.
- **V. Conformance Through Declarative Profiles & Documented Limitations** —
  PASS. The one real limitation — `\Zxxx\` custom sequences have no universal
  decode rule, so their delimiters are stripped and content passed through
  unchanged — is explicitly documented in spec.md's Assumptions and this plan's
  research.md, per this principle's requirement that known limitations be
  documented rather than silently mishandled.
- **Performance & Portability Standards** — Addressed directly by this plan's
  Performance Goals: this feature touches the extraction hot path spec `009`
  benchmarked, so the zero-copy fast path is a hard requirement, not an
  optimization nicety. No new comparative Scala-vs-Rust benchmark run is
  required (this isn't a new capability being measured against a Scala
  equivalent — Scala has no escape decoding at all, `SPEC.md` §7), but a
  sanity re-run of spec `009`'s existing `cargo bench` extraction target is
  good practice before merge to directly confirm the fast path costs nothing
  measurable against messages that (like that benchmark's entire corpus)
  contain no escape sequences.
- **Development Workflow — Phased Migration Discipline** — N/A. Parsing &
  Extraction module (`1000`-`1999`) work, not `0`-`999` Rust Core
  Migration-Plan-phase work.

No violations requiring Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/1001-escape-sequence-decoding/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── escape-decoding-api.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
crates/core/
├── MIGRATION.md                    # NEW — crate-level migration guide (FR-008/FR-009), discoverable by real consumers
├── src/
│   ├── scanner.rs                  # spec 005 — unchanged; DelimiterSet already carries every char decoding needs
│   ├── parser.rs                   # spec 006 — unchanged
│   ├── query.rs                    # MODIFIED: LocatedValue.value -> Cow<'m, str>; resolve_field_values/
│   │                                # resolve_field_values_located decode in place; new decode_escapes() helper;
│   │                                # execute/execute_located/first_located return types change to Cow
│   ├── hierarchy.rs                 # MODIFIED: execute_hierarchy's return type changes to Cow (delegates to
│   │                                # query::execute for non-hierarchy case; hierarchy branch calls the same
│   │                                # decoding resolve_field_values now performs)
│   └── lib.rs                      # No text change needed — re-export lines don't name types explicitly
└── tests/
    ├── query_vectors.rs            # MODIFIED: assert_get_value's `&[Vec<&str>]` parameter type -> Cow-compatible
    ├── hierarchy_vectors.rs        # Reviewed for the same Cow ripple; fixed if needed
    ├── located_vectors.rs          # Reviewed: LocatedValue field access already goes through PartialEq<&str>,
    │                                # likely needs no change beyond recompiling
    └── escape_vectors.rs           # NEW — runs fixtures/vectors/escapes/ against execute()

fixtures/
├── schemas/
│   └── escape-conformance-vector.schema.json   # NEW
├── messages/
│   └── escapes/                    # NEW — dedicated location, kept separate from the existing corpus
│       └── *.hl7                   # messages containing every documented escape-sequence type
├── vectors/
│   └── escapes/                    # NEW vector family
│       └── valid.json
└── scripts/
    └── validate_corpus.py          # MODIFIED: register the "escapes" family + its coverage dimension field

crates/cli/
└── src/main.rs                    # No change expected (research.md #5) — verified during implementation
```

**Structure Decision**: No new crate. This feature modifies the existing
`hl7pet-core` query-execution and hierarchy modules in place (`query.rs`,
`hierarchy.rs`, both already home to specs `007`/`1000` and `008`
respectively) rather than adding a new sibling module, since there is no new
navigation or extraction *shape* being introduced — only a value-content
transformation applied at each module's existing final-value production
point. A new, dedicated fixture location and vector family
(`fixtures/{messages,vectors}/escapes/`) is added rather than extending the
existing `fixtures/vectors/path/` family, per spec.md's Assumptions: keeping
escape-sequence fixtures separate makes unambiguous which data exists
specifically to validate decoding, and confirms by construction that no
pre-existing vector's expected output needed to change.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations — this section is not applicable to this feature.
