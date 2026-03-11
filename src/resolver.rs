use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::future;

use crate::config::Config;
use crate::document::Document;
use crate::error::*;
use crate::fs;
use crate::parser::{PathCandidate, parse_document};

#[derive(Debug, Clone)]
pub struct PathToken {
    pub start: (usize, usize), // (line, character) in utf16
    pub end: (usize, usize),   // (line, character) in utf16
    pub target: PathBuf,
    pub is_dir: bool,
}

#[derive(Debug)]
pub struct PathTokenCache {
    tokens: Option<Arc<Vec<PathToken>>>,
    config_signature: String,
}

impl PathTokenCache {
    pub fn new() -> Self {
        Self {
            tokens: None,
            config_signature: String::new(),
        }
    }
}

pub async fn get_or_resolve_tokens(
    document: &Document,
    config: &Config,
    workspace_roots: &HashSet<PathBuf>,
    doc_path: &Path,
) -> PathServerResult<Arc<Vec<PathToken>>> {
    let mut cache = document.tokens.lock().await;
    let signature = config.signature()?;
    if let Some(tokens) = &cache.tokens
        && cache.config_signature == signature
    {
        // hit
        return Ok(tokens.clone());
    }
    // miss
    let tokens = compute_tokens(document, config, workspace_roots, doc_path).await?;
    let shared_tokens = Arc::new(tokens);
    *cache = PathTokenCache {
        tokens: Some(Arc::clone(&shared_tokens)),
        config_signature: signature,
    };
    Ok(shared_tokens)
}

async fn compute_tokens(
    document: &Document,
    config: &Config,
    workspace_roots: &HashSet<PathBuf>,
    doc_path: &Path,
) -> PathServerResult<Vec<PathToken>> {
    let workspace_roots = workspace_roots
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let parent = doc_path.parent().map(|p| p.to_string_lossy().into_owned());
    let home = std::env::var("HOME").ok();
    let tokens: Vec<PathToken> = future::try_join_all(parse_document(document).into_iter().map(
        |candidates| async {
            filter_exist_path(
                candidates,
                config,
                &workspace_roots,
                parent.as_ref(),
                home.as_ref(),
                document,
            )
            .await
        },
    ))
    .await?
    .into_iter()
    .flatten()
    .collect();
    Ok(tokens)
}

async fn filter_exist_path(
    candidates: Vec<PathCandidate>,
    config: &Config,
    workspace_roots: &[String],
    parent: Option<&String>,
    home: Option<&String>,
    document: &Document,
) -> PathServerResult<Option<PathToken>> {
    for candidate in candidates {
        let path = PathBuf::from(&candidate.content);
        if path.is_absolute() {
            if fs::exists(&path).await {
                return PathServerResult::Ok(Some(
                    candidate_to_token(&candidate, &path, document).await?,
                ));
            }
        } else if path.is_relative() {
            for base_path in config.base_paths(workspace_roots, parent, home) {
                let full_path = base_path.join(&path);
                if fs::exists(&full_path).await {
                    return PathServerResult::Ok(Some(
                        candidate_to_token(&candidate, &full_path, document).await?,
                    ));
                }
            }
        } else {
            unreachable!();
        }
    }
    Ok(None)
}

async fn candidate_to_token(
    candidate: &PathCandidate,
    path: &PathBuf,
    document: &Document,
) -> PathServerResult<PathToken> {
    let start = document.offset_to_utf16_pos(candidate.start_byte)?;
    let end = document.offset_to_utf16_pos(candidate.end_byte)?;
    Ok(PathToken {
        start,
        end,
        target: tokio::fs::canonicalize(&path).await?,
        is_dir: fs::is_dir(path).await,
    })
}
