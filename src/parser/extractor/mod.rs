use crate::{document::Document, parser::PathRef};
mod general;
mod tree_sitter;

pub use tree_sitter::update_tree;

pub fn extract_string(document: &Document) -> Vec<PathRef> {
    let res = tree_sitter::extract_strings(document);
    if res.is_none() {
        // fall back to general parser
        general::extract_string(document).unwrap_or_default()
    } else {
        res.unwrap()
    }
}
