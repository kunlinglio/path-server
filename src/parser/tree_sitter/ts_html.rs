//! Extract from html
use regex::Regex;

use crate::document::Language;

use super::PathCandidate;

pub fn extract_strings(
    source: &str,
    node: &tree_sitter::Node,
    language: &Language,
) -> Vec<PathCandidate> {
    assert_eq!(language, &Language::html);

    let mut strings = Vec::new();
    // check if this node is a string
    if is_string_node(node) {
        strings.extend(extract_string_content(source, node));
    }

    // recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        strings.extend(extract_strings(source, &child, language));
    }
    strings
}

fn extract_string_content(source: &str, node: &tree_sitter::Node) -> Vec<PathCandidate> {
    let mut candidates = Vec::new();
    let mut cursor = node.walk();
    let mut begin_byte = node.start_byte();
    let mut end_byte = node.end_byte();
    let mut have_string_fragment = false;
    for child in node.children(&mut cursor) {
        if is_string_fragment_node(&child) {
            if !have_string_fragment {
                begin_byte = child.start_byte();
                have_string_fragment = true;
            }
            end_byte = child.end_byte();
            continue;
        }
    }
    // add the last candidate after the last fragment
    if begin_byte < end_byte {
        let candidate = PathCandidate {
            content: source.get(begin_byte..end_byte).unwrap_or("").to_string(),
            start_byte: begin_byte,
            end_byte,
        };
        candidates.push(candidate);
    }
    // fall back regex parse
    if need_regex_parse(node) {
        // parse based on `'` and `"` and space
        let node_text = &source[node.start_byte()..node.end_byte()];
        let offset = node.start_byte();
        let regex = [r#"'([^']+)'"#, r#""([^"]+)""#]; // extract content in `'` and `"`

        for pattern in regex {
            let re = Regex::new(pattern).unwrap();
            for cap in re.captures_iter(node_text) {
                if let Some(inner) = cap.get(1) {
                    let content = inner.as_str();
                    candidates.push(PathCandidate {
                        content: content.to_string(),
                        start_byte: offset + inner.start(),
                        end_byte: offset + inner.end(),
                    });
                }
            }
        }
        candidates.extend(
            PathCandidate {
                content: node_text.to_string(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            }
            .split(node_text, &[' ', '\n']),
        );
    }
    candidates
}

/// Determine if a node represents a string literal
fn is_string_node(node: &tree_sitter::Node) -> bool {
    let kind = node.kind();
    kind == "text" || kind == "quoted_attribute_value"
}

/// Determine if a node represents a part of string literal
fn is_string_fragment_node(node: &tree_sitter::Node) -> bool {
    let kind = node.kind();
    kind == "attribute_value"
}

fn need_regex_parse(node: &tree_sitter::Node) -> bool {
    node.kind() == "text"
}
