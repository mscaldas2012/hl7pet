# Migration Guide

## Escape-sequence decoding is now unconditional (spec 1001)

As of this change, every value `hl7pet-core` returns from `query::execute`,
`query::execute_located`, `query::first_located`, and
`hierarchy::execute_hierarchy` is decoded for standard HL7 v2 escape
sequences instead of being passed through raw. This fixes the documented
Scala engine limitation (`SPEC.md` §7: "No escaped character support").

**What decodes, and to what:**

| Sequence | Decodes to |
|---|---|
| `\F\` | that message's own field-separator character |
| `\S\` | that message's own component-separator character |
| `\T\` | that message's own subcomponent-separator character |
| `\R\` | that message's own repetition-separator character |
| `\E\` | that message's own escape character |
| `\H\` ... `\N\` | *(removed — these are highlighting markers, not characters; the text between them is kept)* |
| `\Xdddd..\` | the character(s) the hexadecimal digit pairs represent |
| `\Zxxx\` | `xxx`, unchanged (the `\Z`/`\` delimiters are stripped; there is no application-specific interpretation of the content) |

A value containing none of the above is returned exactly as before — this
change only affects values that actually contain an escape sequence. An
unterminated or unrecognized escape sequence is left completely unmodified
in the output; it is never an error.

**There is no opt-out.** No function above gained a parameter, and no
parallel "raw" function exists. Decoding is unconditional — there is no way,
per-call or globally, to get the pre-decode raw text from this crate's API.

**If you need the pre-decode raw text**: this crate does not provide it. If
your code depends on receiving raw, undecoded escape sequences, you have two
options: pin your dependency to the version of `hl7pet-core` prior to this
change, or handle re-encoding yourself downstream of the values this crate
now returns you (for example, replacing a message's own delimiter characters
back with their escape sequences, if that is sufficient for your case).

**API surface note**: `execute`, `execute_located`, `first_located`, and
`execute_hierarchy` now return `Cow<'_, str>`-based values instead of
`&str`-based ones (and `LocatedValue::value` is now `Cow<'_, str>`, no longer
`Copy`). A value with no escape sequence is still borrowed at zero
allocation cost (`Cow::Borrowed`); only a value that actually contains one
allocates (`Cow::Owned`).
