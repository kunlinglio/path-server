use std::collections::HashSet;

use regex::Regex;

use crate::document::{Document, Language};
use crate::error::*;

use super::super::PathCandidate;
use super::super::split;

/// Tree sitter languages
mod ts_languages {
    use crate::document::Language;
    use std::sync::OnceLock;

    static JS_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();
    static TS_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();
    static PY_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();
    static RS_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();
    static MD_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();
    static MD_INLINE_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();
    static HTML_LANGUAGE: OnceLock<tree_sitter::Language> = OnceLock::new();

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

    pub fn get_md_language() -> tree_sitter::Language {
        MD_LANGUAGE
            .get_or_init(|| tree_sitter_md::LANGUAGE.into())
            .clone()
    }

    pub fn get_md_inline_language() -> tree_sitter::Language {
        MD_INLINE_LANGUAGE
            .get_or_init(|| tree_sitter_md::INLINE_LANGUAGE.into())
            .clone()
    }

    pub fn get_html_language() -> tree_sitter::Language {
        HTML_LANGUAGE
            .get_or_init(|| tree_sitter_html::LANGUAGE.into())
            .clone()
    }

    /// Convert from Language, return None if not supported
    pub fn from_language(language: &Language) -> Option<tree_sitter::Language> {
        match language {
            Language::javascript => Some(get_js_language()),
            Language::typescript => Some(get_ts_language()),
            Language::python => Some(get_python_language()),
            Language::rust => Some(get_rust_language()),
            Language::markdown => Some(get_md_language()),
            Language::html => Some(get_html_language()),
            _ => None,
        }
    }
}

pub fn new_tree(document: &Document) -> PathServerResult<Option<tree_sitter::Tree>> {
    let Some(ts_language) = ts_languages::from_language(&document.language) else {
        return Ok(None);
    };
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&ts_language).map_err(|e| {
        PathServerError::ParseError(format!("Set language to tree-sitter failed: {}", e))
    })?;
    Ok(parser.parse(&document.text, None))
}

pub fn update_tree(
    old_document: &Document,
    mut old_tree: Option<tree_sitter::Tree>,
    new_document: &Document, // the document has updated every member except the tree
    change_start_byte: usize,
    change_old_end_byte: usize, // the byte range of the change in the old document
    change_new_end_byte: usize, // the byte range of the change in the new document
) -> PathServerResult<Option<tree_sitter::Tree>> {
    let Some(ts_language) = ts_languages::from_language(&new_document.language) else {
        return Ok(None);
    };
    // prepare InputEdit for tree-sitter
    let start = old_document.offset_to_utf8_pos(change_start_byte)?;
    let old_end = old_document.offset_to_utf8_pos(change_old_end_byte)?;
    let new_end = new_document.offset_to_utf8_pos(change_new_end_byte)?;
    let edit = tree_sitter::InputEdit {
        start_byte: change_start_byte,
        old_end_byte: change_old_end_byte,
        new_end_byte: change_new_end_byte,
        start_position: tree_sitter::Point {
            row: start.0,
            column: start.1,
        },
        old_end_position: tree_sitter::Point {
            row: old_end.0,
            column: old_end.1,
        },
        new_end_position: tree_sitter::Point {
            row: new_end.0,
            column: new_end.1,
        },
    };
    if let Some(ref mut tree) = old_tree {
        tree.edit(&edit);
    }
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&ts_language).map_err(|e| {
        PathServerError::ParseError(format!("Set language to tree-sitter failed: {}", e))
    })?;
    Ok(parser.parse(&new_document.text, old_tree.as_ref()))
}

/// Extract string literals from source code using tree-sitter
/// Returns a vector of StringLiteral with their positions in the source
pub fn extract_strings(document: &Document) -> PathServerResult<Option<Vec<PathCandidate>>> {
    let Some(tree) = document.get_tree() else {
        return Ok(None);
    };

    if document.language == Language::markdown {
        Ok(Some(extract_strings_markdown(
            &document.text,
            &tree.root_node(),
        )?))
    } else {
        Ok(Some(extract_strings_normal(
            &document.text,
            &tree.root_node(),
            &document.language,
        )))
    }
}

fn extract_strings_markdown(
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
            strings.extend(split(
                node_text,
                &PathCandidate {
                    content: node_text.to_string(),
                    start_byte: effective_node.start_byte(),
                    end_byte: effective_node.end_byte(),
                },
                &[' ', '\n'],
            ));
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
            strings.extend(split(
                node_text,
                &PathCandidate {
                    content: node_text.to_string(),
                    start_byte: effective_node.start_byte(),
                    end_byte: effective_node.end_byte(),
                },
                &[' ', '\n'],
            ));
        }
        "code_block" | "fenced_code_block" => {
            // split by space and \n
            let node_text = &source[effective_node.start_byte()..effective_node.end_byte()];
            strings.extend(split(
                node_text,
                &PathCandidate {
                    content: node_text.to_string(),
                    start_byte: effective_node.start_byte(),
                    end_byte: effective_node.end_byte(),
                },
                &[' ', '\n'],
            ));
        }
        _ => {}
    }

    // recursively process children
    let mut cursor = effective_node.walk();
    for child in effective_node.children(&mut cursor) {
        strings.extend(extract_strings_markdown(source, &child)?);
    }

    Ok(strings.into_iter().collect::<Vec<_>>())
}

/// Recursively walk the syntax tree and extract string nodes
fn extract_strings_normal(
    source: &str,
    node: &tree_sitter::Node,
    language: &Language,
) -> Vec<PathCandidate> {
    let mut strings = Vec::new();
    // check if this node is a string
    if is_string_node(node, language) {
        strings.extend(extract_string_content(source, node, language));
    }

    // recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        strings.extend(extract_strings_normal(source, &child, language));
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
        eprintln!("Child kind: {}", child.kind());

        if is_string_fragment_node(&child, language) {
            if !have_string_fragment {
                begin_byte = child.start_byte();
                have_string_fragment = true;
            }
            // if i == node.child_count() - 1 {
            end_byte = child.end_byte();
            // }
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
            end_byte: end_byte,
        };
        candidates.push(candidate);
    }
    // fall back regex parse
    if need_regex_parse(node, language) {
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
        candidates.extend(split(
            node_text,
            &PathCandidate {
                content: node_text.to_string(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            },
            &[' ', '\n'],
        ));
    }
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
        Language::markdown => unreachable!("is_string_node called with markdown language"),
        Language::html => kind == "text" || kind == "quoted_attribute_value",
        _ => false,
    }
}

/// Determine if a node represents a part of string literal
fn is_string_fragment_node(node: &tree_sitter::Node, language: &Language) -> bool {
    let kind = node.kind();
    match language {
        Language::javascript | Language::typescript => kind == "string_fragment",
        Language::python => kind == "string_content",
        Language::rust => kind == "string_content",
        Language::markdown => unreachable!("is_string_fragment_node called with markdown language"),
        Language::html => kind == "attribute_value",
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
        Language::markdown => {
            unreachable!("is_escaped_character_node called with markdown language")
        }
        Language::html => true,
        _ => unreachable!("is_escaped_character_node called with unsupported language"),
    }
}

fn need_regex_parse(node: &tree_sitter::Node, language: &Language) -> bool {
    match language {
        Language::javascript | Language::typescript => false,
        Language::python => false,
        Language::rust => false,
        Language::markdown => {
            unreachable!("need_regex_parse called with markdown language")
        }
        Language::html => node.kind() == "text",
        _ => unreachable!("need_regex_parse called with unsupported language"),
    }
}

// TODO: tree-sitter-html

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Language;

    fn parse_and_extract(lang: Language, src: &str) -> Vec<PathCandidate> {
        let doc = Document::new(src.to_string(), &lang.to_string())
            .expect("failed to create Document for parsing");
        extract_strings(&doc).unwrap().unwrap()
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
        print_tree_node(source, &tree.root_node(), "", true, language);
    }

    fn print_tree_node(
        source: &str,
        node: &tree_sitter::Node,
        prefix: &str,
        is_last: bool,
        language: &Language,
    ) {
        let inline_tree = if language == &Language::markdown && node.kind() == "inline" {
            let mut inline_parser = tree_sitter::Parser::new();
            inline_parser
                .set_language(&ts_languages::get_md_inline_language())
                .expect("failed to set inline language");
            inline_parser
                .set_included_ranges(&vec![node.range()])
                .expect("failed to set included ranges");
            Some(
                inline_parser
                    .parse(source, None)
                    .expect("failed to parse inline source"),
            )
        } else {
            None
        };
        let node = if let Some(inline_tree) = &inline_tree {
            &inline_tree.root_node()
        } else {
            node
        };
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
            print_tree_node(source, child, &new_prefix, last, language);
        }
    }

    #[test]
    fn test_javascript_extract_strings() {
        // normal string
        let normal_src = r#"const tpl = "hello world";"#;
        print_tree(&Language::javascript, normal_src);
        let res = parse_and_extract(Language::javascript, normal_src);
        assert!(
            res.iter().any(|c| c.content == "hello world"),
            "missing 'hello world' fragment"
        );
        // template string with interpolation
        let template_src = r#"const tpl = `hello ${name} world`;"#;
        print_tree(&Language::javascript, template_src);
        let res = parse_and_extract(Language::javascript, template_src);
        assert!(
            res.iter().any(|c| c.content == "hello "),
            "missing 'hello ' fragment"
        );
        assert!(
            res.iter().any(|c| c.content == " world"),
            "missing ' world' fragment"
        );
        // string with escaped characters
        let escape_src = r#"const s = "line1\\line2";"#;
        print_tree(&Language::javascript, escape_src);
        let res = parse_and_extract(Language::javascript, escape_src);
        assert!(
            res.iter().any(|c| c.content == "line1\\\\line2"),
            "missing 'line1\\\\line2' with escaped newline"
        );
    }

    #[test]
    fn test_typescript_extract_string() {
        // normal string
        let normal_src = r#"const tpl: string = "hello world";"#;
        print_tree(&Language::typescript, normal_src);
        let res = parse_and_extract(Language::typescript, normal_src);
        assert!(
            res.iter().any(|c| c.content == "hello world"),
            "missing 'hello world' fragment"
        );
        // template string with interpolation
        let template_src = r#"const tpl: string = `ts ${val} end`;"#;
        print_tree(&Language::typescript, template_src);
        let res = parse_and_extract(Language::typescript, template_src);
        assert!(
            res.iter().any(|c| c.content == "ts "),
            "missing 'ts ' fragment"
        );
        assert!(
            res.iter().any(|c| c.content == " end"),
            "missing ' end' fragment"
        );
        // string with escaped characters
        let escape_src = r#"const s: string = "line1\\line2";"#;
        print_tree(&Language::typescript, escape_src);
        let res = parse_and_extract(Language::typescript, escape_src);
        assert!(
            res.iter().any(|c| c.content == "line1\\\\line2"),
            "missing 'line1\\\\line2' with escaped newline"
        );
    }

    #[test]
    fn test_python_extract_strings() {
        // normal string with single, double, and triple quotes
        let normal_src = r#"
        s = "hello"
        t = 'world'
        u = """multi\nline"""
        "#;
        print_tree(&Language::python, normal_src);
        let res = parse_and_extract(Language::python, normal_src);
        assert!(res.iter().any(|c| c.content == "hello"), "missing 'hello'");
        assert!(res.iter().any(|c| c.content == "world"), "missing 'world'");
        assert!(
            res.iter().any(|c| c.content.trim() == r#"multi\nline"#),
            "missing 'multi\nline' in triple-quoted string"
        );
        // f-string
        let f_string_src = r#"s = f"hello {name}""#;
        print_tree(&Language::python, f_string_src);
        let res = parse_and_extract(Language::python, f_string_src);
        assert!(
            res.iter().any(|c| c.content == "hello "),
            "missing 'hello' in f-string"
        );
        // string with escaped characters
        let escape_src = r#"s = "line1\\line2""#;
        print_tree(&Language::python, escape_src);
        let res = parse_and_extract(Language::python, escape_src);
        assert!(
            res.iter().any(|c| c.content == "line1\\\\line2"),
            "missing 'line1\\\\line2' with escaped newline"
        );
    }

    #[test]
    fn test_rust_extract_strings() {
        let src = "let a = \"hello\"; let b = r#\"raw content\"#";
        print_tree(&Language::rust, src);
        let res = parse_and_extract(Language::rust, src);
        assert!(res.iter().any(|c| c.content == "hello"), "missing 'hello'");
        assert!(
            res.iter().any(|c| c.content == "raw content"),
            "missing raw string content"
        );
        let escaped_src = "let s = \"line1\\\\nline2\";";
        print_tree(&Language::rust, escaped_src);
        let res = parse_and_extract(Language::rust, escaped_src);
        assert!(
            res.iter().any(|c| c.content == "line1\\\\nline2"),
            "missing 'line1\\\\nline2' with escaped newline"
        );
    }

    #[test]
    fn test_markdown_extract_strings() {
        let link = "![a picture](./public/image.png)";
        print_tree(&Language::markdown, link);
        let res = parse_and_extract(Language::markdown, link);
        assert!(
            res.iter().any(|c| c.content == "./public/image.png"),
            "missing link destination"
        );
        let text_in_quotes = "some text and `./public/image1.png`\nmore text and './public/image2.png'\n even more and \"./public/image3.png\"";
        print_tree(&Language::markdown, text_in_quotes);
        let res = parse_and_extract(Language::markdown, text_in_quotes);
        eprintln!("{:?}", res);
        assert!(
            res.iter().any(|c| c.content == "./public/image1.png"),
            "missing path in code span"
        );
        assert!(
            res.iter().any(|c| c.content == "./public/image2.png"),
            "missing path in code span"
        );
        assert!(
            res.iter().any(|c| c.content == "./public/image3.png"),
            "missing path in code span"
        );
        let text_in_starts = "some text and *bold* and **strong**";
        print_tree(&Language::markdown, text_in_starts);
        let res = parse_and_extract(Language::markdown, text_in_starts);
        assert!(res.iter().any(|c| c.content == "bold"), "missing bold text");
        assert!(
            res.iter().any(|c| c.content == "strong"),
            "missing strong text"
        );
        let common_path_in_text = r#"
# h1
## h2
```code
cd ./extensions/vscode/
```
        "#;
        print_tree(&Language::markdown, common_path_in_text);
        let res = parse_and_extract(Language::markdown, common_path_in_text);
        assert!(
            res.iter().any(|c| c.content == "./extensions/vscode/"),
            "missing path in code block"
        );
        let complicated_case = r#"
## Usage
You can use Path Server by installing the extension for your editor, or by building it from source.

After installing, start typing a path prefix like `./`, `/` or `C:\` in any file to trigger path suggestions.


### File Structure
The **Path Server** project is organized in mono-repository structure with core LSP server implementation and extensions for different editors.

- The core LSP server implementation and tests are located in the repository root.
- The **Zed Extension** is located in `./extensions/zed`.
- The **VS Code** is located in `./extensions/vscode`.

> Quote: ./extensions/vscode/more
        "#;
        print_tree(&Language::markdown, complicated_case);
        let res = parse_and_extract(Language::markdown, complicated_case);
        eprintln!("{:?}", res);
        assert!(
            res.iter().any(|c| c.content == "./extensions/zed"),
            "missing path in Zed extension"
        );
        assert!(
            res.iter().any(|c| c.content == "./extensions/vscode"),
            "missing path in VS Code extension"
        );
        assert!(
            res.iter().any(|c| c.content == "./extensions/vscode/more"),
            "missing path in quote"
        );
        let md_with_html = r#"
# Project Timer

Project Timer is a lightweight VS Code extension that tracks the time you spend on your projects. It provides detailed insights into your productivity by analyzing your coding activity by dates, programming languages and specific files.

<div align="center">
    <img src="./resources/demo.gif" alt="demo" style="width: 600px">
</div>
"#;
        print_tree(&Language::markdown, md_with_html);
        let res = parse_and_extract(Language::markdown, md_with_html);
        eprintln!("{:?}", res);
        assert!(
            res.iter().any(|c| c.content == "./resources/demo.gif"),
            "missing path in HTML block"
        );
    }

    #[test]
    fn test_html_extract_string() {
        let simple = r#"    <script src="echarts.min.js"></script>"#;
        print_tree(&Language::html, simple);
        let res = parse_and_extract(Language::html, simple);
        eprintln!("{:?}", res);
        assert!(res.iter().any(|c| c.content == "echarts.min.js"));
        let html = r#"
<!DOCTYPE html>
<html lang="en">

<head>
    <script src="echarts.min.js"></script>
    <link rel="stylesheet" href="statistics.css">
    <meta name="viewport" content="width=device-width,initial-scale=1">
</head>

<body>
<h1>Title</h1>
<div>Some content include a path ./extension.toml</div>
</body>
        "#;
        print_tree(&Language::html, html);
        let res = parse_and_extract(Language::html, html);
        eprintln!("{:?}", res);
        assert!(res.iter().any(|c| c.content == "echarts.min.js"));
        assert!(res.iter().any(|c| c.content == "statistics.css"));
        assert!(res.iter().any(|c| c.content == "./extension.toml"));
    }
}
