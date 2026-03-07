//! Parsers for document path parsing.
use std::vec::Vec;

use crate::document::Document;

use super::PathRef;
use super::extractor::extract_string;

pub fn parse_document(document: &Document) -> Vec<PathRef> {
    extract_string(document)
        .into_iter()
        .filter(|s| is_path(&s.content))
        .collect()
}

fn is_path(path: &str) -> bool {
    path.contains('/') || path.contains('\\')
}
