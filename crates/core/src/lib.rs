pub mod hierarchy;
pub mod parser;
pub mod query;
pub mod scanner;

pub use hierarchy::{execute_hierarchy, HierarchyProfile, ProfileError};
pub use parser::{
    parse, ChildPath, CompiledPath, FieldExpr, FieldIndex, FilterClause, FilterOperator,
    ParseError, ParseErrorKind, SegIndex, SegmentExpr,
};
pub use query::{execute, execute_located, first_located, LocatedValue, QueryError};
pub use scanner::{
    scan, DelimiterKind, DelimiterOccurrence, DelimiterSet, ScanError, ScanResult, SegmentSpan,
};

#[cfg(test)]
mod test_alloc;
