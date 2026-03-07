use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types;

use crate::common::*;
use crate::completion;
use crate::config;
use crate::document::Document;
use crate::logger::{self, *};
use crate::parser;
use crate::utils::url_to_path;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub struct PathServer {
    client: tower_lsp::Client,
    workspace_roots: RwLock<HashSet<PathBuf>>,
    /// file path -> document
    documents: Mutex<HashMap<PathBuf, Document>>,
    /// To override configuration from lsp client
    config_override: RwLock<Option<config::Config>>,
}

impl PathServer {
    pub fn new(client: tower_lsp::Client) -> Self {
        logger::init(&client);
        Self {
            client,
            workspace_roots: RwLock::new(HashSet::new()),
            documents: Mutex::new(HashMap::new()),
            config_override: RwLock::new(None),
        }
    }

    async fn get_config(&self) -> config::Config {
        if let Some(cfg) = self.config_override.read().await.clone() {
            return cfg;
        }
        config::get(&self.client).await
    }

    pub async fn set_test_config(&self, cfg: config::Config) {
        let mut guard = self.config_override.write().await;
        *guard = Some(cfg);
    }
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for PathServer {
    async fn initialize(
        &self,
        params: lsp_types::InitializeParams,
    ) -> jsonrpc::Result<lsp_types::InitializeResult> {
        // for backward compatibility
        if let Some(uri) = params.root_uri {
            let Ok(root) = url_to_path(&uri) else {
                warn(format!("Failed to convert root URI to file path: {}", uri)).await;
                return Err(jsonrpc::Error::invalid_params("Invalid root URI"));
            };
            let mut roots = self.workspace_roots.write().await;
            roots.insert(root);
        }
        if let Some(folders) = params.workspace_folders {
            let mut roots = self.workspace_roots.write().await;
            for folder in folders {
                log(format!("Adding workspace root: {}", folder.uri)).await;
                let Ok(root) = url_to_path(&folder.uri) else {
                    warn(format!(
                        "Failed to convert URI to file path: {}",
                        folder.uri
                    ))
                    .await;
                    continue;
                };
                roots.insert(root);
            }
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
                    lsp_types::TextDocumentSyncKind::INCREMENTAL,
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
        log("Path Server initialized".to_string()).await;
        log(format!("Path Server version: {}", VERSION)).await;
    }

    async fn did_change_configuration(&self, _: lsp_types::DidChangeConfigurationParams) {
        // TODO: implement it
    }

    async fn did_change_workspace_folders(
        &self,
        params: lsp_types::DidChangeWorkspaceFoldersParams,
    ) {
        for folder in params.event.added {
            log(format!("Adding workspace folder: {}", folder.uri)).await;
            let mut roots = self.workspace_roots.write().await;
            let Ok(root) = url_to_path(&folder.uri) else {
                warn(format!(
                    "Failed to convert URI to file path: {}",
                    folder.uri
                ))
                .await;
                continue;
            };
            roots.insert(root);
        }
        for folder in params.event.removed {
            log(format!("Removing workspace folder: {}", folder.uri)).await;
            let mut roots = self.workspace_roots.write().await;
            let Ok(root) = url_to_path(&folder.uri) else {
                continue;
            };
            roots.remove(&root);
        }
    }

    async fn did_open(&self, params: lsp_types::DidOpenTextDocumentParams) {
        let mut documents = self.documents.lock().await;
        let Ok(path) = url_to_path(&params.text_document.uri) else {
            warn(format!(
                "Failed to convert URI to file path: {}",
                params.text_document.uri
            ))
            .await;
            return;
        };
        documents.insert(path, Document::new(params.text_document.text));
    }

    async fn did_change(&self, params: lsp_types::DidChangeTextDocumentParams) {
        let Ok(path) = url_to_path(&params.text_document.uri) else {
            return;
        };
        let mut docs = self.documents.lock().await;
        let doc = docs
            .entry(path)
            .or_insert_with(|| Document::new(String::new()));
        // apply each change in order
        for change in params.content_changes.into_iter() {
            let result = doc.apply_change(&change);
            if let Err(e) = result {
                error(format!("Failed to apply change: {}", e)).await;
                continue;
            }
            debug(format!(
                "Applied change to document: {}",
                params.text_document.uri
            ))
            .await;
            debug(format!("Document text: {}", doc.text)).await;
        }
    }

    async fn did_close(&self, params: lsp_types::DidCloseTextDocumentParams) {
        let Ok(path) = url_to_path(&params.text_document.uri) else {
            return;
        };
        self.documents.lock().await.remove(&path);
    }

    async fn completion(
        &self,
        params: lsp_types::CompletionParams,
    ) -> jsonrpc::Result<Option<lsp_types::CompletionResponse>> {
        // get the line prefix
        let line_number = params.text_document_position.position.line as usize;
        let character = params.text_document_position.position.character as usize;
        let Ok(path) = url_to_path(&params.text_document_position.text_document.uri) else {
            warn(format!(
                "Failed to convert URI to file path: {}",
                params.text_document_position.text_document.uri
            ))
            .await;
            return Ok(None);
        };
        let documents = self.documents.lock().await;
        let Some(doc) = documents.get(&path) else {
            warn(format!("Document not found: {}", path.display())).await;
            return Err(PathServerError::Unknown(format!(
                "Document not found: {}",
                path.display()
            ))
            .into());
        };
        let line_prefix = doc.get_line(line_number, Some(character))?;

        // parse the line
        let raw_path = parser::parse_line(&line_prefix);
        debug(format!("Completing for prefix: '{}'", raw_path)).await;

        // completion
        let completion_config = self.get_config().await.completion;
        let Ok(file_path) = url_to_path(&params.text_document_position.text_document.uri) else {
            warn(format!(
                "Failed to convert URI to file path: {}",
                params.text_document_position.text_document.uri
            ))
            .await;
            return Ok(None);
        };
        let workspace_roots = self.workspace_roots.read().await;
        let completions =
            completion::complete(&raw_path, &workspace_roots, &file_path, &completion_config)
                .await?;

        return Ok(Some(lsp_types::CompletionResponse::Array(completions)));
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }
}
