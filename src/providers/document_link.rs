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
    let tokens: Vec<(PathCandidate, PathBuf)> = future::try_join_all(
        parse_document(doc)
            .into_iter()
            .map(|candidates| async move {
                for candidate in candidates {
                    let path = PathBuf::from(&candidate.content);
                    if path.is_absolute() {
                        if fs::exists(&path).await {
                            return PathServerResult::Ok(Some((candidate, path)));
                        }
                    } else if path.is_relative() {
                        let Some(base_path) = doc_path.parent() else {
                            warn(format!("Failed to get parent directory of {}, give up provide document links.", doc_path.display())).await;
                            continue;
                        };
                        let full_path = base_path.join(&path);
                        if fs::exists(&full_path).await {
                            return Ok(Some((candidate, tokio::fs::canonicalize(&full_path).await?)));
                        }
                    } else {
                        unreachable!();
                    }
                }
                Ok(None)
            }),
    )
    .await?
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
            tooltip: Some("Open file".into()),
            data: None,
        });
    }

    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Language;
    use std::fs;
    use tempfile::tempdir;
    use tokio;

    #[tokio::test]
    async fn test_provide_document_links_absolute() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("target.txt");
        fs::File::create(&target).unwrap();

        let current_file = tmp.path().join("src").join("main.rs");
        fs::create_dir_all(current_file.parent().unwrap()).unwrap();
        fs::File::create(&current_file).unwrap();

        let text = format!("let s = \"{}\";\n", target.display());
        let doc = Document::new(text.clone(), &Language::rust.to_string()).unwrap();

        let links = provide_document_links(&doc, &current_file).await.unwrap();
        assert_eq!(links.len(), 1);
        let url = links[0].target.as_ref().unwrap();
        assert_eq!(url.to_file_path().unwrap(), target);
    }

    #[tokio::test]
    async fn test_provide_document_links_relative() {
        let tmp = tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let target = data_dir.join("rel_target.txt");
        fs::File::create(&target).unwrap();

        let current_file = tmp.path().join("src").join("main.rs");
        fs::create_dir_all(current_file.parent().unwrap()).unwrap();
        fs::File::create(&current_file).unwrap();

        let rel_path = "../data/rel_target.txt";
        let text = format!("let s = \"{}\";\n", rel_path);
        let doc = Document::new(text.clone(), &Language::rust.to_string()).unwrap();

        let links = provide_document_links(&doc, &current_file).await.unwrap();
        assert_eq!(links.len(), 1);
        let url = links[0].target.as_ref().unwrap();
        let expected = tokio::fs::canonicalize(&target).await.unwrap();
        assert_eq!(url.to_file_path().unwrap(), expected);
    }
}
