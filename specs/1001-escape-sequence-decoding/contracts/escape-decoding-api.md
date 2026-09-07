# Contract: `hl7pet-core` Escape-Sequence Decoding

Amends `specs/007-query-execution/contracts/query-api.md`,
`specs/1000-located-extraction-api/contracts/located-extraction-api.md`, and
`specs/008-lazy-hierarchy-nav/contracts/hierarchy-api.md` in place — this
document is the authority for what changes in each; everything those
contracts already state about preconditions, matching/selection semantics,
and error handling continues to hold unless explicitly amended below.

## What changes

Every function's **value type** changes from `&'m str` to
`std::borrow::Cow<'m, str>`. No function's name, arity, precondition, or
matching/selection behavior changes. No new error variant.

| Function | Old value type | New value type |
|---|---|---|
| `query::execute` | `Vec<Vec<&'m str>>` | `Vec<Vec<Cow<'m, str>>>` |
| `query::execute_located` | `Vec<Vec<LocatedValue<'m>>>` (`value: &'m str`) | `Vec<Vec<LocatedValue<'m>>>` (`value: Cow<'m, str>`) |
| `query::first_located` | `Option<LocatedValue<'m>>` (`value: &'m str`) | `Option<LocatedValue<'m>>` (`value: Cow<'m, str>`) |
| `hierarchy::execute_hierarchy` | `Vec<Vec<&'m str>>` | `Vec<Vec<Cow<'m, str>>>` |

`LocatedValue<'m>` drops `#[derive(Copy)]` (retains `Clone`, `Debug`,
`PartialEq`, `Eq`) since `Cow<'_, str>` is not `Copy`.

## Decoding postcondition (new, applies to every function above)

For every value `v` any of the above functions returns:

- If the segment content `v` was resolved from contains no occurrence of the
  message's own escape-delimiter byte (`scan.delimiters.escape`), `v` MUST be
  `Cow::Borrowed`, identical byte-for-byte to what the pre-decoding version
  of that function would have returned for the same input (SC-002) — this
  case MUST NOT allocate.
- If it contains at least one occurrence, `v` MUST be `Cow::Owned`, containing
  the fully decoded text per data-model.md's grammar/mapping table (research.md
  #2): `\F\`/`\S\`/`\T\`/`\R\`/`\E\` replaced by that message's own actual
  delimiter character; `\H\`/`\N\` removed with no substitution; `\Xdddd\`
  replaced by the character(s) its hex digit pairs represent; `\Zxxx\`
  replaced by `xxx` with its delimiters stripped; anything else left
  completely unmodified, byte-for-byte, including the escape-delimiter byte
  itself (FR-006).
- Decoding is applied exactly once, to the final leaf value, after all
  field/component/subcomponent/repetition selection has already resolved —
  never before, and never triggering re-selection of its own output.

## What does NOT change

- `QueryError`: unchanged, no new variant — decoding never fails (see above).
- `ProfileError`, `HierarchyProfile`, `ScanError`, `ParseError`: untouched.
- Filter-clause (`@field=value`) evaluation (`filter_matches`): continues to
  compare **raw**, undecoded content — this contract amendment does not
  extend to it (research.md #3).
- Every precondition each amended function already had (e.g. `execute`'s
  "`path.child` MUST be `None`") is unchanged.
- There is no opt-out. No function above gains a parameter, and no sibling
  "raw" function is introduced — decoding is unconditional (spec.md FR-007).

## Non-goals (explicitly out of contract)

- Re-encoding (the reverse operation) — out of scope entirely (spec.md
  Assumptions).
- Any application-specific `\Zxxx\` lookup/interpretation mechanism —
  `\Zxxx\` decodes to its own stripped-delimiter content, nothing more.
- Any change to `filter_matches`'s comparison semantics.
