use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tower_lsp::lsp_types;

use crate::Config;
use crate::document::Document;
use crate::error::*;
use crate::logger::*;
use crate::resolver::resolve_at_pos;

pub async fn provide_hover(
    doc: &Document,
    doc_path: &Path,
    line: usize,
    character: usize,
    config: &Config,
    workspace_roots: &HashSet<PathBuf>,
) -> PathServerResult<Option<lsp_types::Hover>> {
    let Some(current_token) =
        resolve_at_pos(doc, config, workspace_roots, doc_path, (line, character)).await?
    else {
        return Ok(None);
    };
    if !config.highlight.highlight_directory && current_token.is_dir {
        return Ok(None);
    }
    let origin_start =
        lsp_types::Position::new(current_token.start.0 as u32, current_token.start.1 as u32);
    let origin_end =
        lsp_types::Position::new(current_token.end.0 as u32, current_token.end.1 as u32);
    let origin_range = lsp_types::Range::new(origin_start, origin_end);

    let Ok(url) = lsp_types::Url::from_file_path(&current_token.target) else {
        warn(format!(
            "Failed to convert path to URL: {}",
            current_token.target.display()
        ))
        .await;
        return Ok(None);
    };

    Ok(Some(lsp_types::Hover {
        contents: lsp_types::HoverContents::Scalar(lsp_types::MarkedString::String(
            url.to_string(),
        )),
        range: Some(origin_range),
    }))
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
    async fn test_provide_hover_absolute() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("target.txt");
        fs::File::create(&target).unwrap();

        let current_file = tmp.path().join("src").join("main.rs");
        fs::create_dir_all(current_file.parent().unwrap()).unwrap();
        fs::File::create(&current_file).unwrap();

        let text = format!("let s = \"{}\";\n", target.display());
        let doc = Document::new(text.clone(), &Language::rust.to_string()).unwrap();

        let start_offset = text.find(&target.display().to_string()).unwrap();
        let (line, character) = doc.offset_to_utf16_pos(start_offset).unwrap();

        let res = provide_hover(
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
        let hover = res.unwrap();
        match hover.contents {
            lsp_types::HoverContents::Scalar(lsp_types::MarkedString::String(s)) => {
                let url = lsp_types::Url::parse(&s).unwrap();
                assert_eq!(
                    tokio::fs::canonicalize(&url.to_file_path().unwrap())
                        .await
                        .unwrap(),
                    tokio::fs::canonicalize(&target).await.unwrap()
                );
            }
            _ => panic!("Expected string hover content"),
        }
    }

    #[tokio::test]
    async fn test_provide_hover_relative() {
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

        let start_offset = text.find(rel_path).unwrap();
        let (line, character) = doc.offset_to_utf16_pos(start_offset).unwrap();

        let res = provide_hover(
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
        let hover = res.unwrap();
        match hover.contents {
            lsp_types::HoverContents::Scalar(lsp_types::MarkedString::String(s)) => {
                let url = lsp_types::Url::parse(&s).unwrap();
                let expected = tokio::fs::canonicalize(&target).await.unwrap();
                assert_eq!(
                    tokio::fs::canonicalize(&url.to_file_path().unwrap())
                        .await
                        .unwrap(),
                    expected
                );
            }
            _ => panic!("Expected string hover content"),
        }
    }
}
