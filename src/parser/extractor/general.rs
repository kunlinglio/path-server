//! Regex based parser for fullback
use regex::Regex;

use super::PathRef;
use crate::{common::PathServerError, document::Document};

pub fn extract_string(document: &Document) -> Option<Vec<PathRef>> {
    let string_regexes = [r#"\"([^\"]*)\""#, r#"'([^']*)'"#, r#"`([^`]*)`"#];
    let regex = Regex::new(&string_regexes.join("|"))
        .map_err(|e| PathServerError::Unknown(format!("Failed to compile regex expression: {}", e)))
        .unwrap();
    let mut strings = vec![];
    for matched in regex.find_iter(&document.text) {
        strings.push(PathRef {
            content: matched.as_str().to_string(),
            start_byte: matched.start(),
            end_byte: matched.end(),
        })
    }
    Some(strings)
}
