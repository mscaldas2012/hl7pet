# HL7-PET Roadmap

This is a hand-maintained index of module → spec-number ranges. Spec Kit itself has
no concept of "module" — every `/speckit-specify` call creates a flat, globally
numbered folder under `specs/`. This file is the convention we use to keep specs
grouped by module anyway, since specs from different modules will otherwise
interleave in a single ascending sequence.

Nothing in `.specify/` reads this file automatically — it's for humans (and for
Claude) to stay oriented, and it should be updated whenever a module range fills
up or a new module is added.

## How to use this

When starting a new spec for a module, pass both flags to
`.specify/scripts/bash/create-new-feature.sh` (or let `/speckit-specify` do it,
telling it which module/number to use):

```bash
.specify/scripts/bash/create-new-feature.sh --number 1000 --short-name field-extraction "..."
```

This produces `specs/1000-field-extraction/`. Use the next free number within the
module's range; update the "Next free" column below after each spec is created.

## Module ranges

Ranges are 1000-wide, with Rust Core first since it's the foundation everything
else in the migration builds on.

| Range       | Module                        | Maps to (current Scala)                                   | Next free |
|-------------|--------------------------------|-------------------------------------------------------------|-----------|
| 0-999       | Rust Core / Engine Migration    | Migration Plan Phases 1-3, 6 (scanner, PATH parser, hierarchy, perf) | 10        |
| 1000-1999   | Parsing & Extraction            | `HL7ParseUtils`, `HL7StaticParser`, PATH grammar             | 1002      |
| 2000-2999   | Validation                      | `StructureValidator`, `RulesValidator`, `BatchValidator`      | 2000      |
| 3000-3999   | De-identification                | `DeIdentifier`                                                | 3000      |
| 4000-4999   | Transformation                   | (new — no current Scala equivalent)                          | 4000      |
| 5000-5999   | File & Batch Utilities           | `HL7FileUtils`                                                | 5000      |
| 6000-6999   | Language Bindings                | Migration Plan Phase 4-5 (Arrow, PyO3, JNI/JNA)               | 6000      |
| 9000-9999   | Cross-cutting / Infra            | tooling, CI, docs, benchmarking harness                       | 9000      |

7000-8999 is intentionally unassigned buffer for a future module.

Specs within a module are numbered sequentially by ones (1, 2, 3, ...), not
spaced out — inserting a spec later just takes the next free number rather
than a reserved gap.

Note: the 0-999 range starts numbering at 1, not 0, so the first spec reads
`001-...` rather than `000-...`.

### 0-999: Rust Core / Engine Migration -- planned specs

The Scala library referenced throughout lives at
[github.com/mscaldas2012/hl7-pet](https://github.com/mscaldas2012/hl7-pet)
(see `HL7-PET-Rust-Migration-Plan.md` > Repository Layout). This repo has no
hard dependency on it (no submodule, no build-time fetch) -- any fixtures or
benchmark numbers needed from it are downloaded once and committed here as
static data.

| #   | Short name              | Migration plan phase | Scope |
|-----|--------------------------|-----------------------|-------|
| 001 | `path-grammar-spec`      | Phase 1               | Formal PATH grammar specification + conformance test vectors, derived from `SPEC.md` section 3.1 |
| 002 | `hierarchy-semantics`    | Phase 1               | Document hierarchy-mode semantics (`segmentDefinition`-driven parent→child navigation, cardinality, `->` operator) |
| 003 | `regression-suite`       | Phase 1               | Shared golden-message corpus (`fixtures/`) + expected outputs exported from the Scala baseline |
| 004 | `scala-baseline-bench`   | Phase 1               | Benchmark harness capturing the Scala engine's current throughput/memory/allocation/latency numbers, to compare against later |
| 005 | `message-scanner`        | Phase 2               | Single-pass segment/delimiter scanner, offsets only, no field/component allocations. MUST read the field separator from MSH-1 and the encoding characters (component/repetition/escape/subcomponent) from MSH-2 rather than hardcoding `\|` and `^~\&` -- fixes the Scala engine's "MSH-1/MSH-2 must be standard" limitation (`SPEC.md` §7). For messages using standard delimiters (the common case), output is unchanged; this only changes behavior for non-standard-delimiter messages, which previously mis-parsed or errored. Needs its own conformance vectors: at least one message with non-default delimiters, plus malformed-MSH error cases. |
| 006 | `path-parser`            | Phase 2               | Hand-written PATH parser/state machine replacing regex; compiles paths into reusable query objects |
| 007 | `query-execution`        | Phase 2               | Navigate offsets to extract values; validated against the `003` regression suite |
| 008 | `lazy-hierarchy-nav`     | Phase 3               | Contextual parent→child navigation without building a full tree |
| 009 | `core-perf-validation`   | Phase 6               | Benchmark `005`-`008` against the `004` Scala baseline; confirm zero-copy/lazy targets are met |

Next spec in this module starts at **010**.

### 1000-1999: Parsing & Extraction -- planned specs

| #    | Short name              | Scope |
|------|--------------------------|-------|
| 1000 | `located-extraction-api` | New capability (no current Scala equivalent): a location-aware extraction API returning each value paired with its 1-based source line number, e.g. `getValueLocated(path)`. MUST be additive -- existing `getValue`/`getFirstValue` stay unchanged, this is a new method alongside them, per the Backward-Compatible-Additions convention below. Depends on offset data already tracked internally by the message scanner (spec `005`) and query executor (spec `007`). Conformance vectors for this can reuse the line-number metadata collected in spec `001` (FR-008) rather than re-deriving it. |
| 1001 | `escape-sequence-decoding` | Fixes the Scala engine's "no escaped character support" limitation (`SPEC.md` §7: `\H\`, `\N\`, `\F\`, `\S\`, `\R\`, `\E\`, `\X..\` hex, custom `\Zxxx\`). Unlike `1000`, this is a **deliberate exception** to Backward-Compatible Additions: rather than a parallel method, `getValue`/`getFirstValue` gain a `decodeEscapes` parameter defaulting to `true` (decode). Passing `false` explicitly disables decoding and reproduces today's raw-passthrough behavior exactly -- so the opt-out always exists, it just isn't what a caller gets by not specifying anything. The default still changes existing callers' output for any value containing an escape sequence, so per Constitution Principle I this still requires a MAJOR version bump and a documented migration guide (which should tell callers who need the old behavior unconditionally to pass `decodeEscapes=false`) -- this spec's plan.md MUST include both. Note for the Java binding (spec `6000`-range): Java has no native default-parameter syntax, so this likely needs an explicit overload (`getValue(path)` vs `getValue(path, decodeEscapes)`) rather than a single method with a default. |

Next spec in this module starts at **1002**.

## Conventions

### Backward-Compatible Additions

When a new capability has no equivalent in the current Scala API (or would
require changing an existing method's return type), it MUST be added as a
new method alongside the existing ones, not by changing an existing
method's signature or return shape. Existing callers of `getValue`,
`getFirstValue`, etc. must never be forced to change to pick up an
unrelated improvement. This mirrors the single-call-vs-batched API decision
already made for the Python/Java bindings in
`HL7-PET-Rust-Migration-Plan.md` Phase 5.

Exception: when the current behavior is being treated as a bug fix rather
than a missing capability (e.g. spec `1001`, escape-sequence decoding), a
breaking change to an existing method's default behavior is acceptable in
place of a parallel method -- but only with an explicit, documented
decision to that effect (not by default), a parameter that lets a caller
opt back into the old behavior explicitly, and the MAJOR version bump and
migration guide Constitution Principle I requires for breaking changes.

### Documented Breaking Changes

Running registry of every deliberate breaking change decided so far, each requiring
a MAJOR version bump and migration guide per Constitution Principle I. Kept in one
place so `/speckit-plan` runs for later specs can check this list rather than
rediscover each decision independently.

| Spec | Change | Why it's breaking |
|---|---|---|
| `001` | `SEG` requires an alphabetic first character (was `[A-Z0-9]{3}`, any 3 chars) | A previously syntax-valid segment name like `999` is no longer valid |
| `001` | `SEG_IDX`/`FIELD_IDX` reject non-numeric/non-`$LAST`/non-`*` content at parse time (was accepted by regex, then crashed at evaluation with an uncaught exception) | Behavior changes from "crash at runtime" to "rejected at parse time" for inputs like `PID[ABC]-1` |
| `001` | `OPERATOR` restricted to `=`, `!=`, `>`, `>=`, `<`, `<=` (was any 1-2 chars from `[!><=]`, e.g. `==` matched syntactically then crashed) | Same shift from runtime crash to parse-time rejection |
| `1001` | `getValue`/`getFirstValue` decode escape sequences by default (`decodeEscapes: Boolean = true`) | Changes returned value content for any message containing an escape sequence, unless the caller passes `decodeEscapes=false` |

## Status

| Spec # | Module    | Short name          | Status |
|--------|-----------|----------------------|--------|
| 001    | Rust Core | `path-grammar-spec`  | Complete — grammar finalized (`contracts/path-grammar.md`), 17 conformance vectors authored and verified against the real Scala library with zero discrepancies, scope boundary vs. spec `002` cross-referenced. Ready for spec `006` (Rust PATH parser) to build against. |

## Notes

- If a module's range fills up (unlikely at 1000 slots), extend it into the
  7000-8999 buffer rather than renumbering existing specs.
- Cross-module specs (e.g. a change that touches both Validation and
  De-identification) belong to whichever module owns the primary change; note the
  secondary module in that spec's own `spec.md`.
- This file complements, but does not replace, `.specify/memory/constitution.md`
  (project-wide non-negotiable principles) and `HL7-PET-Rust-Migration-Plan.md`
  (the Rust core's phased build-out, which the 0-999 and 6000-6999 ranges above
  track).
