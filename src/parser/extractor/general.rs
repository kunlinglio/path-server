//! Regex based parser for fullback
use regex::Regex;

use super::PathCandidate;
use crate::{document::Document, error::PathServerError};

pub fn extract_string(document: &Document) -> Option<Vec<PathCandidate>> {
    let string_regexes = [r#"\"([^\"]*)\""#, r#"'([^']*)'"#, r#"`([^`]*)`"#];
    let regex = Regex::new(&string_regexes.join("|"))
        .map_err(|e| PathServerError::Unknown(format!("Failed to compile regex expression: {}", e)))
        .unwrap();
    let mut strings = vec![];
    for matched in regex.find_iter(&document.text) {
        strings.push(PathCandidate {
            content: matched.as_str().to_string(),
            start_byte: matched.start(),
            end_byte: matched.end(),
        })
    }
    Some(strings)
}
