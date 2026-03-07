//! Parsers for document path parsing.

use crate::common::*;
use crate::document::Document;
mod general;
mod tree_sitter;

/// Represents a parsed string in the source code with its range
#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub content: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_col: usize,
}

pub use tree_sitter::update_tree;

pub fn extract_string(document: &Document) -> PathServerResult<Vec<StringLiteral>> {
    let res = tree_sitter::extract_strings(document)?;
    if res.is_none() {
        Ok(general::extract_string(document).unwrap_or_default())
    } else {
        Ok(res.unwrap())
    }
}
