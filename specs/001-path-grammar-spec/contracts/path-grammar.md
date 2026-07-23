# HL7-PET PATH Grammar

Formal grammar for the PATH expression language (spec FR-001). This is the build
contract for spec `006` (Rust PATH parser) and the classification reference for every
conformance vector produced by this spec.

Notation follows the informal EBNF dialect `SPEC.md` §3.1 already uses (`::=`, `|`,
`[...]` optional, quoted literals), per `research.md` Decision 1.

## Grammar

```text
PATH         ::= SEGMENT_EXPR [ "-" FIELD_EXPR ]
               | SEGMENT_EXPR " -> " CHILD_PATH      (hierarchy mode only — see Non-Goals)

SEGMENT_EXPR ::= SEG [ "[" SEG_IDX "]" ]

SEG          ::= ALPHA ALPHANUM ALPHANUM
ALPHA        ::= /[A-Z]/
ALPHANUM     ::= /[A-Z0-9]/

SEG_IDX      ::= NUMBER                       (1-based occurrence index)
               | "$LAST"                      (last occurrence)
               | "*"                          (all occurrences — same as omitting the index)
               | FILTER                       (field-value filter, no "@" — see FILTER)

FILTER       ::= "@" field_num [ "." comp_num [ "." subcomp_num ] ] WS? OPERATOR WS? "'" VALUE { "||" VALUE } "'"
OPERATOR     ::= "=" | "!=" | ">" | ">=" | "<" | "<="
VALUE        ::= /[A-Za-z0-9_.-]+/            (no escaping needed — quote/pipe chars are outside this class)
WS           ::= /[ \t]*/                     (optional; only significant immediately around OPERATOR)

FIELD_EXPR   ::= field_num [ "[" FIELD_IDX "]" ] [ "." comp_num [ "." subcomp_num ] ]
FIELD_IDX    ::= NUMBER | "$LAST" | "*"       ("*" documented here for the first time — see Notes)

NUMBER       ::= /[0-9]+/
field_num    ::= NUMBER                       (1-based; MSH field 1 = "|", field 2 = "^~\&")
comp_num     ::= NUMBER                       (1-based component)
subcomp_num  ::= NUMBER                       (1-based sub-component)

CHILD_PATH   ::= SEGMENT_EXPR [ "-" FIELD_EXPR ]
```

## Validity examples

| PATH | Valid? | Why |
|---|---|---|
| `PID` | Yes | Bare `SEGMENT_EXPR`, no index or field |
| `PID-1` | Yes | |
| `PID[1]-1` | Yes | |
| `PID-3.2` | Yes | |
| `ZL1` | Yes | `Z`,`L` alpha, `1` alphanumeric — matches `ALPHA ALPHANUM ALPHANUM` |
| `Z01` | Yes | `Z` alpha, `0`,`1` alphanumeric |
| `XYZ-99` | Yes (syntax) | Well-formed; evaluates to zero values at runtime (FR-004) since no such segment/field typically exists — **not** a grammar violation |
| `OBX[@3.1='94500-6']-5` | Yes | `FILTER` with component, no subcomponent |
| `OBX[@3.1.2='94500-6']-5` | Yes | `FILTER` with subcomponent (see Notes) |
| `OBX[@3.1 = '94500-6']-5` | Yes | Optional whitespace around `OPERATOR` |
| `999[1].1.1` | **No** | Invalid for two independent reasons: `9` fails `ALPHA` in `SEG`'s first position (`999` alone is no longer a valid segment name under this tightened grammar — see Notes #1), *and* separately `.` is used where `-` is required between `SEGMENT_EXPR` and `FIELD_EXPR` |
| `9BC` | **No** | First character `9` fails `ALPHA` |
| `PID[ABC]-1` | **No** | `ABC` matches neither `NUMBER`, `$LAST`, `*`, nor `FILTER` |
| `OBX[@3=='9945-3']` | **No** | `==` is not a valid `OPERATOR` |

## Notes (deviations from `SPEC.md` §3.1 and the current Scala regex)

These are documented decisions, not silent guesses (FR-006), made during this
session:

1. **`SEG` is tightened to alpha-first.** `SPEC.md` and the current regex
   (`[A-Z0-9]{3}`) both technically accept an all-digit segment name like `999`.
   The new grammar requires the first character to be alphabetic — `ZL1` and `Z01`
   (real-world Z-segment conventions) remain valid; `999` does not. This is a
   deliberate improvement, not a preserved bug.

2. **`SEG_IDX` and `FIELD_IDX` are tightened to reject non-numeric, non-`$LAST`,
   non-`*` content at parse time.** The current regex's index bracket accepts any
   alphanumeric string (`[0-9a-zA-Z\$]+`); something like `PID[ABC]-1` matches the
   regex today but then throws an uncaught `NumberFormatException` at evaluation
   time. The new grammar rejects this as a syntax error (FR-012 invalid-PATH
   vector) instead of reproducing the crash.

3. **`OPERATOR` is tightened to the six defined tokens.** The current regex's
   operator class (`[!><\=]{1,2}`) would also match nonsense like `==` or `<>`,
   which then hits an unhandled case in the evaluator (`MatchError`). The new
   grammar only accepts `=`, `!=`, `>`, `>=`, `<`, `<=`.

4. **Optional whitespace around `OPERATOR` is newly allowed** (`OBX[@3.1 =
   '94500-6']-5` parses identically to no spaces). This is purely additive — it
   only accepts more strings, never changes the meaning of an already-valid one —
   so it isn't a breaking change under Constitution Principle I.

5. **`FILTER` gains an optional subcomponent level** (`@field.comp.subcomp`). This
   was already present in the real `FILTER_REGEX` and handled by the evaluator
   (`filterValues`'s `subcomp` handling), but `SPEC.md` §3.1's prose grammar never
   documented it. This is a documentation-completeness fix, not a behavior change —
   the capability already worked, it just wasn't written down.

6. **`FIELD_IDX` documents `"*"` for the first time.** The current regex's field-index
   bracket already accepts `*` (`[0-9a-zA-Z\$]+|\*`), and it degrades gracefully to
   "all repetitions" (the same as omitting the index) rather than crashing, but
   `SPEC.md` never documented it as a valid `FIELD_IDX` value. Documentation fix
   only, no behavior change.

7. **`VALUE`'s character class needs no escaping.** The real `FILTER_REGEX`
   restricts filter values to `[A-Za-z0-9_.-]+` — this excludes the terminating
   quote (`'`) and the `||` OR-separator by construction, so there is no case where
   a value needs to contain (and therefore escape) either delimiter. This closes
   what would otherwise have been an open escaping question.

8. **`SEG_IDX`'s `FILTER` alternative no longer shows `"@"` as its own literal
   token.** `SPEC.md` writes it as `SEG_IDX ::= ... | "@" FILTER`; this grammar
   instead folds the `"@"` into `FILTER`'s own definition (`FILTER ::= "@"
   field_num ...`). This is a pure notational refactor, not a syntax change — the
   same strings are accepted either way, and `@` is still required exactly as
   before (confirmed explicitly during clarification: dropping `@` would be a
   breaking change, and that was not the decision made).

## Non-Goals

- **Hierarchy evaluation semantics** (what `->` does with parent-scoping and
  cardinality) are explicitly out of scope — owned by spec `002`. This grammar only
  defines `->` as a syntax token in the `PATH` production. See `../spec.md`'s
  Assumptions section and User Story 3 for why this boundary is drawn here.
- **Whitespace handling around `->`** is left to spec `002`, since it's a
  hierarchy-mode-only construct and this spec found no evidence in the source that
  it needs the same treatment as `FILTER`'s `OPERATOR`.
- **`SEG_IDX`/`FIELD_IDX` numeric range validation** (e.g. is `0` meaningful?) is not
  a grammar concern. Any syntactically valid `NUMBER` is accepted here; whether a
  particular value matches real data is FR-004's zero-values case, not a grammar
  violation — same treatment as `XYZ-99`.
