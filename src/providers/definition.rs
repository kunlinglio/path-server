use std::path::{Path, PathBuf};

use futures::future;
use tower_lsp::lsp_types;

use crate::document::Document;
use crate::error::*;
use crate::fs;
use crate::logger::*;
use crate::parser::{PathCandidate, parse_document};

/// Based on document url for now.
/// TODO: support configurable base url
pub async fn provide_definition(
    doc: &Document,
    line: usize,
    character: usize,
    doc_path: &Path,
) -> PathServerResult<Option<lsp_types::GotoDefinitionResponse>> {
    let cursor_offset = doc.utf16_pos_to_offset(line, character)?;
    // gather all string tokens, a very slow implement
    // TODO: optimize performance
    let tokens = future::try_join_all(parse_document(doc).into_iter().map(
        |candidates| async move {
            for candidate in candidates {
                let path = PathBuf::from(&candidate.content);
                if path.is_absolute() {
                    if fs::exists(&path).await {
                        return PathServerResult::Ok(Some((
                            candidate,
                            tokio::fs::canonicalize(path).await?,
                        )));
                    }
                } else if path.is_relative() {
                    let Some(base_path) = doc_path.parent() else {
                        warn(format!(
                            "Failed to get parent directory of {}, give up provide document links.",
                            doc_path.display()
                        ))
                        .await;
                        continue;
                    };
                    let full_path = base_path.join(&path);
                    if fs::exists(&full_path).await {
                        return Ok(Some((candidate, tokio::fs::canonicalize(full_path).await?)));
                    }
                } else {
                    unreachable!();
                }
            }
            Ok(None)
        },
    ))
    .await?
    .into_iter()
    .flatten();

    let current_token: Vec<(PathCandidate, PathBuf)> = tokens
        .filter(|token| cursor_offset >= token.0.start_byte && cursor_offset < token.0.end_byte)
        .collect();

    if current_token.is_empty() {
        return Ok(None);
    }

    if current_token.len() != 1 {
        unreachable!("Expected exactly one token, found {}", current_token.len());
    }

    let current_token = current_token[0].clone();
    let Ok(url) = lsp_types::Url::from_file_path(&current_token.1) else {
        warn(format!(
            "Failed to convert path to URL: {}",
            current_token.1.display()
        ))
        .await;
        return Ok(None);
    };
    let (line, character) = doc.offset_to_utf16_pos(current_token.0.start_byte)?;
    let start = lsp_types::Position::new(line as u32, character as u32);
    let (line, character) = doc.offset_to_utf16_pos(current_token.0.end_byte)?;
    let end = lsp_types::Position::new(line as u32, character as u32);

    Ok(Some(lsp_types::GotoDefinitionResponse::Scalar(
        lsp_types::Location::new(url, lsp_types::Range::new(start, end)),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Language;
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

        let res = provide_definition(&doc, line, character + 1, &current_file)
            .await
            .unwrap();
        assert!(res.is_some());
        match res.unwrap() {
            lsp_types::GotoDefinitionResponse::Scalar(loc) => {
                assert_eq!(
                    tokio::fs::canonicalize(&loc.uri.to_file_path().unwrap())
                        .await
                        .unwrap(),
                    tokio::fs::canonicalize(&target).await.unwrap()
                );
            }
            _ => panic!("Expected scalar location"),
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

        let res = provide_definition(&doc, line, character + 1, &current_file)
            .await
            .unwrap();
        assert!(res.is_some());
        match res.unwrap() {
            lsp_types::GotoDefinitionResponse::Scalar(loc) => {
                // normalize expected path to match canonicalized result
                let expected = tokio::fs::canonicalize(&target).await.unwrap();
                assert_eq!(
                    tokio::fs::canonicalize(&loc.uri.to_file_path().unwrap())
                        .await
                        .unwrap(),
                    expected
                );
            }
            _ => panic!("Expected scalar location"),
        }
    }
}
