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
pub async fn provide_document_links(
    doc: &Document,
    doc_path: &Path,
) -> PathServerResult<Vec<lsp_types::DocumentLink>> {
    let tokens: Vec<(PathCandidate, PathBuf)> = future::join_all(
        parse_document(doc)
            .into_iter()
            .map(|candidates| async move {
                for candidate in candidates {
                    let path = PathBuf::from(&candidate.content);
                    if path.is_absolute() {
                        if fs::exists(&path).await {
                            return Some((candidate, path));
                        }
                    } else if path.is_relative() {
                        let Some(base_path) = doc_path.parent() else {
                            warn(format!("Failed to get parent directory of {}, give up provide document links.", doc_path.display())).await;
                            continue;
                        };
                        let full_path = base_path.join(&path);
                        if fs::exists(&full_path).await {
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
    .flatten()
    .collect();

    let mut links = vec![];
    for token in tokens {
        let candidate = token.0;
        let path = token.1;
        let start = doc.offset_to_utf16_pos(candidate.start_byte)?;
        let end = doc.offset_to_utf16_pos(candidate.end_byte)?;
        let range = lsp_types::Range::new(
            lsp_types::Position::new(start.0 as u32, start.1 as u32),
            lsp_types::Position::new(end.0 as u32, end.1 as u32),
        );

        links.push(lsp_types::DocumentLink {
            range,
            target: Some(lsp_types::Url::from_file_path(path.clone()).map_err(|_| {
                PathServerError::Unknown(format!(
                    "Failed to convert path {} into url",
                    path.display()
                ))
            })?),
            tooltip: Some("Follow path".into()),
            data: None,
        });
    }

    Ok(links)
}
