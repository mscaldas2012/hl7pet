pub mod scanner;

pub use scanner::{
    scan, DelimiterKind, DelimiterOccurrence, DelimiterSet, ScanError, ScanResult, SegmentSpan,
};
