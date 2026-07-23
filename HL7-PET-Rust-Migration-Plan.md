# HL7-PET Rust Migration Plan

## Vision

Build a high-performance, zero-copy HL7 v2 query engine with first-class
support for Python, Java, and Apache Arrow.

Goals:

-   Preserve current PATH syntax (e.g. `OBX[3]-5[2].1`)
-   Keep extraction lazy (no full object model)
-   Become a first-class Python library
-   Maintain JVM interoperability
-   Produce Apache Arrow output for analytics

## Guiding Principles

1.  Zero-copy whenever possible.
2.  Do not parse the entire message into objects.
3.  Index only what is necessary.
4.  Build hierarchy lazily.
5.  Optimize for extracting data, not representing HL7.

## Architecture

``` text
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

## Repository Layout

This repo (`hl7pet`) is a fresh monorepo for the Rust core and both language
bindings. The existing Scala library (`gov.cdc.hl7`, v1.2.10) lives at
[github.com/mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet) and
is not vendored in here -- it is referenced only as the parity target for the
regression suite (Phase 1) and baseline benchmarks (Phase 1, Phase 6).

``` text
hl7pet/
├── Cargo.toml                  # workspace manifest
├── crates/
│   ├── core/                   # hl7pet-core: pure Rust engine, zero FFI deps
│   │   ├── src/
│   │   │   ├── scanner.rs      # Phase 2 -- message scanner
│   │   │   ├── path/           # Phase 2 -- PATH parser + compiled query objects
│   │   │   └── hierarchy.rs    # Phase 3 -- lazy hierarchy navigation
│   │   ├── benches/            # Phase 6 -- criterion benchmarks
│   │   └── tests/              # reads ../../fixtures/
│   ├── arrow/                  # hl7pet-arrow: Phase 4, depends on core
│   └── python-ext/             # hl7pet-python: thin PyO3 glue crate (cdylib)
├── bindings/
│   ├── python/                 # PyPI packaging root
│   │   ├── pyproject.toml      # maturin backend -> crates/python-ext
│   │   ├── python/hl7pet/      # __init__.py, .pyi type stubs
│   │   └── tests/              # pytest, runs against ../../fixtures/
│   └── java/                   # Maven packaging root
│       ├── build.sbt           # SBT, for consistency with the existing Scala tooling
│       ├── src/main/java/gov/cdc/hl7/
│       └── src/test/java/
├── fixtures/                   # shared golden-message corpus + expected outputs;
│                                # consumed by core/tests, pytest, and Java tests alike
├── specs/                      # Spec Kit specs (see ROADMAP.md)
└── ROADMAP.md
```

Design rationale:

-   `crates/core` has no PyO3/JNI dependency and can be built, tested, and
    benchmarked entirely on its own -- this keeps Principle II (Zero-Copy &
    Lazy Evaluation) honest, since the engine's performance can't be hidden
    behind FFI marshaling in its own benchmarks.
-   Both bindings wrap the same `crates/core` (`crates/python-ext` links it
    directly; the Java binding will link it too, whichever of JNI or the
    Panama FFM API Phase 5 lands on) -- this is what makes Principle IV
    (Multi-Language Interoperability) enforceable: there is only one place
    query semantics can drift.
-   `fixtures/` is a single shared corpus rather than per-language copies, so
    "does Rust agree with Scala" and "does Python agree with Java" are the
    same question answered by the same data.
-   This repo has no hard dependency on
    [mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet) -- no
    submodule, no build-time fetch. When Phase 1's regression-suite spec
    (`003`) or baseline-benchmark spec (`004`) needs message fixtures,
    expected outputs, or timing numbers from the Scala repo, those are
    downloaded/exported once and committed as static files under
    `fixtures/`. If the Scala repo changes later, re-export by hand; this
    repo never reaches out to it automatically.

## Phase 1 -- Document Current Behavior

-   Document PATH grammar.
-   Document hierarchy semantics (`OBR[1]->OBX-3`).
-   Create a comprehensive regression suite.
-   Benchmark the Scala implementation.

## Phase 2 -- Rust Core

### Message Scanner

Perform a single pass over the message to:

-   locate segment boundaries
-   identify delimiters
-   record offsets only

No allocations for fields or components.

### PATH Parser

Replace regex-heavy parsing with a small hand-written parser/state
machine.

Compile paths into reusable query objects.

### Query Execution

Navigate offsets to extract values.

Return borrowed slices where possible.

## Phase 3 -- Hierarchy

Support contextual navigation without building a full tree.

Example:

``` text
OBR
  OBX
  OBX
OBR
  OBX
```

Evaluate hierarchy lazily during query execution.

## Phase 4 -- Arrow Integration

Return Arrow arrays/tables directly.

Benefits:

-   Pandas
-   Polars
-   PySpark
-   Databricks
-   DuckDB

without unnecessary conversions.

## Phase 5 -- Language Bindings

API shape (applies to both bindings):

-   Primary API is one-call-per-field (e.g. `getFirstValue(path)`, `getValue(path)`),
    matching the current Scala API so callers migrating from Scala don't restructure.
-   Also offer a batched variant (e.g. `getValues(paths: list[str])`) that returns all
    requested paths in a single Rust crossing. This is the fast path for hot loops
    (bulk field extraction, profile-driven validation) where every path is known
    up front -- it amortizes PyO3/JNI per-call overhead across many fields instead
    of paying it once per field.

### Python

-   PyO3
-   maturin
-   pip-installable wheels

### Java

-   JNI/JNA wrapper
-   API similar to current Scala library

## Phase 6 -- Performance

Benchmark against Scala:

-   parsing throughput
-   extraction throughput
-   memory usage
-   allocations
-   latency

Target:

-   lower memory usage
-   fewer allocations
-   faster extraction

## Stretch Goals

-   Streaming parser
-   Predicate filtering
-   Batch extraction
-   Parallel extraction
-   Arrow Flight support
-   Parquet export

## Deliverables

-   Rust core crate
-   Python package
-   Java bindings
-   Arrow integration
-   Benchmarks
-   Migration guide
-   API documentation
