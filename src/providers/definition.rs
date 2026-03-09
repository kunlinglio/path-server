use std::path::{Path, PathBuf};

use futures::future;
use tower_lsp::lsp_types;

use crate::async_fs;
use crate::common::*;
use crate::document::Document;
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
    let tokens = future::join_all(
        parse_document(doc)
            .into_iter()
            .map(|candidates| async move {
                for candidate in candidates {
                    let path = PathBuf::from(&candidate.content);
                    if path.is_absolute() {
                        if async_fs::exists(&path).await {
                            return Some((candidate, path));
                        }
                    } else if path.is_relative() {
                        let Some(base_path) = doc_path.parent() else {
                            warn(format!("Failed to get parent directory of {}, give up provide document links.", doc_path.display())).await;
                            continue;
                        };
                        let full_path = base_path.join(&path);
                        if async_fs::exists(&full_path).await {
                            return Some((candidate, full_path));
                        }
                    } else {
                        unreachable!();
                    }
                }
                None
            }),
    )
    .await
    .into_iter()
    .flatten();

    let current_token: Vec<(PathCandidate, PathBuf)> = tokens
        .filter(|token| cursor_offset > token.0.start_byte && cursor_offset < token.0.end_byte)
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
