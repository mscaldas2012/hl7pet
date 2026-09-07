# Data Model: Escape-Sequence Decoding

## Modified: `LocatedValue<'m>` (spec `1000`)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]   // Copy dropped — Cow<str> is not Copy
pub struct LocatedValue<'m> {
    pub value: Cow<'m, str>,             // was: &'m str
    pub line: usize,                     // unchanged
}
```

**Invariants** (unchanged from spec `1000`, restated against the new type):
- `value` is `Cow::Borrowed` whenever the underlying raw content contains no
  escape sequence; `Cow::Owned` only when decoding actually rewrote something.
- `line` semantics are entirely unaffected by this feature.

## New: `decode_escapes` (crate-private, `query.rs`)

```rust
/// Decodes standard HL7 v2 escape sequences in `raw`, using `delimiters`'
/// own escape/field/component/subcomponent/repetition characters (never a
/// hardcoded `\`). Returns `Cow::Borrowed(raw)` unchanged, at zero
/// allocation cost, when `raw` contains no escape-delimiter byte at all —
/// the fast path research.md #1 requires. A malformed or unrecognized
/// escape sequence is left completely unmodified in the output (FR-006);
/// this function never fails and never panics.
pub(crate) fn decode_escapes<'m>(raw: &'m str, delimiters: &DelimiterSet) -> Cow<'m, str>;
```

**Behavior**: see research.md #2 for the full grammar and decode-mapping
table. Applied once, to each final leaf value, strictly after
field/component/subcomponent boundaries have already been resolved (spec.md
Edge Cases) — never re-triggers re-splitting of its own output.

## Modified: `resolve_field_values` / `resolve_field_values_located`

```rust
pub(crate) fn resolve_field_values<'m>(
    field_expr: Option<&FieldExpr>,
    segment_content: &'m str,
    segment_name: &str,
    delimiters: &DelimiterSet,
) -> Vec<Cow<'m, str>>;                  // was: Vec<&'m str>

pub(crate) fn resolve_field_values_located<'m>(
    line: usize,
    field_expr: Option<&FieldExpr>,
    segment_content: &'m str,
    segment_name: &str,
    delimiters: &DelimiterSet,
) -> Vec<LocatedValue<'m>>;              // unchanged signature; LocatedValue.value now Cow
```

Both call `decode_escapes` on each resolved leaf value before wrapping it
into their result, in place of the direct `&'m str` they previously
produced. No other change to either function's selection/navigation logic
(field index, component/subcomponent extraction) — decoding is strictly a
final wrapping step (research.md #4).

Internally, both now select repetitions and map/decode them in a single step
via a new `select_and_map<T>(repetitions, index, f) -> Vec<T>`, replacing the
former `select_by_field_index(repetitions, index) -> Vec<&'m str>` entirely
(research.md #1's verified benchmark finding: mapping an already-materialized
`Vec<&str>` into a `Vec<Cow<str>>`/`Vec<LocatedValue>` afterward costs one
avoidable extra allocation per call, regardless of message size — `select_and_map`
collects directly from the repetitions slice instead, which never had a
buffer to reuse in the first place, so there is nothing lost).

## Modified: `execute`, `execute_located`, `first_located`

```rust
pub fn execute<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
) -> Result<Vec<Vec<Cow<'m, str>>>, QueryError>;      // was: Vec<Vec<&'m str>>

pub fn execute_located<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
) -> Result<Vec<Vec<LocatedValue<'m>>>, QueryError>;  // unchanged signature; LocatedValue.value now Cow

pub fn first_located<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
) -> Result<Option<LocatedValue<'m>>, QueryError>;    // unchanged signature; LocatedValue.value now Cow
```

No precondition, postcondition-shape, or error-handling change beyond the
value type itself — every existing invariant from
`specs/007-query-execution/contracts/query-api.md` and
`specs/1000-located-extraction-api/contracts/located-extraction-api.md`
still holds, with "the value" now meaning "the *decoded* value."

## Modified: `execute_hierarchy` (spec `008`)

```rust
pub fn execute_hierarchy<'m>(
    scan: &ScanResult<'m>,
    path: &CompiledPath<'_>,
    profile: Option<&HierarchyProfile>,
) -> Result<Vec<Vec<Cow<'m, str>>>, QueryError>;      // was: Vec<Vec<&'m str>>
```

The non-hierarchy branch (`path.child.is_none()`) already delegates directly
to `query::execute(scan, path)` — once `execute`'s return type changes, this
delegation requires no logic change at all, only a matching return-type
update. The hierarchy branch's own final step (currently calling
`query::resolve_field_values`) picks up decoding automatically once that
function is modified per above.

## `QueryError`

**Unchanged.** No new variant. Confirmed by research.md #2: every decode
failure mode (unterminated sequence, unrecognized type character, invalid
hex, non-UTF-8 result) resolves to "leave the input unmodified," never an
`Err`.

## Relationship to existing entities

| Existing entity (spec) | Relationship |
|---|---|
| `DelimiterSet` (spec `005`) | Read-only input to `decode_escapes` — supplies the message's own field/component/subcomponent/repetition/escape characters, exactly as `field_at`/`extract_component` already consume it. Unmodified. |
| `ScanResult<'m>` (spec `005`) | Unmodified — `scan.message`'s lifetime `'m` is what `Cow::Borrowed` variants continue to borrow from. |
| `CompiledPath<'_>` (spec `006`) | Unmodified — no grammar or navigation change. |
| `FilterClause`/`filter_matches` (spec `007`) | Unmodified — filter comparisons stay against raw content (research.md #3). |
