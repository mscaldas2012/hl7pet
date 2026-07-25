//! Hand-written recursive-descent PATH parser (spec 006-path-parser).
//!
//! Compiles a PATH string into a structured, reusable [`CompiledPath`] strictly
//! per the grammar in `specs/001-path-grammar-spec/contracts/path-grammar.md`,
//! or a located [`ParseError`] — never a panic, never a partial result. Pure
//! function of the PATH string alone: no message, scanner offsets, or
//! hierarchy profile involved.

/// The six comparison tokens `OPERATOR` accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOperator {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

/// The parsed form of a `SEG_IDX`'s `@field.comp.subcomp OPERATOR
/// 'value||value...'` alternative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterClause<'a> {
    pub field_num: u32,
    pub component: Option<u32>,
    pub subcomponent: Option<u32>,
    pub operator: FilterOperator,
    /// One or more OR'd literal values, borrowed from the original PATH
    /// string. Never empty by construction — the grammar's `VALUE { "||"
    /// VALUE }` always yields at least one value.
    pub values: Vec<&'a str>,
}

/// A segment expression's optional index selector (`SEG_IDX`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegIndex<'a> {
    Numeric(u32),
    Last,
    Star,
    Filter(FilterClause<'a>),
}

/// One `SEGMENT_EXPR` — a segment name and its optional index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentExpr<'a> {
    pub name: &'a str,
    /// `None` when `[SEG_IDX]` is omitted entirely — distinct from an
    /// explicit `[*]` (`Some(SegIndex::Star)`), though both mean "all
    /// occurrences" semantically.
    pub index: Option<SegIndex<'a>>,
}

/// A field expression's optional repetition index (`FIELD_IDX`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldIndex {
    Numeric(u32),
    Last,
    Star,
}

/// One `FIELD_EXPR` — present only when the PATH includes a `-FIELD_EXPR`
/// suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldExpr {
    pub field_num: u32,
    pub index: Option<FieldIndex>,
    pub component: Option<u32>,
    pub subcomponent: Option<u32>,
}

/// One `CHILD_PATH` — the hierarchy operator's right-hand side. Deliberately
/// not recursive: the grammar's `CHILD_PATH` production is single-hop only
/// (`contracts/path-grammar.md` Non-Goals) — a second `" -> "` is rejected
/// with [`ParseErrorKind::MultipleHierarchyHops`], not represented in the type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildPath<'a> {
    pub segment: SegmentExpr<'a>,
    pub field: Option<FieldExpr>,
}

/// The parser's success output for one PATH string — a reusable, structured
/// representation. Borrows from the original PATH string throughout; never
/// copies substrings where a slice suffices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledPath<'a> {
    pub source: &'a str,
    /// The top-level (or, in hierarchy mode, parent) segment expression.
    pub segment: SegmentExpr<'a>,
    /// Present only for the non-hierarchy `SEGMENT_EXPR [-FIELD_EXPR]` form.
    pub field: Option<FieldExpr>,
    /// Present only for the hierarchy `SEGMENT_EXPR -> CHILD_PATH` form.
    /// Mutually exclusive with `field` — the grammar's two `PATH`
    /// alternatives never combine.
    pub child: Option<ChildPath<'a>>,
}

/// Which grammar rule a [`ParseError`] reports as violated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// `SEG` violated — first character not alphabetic, or wrong length.
    InvalidSegmentName,
    /// `SEG_IDX` bracket content matches neither `NUMBER`, `$LAST`, `*`, nor `FILTER`.
    InvalidSegIndex,
    /// `FIELD_IDX` bracket content matches neither `NUMBER`, `$LAST`, nor `*`.
    InvalidFieldIndex,
    /// A `FILTER`'s comparison token is not one of the six `OPERATOR` values.
    InvalidOperator,
    /// A `FILTER`'s opening `'` has no matching closing `'`, or its value list
    /// is otherwise malformed.
    UnterminatedFilter,
    /// A `.`/`-` appears where the grammar requires the other (e.g. `OBX[1].5`).
    UnexpectedSeparator,
    /// A second `" -> "` follows a `CHILD_PATH`, which the current
    /// single-hop grammar does not allow.
    MultipleHierarchyHops,
    /// The string ends where the grammar requires more input.
    UnexpectedEnd,
    /// A well-formed `PATH` production matched but unconsumed characters remain.
    TrailingInput,
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ParseErrorKind::InvalidSegmentName => {
                "segment name must be 3 characters, alphabetic-led"
            }
            ParseErrorKind::InvalidSegIndex => {
                "segment index must be a number, $LAST, *, or a filter clause"
            }
            ParseErrorKind::InvalidFieldIndex => {
                "field number/index must be numeric, $LAST, or *"
            }
            ParseErrorKind::InvalidOperator => {
                "filter operator must be one of =, !=, >, >=, <, <="
            }
            ParseErrorKind::UnterminatedFilter => {
                "filter clause is missing its closing quote or is otherwise malformed"
            }
            ParseErrorKind::UnexpectedSeparator => {
                "expected '-' or ' -> ' after the segment expression"
            }
            ParseErrorKind::MultipleHierarchyHops => {
                "only a single ' -> ' hierarchy hop is supported"
            }
            ParseErrorKind::UnexpectedEnd => "PATH ended where more input was required",
            ParseErrorKind::TrailingInput => "unexpected characters after a complete PATH",
        };
        f.write_str(msg)
    }
}

/// A structural failure returned instead of a [`CompiledPath`] — never a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub offset: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (offset {})", self.kind, self.offset)
    }
}

impl std::error::Error for ParseError {}

/// A byte-offset cursor over the PATH string. All positions it hands out are
/// UTF-8 boundary-safe by construction: every advance either consumes a byte
/// already confirmed ASCII, or comes from a boundary-safe `str` operation
/// (`find`, `starts_with`) — never an arbitrary offset.
struct Cursor<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        Cursor { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

fn is_alphanum_upper(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit()
}

/// `SEG ::= ALPHA ALPHANUM ALPHANUM` — spec `001`'s tightened, alpha-first rule.
fn parse_seg<'a>(cur: &mut Cursor<'a>) -> Result<&'a str, ParseError> {
    let start = cur.pos;
    let bytes = cur.input.as_bytes();
    if start + 3 > bytes.len()
        || !bytes[start].is_ascii_uppercase()
        || !is_alphanum_upper(bytes[start + 1])
        || !is_alphanum_upper(bytes[start + 2])
    {
        return Err(ParseError {
            kind: ParseErrorKind::InvalidSegmentName,
            offset: start,
        });
    }
    cur.pos = start + 3;
    Ok(&cur.input[start..start + 3])
}

fn skip_ws(cur: &mut Cursor) {
    while matches!(cur.peek(), Some(b' ') | Some(b'\t')) {
        cur.bump();
    }
}

/// `NUMBER ::= [0-9]+`, reported as `err_kind` (contextual: `InvalidSegIndex`
/// within a filter target, `InvalidFieldIndex` within a field expression)
/// when no digit is present or the digits overflow `u32`.
fn parse_number(
    cur: &mut Cursor,
    base_offset: usize,
    err_kind: ParseErrorKind,
) -> Result<u32, ParseError> {
    let start = cur.pos;
    while matches!(cur.peek(), Some(b) if b.is_ascii_digit()) {
        cur.bump();
    }
    if cur.pos == start {
        return Err(ParseError {
            kind: err_kind,
            offset: base_offset + start,
        });
    }
    cur.input[start..cur.pos]
        .parse::<u32>()
        .map_err(|_| ParseError {
            kind: err_kind,
            offset: base_offset + start,
        })
}

/// `OPERATOR ::= "=" | "!=" | ">" | ">=" | "<" | "<="` — the maximal run of
/// `[!<>=]` characters must match one of these six tokens exactly (spec `001`
/// Notes #3: `==`, `<>`, etc. are syntax errors, not accepted-then-crashed).
fn parse_operator(cur: &mut Cursor, base_offset: usize) -> Result<FilterOperator, ParseError> {
    let start = cur.pos;
    let bytes = cur.input.as_bytes();
    let mut end = start;
    while end < bytes.len() && matches!(bytes[end], b'!' | b'<' | b'>' | b'=') {
        end += 1;
    }
    let token = &cur.input[start..end];
    let op = match token {
        "=" => Some(FilterOperator::Eq),
        "!=" => Some(FilterOperator::Ne),
        ">" => Some(FilterOperator::Gt),
        ">=" => Some(FilterOperator::Ge),
        "<" => Some(FilterOperator::Lt),
        "<=" => Some(FilterOperator::Le),
        _ => None,
    };
    match op {
        Some(op) => {
            cur.pos = end;
            Ok(op)
        }
        None => Err(ParseError {
            kind: ParseErrorKind::InvalidOperator,
            offset: base_offset + start,
        }),
    }
}

fn is_value_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'-'
}

/// `FILTER ::= "@" field_num ["." comp_num ["." subcomp_num]] WS? OPERATOR
/// WS? "'" VALUE { "||" VALUE } "'"`. `content` is the `SEG_IDX` bracket's
/// content (starts with `@`); `base_offset` is `content`'s absolute position
/// in the original PATH string, so errors report an offset into the whole
/// PATH, not just this fragment.
fn parse_filter<'a>(content: &'a str, base_offset: usize) -> Result<FilterClause<'a>, ParseError> {
    let mut cur = Cursor { input: content, pos: 1 }; // skip '@'

    let field_num = parse_number(&mut cur, base_offset, ParseErrorKind::InvalidSegIndex)?;

    let mut component = None;
    let mut subcomponent = None;
    if cur.eat(b'.') {
        component = Some(parse_number(&mut cur, base_offset, ParseErrorKind::InvalidSegIndex)?);
        if cur.eat(b'.') {
            subcomponent =
                Some(parse_number(&mut cur, base_offset, ParseErrorKind::InvalidSegIndex)?);
        }
    }

    skip_ws(&mut cur);
    let operator = parse_operator(&mut cur, base_offset)?;
    skip_ws(&mut cur);

    if !cur.eat(b'\'') {
        return Err(ParseError {
            kind: ParseErrorKind::UnterminatedFilter,
            offset: base_offset + cur.pos,
        });
    }
    let value_start = cur.pos;
    let close_rel = cur.remaining().find('\'').ok_or(ParseError {
        kind: ParseErrorKind::UnterminatedFilter,
        offset: base_offset + value_start,
    })?;
    let value_str = &content[value_start..value_start + close_rel];
    cur.pos = value_start + close_rel + 1;

    if cur.pos != content.len() {
        return Err(ParseError {
            kind: ParseErrorKind::UnterminatedFilter,
            offset: base_offset + cur.pos,
        });
    }
    if value_str.is_empty() {
        return Err(ParseError {
            kind: ParseErrorKind::UnterminatedFilter,
            offset: base_offset + value_start,
        });
    }

    let mut values = Vec::new();
    for part in value_str.split("||") {
        if part.is_empty() || !part.bytes().all(is_value_char) {
            return Err(ParseError {
                kind: ParseErrorKind::UnterminatedFilter,
                offset: base_offset + value_start,
            });
        }
        values.push(part);
    }

    Ok(FilterClause {
        field_num,
        component,
        subcomponent,
        operator,
        values,
    })
}

/// The content of a `[SEG_IDX]` bracket (no `[`/`]`), dispatched to whichever
/// of `NUMBER | $LAST | * | FILTER` it matches.
fn parse_seg_idx_content<'a>(
    content: &'a str,
    base_offset: usize,
) -> Result<SegIndex<'a>, ParseError> {
    if content == "$LAST" {
        return Ok(SegIndex::Last);
    }
    if content == "*" {
        return Ok(SegIndex::Star);
    }
    if content.starts_with('@') {
        return parse_filter(content, base_offset).map(SegIndex::Filter);
    }
    if !content.is_empty() && content.bytes().all(|b| b.is_ascii_digit()) {
        return content
            .parse::<u32>()
            .map(SegIndex::Numeric)
            .map_err(|_| ParseError {
                kind: ParseErrorKind::InvalidSegIndex,
                offset: base_offset,
            });
    }
    Err(ParseError {
        kind: ParseErrorKind::InvalidSegIndex,
        offset: base_offset,
    })
}

/// The content of a `[FIELD_IDX]` bracket, dispatched to `NUMBER | $LAST | *`.
fn parse_field_idx_content(content: &str, base_offset: usize) -> Result<FieldIndex, ParseError> {
    if content == "$LAST" {
        return Ok(FieldIndex::Last);
    }
    if content == "*" {
        return Ok(FieldIndex::Star);
    }
    if !content.is_empty() && content.bytes().all(|b| b.is_ascii_digit()) {
        return content
            .parse::<u32>()
            .map(FieldIndex::Numeric)
            .map_err(|_| ParseError {
                kind: ParseErrorKind::InvalidFieldIndex,
                offset: base_offset,
            });
    }
    Err(ParseError {
        kind: ParseErrorKind::InvalidFieldIndex,
        offset: base_offset,
    })
}

/// Locates a `[...]` bracket's content (no nested `]` is possible: neither
/// `NUMBER`/`$LAST`/`*` nor `FILTER`'s `VALUE` character class contains `]`),
/// and advances `cur` past the closing `]`.
fn parse_bracket_content<'a>(cur: &mut Cursor<'a>) -> Result<Option<(&'a str, usize)>, ()> {
    if !cur.eat(b'[') {
        return Ok(None);
    }
    let start = cur.pos;
    match cur.remaining().find(']') {
        Some(close_rel) => {
            let content = &cur.input[start..start + close_rel];
            cur.pos = start + close_rel + 1;
            Ok(Some((content, start)))
        }
        None => Err(()),
    }
}

fn parse_optional_seg_index<'a>(cur: &mut Cursor<'a>) -> Result<Option<SegIndex<'a>>, ParseError> {
    let bracket_start = cur.pos;
    match parse_bracket_content(cur) {
        Ok(None) => Ok(None),
        Ok(Some((content, start))) => parse_seg_idx_content(content, start).map(Some),
        Err(()) => Err(ParseError {
            kind: ParseErrorKind::InvalidSegIndex,
            offset: bracket_start,
        }),
    }
}

fn parse_optional_field_index(cur: &mut Cursor) -> Result<Option<FieldIndex>, ParseError> {
    let bracket_start = cur.pos;
    match parse_bracket_content(cur) {
        Ok(None) => Ok(None),
        Ok(Some((content, start))) => parse_field_idx_content(content, start).map(Some),
        Err(()) => Err(ParseError {
            kind: ParseErrorKind::InvalidFieldIndex,
            offset: bracket_start,
        }),
    }
}

/// `SEGMENT_EXPR ::= SEG ["[" SEG_IDX "]"]`.
fn parse_segment_expr<'a>(cur: &mut Cursor<'a>) -> Result<SegmentExpr<'a>, ParseError> {
    if cur.is_empty() {
        return Err(ParseError {
            kind: ParseErrorKind::UnexpectedEnd,
            offset: cur.pos,
        });
    }
    let name = parse_seg(cur)?;
    let index = parse_optional_seg_index(cur)?;
    Ok(SegmentExpr { name, index })
}

/// `FIELD_EXPR ::= field_num ["[" FIELD_IDX "]"] ["." comp_num ["." subcomp_num]]`.
/// Caller has already consumed the leading `-` and confirmed more input follows.
fn parse_field_expr(cur: &mut Cursor) -> Result<FieldExpr, ParseError> {
    let field_num = parse_number(cur, 0, ParseErrorKind::InvalidFieldIndex)?;
    let index = parse_optional_field_index(cur)?;
    let mut component = None;
    let mut subcomponent = None;
    if cur.eat(b'.') {
        component = Some(parse_number(cur, 0, ParseErrorKind::InvalidFieldIndex)?);
        if cur.eat(b'.') {
            subcomponent = Some(parse_number(cur, 0, ParseErrorKind::InvalidFieldIndex)?);
        }
    }
    Ok(FieldExpr {
        field_num,
        index,
        component,
        subcomponent,
    })
}

/// Consumes an optional `"-" FIELD_EXPR` suffix. `PID-` (trailing `-` with
/// nothing after) is `UnexpectedEnd`, matching the grammar's requirement that
/// `FIELD_EXPR` follow a `-` unconditionally once one is present.
fn parse_optional_field_expr(cur: &mut Cursor) -> Result<Option<FieldExpr>, ParseError> {
    if !cur.eat(b'-') {
        return Ok(None);
    }
    if cur.is_empty() {
        return Err(ParseError {
            kind: ParseErrorKind::UnexpectedEnd,
            offset: cur.pos,
        });
    }
    Ok(Some(parse_field_expr(cur)?))
}

/// After a `SEGMENT_EXPR` (top-level or `CHILD_PATH`'s), the only valid
/// continuations are `-FIELD_EXPR`, a `" -> "` hop (left for the caller to
/// classify — either a first hop or, for `CHILD_PATH`, a disallowed second
/// one), or the end of input. Anything else (e.g. `OBX[1].5`) is
/// `UnexpectedSeparator`.
fn parse_field_and_boundary_check(cur: &mut Cursor) -> Result<Option<FieldExpr>, ParseError> {
    if !cur.is_empty() && cur.peek() != Some(b'-') && !cur.remaining().starts_with(" -> ") {
        return Err(ParseError {
            kind: ParseErrorKind::UnexpectedSeparator,
            offset: cur.pos,
        });
    }
    parse_optional_field_expr(cur)
}

/// Parses `path` into a [`CompiledPath`], or the first [`ParseError`]
/// encountered. Pure function of `path` alone — no message, scanner output,
/// or profile is read. Never panics.
pub fn parse(path: &str) -> Result<CompiledPath<'_>, ParseError> {
    let mut cur = Cursor::new(path);
    let segment = parse_segment_expr(&mut cur)?;

    if cur.remaining().starts_with(" -> ") {
        cur.pos += " -> ".len();
        let child_segment = parse_segment_expr(&mut cur)?;
        let child_field = parse_field_and_boundary_check(&mut cur)?;
        let child = ChildPath {
            segment: child_segment,
            field: child_field,
        };

        if cur.remaining().starts_with(" -> ") {
            return Err(ParseError {
                kind: ParseErrorKind::MultipleHierarchyHops,
                offset: cur.pos,
            });
        }
        if !cur.is_empty() {
            return Err(ParseError {
                kind: ParseErrorKind::TrailingInput,
                offset: cur.pos,
            });
        }
        return Ok(CompiledPath {
            source: path,
            segment,
            field: None,
            child: Some(child),
        });
    }

    let field = parse_field_and_boundary_check(&mut cur)?;
    if !cur.is_empty() {
        return Err(ParseError {
            kind: ParseErrorKind::TrailingInput,
            offset: cur.pos,
        });
    }
    Ok(CompiledPath {
        source: path,
        segment,
        field,
        child: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_alloc::count_allocs;

    // T014 (US2, SC-004): a CompiledPath is parsed exactly once — reusing it
    // across multiple simulated call sites allocates nothing further, and
    // parse() itself allocates only for a filter clause's value list (0 or 1
    // allocations total, never growing with reuse count, research.md #5).
    #[test]
    fn reuse_without_reparse() {
        fn use_compiled(cp: &CompiledPath) -> usize {
            cp.segment.name.len()
        }

        let compiled = parse("PID[1]-5").expect("PID[1]-5 must parse");
        let reuse_allocs = count_allocs(|| {
            for _ in 0..100 {
                std::hint::black_box(use_compiled(&compiled));
            }
        });
        assert_eq!(reuse_allocs, 0, "reusing a CompiledPath must not allocate");

        let no_filter_allocs = count_allocs(|| {
            let cp = parse("PID[1]-5").unwrap();
            std::hint::black_box(&cp);
        });
        assert_eq!(no_filter_allocs, 0, "parsing a non-filter PATH allocates nothing");

        let filter_allocs = count_allocs(|| {
            let cp = parse("OBX[@3.1='94500-6||85477-8']-5").unwrap();
            std::hint::black_box(&cp);
        });
        assert_eq!(
            filter_allocs, 1,
            "parsing a filter PATH allocates exactly the values Vec, once"
        );
    }

    // T015 (US2, FR-009): parse() is a pure function of the PATH string alone
    // — its signature takes only `&str` (no message/scanner/profile
    // parameter, visible directly in the type signature above), and parsing
    // the same string twice yields observably-equal results.
    #[test]
    fn parse_is_pure() {
        let a = parse("OBX[@3.1='94500-6']-5").unwrap();
        let b = parse("OBX[@3.1='94500-6']-5").unwrap();
        assert_eq!(a, b);

        let h1 = parse("OBR[1] -> OBX-5").unwrap();
        let h2 = parse("OBR[1] -> OBX-5").unwrap();
        assert_eq!(h1, h2);
    }

    // T018 (US3): compiled-field shape for every grammar form, independent
    // of the fixtures corpus — fast, no file I/O.
    #[test]
    fn compiled_shape_bare_segment() {
        let cp = parse("PID").unwrap();
        assert_eq!(cp.segment.name, "PID");
        assert_eq!(cp.segment.index, None);
        assert_eq!(cp.field, None);
        assert_eq!(cp.child, None);
    }

    #[test]
    fn compiled_shape_seg_index_last() {
        let cp = parse("OBX[$LAST]-5").unwrap();
        assert_eq!(cp.segment.index, Some(SegIndex::Last));
    }

    #[test]
    fn compiled_shape_seg_index_star() {
        let cp = parse("OBX[*]-5").unwrap();
        assert_eq!(cp.segment.index, Some(SegIndex::Star));
    }

    #[test]
    fn compiled_shape_seg_index_omitted_differs_from_explicit_star() {
        let omitted = parse("OBX-5").unwrap();
        let explicit_star = parse("OBX[*]-5").unwrap();
        assert_eq!(omitted.segment.index, None);
        assert_eq!(explicit_star.segment.index, Some(SegIndex::Star));
        assert_ne!(omitted.segment.index, explicit_star.segment.index);
    }

    #[test]
    fn compiled_shape_field_index_numeric() {
        let cp = parse("OBX-5[2]").unwrap();
        assert_eq!(cp.field.unwrap().index, Some(FieldIndex::Numeric(2)));
    }

    #[test]
    fn compiled_shape_field_index_last_and_star() {
        assert_eq!(
            parse("OBX-5[$LAST]").unwrap().field.unwrap().index,
            Some(FieldIndex::Last)
        );
        assert_eq!(
            parse("OBX-5[*]").unwrap().field.unwrap().index,
            Some(FieldIndex::Star)
        );
    }

    #[test]
    fn compiled_shape_field_component_subcomponent() {
        let cp = parse("OBR-4.2.2").unwrap();
        let field = cp.field.unwrap();
        assert_eq!(field.field_num, 4);
        assert_eq!(field.component, Some(2));
        assert_eq!(field.subcomponent, Some(2));
    }

    #[test]
    fn compiled_shape_filter_all_six_operators() {
        for (token, op) in [
            ("=", FilterOperator::Eq),
            ("!=", FilterOperator::Ne),
            (">", FilterOperator::Gt),
            (">=", FilterOperator::Ge),
            ("<", FilterOperator::Lt),
            ("<=", FilterOperator::Le),
        ] {
            let path = format!("OBX[@3{token}'X']-5");
            let cp = parse(&path).unwrap_or_else(|e| panic!("{path:?} must parse: {e}"));
            match cp.segment.index {
                Some(SegIndex::Filter(ref clause)) => assert_eq!(clause.operator, op),
                other => panic!("{path:?}: expected a filter clause, got {other:?}"),
            }
        }
    }

    // T012 (US1): parse() must return a Result for any input, never panic —
    // including empty strings, single tokens, an unterminated filter, a
    // multi-hop hierarchy chain, and non-ASCII payload data.
    #[test]
    fn parse_never_panics_on_pathological_input() {
        let mut inputs: Vec<String> = vec![
            String::new(),
            "   ".to_string(),
            "-".to_string(),
            "@".to_string(),
            "->".to_string(),
            " -> ".to_string(),
            "PID-".to_string(),
            "PID -> ".to_string(),
            "OBX[@3.1='9945".to_string(),
            "OBX[@3.1='9945-3]-5".to_string(),
            "ORC[1] -> OBR[1] -> OBX-5".to_string(),
            "9BC-1".to_string(),
            "PID[Motörhead]-1".to_string(),
            "OBX[@3.1='94500-6||85477-8']-5".to_string(),
        ];

        let full = "PID[1]-5.2.3";
        for i in 0..=full.len() {
            inputs.push(full[..i].to_string());
        }

        for input in inputs {
            // Discarding the Result is deliberate: this test only asserts
            // parse() returns rather than panics, for every input above.
            let _ = parse(&input);
        }
    }

    // T013 (US1): each ParseErrorKind variant not already exercised by an
    // existing fixtures/vectors/path/invalid.json entry is reachable and
    // offset-correct.
    #[test]
    fn multiple_hierarchy_hops_is_rejected() {
        let path = "ORC[1] -> OBR[1] -> OBX-5";
        let err = parse(path).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::MultipleHierarchyHops);
        assert_eq!(&path[err.offset..], " -> OBX-5");
    }

    #[test]
    fn unexpected_end_on_empty_string() {
        let err = parse("").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnexpectedEnd);
        assert_eq!(err.offset, 0);
    }

    #[test]
    fn unexpected_end_on_trailing_hyphen() {
        let err = parse("PID-").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnexpectedEnd);
        assert_eq!(err.offset, 4);
    }

    #[test]
    fn trailing_input_after_complete_path() {
        let path = "PID-1 extra";
        let err = parse(path).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::TrailingInput);
        assert_eq!(&path[err.offset..], " extra");
    }

    #[test]
    fn invalid_operator_when_no_token_matches() {
        let path = "OBX[@3~'9945-3']-5";
        let err = parse(path).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidOperator);
        assert_eq!(&path[err.offset..], "~'9945-3']-5");
    }

    #[test]
    fn unexpected_separator_between_segment_and_field() {
        let path = "OBX[1].5";
        let err = parse(path).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnexpectedSeparator);
        assert_eq!(&path[err.offset..], ".5");
    }
}
