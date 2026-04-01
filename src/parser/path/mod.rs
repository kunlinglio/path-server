//! Parsers for document path parsing.
mod regex;

use std::vec::Vec;

use super::PathCandidate;
use super::tree_sitter;
use crate::document::Document;
use crate::error::*;

pub fn parse_document(document: &Document) -> PathServerResult<Vec<Vec<PathCandidate>>> {
    Ok(extract_string(document)?
        .into_iter()
        .map(extract_paths_from_string)
        .collect())
}

/// Extract string tokens from the document
fn extract_string(document: &Document) -> PathServerResult<Vec<PathCandidate>> {
    let res = tree_sitter::extract_strings(document)?;
    let res = if let Some(res) = res {
        res
    } else {
        // fall back to general parser
        regex::extract_string(document).unwrap_or_default()
    };
    Ok(res)
}

/// Try to extract paths from a string token,
/// return candidates, from high priority to low priority
fn extract_paths_from_string(path_ref: PathCandidate) -> Vec<PathCandidate> {
    let mut results = Vec::new();
    let content = &path_ref.content;

    // Level 1: whole string is a path or not
    if content.contains('/') || content.contains('\\') {
        results.push(path_ref.clone().trim());
    }

    // Level 2: the part of string (split by space) is a path or not
    results.extend(path_ref.split(&[' ', '\n']));

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn parse_document_detects_whole_and_tail_paths() {
        let src = r#"const a = "/home/user/project/src/main.rs"; const b = "see /tmp/dir";"#;
        let doc = Document::new(src.to_string(), "javascript").unwrap();
        let res = crate::parser::parse_document(&doc).unwrap();
        let flat: Vec<String> = res.into_iter().flatten().map(|p| p.content).collect();
        assert!(
            flat.iter()
                .any(|c| c.contains("/home/user/project/src/main.rs"))
        );
        assert!(flat.iter().any(|c| c.contains("/tmp/dir")));
    }

    #[test]
    fn extractor_fallback_for_unsupported_language() {
        let src = r#"const a = "/tmp/test/path";"#.to_string();
        let doc = Document::new(src.clone(), "unknown").unwrap();
        let res = extract_string(&doc).unwrap();
        assert!(res.iter().any(|p| p.content.contains("/tmp/test/path")));
    }

    #[test]
    fn test_extract_paths_from_string_multiple_segments() {
        let candidate = PathCandidate {
            content: "Check logs at /var/log/syslog and config at /etc/nginx.conf".to_string(),
            start_byte: 0,
            end_byte: 59,
        };

        let res = extract_paths_from_string(candidate);

        for p in &res {
            eprintln!("Extracted: {};", p.content);
        }
        assert!(res.iter().any(|p| p.content == "/etc/nginx.conf"));
        assert!(res.iter().any(|p| p.content == "/var/log/syslog"));
    }

    #[test]
    fn test_extract_paths_with_trailing_spaces() {
        let candidate = PathCandidate {
            content: "path is /tmp/dir/ ".to_string(), // tailing space
            start_byte: 0,
            end_byte: 18,
        };
        let res = extract_paths_from_string(candidate);
        for p in &res {
            eprintln!("Extracted: {};", p.content);
        }
        assert!(res.iter().any(|p| p.content.trim() == "/tmp/dir/"));
    }

    #[test]
    fn test_extract_paths_with_utf8() {
        let candidate = PathCandidate {
            content: "路径在 /tmp/目录/ ".to_string(), // UTF-8 characters
            start_byte: 0,
            end_byte: 24,
        };
        let res = extract_paths_from_string(candidate);
        for p in &res {
            eprintln!("Extracted: {};", p.content);
        }
        assert!(res.iter().any(|p| p.content.trim() == "/tmp/目录/"));
    }
}
