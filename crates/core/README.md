# hl7pet-core

Pure Rust HL7 v2 engine, zero FFI dependencies (`HL7-PET-Rust-Migration-Plan.md`).

Currently implements the message scanner (`src/scanner.rs`, spec
[`005-message-scanner`](../../specs/005-message-scanner/)): a single-pass,
zero-copy scan producing segment and delimiter byte offsets only, reading the
field separator and encoding characters from each message's own MSH-1/MSH-2.

## Build and test

```bash
cargo build --workspace
cargo test -p hl7pet-core
```

`cargo test -p hl7pet-core --lib` runs unit tests (delimiter resolution,
allocation-count, panic-safety). `cargo test -p hl7pet-core --test
scanner_vectors` runs every conformance vector under
`../../fixtures/vectors/scanner/`; `--test scanner_regression` confirms zero
behavior change across the pre-existing standard-delimiter corpus.

See [`../../specs/005-message-scanner/quickstart.md`](../../specs/005-message-scanner/quickstart.md)
for the full validation walkthrough and
[`../../specs/005-message-scanner/contracts/scanner-api.md`](../../specs/005-message-scanner/contracts/scanner-api.md)
for the public API contract.
