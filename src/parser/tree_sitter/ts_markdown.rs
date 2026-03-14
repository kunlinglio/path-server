//! Extract from markdown
use std::collections::HashSet;

use regex::Regex;

use crate::error::*;

use super::super::PathCandidate;
use super::ts_languages;

pub fn extract_strings(
    source: &str,
    node: &tree_sitter::Node,
) -> PathServerResult<Vec<PathCandidate>> {
    // if inline node, parse it first
    let inline_tree = if node.kind() == "inline" {
        let mut inline_parser = tree_sitter::Parser::new();
        inline_parser
            .set_language(&ts_languages::get_md_inline_language())
            .map_err(|_| PathServerError::ParseError("Failed to set inline language".into()))?;
        inline_parser
            .set_included_ranges(&[node.range()])
            .map_err(|_| PathServerError::ParseError("Failed to set ranges".into()))?;
        Some(
            inline_parser
                .parse(source, None)
                .ok_or(PathServerError::ParseError("Failed to parse inline".into()))?,
        )
    } else {
        None
    };

    let effective_node = inline_tree.as_ref().map(|t| t.root_node()).unwrap_or(*node);

    // extract strings
    let mut strings = HashSet::new();
    match effective_node.kind() {
        "link_destination" => {
            // extract content of link_destination
            return Ok(vec![PathCandidate {
                content: source[effective_node.start_byte()..effective_node.end_byte()].to_string(),
                start_byte: effective_node.start_byte(),
                end_byte: effective_node.end_byte(),
            }]);
        }
        "code_span" => {
            // resolve the content except code_span_delimiter
            let mut content_start = effective_node.start_byte();
            let mut content_end = effective_node.end_byte();
            let mut cursor = effective_node.walk();
            for child in effective_node.children(&mut cursor) {
                if child.kind() == "code_span_delimiter" {
                    if child.start_byte() == effective_node.start_byte() {
                        content_start = child.end_byte();
                    }
                    if child.end_byte() == effective_node.end_byte() {
                        content_end = child.start_byte();
                    }
                }
            }
            return Ok(vec![PathCandidate {
                content: source[content_start..content_end].to_string(),
                start_byte: content_start,
                end_byte: content_end,
            }]);
        }
        "emphasis" => {
            // strip *text* or _text_
            let inner_start = effective_node.start_byte() + 1;
            let inner_end = effective_node.end_byte() - 1;
            return Ok(vec![PathCandidate {
                content: source[inner_start..inner_end].to_string(),
                start_byte: inner_start,
                end_byte: inner_end,
            }]);
        }
        "strong_emphasis" => {
            // strip **text** or __text__
            let inner_start = effective_node.start_byte() + 2;
            let inner_end = effective_node.end_byte() - 2;
            return Ok(vec![PathCandidate {
                content: source[inner_start..inner_end].to_string(),
                start_byte: inner_start,
                end_byte: inner_end,
            }]);
        }
        "inline" if inline_tree.is_some() => {
            // fall back extractor
            let node_text = &source[effective_node.start_byte()..effective_node.end_byte()];
            let offset = effective_node.start_byte();
            let regex = [r#"'([^']+)'"#, r#""([^"]+)""#]; // extract content in `'` and `"`

            for pattern in regex {
                let re = Regex::new(pattern).unwrap();
                for cap in re.captures_iter(node_text) {
                    if let Some(inner) = cap.get(1) {
                        let content = inner.as_str();
                        strings.insert(PathCandidate {
                            content: content.to_string(),
                            start_byte: offset + inner.start(),
                            end_byte: offset + inner.end(),
                        });
                    }
                }
            }
            strings.extend(
                PathCandidate {
                    content: node_text.to_string(),
                    start_byte: effective_node.start_byte(),
                    end_byte: effective_node.end_byte(),
                }
                .split(node_text, &[' ', '\n']),
            );
        }
        "html_block" => {
            // extract paths from HTML content
            let node_text = &source[effective_node.start_byte()..effective_node.end_byte()];
            let offset = effective_node.start_byte();
            let regex = [r#"'([^']+)'"#, r#""([^"]+)""#]; // extract content in `'` and `"`

            for pattern in regex {
                let re = Regex::new(pattern).unwrap();
                for cap in re.captures_iter(node_text) {
                    if let Some(inner) = cap.get(1) {
                        let content = inner.as_str();
                        strings.insert(PathCandidate {
                            content: content.to_string(),
                            start_byte: offset + inner.start(),
                            end_byte: offset + inner.end(),
                        });
                    }
                }
            }
            strings.extend(
                PathCandidate {
                    content: node_text.to_string(),
                    start_byte: effective_node.start_byte(),
                    end_byte: effective_node.end_byte(),
                }
                .split(node_text, &[' ', '\n']),
            );
        }
        "code_block" | "fenced_code_block" => {
            // split by space and \n
            let node_text = &source[effective_node.start_byte()..effective_node.end_byte()];
            strings.extend(
                PathCandidate {
                    content: node_text.to_string(),
                    start_byte: effective_node.start_byte(),
                    end_byte: effective_node.end_byte(),
                }
                .split(node_text, &[' ', '\n']),
            );
        }
        _ => {}
    }

    // recursively process children
    let mut cursor = effective_node.walk();
    for child in effective_node.children(&mut cursor) {
        strings.extend(extract_strings(source, &child)?);
    }

    Ok(strings.into_iter().collect::<Vec<_>>())
}
