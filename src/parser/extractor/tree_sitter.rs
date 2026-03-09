use crate::document::{Document, Language};
use crate::error::*;

use super::super::PathCandidate;

/// Tree sitter languages
mod ts_languages {
    use crate::document::Language;
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
            Language::javascript => Some(get_js_language()),
            Language::typescript => Some(get_ts_language()),
            Language::python => Some(get_python_language()),
            Language::rust => Some(get_rust_language()),
            _ => None,
        }
    }
}

pub fn update_tree(new_document: &Document) -> PathServerResult<Option<tree_sitter::Tree>> {
    let old_tree = new_document.get_tree();
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
pub fn extract_strings(document: &Document) -> Option<Vec<PathCandidate>> {
    let Some(tree) = document.get_tree() else {
        return None;
    };

    // Query to extract string nodes (varies by language)
    Some(extract_strings_recursive(
        &document.text,
        &tree.root_node(),
        &document.language,
    ))
}

/// Recursively walk the syntax tree and extract string nodes
fn extract_strings_recursive(
    source: &str,
    node: &tree_sitter::Node,
    language: &Language,
) -> Vec<PathCandidate> {
    let mut strings = Vec::new();
    // Check if this node is a string
    if is_string_node(node, language)
        && let Some(literal) = extract_string_content(source, node)
    {
        strings.push(literal);
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        strings.extend(extract_strings_recursive(source, &child, language));
    }
    strings
}

/// Determine if a node represents a string literal
fn is_string_node(node: &tree_sitter::Node, language: &Language) -> bool {
    let kind = node.kind();
    match language {
        Language::javascript | Language::typescript => kind == "string_fragment",
        Language::python => kind == "string_content",
        Language::rust => kind == "string_content",
        _ => false,
    }
}

/// Extract content from a string node
fn extract_string_content(source: &str, node: &tree_sitter::Node) -> Option<PathCandidate> {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte();
    let content = source.get(start_byte..end_byte).unwrap_or("").to_string();

    Some(PathCandidate {
        content,
        start_byte,
        end_byte,
    })
}

// TODO: tree-sitter-markdown tree-sitter-html

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Language;

    fn parse_and_extract(lang: Language, src: &str) -> Vec<PathCandidate> {
        let doc = Document::new(src.to_string(), &lang.to_string())
            .expect("failed to create Document for parsing");
        extract_strings(&doc).unwrap_or_default()
    }

    /// Print the entire tree-sitter AST
    fn print_tree(language: &Language, source: &str) {
        let ts_lang =
            ts_languages::from_language(language).expect("tree-sitter language not available");
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&ts_lang)
            .expect("failed to set language");
        let tree = parser.parse(source, None).expect("failed to parse source");
        print_tree_node(source, &tree.root_node(), "", true);
    }

    fn print_tree_node(source: &str, node: &tree_sitter::Node, prefix: &str, is_last: bool) {
        let kind = node.kind();
        let start = node.start_byte();
        let end = node.end_byte();
        let raw = source.get(start..end).unwrap_or("");
        // escape newlines so each node stays on one line
        let content = raw.replace('\n', "\\n");

        // choose connector (no connector for root when prefix is empty)
        let connector = if prefix.is_empty() {
            ""
        } else if is_last {
            "└─ "
        } else {
            "├─ "
        };
        eprintln!("{}{}[{}]: {}", prefix, connector, kind, content);

        // collect children so we can know which is last
        let mut cursor = node.walk();
        let children: Vec<tree_sitter::Node> = node.children(&mut cursor).collect();
        for (i, child) in children.iter().enumerate() {
            let last = i + 1 == children.len();
            // extend prefix: if current node is last, add spaces, else add vertical bar
            let new_prefix = if prefix.is_empty() {
                if is_last {
                    "   ".to_string()
                } else {
                    "│  ".to_string()
                }
            } else {
                format!("{}{}", prefix, if is_last { "   " } else { "│  " })
            };
            print_tree_node(source, child, &new_prefix, last);
        }
    }

    #[test]
    fn test_javascript_extract_strings() {
        let normal_src = r#"const tpl = "hello world";"#;
        print_tree(&Language::javascript, normal_src);
        let res = parse_and_extract(Language::javascript, normal_src);
        assert!(
            res.iter().any(|c| c.content.contains("hello world")),
            "missing 'hello world' fragment"
        );
        let template_src = r#"const tpl = `hello ${name} world`;"#;
        print_tree(&Language::javascript, template_src);
        let res = parse_and_extract(Language::javascript, template_src);
        assert!(
            res.iter().any(|c| c.content.contains("hello")),
            "missing 'hello' fragment"
        );
        assert!(
            res.iter().any(|c| c.content.contains(" world")),
            "missing ' world' fragment"
        );
    }

    #[test]
    fn test_typescript_extract_string() {
        let normal_src = r#"const tpl: string = "hello world";"#;
        print_tree(&Language::typescript, normal_src);
        let res = parse_and_extract(Language::typescript, normal_src);
        assert!(
            res.iter().any(|c| c.content.contains("hello world")),
            "missing 'hello world' fragment"
        );
        let template_src = r#"const tpl: string = `ts ${val} end`;"#;
        print_tree(&Language::typescript, template_src);
        let res = parse_and_extract(Language::typescript, template_src);
        assert!(
            res.iter().any(|c| c.content.contains("ts ")),
            "missing 'ts ' fragment"
        );
        assert!(
            res.iter().any(|c| c.content.contains(" end")),
            "missing ' end' fragment"
        );
    }

    #[test]
    fn test_python_extract_strings() {
        let normal_src = r#"
        s = "hello"
        t = 'world'
        u = """multi\nline"""
        "#;
        print_tree(&Language::python, normal_src);
        let res = parse_and_extract(Language::python, normal_src);
        assert!(
            res.iter().any(|c| c.content.contains("hello")),
            "missing 'hello'"
        );
        assert!(
            res.iter().any(|c| c.content.contains("world")),
            "missing 'world'"
        );
        assert!(
            res.iter()
                .any(|c| c.content.trim().contains(r#"multi\nline"#)),
            "missing 'multi\nline' in triple-quoted string"
        );
        let f_string_src = r#"s = f"hello {name}""#;
        print_tree(&Language::python, f_string_src);
        let res = parse_and_extract(Language::python, f_string_src);
        assert!(
            res.iter().any(|c| c.content.contains("hello")),
            "missing 'hello' in f-string"
        );
    }

    #[test]
    fn test_rust_extract_strings() {
        let src = "let a = \"hello\"; let b = r#\"raw content\"#";
        print_tree(&Language::rust, src);
        let res = parse_and_extract(Language::rust, src);
        assert!(
            res.iter().any(|c| c.content.contains("hello")),
            "missing 'hello'"
        );
        assert!(
            res.iter().any(|c| c.content.contains("raw content")),
            "missing raw string content"
        );
    }
}
