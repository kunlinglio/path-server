use crate::common::*;
use crate::document::Document;

use super::super::languages::Language;
use super::StringLiteral;

/// Tree sitter languages
mod ts_languages {
    use super::super::super::languages::Language;
    use std::sync::OnceLock;

    static JS_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();
    static TS_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();
    static PY_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();
    static RS_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();

    pub fn get_js_language() -> tree_sitter::Language {
        JS_LANGUAGE
            .get_or_init(|| tree_sitter_javascript::LANGUAGE.into())
            .clone()
    }

    pub fn get_ts_language() -> tree_sitter::Language {
        TS_LANGUAGE
            .get_or_init(|| tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .clone()
    }

    pub fn get_python_language() -> tree_sitter::Language {
        PY_LANGUAGE
            .get_or_init(|| tree_sitter_python::LANGUAGE.into())
            .clone()
    }

    pub fn get_rust_language() -> tree_sitter::Language {
        RS_LANGUAGE
            .get_or_init(|| tree_sitter_rust::LANGUAGE.into())
            .clone()
    }

    /// Convert from Language, return None if not supported
    pub fn from_language(language: &Language) -> Option<tree_sitter::Language> {
        match language {
            Language::JavaScript => Some(get_js_language()),
            Language::TypeScript => Some(get_ts_language()),
            Language::Python => Some(get_python_language()),
            Language::Rust => Some(get_rust_language()),
            _ => None,
        }
    }
}

pub fn update_tree(new_document: &Document) -> PathServerResult<Option<tree_sitter::Tree>> {
    let old_tree = new_document.tree.as_ref();
    let Some(ts_language) = ts_languages::from_language(&new_document.language) else {
        return Ok(None);
    };
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&ts_language).map_err(|e| {
        PathServerError::ParseError(format!("Set language to tree-sitter failed: {}", e))
    })?;
    Ok(parser.parse(&new_document.text, old_tree))
}

/// Extract string literals from source code using tree-sitter
/// Returns a vector of StringLiteral with their positions in the source
pub fn extract_strings(document: &Document) -> PathServerResult<Option<Vec<StringLiteral>>> {
    let Some(tree) = &document.tree else {
        return Ok(None);
    };

    let mut strings = Vec::new();

    // Query to extract string nodes (varies by language)
    // This is a simplified version - in production we'd use more sophisticated queries
    extract_strings_recursive(
        &document.text,
        &tree.root_node(),
        &mut strings,
        &document.language,
    );

    Ok(Some(strings))
}

/// Recursively walk the syntax tree and extract string nodes
fn extract_strings_recursive(
    source: &str,
    node: &tree_sitter::Node,
    strings: &mut Vec<StringLiteral>,
    language: &Language,
) {
    // Check if this node is a string
    if is_string_node(node, &language) {
        if let Some(literal) = extract_string_content(source, node) {
            strings.push(literal);
        }
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_strings_recursive(source, &child, strings, language);
    }
}

/// Determine if a node represents a string literal
fn is_string_node(node: &tree_sitter::Node, language: &Language) -> bool {
    let kind = node.kind();
    match language {
        Language::JavaScript | Language::TypeScript => {
            kind == "string" || kind == "template_string"
        }
        Language::Python => kind == "string",
        Language::Rust => kind == "string_literal",
        Language::Markdown => kind == "code_span" || kind == "inline_code",
        Language::HTML => kind == "attribute_value" || kind == "text",
        _ => false,
    }
}

/// Extract content from a string node
fn extract_string_content(source: &str, node: &tree_sitter::Node) -> Option<StringLiteral> {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();
    let start_pos = node.start_position();
    let content = source.get(start_byte..end_byte).unwrap_or("").to_string();

    Some(StringLiteral {
        content,
        start_byte,
        end_byte,
        start_line: start_pos.row,
        start_col: start_pos.column,
    })
}
