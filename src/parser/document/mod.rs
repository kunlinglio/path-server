//! Parsers for document path parsing.

use crate::document::Document;
mod general;
mod tree_sitter;

/// Represents a parsed string in the source code with its range
#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub content: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

pub use tree_sitter::update_tree;

pub fn extract_string(document: &Document) -> Vec<StringLiteral> {
    let res = tree_sitter::extract_strings(document);
    if res.is_none() {
        // fall back to general parser
        general::extract_string(document).unwrap_or_default()
    } else {
        res.unwrap()
    }
}
