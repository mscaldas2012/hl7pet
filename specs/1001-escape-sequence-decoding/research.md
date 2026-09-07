# Phase 0 Research: Escape-Sequence Decoding

Two scope questions were resolved directly with the user during this planning
session (not left as spec.md `[NEEDS CLARIFICATION]` markers, since they
surfaced only once the real code was examined): whether decoding should ripple
into spec `1000`'s `execute_located`/`first_located` and spec `008`'s
`execute_hierarchy` (yes — broad scope, confirmed), and whether a per-call
opt-out should exist (no — decoding is unconditional, confirmed, since the
existing shared fixtures corpus has zero messages containing escape sequences
today, so this changes no pre-existing test's output regardless). spec.md was
updated in place to reflect both. The findings below are the concrete
technical decisions made while turning that confirmed scope into an actual
design.

## 1. Return type: `&'m str` -> `Cow<'m, str>`, with a zero-copy fast path

**Decision**: `execute`, `execute_located`, `first_located`, and
`execute_hierarchy` all change their value type from `&'m str` to
`std::borrow::Cow<'m, str>` (and `LocatedValue<'m>.value` likewise). Decoding
is implemented as: scan the raw value once for the message's own escape byte
(`delimiters.escape`); if absent, return `Cow::Borrowed(raw)` immediately —
zero allocation, identical cost to today. Only when at least one escape byte
is present does decoding allocate a `String` and return `Cow::Owned(_)`.

**Rationale**: Decoded text is not always a slice of the original message —
removing `\H\`/`\N\` markers deletes characters, `\Xdddd\` decodes hex pairs
into different bytes, and `\Zxxx\` strips its own delimiters. None of these
can be represented as `&'m str` into the untouched original message. `Cow`
is the idiomatic Rust answer to "usually borrowed, occasionally owned" and
lets the overwhelmingly common case (no escape sequence present at all,
which is every message in the current corpus) stay exactly as cheap as
today — satisfying Constitution Principle II's "prefer borrowed views...
wherever... allow it" for the case where it demonstrably does allow it, while
being honest that a real, unavoidable allocation happens for the case where
it doesn't.

**Alternatives considered**:
- *Always return an owned `String`*: rejected — pays an allocation on every
  single call, including the common case of a value with no escape sequence
  at all, directly contradicting Constitution Principle II and risking a
  real regression against spec `009`'s baseline.
- *A new parallel `execute_decoded` function, leaving `execute` returning
  `&'m str` forever*: rejected per the user's explicit direction — the
  existing functions themselves must decode; introducing a permanent parallel
  raw API when nothing in this codebase still needs the raw form (finding
  #4) would be needless API surface.

**Verified finding, corrected during implementation**: the fast path alone
did not actually prevent a regression. Re-running spec `009`'s
`extraction` benchmark against the initial implementation (which routed
`resolve_field_values`'s `Some(fe)` branch through the pre-existing
`select_by_field_index(&repetitions, fe.index) -> Vec<&'m str>` and then a
separate `.into_iter().map(decode_escapes).collect()`) showed a real,
significant regression: average throughput **-17.4%**, worst case **-67.9%**,
even on paths with zero escape sequences. Root cause: the standard library's
same-element-size in-place-collect optimization (the same mechanism spec
`1000`'s research.md #5 already found) applies to `execute()`'s *old*
`&str -> &str` map/collect, letting it reuse `select_by_field_index`'s own
`Vec` buffer at zero extra cost — but `&str -> Cow<str>` can never use that
optimization (different element size), so *every* call paid one extra
`Vec` allocation for the outer `.collect()`, regardless of whether
`decode_escapes` itself did any work. The fast path prevents extra
allocation *inside* `decode_escapes`; it says nothing about the *surrounding*
`Vec` collection, which is where the regression actually was.

**Fix**: introduced `select_and_map<T>(repetitions, index, f: impl FnMut(&str) -> T) -> Vec<T>`
(replacing `select_by_field_index` entirely, including migrating its 4
existing unit tests to call it with `f = |s| s`) — it selects repetitions and
maps them in the *same* step, collecting directly from `repetitions.iter()`
(a borrowed slice) into `Vec<T>`. Collecting from a slice iterator always
costs exactly one allocation regardless of `T`'s size, since there is no
existing owned buffer to potentially reuse in the first place — there is
nothing to lose by not fusing, because fusion was never possible here either
way. This restores exact allocation-*count* parity with the pre-decoding
implementation. Re-running the same benchmark after this fix: average
throughput **+1.7%**, worst case **-8.4%** — within the ±10-18% run-to-run
noise band spec `004`'s own baseline documentation already established for
fast microbenchmark calls, i.e., no real regression. A small, expected,
*non-allocation-count* memory footprint increase remains and is not further
reducible: `Cow<str>` is a larger struct than `&str` (24 bytes vs. 16 on this
platform), so each `Vec`'s buffer is proportionally larger — confirmed via
`allocationBytesPerOp` still showing a small positive delta on every row.
Not concerning: it is a fixed per-element constant, not a scaling cost (it
grows linearly with result size exactly as the pre-existing byte count
already did), and Constitution Principle II's concern is allocation
avoidance, not struct-size minimization.

## 2. Escape-sequence grammar and decode mapping

**Decision**: A single left-to-right byte scan, using the message's own
`delimiters.escape` byte as both the opening and closing delimiter of every
sequence (never a hardcoded `\`, matching spec `005`'s "read delimiters from
the message, don't hardcode them" precedent):

```text
escaped-text     := (plain-byte | escape-sequence)*
escape-sequence  := ESC type-char ESC                    ; type-char in {F,S,T,R,E,H,N}
                   | ESC "X" hex-pair+ ESC
                   | ESC "Z" (any-byte-except-ESC)* ESC
malformed        := ESC followed by anything not matching the above
```

Decode mapping:

| Sequence | Decodes to |
|---|---|
| `\F\` | `delimiters.field` |
| `\S\` | `delimiters.component` |
| `\T\` | `delimiters.subcomponent` |
| `\R\` | `delimiters.repetition` |
| `\E\` | `delimiters.escape` |
| `\H\`, `\N\` | *(removed — no substitution; these are highlighting toggles, not characters)* |
| `\Xdddd..\` | the byte sequence the hex digit pairs represent, UTF-8-validated as a whole |
| `\Zxxx\` | `xxx` unchanged (delimiters stripped, content passed through — FR-005) |
| anything else (unterminated, unrecognized type-char, odd hex-digit count, or a hex byte sequence that isn't valid UTF-8) | left completely unmodified, `ESC` included (FR-006) |

**Rationale**: This is the standard HL7 v2 escape-sequence set (`\F\` field,
`\S\` component, `\T\` subcomponent, `\R\` repetition, `\E\` escape, `\H\`/
`\N\` highlighting, `\Xdddd\` hex, `\Zxxx\` custom) — `\T\` (subcomponent) is
included even though `ROADMAP.md`'s original spec `1001` prose omitted it
from its inline list while `SPEC.md` §7's own example only shows `\H\`/`\N\`;
omitting it would be an arbitrary, inconsistent gap next to `\S\`/`\R\` which
follow the identical mechanism. Hex decoding validates the *whole* decoded
byte run as UTF-8 (not per-byte), which correctly handles both
multiple-single-byte-ASCII-characters (e.g. `\X0D0A\` -> CR LF, two bytes)
and a single multi-byte UTF-8 character split across consecutive hex pairs.

**Alternatives considered**:
- *Treat each hex pair as its own independent character*: rejected — breaks
  multi-byte UTF-8 characters split across consecutive pairs; validating the
  whole decoded run at once is correct for both cases and no more complex.
- *Error on an invalid hex sequence or invalid UTF-8 result*: rejected — FR-
  006 requires malformed input to be left unmodified, never an error;
  treating "decodes to invalid UTF-8" as just another form of "malformed"
  keeps this feature's one behavioral rule (never fail, never fabricate)
  uniform across every failure mode.

## 3. Filter-clause comparisons stay on raw content

**Decision**: `filter_matches` (the `@field=value` filter-clause evaluator in
`query.rs`) is **not** changed — it continues to compare a segment's raw,
undecoded field/component/subcomponent content against a filter's literal
value, exactly as today.

**Rationale**: FR-001 scopes this feature to "values returned by
value-extraction methods." A filter clause's internal comparison value is
navigation machinery used to *select* which segment occurrence matches — it
is never itself returned to a caller. Decoding it would be a second, entirely
separate scope question (should filter literal syntax itself gain escape
support too?) that neither `ROADMAP.md` nor spec.md raises, and introducing
it silently would be unrequested scope expansion.

**Alternatives considered**:
- *Decode before filter comparison too*: rejected — no stated requirement,
  and it would change which segment occurrences a filter matches for any
  message with escape sequences in a filtered field, a behavior change with
  no corresponding acceptance scenario in spec.md.

## 4. `resolve_field_values`/`resolve_field_values_located` change in place — no parallel "raw" variant

**Decision**: Unlike spec `1000`'s treatment of `resolve_segment_candidates`
(which kept the old function and added a new `_indexed` sibling because a
real caller — `hierarchy.rs` — still needed the un-indexed shape), this
feature changes `resolve_field_values` and `resolve_field_values_located`
directly in place, since every current caller of either (`execute`,
`execute_hierarchy`, `execute_located`) now wants decoded output
unconditionally — there is no remaining consumer that needs the old raw
`Vec<&'m str>`/`Vec<LocatedValue<{&str}>>` shape once the no-opt-out
decision is applied everywhere.

**Rationale**: Introducing a parallel undecoded helper alongside these two
would be dead code from the moment it's written — nothing in this codebase
would ever call it. Changing them in place is the smaller, more honest diff.

**Alternatives considered**:
- *Mirror spec 1000's `_indexed` pattern with a `_decoded` sibling*:
  rejected — that pattern existed specifically to avoid breaking a real,
  still-needed caller (`hierarchy.rs`'s pre-decoding call). No such
  caller exists here.

## 5. The `hl7pet` dev CLI needs no code change

**Decision**: `crates/cli/src/main.rs` requires no modification for this
feature (to be confirmed by successful compilation during implementation,
not assumed blindly).

**Rationale**: Traced every use of extracted values in the CLI:
`println!("{v}", ...)`-style formatting uses `Display`, which `Cow<str>`
implements identically to `&str`; `.join(" ~ ")` on a slice of values uses
the standard library's `impl<S: Borrow<str>> Join<&str> for [S]`, which
`Cow<'_, str>` satisfies via its own `Borrow<str>` impl; and
`values.first().and_then(|reps| reps.first())` doesn't care about the
element type. The CLI never re-slices, indexes into, or otherwise assumes
`&str`-specific layout of an extracted value.

**Alternatives considered**: None — this is a verification finding, not a
design choice with alternatives.

## 6. Existing test-code ripple

**Decision**: Several existing tests need small, mechanical updates (not
logic changes) once the type changes land:
- Any `LocatedValue { value: "literal", .. }` struct-literal construction
  (spec `1000`'s own tests) needs `value: Cow::Borrowed("literal")` or
  `.into()` — a bare `&str` no longer coerces into the `Cow<str>` field.
- `query_vectors.rs`'s `assert_get_value(id: &str, values: &[Vec<&str>], ...)`
  helper's parameter type must change to accept `Cow`-typed values (e.g. a
  generic `impl AsRef<str>` bound, or `&[Vec<Cow<'_, str>>]` directly).
- Direct `assert_eq!(value, "literal")`-style comparisons need **no** change
  — `Cow<'_, str>` implements `PartialEq<str>`/`PartialEq<&str>`, so these
  keep compiling and asserting correctly unchanged.

**Rationale**: Surfaced by tracing every existing call site rather than
discovering it mid-implementation; this is the honest, small blast radius of
the return-type change, not a reason to avoid it.

**Found during implementation, not anticipated by this list**: spec `009`'s
`crates/core/benches/extraction.rs` also called `.copied()` on a `&LocatedValue`-
adjacent `Option<&Cow<str>>` (from `execute()`'s result) to get an owned
value for its measurement closure — `.copied()` requires `Copy`, which
`Cow<str>` isn't. Surfaced by `cargo build --workspace --benches` (run as
part of this feature's Polish-phase benchmark sanity check, not by
`cargo build`/`cargo test` alone, since bench targets aren't built by
either) — fixed by changing it to `.cloned()`. Plan.md's Project Structure
listing of touched files didn't originally include `benches/`; noted here so
a future spec remembers `--benches` needs its own explicit build check, not
just `--workspace`.

## 7. Fixture vector family, schema, and location

**Decision**: A new vector family, `fixtures/vectors/escapes/`, with its own
schema `fixtures/schemas/escape-conformance-vector.schema.json` (core fields
mirrored from `conformance-vector.schema.json`: `id`, `path`, `message_ref`,
`method`, `expected` — plus a required `escape_types` array naming which of
`F`/`S`/`T`/`R`/`E`/`H_N`/`X`/`Z` that vector exercises, this family's
coverage dimension), registered in `fixtures/scripts/validate_corpus.py`'s
`KNOWN_FAMILIES`/`COVERAGE_FIELD` maps exactly as specs `005`/`002` did for
their own new families. New fixture messages live under
`fixtures/messages/escapes/`, kept separate from the pre-existing corpus per
spec.md's Assumptions.

**Rationale**: Confirmed empirically (`grep` across every existing fixture
message) that zero messages in the current corpus contain any escape
character usage at all — this feature has no existing vector to extend, it
needs entirely new fixture data. A dedicated family (mirroring the
`scanner`/`hierarchy` family precedent) makes the coverage report track
escape-sequence-type coverage explicitly (SC-001), the same way `path`
tracks grammar-production coverage and `hierarchy` tracks semantic-rule
coverage.

**Alternatives considered**:
- *Add `escape_types`/messages into the existing `path` family*: rejected
  per spec.md's explicit Assumption — mixing them would make it unclear
  which vectors exist specifically to validate decoding, and risks an
  accidental behavior change in a vector that was meant to test something
  else entirely.

## 8. Migration guide: location and content

**Decision**: `crates/core/MIGRATION.md` — a real, crate-level file a Rust
consumer would actually find (not a spec-log artifact under `specs/`).
Content: state plainly that escape-sequence decoding is now unconditional
(list every sequence type, per FR-009), that there is no per-call or
global opt-out, and that a consumer who genuinely needs the pre-decode raw
text has no built-in way to get it from this crate and must handle that
externally (e.g., pin the prior crate version, or re-encode known delimiter
characters back if that's sufficient for their case).

**Rationale**: FR-008 requires the guide be "delivered as part of this
feature, not deferred," and Constitution Principle I frames it as something
real callers consult, not an internal planning note — `crates/core`'s own
directory, next to its `README.md`, is where a consumer already looks.

## 9. Crate version: no bump, following spec `008`'s own precedent

**Decision**: Do not bump `crates/core/Cargo.toml`'s `version` field (stays
`0.1.0`). Record this as a Documented Breaking Change in `ROADMAP.md` only,
exactly as spec `008`'s child-index-resolution breaking change was recorded
without a version bump.

**Rationale**: Checked `crates/core/Cargo.toml`'s git history: it has never
changed from `0.1.0`, including through spec `008`'s own already-shipped,
`ROADMAP.md`-documented breaking change. The crate is pre-1.0 and
workspace-internal only (not published, no external semver contract yet) —
following the established precedent keeps versioning discipline consistent
rather than introducing a new convention unilaterally for one spec. Flagged
to the user during planning with no objection raised.

**Alternatives considered**:
- *Bump to `0.2.0` (treating MINOR as pre-1.0's "MAJOR" signal)*: a
  reasonable alternative, not rejected on the merits — deferred to
  consistency with the existing, already-shipped precedent instead.
