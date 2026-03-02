use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types;

use std::collections::HashSet;
use std::path::PathBuf;

use crate::document::Document;
use crate::logger::{self, *};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct PathServer {
    // client: tower_lsp::Client,
    workspace_root: RwLock<HashSet<lsp_types::Url>>,
    documents: Arc<Mutex<HashMap<String, Document>>>, // url -> document
}

impl PathServer {
    pub fn new(client: tower_lsp::Client) -> Self {
        logger::init(&client);
        Self {
            // client,
            workspace_root: RwLock::new(HashSet::new()),
            documents: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for PathServer {
    async fn initialize(
        &self,
        params: lsp_types::InitializeParams,
    ) -> Result<lsp_types::InitializeResult> {
        if let Some(url) = params.root_uri {
            info(format!("Adding workspace root: {}", url)).await;
            let mut roots = self.workspace_root.write().await;
            roots.insert(url.clone());
        }
        Ok(lsp_types::InitializeResult {
            capabilities: lsp_types::ServerCapabilities {
                completion_provider: Some(lsp_types::CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        "/".to_string(),
                        "\\".to_string(), // For windows paths
                        ":".to_string(),
                    ]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Kind(
                    lsp_types::TextDocumentSyncKind::FULL,
                )),
                workspace: Some(lsp_types::WorkspaceServerCapabilities {
                    workspace_folders: Some(lsp_types::WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: lsp_types::InitializedParams) {
        info(format!("Path Server initialized")).await;
    }

    async fn did_change_workspace_folders(
        &self,
        params: lsp_types::DidChangeWorkspaceFoldersParams,
    ) {
        for folder in params.event.added {
            log(format!("Adding workspace folder: {}", folder.uri)).await;
            let mut roots = self.workspace_root.write().await;
            roots.insert(folder.uri.clone());
        }
        for folder in params.event.removed {
            log(format!("Removing workspace folder: {}", folder.uri)).await;
            let mut roots = self.workspace_root.write().await;
            roots.remove(&folder.uri);
        }
    }

    async fn did_open(&self, params: lsp_types::DidOpenTextDocumentParams) {
        self.documents.lock().await.insert(
            params.text_document.uri.to_string(),
            Document::new(params.text_document.text),
        );
    }

    async fn did_change(&self, params: lsp_types::DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents
                .lock()
                .await
                .entry(params.text_document.uri.to_string())
                .and_modify(|doc| doc.update_text(change.text.clone()))
                .or_insert_with(|| Document::new(change.text));
        }
    }

    async fn did_close(&self, params: lsp_types::DidCloseTextDocumentParams) {
        self.documents
            .lock()
            .await
            .remove(&params.text_document.uri.to_string());
    }

    async fn completion(
        &self,
        params: lsp_types::CompletionParams,
    ) -> Result<Option<lsp_types::CompletionResponse>> {
        // 1. get prefix of current line
        let line_number = params.text_document_position.position.line as usize;
        let character = params.text_document_position.position.character as usize;
        let prefix_string = self
            .documents
            .lock()
            .await
            .get(&params.text_document_position.text_document.uri.to_string())
            .and_then(|doc| doc.get_line(line_number))
            .map(|line| {
                // let character = character.min(line.len());
                line[..character].to_string()
            })
            .unwrap_or("".into());
        let prefix = PathBuf::from(prefix_string.clone()); // the path to be completed
        info(format!("Completing prefix: '{}'", prefix.display())).await;
        // 2. parse prefix into finished and remains
        // let Some(finished) = prefix.parent() else {
        //     info(format!(
        //         "Failed to get parent of prefix: {}",
        //         prefix.display()
        //     ))
        //     .await;
        //     return Ok(None);
        // };
        // info(format!("@@@ finished: {}", finished.display())).await;
        // let remains = prefix.file_name();
        let (finished, remains) = separate_prefix(&prefix_string);
        let finished = PathBuf::from(finished);
        // 3. fs access
        let mut completion_filenames: Vec<lsp_types::CompletionItem> = vec![];
        if !finished.exists() {
            info(format!(
                "Prefix parent does not exist: {}",
                finished.display()
            ))
            .await;
            return Ok(None);
        }
        if !finished.is_dir() {
            info(format!(
                "Prefix parent is not a directory: {}",
                finished.display()
            ))
            .await;
            return Ok(None);
        }
        if prefix.is_absolute() {
            // a. absolute path
            let Ok(files) = finished.read_dir() else {
                info(format!("Failed to read directory: {}", finished.display())).await;
                return Ok(None);
            };
            for file in files {
                let Ok(file) = file else {
                    info(format!(
                        "Failed to read file in directory: {}",
                        finished.display()
                    ))
                    .await;
                    continue;
                };
                let Ok(filename) = file.file_name().into_string() else {
                    info(format!(
                        "Failed to convert file name to string: {}",
                        file.path().display()
                    ))
                    .await;
                    continue;
                };
                if !filename.starts_with(&remains) {
                    continue;
                }
                if file.path().is_dir() {
                    completion_filenames.push(lsp_types::CompletionItem {
                        label: filename,
                        kind: Some(lsp_types::CompletionItemKind::FOLDER),
                        ..Default::default()
                    });
                } else {
                    completion_filenames.push(lsp_types::CompletionItem {
                        label: filename,
                        kind: Some(lsp_types::CompletionItemKind::FILE),
                        ..Default::default()
                    });
                }
            }
        } else if prefix.is_relative() {
            // b. relative path
            let roots = self.workspace_root.read().await;
            for root in roots.iter() {
                let Ok(root_path) = root.to_file_path() else {
                    info(format!(
                        "Failed to convert workspace root to file path: {root}"
                    ))
                    .await;
                    continue;
                };
                // TODO
            }
        } else {
            panic!("Unreachable!")
        }
        return Ok(Some(lsp_types::CompletionResponse::Array(
            completion_filenames,
        )));
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

fn separate_prefix(prefix: &str) -> (String, String) {
    if cfg!(unix) {
        let prefix = prefix.to_string();
        let last_slash = prefix.rfind('/');
        let (mut finished, remains) = if let Some(pos) = last_slash {
            (prefix[..pos + 1].to_string(), prefix[pos + 1..].to_string())
        } else {
            // no slash, e.g. index.htm
            ("".to_string(), prefix)
        };
        if finished.is_empty() {
            finished = "./".to_string();
        }
        return (finished, remains);
    } else if cfg!(windows) {
        let prefix = prefix.to_string();
        let last_backslash = prefix.rfind('\\');
        let (mut finished, remains) = if let Some(pos) = last_backslash {
            (prefix[..pos + 1].to_string(), prefix[pos + 1..].to_string())
        } else {
            // no backslash, e.g. index.htm
            ("".to_string(), prefix)
        };
        if finished.is_empty() {
            finished = "./".to_string();
        }
        return (finished, remains);
    }
    panic!("Unsupported platform!")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separate_prefix() {
        let (finished, remains) = separate_prefix("/home/user/file.txt");
        assert_eq!(finished, "/home/user/");
        assert_eq!(remains, "file.txt");

        let (finished, remains) = separate_prefix("file.txt");
        assert_eq!(finished, "./");
        assert_eq!(remains, "file.txt");

        let (finished, remains) = separate_prefix("./file.txt");
        assert_eq!(finished, "./");
        assert_eq!(remains, "file.txt");

        let (finished, remains) = separate_prefix("../file.txt");
        assert_eq!(finished, "../");
        assert_eq!(remains, "file.txt");
    }
}
