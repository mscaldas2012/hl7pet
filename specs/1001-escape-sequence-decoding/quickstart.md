# Quickstart: Escape-Sequence Decoding

Validates spec.md's user stories end-to-end: every value-extraction call
decodes standard HL7 escape sequences unconditionally (US1), and the
migration guide clearly documents the change with no opt-out to describe
(US2). Because no message in the pre-existing shared fixtures corpus
contains an escape sequence, this feature's correctness is validated
entirely against new, purpose-built fixture data
(`fixtures/{messages,vectors}/escapes/`), while the full pre-existing suite
passing unmodified is itself the proof of SC-002 (zero behavior change for
every message that predates this feature).

## Prerequisites

- Rust `stable` toolchain — same as specs `005`-`1000`, no pinned MSRV.
- No JVM, Scala, or Maven required — the Scala engine has no escape-decoding
  behavior to compare against at all (`SPEC.md` §7); there is nothing to
  verify against a live baseline for this feature.
- Specs `007` (query executor), `1000` (located extraction), and `008`
  (hierarchy navigation) already implemented — this feature changes their
  value type in place (plan.md Project Structure), it does not add a new
  module.

## 1. Build with the changed value type

```bash
cargo build --workspace
```

**Expected outcome**: `crates/core` and `crates/cli` both compile cleanly
with zero warnings. `hl7pet_core::query::{execute, execute_located,
first_located}` and `hl7pet_core::execute_hierarchy` now return
`Cow<'_, str>`-based values instead of `&str`; `crates/cli` needs no source
change to keep compiling (research.md #5).

## 2. Run unit tests (decode algorithm edge cases)

```bash
cargo test -p hl7pet-core --lib query
```

**Expected outcome**: new unit tests colocated in `query.rs` pass, covering:
each of `\F\`/`\S\`/`\T\`/`\R\`/`\E\` decoding to that message's own actual
delimiter character; `\H\`/`\N\` removal; `\Xdddd\` hex decoding, including a
multi-byte UTF-8 character split across consecutive pairs; `\Zxxx\`
delimiter-stripping; an unterminated/unrecognized sequence and an
invalid-hex/invalid-UTF-8 result both left completely unmodified; and a
value with no escape-delimiter byte at all producing `Cow::Borrowed` at zero
allocation (the fast path, research.md #1).

## 3. Run the new escape-sequence conformance vector suite (US1)

```bash
cargo test -p hl7pet-core --test escape_vectors
```

**Expected outcome**: every vector in `fixtures/vectors/escapes/valid.json`
passes — each documented escape-sequence type (`F`, `S`, `T`, `R`, `E`,
`H`/`N`, `X`, `Z`) has at least one vector exercising it, satisfying SC-001,
with the corpus's coverage report (`fixtures/scripts/validate_corpus.py`)
confirming 8/8 escape-type coverage for the new `escapes` family.

## 4. Confirm zero impact on every pre-existing test (SC-002)

```bash
cargo test --workspace
```

**Expected outcome**: the full pre-existing suite (specs `005`-`1000`)
continues to pass **unmodified** — no pre-existing vector's `expected` value
needed to change, since none of them contain an escape sequence. This is
this feature's direct proof of SC-002, not a coincidental side effect.

## 5. Manual check via the dev CLI

```bash
cargo run -p hl7pet-cli -- fixtures/messages/escapes/highlighting.hl7 "NTE-3"
```

**Expected outcome**: the printed value shows the highlighted text with its
`\H\`/`\N\` markers already removed — compare against the raw file content
(`cat fixtures/messages/escapes/highlighting.hl7`) to see the markers are
present in the source message but absent from the CLI's output.

## 6. Read the migration guide (US2)

```bash
cat crates/core/MIGRATION.md
```

**Expected outcome**: the guide names every escape-sequence type this
feature decodes, states plainly that decoding is unconditional, and says
explicitly that no per-call or global opt-out exists (SC-003).

## 7. Full regression check

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
```

**Expected outcome**: clean across the board. As a final sanity check on
this plan's Performance Goal, re-run spec `009`'s existing extraction
benchmark (`cargo bench -p hl7pet-core --bench extraction`, if available in
the environment) and confirm no measurable regression versus its last
recorded baseline — expected, since that benchmark's corpus contains no
escape sequences and should hit this feature's zero-copy fast path on every
call.
