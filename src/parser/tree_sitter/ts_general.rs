//! Extract from multi languages: typescript, javascript, rust, python
use crate::document::Language;

use super::PathCandidate;

pub fn extract_strings(
    source: &str,
    node: &tree_sitter::Node,
    language: &Language,
) -> Vec<PathCandidate> {
    let compatible_languages = [
        Language::typescript,
        Language::javascript,
        Language::python,
        Language::rust,
        Language::c,
        Language::c_plus_plus,
    ];
    assert!(compatible_languages.contains(language));

    let mut strings = Vec::new();
    // check if this node is a string
    if is_string_node(node, language) {
        strings.extend(extract_string_content(source, node, language));
    }

    // recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        strings.extend(extract_strings(source, &child, language));
    }
    strings
}

fn extract_string_content(
    source: &str,
    node: &tree_sitter::Node,
    language: &Language,
) -> Vec<PathCandidate> {
    let mut candidates = Vec::new();
    let mut cursor = node.walk();
    let mut begin_byte = node.start_byte();
    let mut end_byte = node.end_byte();
    let mut have_string_fragment = false;
    for child in node.children(&mut cursor) {
        if is_string_fragment_node(&child, language) {
            if !have_string_fragment {
                begin_byte = child.start_byte();
                have_string_fragment = true;
            }
            end_byte = child.end_byte();
            continue;
        }
        if !is_escaped_character_node(&child, language) {
            // is not a string fragment or escaped character, treat it as a separator
            // the content before it is a candidate
            if child.start_byte() > begin_byte {
                // only add candidate if there is content before the separator
                let candidate = PathCandidate {
                    content: source
                        .get(begin_byte..child.start_byte())
                        .unwrap_or("")
                        .to_string(),
                    start_byte: begin_byte,
                    end_byte: child.start_byte(),
                };
                candidates.push(candidate);
            }
            begin_byte = child.end_byte();
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
    candidates
}

/// Determine if a node represents a string literal
fn is_string_node(node: &tree_sitter::Node, language: &Language) -> bool {
    let kind = node.kind();
    match language {
        Language::javascript | Language::typescript => {
            kind == "string" || kind == "template_string"
        }
        Language::python => kind == "string",
        Language::rust => kind == "string_literal" || kind == "raw_string_literal",
        Language::c | Language::c_plus_plus => kind == "string_literal",
        _ => unreachable!("is_string_node called with unsupported language"),
    }
}

/// Determine if a node represents a part of string literal
fn is_string_fragment_node(node: &tree_sitter::Node, language: &Language) -> bool {
    let kind = node.kind();
    match language {
        Language::javascript | Language::typescript => kind == "string_fragment",
        Language::python => kind == "string_content",
        Language::rust => kind == "string_content",
        Language::c | Language::c_plus_plus => kind == "string_content",
        _ => unreachable!("is_string_fragment_node called with unsupported language"),
    }
}

/// Determine if a node represents an escaped character in a string
/// This will be included in the path candidate
fn is_escaped_character_node(node: &tree_sitter::Node, language: &Language) -> bool {
    let kind = node.kind();
    match language {
        Language::javascript | Language::typescript => kind == "escape_sequence",
        Language::python => kind == "escape_sequence",
        Language::rust => kind == "escape_sequence",
        Language::c | Language::c_plus_plus => kind == "escape_sequence",
        _ => unreachable!("is_escaped_character_node called with unsupported language"),
    }
}
