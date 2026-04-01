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
        .filter(|part| {
            !(part.target.to_str().map(|s| s == "/").unwrap_or(false)
                || part.target.to_str().map(|s| s == "\\").unwrap_or(false))
        }) // drop single slash
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
                                let full_path = base_path.join(path);
                                if fs::exists(&full_path).await {
                                    PathServerResult::Ok(Some(
                                        candidate_to_resolved(candidate, &full_path, document)
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

/// Filter out tokens that point to overlapping positions, keep the one with higher priority (which is generated earlier)
/// Because the order of candidates is from high priority to low priority, we have to use the O(n^2) algorithm
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_resolved_path(
        start_line: usize,
        start_character: usize,
        end_line: usize,
        end_character: usize,
    ) -> ResolvedPath {
        ResolvedPath {
            start: (start_line, start_character),
            end: (end_line, end_character),
            target: PathBuf::from("dummy-target"),
            is_dir: false,
        }
    }
    #[test]
    fn filter_overlapping_drops_later_overlapping_token() {
        // token1: [0:0, 0:5), token2: [0:3, 0:8) => overlapping
        let token1 = make_resolved_path(0, 0, 0, 5);
        let token2 = make_resolved_path(0, 3, 0, 8);
        let filtered = filter_overlapping(vec![token1.clone(), token2]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start, token1.start);
        assert_eq!(filtered[0].end, token1.end);
    }
    #[test]
    fn filter_overlapping_keeps_non_overlapping_tokens() {
        // token1: [0:0, 0:5), token2: [0:6, 0:10) => non-overlapping
        let token1 = make_resolved_path(0, 0, 0, 5);
        let token2 = make_resolved_path(0, 6, 0, 10);
        let filtered = filter_overlapping(vec![token1.clone(), token2.clone()]);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].start, token1.start);
        assert_eq!(filtered[0].end, token1.end);
        assert_eq!(filtered[1].start, token2.start);
        assert_eq!(filtered[1].end, token2.end);
    }
}
