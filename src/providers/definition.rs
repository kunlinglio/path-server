use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tower_lsp::lsp_types;

use crate::Config;
use crate::document::Document;
use crate::error::*;
use crate::logger::*;
use crate::resolver::PathToken;
use crate::resolver::get_or_resolve_tokens;

pub async fn provide_definition(
    doc: &Document,
    doc_path: &Path,
    line: usize,
    character: usize,
    config: &Config,
    workspace_roots: &HashSet<PathBuf>,
) -> PathServerResult<Option<lsp_types::GotoDefinitionResponse>> {
    let tokens = get_or_resolve_tokens(doc, config, workspace_roots, doc_path).await?;
    let filtered = tokens
        .iter()
        .filter(|t| config.highlight.highlight_directory || !t.is_dir);

    let current_token: Vec<&PathToken> = filtered
        .filter(|token| cursor_inside(line, character, token))
        .collect();

    if current_token.is_empty() {
        return Ok(None);
    }

    if current_token.len() != 1 {
        unreachable!("Expected exactly one token, found {}", current_token.len());
    }

    let current_token = current_token[0];
    let Ok(url) = lsp_types::Url::from_file_path(&current_token.target) else {
        warn(format!(
            "Failed to convert path to URL: {}",
            current_token.target.display()
        ))
        .await;
        return Ok(None);
    };
    let origin_start =
        lsp_types::Position::new(current_token.start.0 as u32, current_token.start.1 as u32);
    let origin_end =
        lsp_types::Position::new(current_token.end.0 as u32, current_token.end.1 as u32);
    let origin_range = lsp_types::Range::new(origin_start, origin_end);

    let target_range = lsp_types::Range::new(
        lsp_types::Position::new(0, 0),
        lsp_types::Position::new(0, 0),
    );

    Ok(Some(lsp_types::GotoDefinitionResponse::Link(vec![
        lsp_types::LocationLink {
            origin_selection_range: Some(origin_range),
            target_uri: url,
            target_range,
            target_selection_range: target_range,
        },
    ])))
}

fn cursor_inside(cursor_line: usize, cursor_character: usize, token: &PathToken) -> bool {
    let (start_line, start_character) = token.start;
    let (end_line, end_character) = token.end;
    if cursor_line < start_line || cursor_line > end_line {
        // quick path: cursor do not in the token lines
        return false;
    };
    if start_line == end_line {
        // single line token, most frequent case
        return start_line == cursor_line
            && cursor_character >= start_character
            && cursor_character < end_character;
    };
    // multi-line token
    if cursor_line == start_line {
        return cursor_character >= start_character;
    }
    if cursor_line == end_line {
        return cursor_character < end_character;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Language;
    use std::collections::HashSet;
    use std::fs;
    use tempfile::tempdir;
    use tokio;

    #[tokio::test]
    async fn test_provide_definition_absolute() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("target.txt");
        fs::File::create(&target).unwrap();

        let current_file = tmp.path().join("src").join("main.rs");
        fs::create_dir_all(current_file.parent().unwrap()).unwrap();
        fs::File::create(&current_file).unwrap();

        let text = format!("let s = \"{}\";\n", target.display());
        let doc = Document::new(text.clone(), &Language::rust.to_string()).unwrap();

        // find start offset of the path and convert to utf16 pos
        let start_offset = text.find(&target.display().to_string()).unwrap();
        let (line, character) = doc.offset_to_utf16_pos(start_offset).unwrap();

        let res = provide_definition(
            &doc,
            &current_file,
            line,
            character + 1,
            &Config::default(),
            &HashSet::new(),
        )
        .await
        .unwrap();
        assert!(res.is_some());
        match res.unwrap() {
            lsp_types::GotoDefinitionResponse::Link(loc) => {
                assert_eq!(
                    tokio::fs::canonicalize(&loc[0].target_uri.to_file_path().unwrap())
                        .await
                        .unwrap(),
                    tokio::fs::canonicalize(&target).await.unwrap()
                );
            }
            _ => panic!("Expected link location"),
        }
    }

    #[tokio::test]
    async fn test_provide_definition_relative() {
        let tmp = tempdir().unwrap();
        // create data in workspace root
        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let target = data_dir.join("rel_target.txt");
        fs::File::create(&target).unwrap();

        // put current file in a subfolder so relative ../data/... works
        let current_file = tmp.path().join("src").join("main.rs");
        fs::create_dir_all(current_file.parent().unwrap()).unwrap();
        fs::File::create(&current_file).unwrap();

        // relative path from src/ to data/ is ../data/rel_target.txt
        let rel_path = "../data/rel_target.txt";
        let text = format!("let s = \"{}\";\n", rel_path);
        let doc = Document::new(text.clone(), &Language::rust.to_string()).unwrap();

        let start_offset = text.find(rel_path).unwrap();
        let (line, character) = doc.offset_to_utf16_pos(start_offset).unwrap();

        let res = provide_definition(
            &doc,
            &current_file,
            line,
            character + 1,
            &Config::default(),
            &HashSet::new(),
        )
        .await
        .unwrap();
        assert!(res.is_some());
        match res.unwrap() {
            lsp_types::GotoDefinitionResponse::Link(loc) => {
                // normalize expected path to match canonicalized result
                let expected = tokio::fs::canonicalize(&target).await.unwrap();
                assert_eq!(
                    tokio::fs::canonicalize(&loc[0].target_uri.to_file_path().unwrap())
                        .await
                        .unwrap(),
                    expected
                );
            }
            _ => panic!("Expected link location"),
        }
    }
}
