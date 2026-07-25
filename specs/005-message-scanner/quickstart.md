# Quickstart: Message Scanner

Validates spec.md's user stories end-to-end: zero-copy offset output (US1),
non-standard delimiters read from MSH-1/MSH-2 (US2), and structural errors on
malformed MSH (US3).

## Prerequisites

- Rust `stable` toolchain (`rustup toolchain install stable` if not already present) —
  no pinned MSRV (research.md #1).
- No JVM, Scala, or Maven required — this spec has no dependency on the Scala engine
  or spec `004`'s harness at build/test time.

## 1. Build the new workspace (first Rust code in this repo)

```bash
cargo build --workspace
```

**Expected outcome**: `Cargo.toml` (workspace root) and `crates/core` compile cleanly
with zero warnings. This is the first successful `cargo build` in this repository's
history — confirms the workspace scaffolding itself (plan.md Project Structure) is
correct before anything else is checked.

## 2. Run unit tests (delimiter resolution, boundary logic)

```bash
cargo test -p hl7pet-core --lib
```

**Expected outcome**: unit tests colocated in `scanner.rs` pass — these cover
delimiter-resolution logic and edge cases (empty message, single-segment message)
independently of the shared fixtures corpus.

## 3. Run the conformance vector suite against `fixtures/` (US1, US2, US3)

```bash
cargo test -p hl7pet-core --test scanner_vectors
```

**Expected outcome**: every vector under `fixtures/vectors/scanner/` passes — each
`message_ref` is scanned and the actual `ScanResult`/`ScanError` is compared against
the vector's `expected_delimiters`/`expected_segments` or `expected_error`
(contracts/scanner-conformance-vector.schema.json). This is the single command that
proves all three user stories together: standard-delimiter vectors prove SC-001 (no
regression), non-standard-delimiter vectors prove SC-002 (the fix), malformed-MSH
vectors prove SC-003 (structural errors, not panics).

## 4. Confirm standard-delimiter output is unchanged (SC-001, spec.md FR-005)

```bash
cargo test -p hl7pet-core --test scanner_vectors -- standard_delimiters
```

**Expected outcome**: for every `fixtures/messages/*.hl7` file already used by specs
`001`-`003` (which all use standard `|`/`^~\&` delimiters), the scanner resolves
`DelimiterSet { field: b'|', component: b'^', repetition: b'~', escape: b'\\',
subcomponent: b'&' }` — identical to what a hardcoded scanner would have produced.

## 5. Confirm the specific limitation fix (SC-002)

```bash
cargo test -p hl7pet-core --test scanner_vectors -- non_standard_delimiters
```

**Expected outcome**: the non-standard-delimiter vector (fixtures/vectors/scanner/
non-standard-delimiters.json, FR-010) scans successfully using its message's own
declared characters — where the current Scala engine (SPEC.md §7) would have
mis-parsed or errored on the same input.

## 6. Confirm malformed-MSH structural errors (SC-003)

```bash
cargo test -p hl7pet-core --test scanner_vectors -- malformed_msh
```

**Expected outcome**: each malformed-MSH vector (missing MSH, truncated MSH-2,
unrecognized later segment — FR-006) produces the specific `ScanError` variant and
offset the vector declares, with zero panics across the set (`cargo test` itself
would report a panic as a test failure with a backtrace, making this a strong
negative-case check).

## 7. Confirm allocation-count independence (SC-004)

```bash
cargo test -p hl7pet-core --lib -- allocation_count
```

**Expected outcome**: a unit test (using a global allocator wrapper that counts
`alloc` calls, or an equivalent crate-free counting harness — an implementation
choice for `/speckit-tasks`) confirms `scan()` performs exactly 2 heap allocations
(data-model.md's `ScanResult`) regardless of whether the input message has 5 fields
or 500, varying only when segment *count* changes. Full throughput/memory comparison
against the spec `004` Scala baseline is out of scope here (deferred to spec `009` per
spec.md Assumptions) — this step only proves the structural allocation-count claim.

## 8. Corpus validation still passes with the new vector family

```bash
python3 fixtures/scripts/validate_corpus.py
```

**Expected outcome**: spec `003`'s existing validation script accepts the new
`fixtures/vectors/scanner/` family without modification to its core logic (schema
registration for `scanner-conformance-vector.schema.json` is the only change needed,
per spec `003` FR-007's "unrecognized families are reported, not rejected" design) —
uniqueness (`scan-*` ids don't collide with `path-*`/`hier-*`) and `message_ref`
resolution both pass.
