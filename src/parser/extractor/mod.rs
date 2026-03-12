use crate::document::Document;
use crate::error::*;
use crate::parser::PathCandidate;

mod general;
mod tree_sitter;

pub use tree_sitter::{new_tree, update_tree};

pub fn extract_string(document: &Document) -> PathServerResult<Vec<PathCandidate>> {
    let res = tree_sitter::extract_strings(document)?;
    if let Some(res) = res {
        Ok(res)
    } else {
        // fall back to general parser
        Ok(general::extract_string(document).unwrap_or_default())
    }
}
