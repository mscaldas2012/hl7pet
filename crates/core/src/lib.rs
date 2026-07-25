pub mod parser;
pub mod query;
pub mod scanner;

pub use parser::{
    parse, ChildPath, CompiledPath, FieldExpr, FieldIndex, FilterClause, FilterOperator,
    ParseError, ParseErrorKind, SegIndex, SegmentExpr,
};
pub use query::{execute, QueryError};
pub use scanner::{
    scan, DelimiterKind, DelimiterOccurrence, DelimiterSet, ScanError, ScanResult, SegmentSpan,
};

#[cfg(test)]
mod test_alloc;
