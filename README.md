# HL7-PET

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
![Status: pre-implementation](https://img.shields.io/badge/status-pre--implementation-orange)

A high-performance, zero-copy HL7 v2 query engine — a Rust core with first-class
Python and Java bindings and native Apache Arrow output.

> **Status**: this repository is in the spec-driven planning phase. There is no
> Rust, Python, or Java code here yet — what exists today is the formal
> specification, architecture plan, and constitution for a ground-up rewrite of
> the existing [HL7-PET](https://github.com/mscaldas2012/hl7-pet) Scala library.
> See [Current Status](#current-status) below for exactly what's done.

## What is HL7-PET?

HL7-PET (**HL7 Parsing and Extraction Toolkit**) is a lightweight library for
reading HL7 v2 messages. Instead of parsing a message into a full object model,
it exposes a path-based query language that addresses data directly in the raw
message — for example:

```
OBX[3]-5[2].1        # third OBX segment, field 5, second repetition, component 1
OBX[@3.1='94500-6']-5   # OBX where field 3 component 1 equals a given code, return field 5
OBR[1] -> OBX-5       # field 5 of OBX segments that are children of the first OBR
```

That query surface — the **PATH grammar** — is this project's public contract. It's
preserved unchanged across every implementation and language binding; see
[Constitution Principle I](.specify/memory/constitution.md#i-path-contract-stability-non-negotiable).

## Relationship to the existing Scala library

The current, production HL7-PET implementation is a Scala library
([mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet), `gov.cdc.hl7`,
currently v1.2.10). This repository does not vendor or depend on that repo — it's
a clean-room rewrite that treats the Scala library purely as the **parity
target**: its documented and actual behavior (verified by reading its source, not
just its docs) is what the new engine's conformance vectors are checked against.

## Why a rewrite

Per [HL7-PET-Rust-Migration-Plan.md](HL7-PET-Rust-Migration-Plan.md):

- **Zero-copy, no full object model** — index only what a query needs; build
  hierarchy lazily, only when hierarchy-mode paths (`->`) are actually used.
- **First-class Python**, not an afterthought — published as a real PyPI wheel
  via PyO3/maturin, not a JVM-bridge wrapper.
- **Continued JVM interoperability** — a Java binding published to Maven,
  alongside the Python one.
- **Apache Arrow output** — query results usable directly in Pandas, Polars,
  PySpark, Databricks, and DuckDB without intermediate conversions.

## Architecture

```text
              Python              Java
                 |                  |
              PyO3               JNI/JNA
                 |                  |
                 +------ Rust -------+
                        |
                 HL7 Query Engine
                        |
                 Apache Arrow
```

## Repository layout

```text
hl7pet/
├── crates/
│   ├── core/          # hl7pet-core: pure Rust engine, zero FFI dependencies
│   ├── arrow/          # hl7pet-arrow: Apache Arrow output
│   └── python-ext/     # hl7pet-python: thin PyO3 glue crate
├── bindings/
│   ├── python/          # PyPI packaging root (maturin)
│   └── java/            # Maven packaging root (SBT)
├── fixtures/            # shared golden-message corpus, used by all bindings
├── specs/                # Spec Kit feature specs — see ROADMAP.md
├── SPEC.md               # the current Scala library's documented API (parity reference)
└── HL7-PET-Rust-Migration-Plan.md
```

None of `crates/`, `bindings/`, or `fixtures/` exist yet — this is the target
layout as design work reaches each part of it. See
[Repository Layout](HL7-PET-Rust-Migration-Plan.md#repository-layout) for the
full rationale.

## Development approach: Spec-Driven Development

This project is built with [GitHub Spec Kit](https://github.com/github/spec-kit):
every unit of work starts as a spec (`spec.md`), goes through a plan
(`plan.md` + `research.md` + `data-model.md` + `contracts/`), a task breakdown
(`tasks.md`), and only then implementation — checked throughout against the
project [constitution](.specify/memory/constitution.md).

Specs are organized by module and numbered per [ROADMAP.md](ROADMAP.md) — Spec
Kit itself only knows a flat, globally-numbered `specs/` directory, so
`ROADMAP.md` is the hand-maintained index that keeps Rust Core, Parsing &
Extraction, Validation, De-identification, Transformation, and the language
bindings in their own number ranges.

## Current Status

**Spec `001` (PATH grammar specification) is complete.** See
[specs/001-path-grammar-spec/](specs/001-path-grammar-spec/) for:

- The formal PATH grammar ([contracts/path-grammar.md](specs/001-path-grammar-spec/contracts/path-grammar.md))
- A JSON Schema for machine-checkable conformance vectors
- 17 conformance vectors, verified against the real Scala library with zero
  discrepancies

Everything downstream of it — the Rust message scanner, PATH parser, query
executor, and eventually the Python/Java bindings — is planned but not yet
started. See [ROADMAP.md](ROADMAP.md) for the full module breakdown and what's
next.

## Contributing

This project isn't accepting external contributions yet — it's a solo,
early-stage rewrite. Once the Rust core lands, `ROADMAP.md` and the Spec Kit
workflow above are exactly how a contribution would be scoped: pick an
unclaimed spec number in the relevant module range, run `/speckit-specify`,
and go from there.

## License

Apache License 2.0 — see [LICENSE](LICENSE).
