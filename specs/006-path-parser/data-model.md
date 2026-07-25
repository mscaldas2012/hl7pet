# Data Model: PATH Parser

Entities carried over from [spec.md](spec.md)'s Key Entities section, made concrete
against the design decisions in [research.md](research.md). Types are given in Rust
since this spec's deliverable is Rust source (`crates/core/src/parser.rs`); see
[contracts/path-parser-api.md](contracts/path-parser-api.md) for the full public API
surface.

## SegIndex

A segment expression's optional index selector — `SEG_IDX` in
`contracts/path-grammar.md`.

| Variant | Fields | Corresponds to |
|---|---|---|
| `Numeric` | `u32` | `NUMBER` — 1-based occurrence index. |
| `Last` | — | `$LAST`. |
| `Star` | — | `*` (also the implicit default when `SEG_IDX` is omitted entirely). |
| `Filter` | `FilterClause` | The `@field...` filter alternative (research.md has no open question here — FR-004 made concrete). |

## SegmentExpr

One `SEGMENT_EXPR` — a segment name and its optional index.

| Field | Type | Notes |
|---|---|---|
| `name` | `&'a str` | Borrowed 3-character segment name slice from the original PATH string (no copy, FR-011). |
| `index` | `Option<SegIndex>` | `None` when `SEG_IDX` is omitted (grammar treats omission the same as `*`, `contracts/path-grammar.md`'s `SEG_IDX` production — the distinction between "omitted" and "explicit `*`" is preserved here since it costs nothing and downstream specs may care; both mean "all occurrences" semantically). |

## FieldIndex

A field expression's optional repetition index — `FIELD_IDX` in
`contracts/path-grammar.md`.

| Variant | Fields | Corresponds to |
|---|---|---|
| `Numeric` | `u32` | `NUMBER`. |
| `Last` | — | `$LAST`. |
| `Star` | — | `*`. |

## FieldExpr

One `FIELD_EXPR` — present only when the PATH includes a `-FIELD_EXPR` suffix.

| Field | Type | Notes |
|---|---|---|
| `field_num` | `u32` | `field_num` — required. |
| `index` | `Option<FieldIndex>` | `None` when `FIELD_IDX` bracket is omitted. |
| `component` | `Option<u32>` | `comp_num`, present only if a `.comp_num` suffix was given. |
| `subcomponent` | `Option<u32>` | `subcomp_num`, present only if `.comp_num.subcomp_num` was given (requires `component` to also be `Some`). |

## FilterOperator

The six comparison tokens `OPERATOR` accepts (`contracts/path-grammar.md` — this parser
enforces exactly these six, rejecting anything else per spec `001`'s Notes #3).

| Variant | Token |
|---|---|
| `Eq` | `=` |
| `Ne` | `!=` |
| `Gt` | `>` |
| `Ge` | `>=` |
| `Lt` | `<` |
| `Le` | `<=` |

## FilterClause

The parsed form of a `SEG_IDX`'s `FILTER` alternative.

| Field | Type | Notes |
|---|---|---|
| `field_num` | `u32` | Filter target field. |
| `component` | `Option<u32>` | Filter target component, if given. |
| `subcomponent` | `Option<u32>` | Filter target subcomponent, if given (requires `component` also `Some`). |
| `operator` | `FilterOperator` | One of the six tokens above. |
| `values` | `Vec<&'a str>` | One or more OR'd literal values (`VALUE { "||" VALUE }`), borrowed slices of the original PATH string; never empty by construction (research.md #4). |

## ChildPath

One `CHILD_PATH` — the hierarchy operator's right-hand side. Deliberately *not*
recursive (research.md #3): the current grammar's `CHILD_PATH` production has the same
shape as a `PATH` without a further hierarchy hop.

| Field | Type | Notes |
|---|---|---|
| `segment` | `SegmentExpr<'a>` | The child segment expression. |
| `field` | `Option<FieldExpr>` | The child's optional field expression. |

## CompiledPath

The parser's success output for one PATH string (spec.md's "Compiled PATH" Key
Entity) — the reusable, structured representation specs `007`/`008` build on.

| Field | Type | Notes |
|---|---|---|
| `source` | `&'a str` | Borrowed reference to the original PATH string — never copied (Principle II); useful for error messages and round-tripping. |
| `segment` | `SegmentExpr<'a>` | The top-level (or, in hierarchy mode, parent) segment expression. |
| `field` | `Option<FieldExpr>` | Present only for the non-hierarchy `SEGMENT_EXPR [-FIELD_EXPR]` form. |
| `child` | `Option<ChildPath<'a>>` | Present only for the hierarchy `SEGMENT_EXPR -> CHILD_PATH` form. Mutually exclusive with `field` at the top level — the grammar's two `PATH` alternatives never combine (`contracts/path-grammar.md`). |

Lifetime `'a` ties `CompiledPath` (and every borrowed field within it) to the input
PATH string's lifetime — the same zero-copy pattern spec `005`'s `ScanResult` uses for
its source message.

Total heap allocations for a successful parse: exactly one (`FilterClause.values`, only
when a filter clause is present) or zero (every other case) — no `Vec` is allocated for
the top-level `CompiledPath` structure itself, only for a filter's value list when one
exists.

## ParseErrorKind

The parser's failure output (spec.md's "Parse Error" Key Entity), returned instead of a
`CompiledPath` per Constitution Principle III — never a panic.

| Variant | Corresponds to (spec.md FR-001/FR-008) |
|---|---|
| `InvalidSegmentName` | `SEG` violated — first character not alphabetic, or wrong length. |
| `InvalidSegIndex` | `SEG_IDX` bracket content matches neither `NUMBER`, `$LAST`, `*`, nor `FILTER`. |
| `InvalidFieldIndex` | `FIELD_IDX` bracket content matches neither `NUMBER`, `$LAST`, nor `*`. |
| `InvalidOperator` | `FILTER`'s comparison token is not one of the six `OPERATOR` values. |
| `UnterminatedFilter` | A `FILTER`'s opening `'` has no matching closing `'`. |
| `UnexpectedSeparator` | A `.`/`-` appears where the grammar requires the other (e.g. `OBX[1].5`). |
| `MultipleHierarchyHops` | A second `" -> "` follows a `CHILD_PATH`, which the current single-hop grammar does not allow (research.md and `contracts/path-grammar.md` Non-Goals). |
| `UnexpectedEnd` | The string ends where the grammar requires more input (e.g. empty string, trailing `-`). |
| `TrailingInput` | Well-formed `PATH` production matched but unconsumed characters remain after it. |

Every variant is paired with a byte `offset` (research.md #6) in the top-level
`ParseError` struct — `ParseError { kind: ParseErrorKind, offset: usize }` — satisfying
FR-008 (which rule was violated + where) without a caller needing to re-scan the string
to find the problem.

## Relationships / State

```text
parse(path: &str) -> Result<CompiledPath<'_>, ParseError>
```

There is no mutable state and no intermediate "in-progress parse" object exposed
publicly — `parse` is a pure function from a PATH string to either a complete
`CompiledPath` or the first `ParseError` encountered, scanning left to right (FR-007's
"never both, never neither" exclusivity; the parser does not attempt to collect
multiple errors in one call, matching spec `005`'s scanner precedent for its own
`ScanError`).
