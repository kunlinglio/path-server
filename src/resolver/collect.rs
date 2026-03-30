use std::path::PathBuf;
use std::sync::Arc;

use futures::future;

use crate::config::Config;
use crate::document::Document;
use crate::error::*;
use crate::fs;
use crate::parser::{PathCandidate, parse_document};

use super::{ResolvedPath, ResolvedPathCache};

pub async fn resolve_all(
    document: &Document,
    config: &Config,
    workspace_roots: &[String],
    doc_parent: &Option<String>,
) -> PathServerResult<Arc<Vec<ResolvedPath>>> {
    let mut cache = document.tokens.lock().await;
    let signature = config.signature()?;
    if let Some(tokens) = &cache.tokens
        && cache.config_signature == signature
    {
        // hit
        return Ok(tokens.clone());
    }
    // miss
    let tokens = compute_tokens(document, config, workspace_roots, doc_parent).await?;
    let shared_tokens = Arc::new(tokens);
    *cache = ResolvedPathCache {
        tokens: Some(Arc::clone(&shared_tokens)),
        config_signature: signature,
    };
    Ok(shared_tokens)
}

async fn compute_tokens(
    document: &Document,
    config: &Config,
    workspace_roots: &[String],
    doc_parent: &Option<String>,
) -> PathServerResult<Vec<ResolvedPath>> {
    let home = std::env::var("HOME").ok();
    let tokens: Vec<ResolvedPath> =
        future::try_join_all(parse_document(document).into_iter().flatten().map(
            |candidates| async {
                filter_exist_path(
                    candidates,
                    config,
                    workspace_roots,
                    doc_parent.as_ref(),
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
) -> PathServerResult<Vec<ResolvedPath>> {
    let resolved = future::try_join_all(candidates.into_iter().map(|candidate| async move {
        let path = PathBuf::from(&candidate.content);
        if path.is_absolute() {
            if fs::exists(&path).await {
                PathServerResult::Ok(vec![
                    candidate_to_resolved(&candidate, &path, document).await?,
                ])
            } else {
                PathServerResult::Ok(vec![])
            }
        } else if path.is_relative() {
            PathServerResult::Ok(
                future::try_join_all(
                    config
                        .base_paths(workspace_roots, parent, home)
                        .into_iter()
                        .map(|(base_path, _, _)| {
                            let path = &path;
                            let candidate = &candidate;
                            async move {
                                let full_path = base_path.join(&path);
                                if fs::exists(&full_path).await {
                                    PathServerResult::Ok(Some(
                                        candidate_to_resolved(&candidate, &full_path, document)
                                            .await?,
                                    ))
                                } else {
                                    PathServerResult::Ok(None)
                                }
                            }
                        }),
                )
                .await?
                .into_iter()
                .flatten()
                .collect(),
            )
        } else {
            unreachable!();
        }
    }))
    .await?
    .into_iter()
    .flatten()
    .collect();
    PathServerResult::Ok(filter_overlapping(resolved))
}

fn filter_overlapping(tokens: Vec<ResolvedPath>) -> Vec<ResolvedPath> {
    let mut results: Vec<ResolvedPath> = vec![];
    'token_loop: for token in tokens {
        for result in &results {
            if result.intersects(&token) {
                continue 'token_loop;
            }
        }
        results.push(token);
    }
    results
}

async fn candidate_to_resolved(
    candidate: &PathCandidate,
    path: &PathBuf,
    document: &Document,
) -> PathServerResult<ResolvedPath> {
    let start = document.offset_to_utf16_pos(candidate.start_byte)?;
    let end = document.offset_to_utf16_pos(candidate.end_byte)?;
    Ok(ResolvedPath {
        start,
        end,
        target: tokio::fs::canonicalize(&path).await?,
        is_dir: fs::is_dir(path).await,
    })
}
