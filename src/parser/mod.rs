mod document;
mod extractor;
mod inline;

pub use document::parse_document;
pub use extractor::update_tree;
pub use inline::{parse_line, separate_prefix};

/// Represents a parsed string in the source code with its range
#[derive(Debug, Clone)]
pub struct PathRef {
    pub content: String,
    pub start_byte: usize,
    pub end_byte: usize,
}
