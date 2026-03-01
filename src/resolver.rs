use std::collections::HashSet;
use std::path::PathBuf;

use tokio::sync::RwLock;
use tower_lsp::lsp_types::{self, Url};

use crate::fs;
use crate::logger::*;

#[derive(Debug)]
pub struct PathResolver {
    workspace_root: RwLock<HashSet<lsp_types::Url>>,
}

impl PathResolver {
    pub fn new() -> Self {
        PathResolver {
            workspace_root: RwLock::new(HashSet::new()),
        }
    }

    pub async fn add_workspace_root(&self, url: &Url) {
        let mut roots = self.workspace_root.write().await;
        roots.insert(url.clone());
    }

    pub async fn remove_workspace_root(&self, url: &Url) {
        let mut roots = self.workspace_root.write().await;
        roots.remove(url);
    }

    pub async fn complete(&self, input: &str) -> Vec<PathBuf> {
        let roots = self.workspace_root.read().await;
        let mut completions = Vec::new();

        let path_input = input.split_whitespace().last().unwrap_or(input);

        info(format!("Completing path for input: '{}'", path_input)).await;
        for root in roots.iter() {
            let root_path = match root.to_file_path() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let full_input_path = root_path.join(path_input);

            let (search_dir, filter) = if path_input.ends_with('/') || path_input.ends_with('\\') {
                (full_input_path.clone(), "")
            } else {
                (
                    full_input_path.parent().unwrap_or(&root_path).to_path_buf(),
                    full_input_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(""),
                )
            };

            if search_dir.is_dir() {
                if let Ok(entries) = fs::ls(&search_dir).await {
                    for entry in entries {
                        let file_name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if file_name.starts_with(filter) {
                            completions.push(PathBuf::from(file_name));
                        }
                    }
                }
            }
        }
        completions
    }
}
