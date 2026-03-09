//! Regex based parser for fallback
use regex::Regex;

use super::PathCandidate;
use crate::{document::Document, error::PathServerError};

pub fn extract_string(document: &Document) -> Option<Vec<PathCandidate>> {
    let string_regexes = [
        r#""(?:[^"\\]|\\.)*""#, // match string in double quote, support escaped \"
        r#"'(?:[^'\\]|\\.)*'"#, // match string in single quote, support escaped \'
        r#"`(?:[^`\\]|\\.)*`"#, // match string in back tick, and support escaped \`
    ];
    let regex = Regex::new(&string_regexes.join("|"))
        .map_err(|e| PathServerError::Unknown(format!("Failed to compile regex expression: {}", e)))
        .unwrap();
    let mut strings = vec![];
    for matched in regex.find_iter(&document.text) {
        let content = matched.as_str();
        // slice quotes
        if content.len() >= 2 {
            strings.push(PathCandidate {
                content: content[1..content.len() - 1].to_string(),
                start_byte: matched.start() + 1,
                end_byte: matched.end() - 1,
            })
        }
    }
    Some(strings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn test_extract_strings_regex_multiple() {
        let src = r#"const a = "hello"; const b = 'world'; const c = `tmp`;"#;
        let doc = Document::new(src.to_string(), "javascript").expect("failed to create Document");
        let res = extract_string(&doc).unwrap_or_default();
        assert!(res.iter().any(|p| p.content.contains("hello")));
        assert!(res.iter().any(|p| p.content.contains("world")));
        assert!(res.iter().any(|p| p.content.contains("tmp")));
        assert_eq!(res.len(), 3);
    }

    #[test]
    fn test_extract_no_strings() {
        let src = "let x = 42;";
        let doc = Document::new(src.to_string(), "javascript").expect("failed to create Document");
        let res = extract_string(&doc).unwrap_or_default();
        assert!(res.is_empty());
    }

    #[test]
    fn test_extract_strings_with_escapes_and_nesting() {
        // test nesting quotes
        let src = r#"const a = 'It"s a /path/to/file'; const b = "He said 'hello'";"#;
        let doc = Document::new(src.to_string(), "javascript").expect("failed to create Document");
        let res = extract_string(&doc).unwrap_or_default();
        assert!(
            res.iter()
                .any(|p| p.content.contains("It\"s a /path/to/file"))
        );
        assert!(res.iter().any(|p| p.content.contains("He said 'hello'")));

        // test escaped quotes
        let src_escape = r#"const path = "C:\\projects\\\"my project\"\\src";"#;
        let doc_escape =
            Document::new(src_escape.to_string(), "javascript").expect("failed to create Document");
        let res_escape = extract_string(&doc_escape).unwrap_or_default();
        assert!(
            res_escape
                .iter()
                .any(|p| p.content.contains(r#"C:\\projects\\\"my project\"\\src"#))
        );
    }
}
