use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tower_lsp::lsp_types;

use crate::client::get_client;
use crate::config::Config;
use crate::document::Document;
use crate::error::*;
use crate::fs;
use crate::resolver;

pub async fn provide_document_links(
    doc: &Document,
    doc_path: &Path,
    config: &Config,
    workspace_roots: &HashSet<PathBuf>,
) -> PathServerResult<Vec<lsp_types::DocumentLink>> {
    let client = get_client().await;
    assert!(client.support_document_link);
    assert!(config.highlight.enable); // these should be checked by server
    let tokens = resolver::resolve_all(doc, config, workspace_roots, doc_path).await?;
    let filtered = tokens
        .iter()
        .filter(|t| config.highlight.highlight_directory || !t.is_dir);

    let links = filtered
        .map(|token| {
            let range = lsp_types::Range::new(
                lsp_types::Position::new(token.start.0 as u32, token.start.1 as u32),
                lsp_types::Position::new(token.end.0 as u32, token.end.1 as u32),
            );

            let link = lsp_types::DocumentLink {
                range,
                target: Some(fs::path_to_url(&token.target)?),
                tooltip: Some(format!("Open file: {}", token.target.display())),
                data: None,
            };
            PathServerResult::Ok(link)
        })
        .collect::<PathServerResult<Vec<lsp_types::DocumentLink>>>()?;

    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::document::Language;
    use std::collections::HashSet;
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

        let links =
            provide_document_links(&doc, &current_file, &Config::default(), &HashSet::new())
                .await
                .unwrap();
        assert_eq!(links.len(), 1);
        let url = links[0].target.as_ref().unwrap();
        assert_eq!(
            tokio::fs::canonicalize(url.to_file_path().unwrap())
                .await
                .unwrap(),
            tokio::fs::canonicalize(&target).await.unwrap()
        );
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

        let links =
            provide_document_links(&doc, &current_file, &Config::default(), &HashSet::new())
                .await
                .unwrap();
        assert_eq!(links.len(), 1);
        let url = links[0].target.as_ref().unwrap();
        let expected = tokio::fs::canonicalize(&target).await.unwrap();
        assert_eq!(
            tokio::fs::canonicalize(&url.to_file_path().unwrap())
                .await
                .unwrap(),
            expected
        );
    }
}
