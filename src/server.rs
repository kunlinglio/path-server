use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types;

use crate::common::*;
use crate::completion;
use crate::document::Document;
use crate::logger::{self, *};
use crate::parser;

#[derive(Debug)]
pub struct PathServer {
    // client: tower_lsp::Client,
    workspace_roots: RwLock<HashSet<lsp_types::Url>>,
    documents: Arc<Mutex<HashMap<String, Document>>>, // url -> document
}

impl PathServer {
    pub fn new(client: tower_lsp::Client) -> Self {
        logger::init(&client);
        Self {
            // client,
            workspace_roots: RwLock::new(HashSet::new()),
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
            log(format!("Adding workspace root: {}", url)).await;
            let mut roots = self.workspace_roots.write().await;
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
    }

    async fn did_change_workspace_folders(
        &self,
        params: lsp_types::DidChangeWorkspaceFoldersParams,
    ) {
        for folder in params.event.added {
            log(format!("Adding workspace folder: {}", folder.uri)).await;
            let mut roots = self.workspace_roots.write().await;
            roots.insert(folder.uri.clone());
        }
        for folder in params.event.removed {
            log(format!("Removing workspace folder: {}", folder.uri)).await;
            let mut roots = self.workspace_roots.write().await;
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
        let uri = params.text_document.uri.to_string();
        let mut docs = self.documents.lock().await;
        let doc = docs
            .entry(uri)
            .or_insert_with(|| Document::new(String::new()));
        // apply each change in order
        for change in params.content_changes.into_iter() {
            let result = doc.apply_change(&change);
            if let Err(e) = result {
                error(format!("Failed to apply change: {}", e)).await;
                continue;
            }
            log(format!(
                "Applied change to document: {}",
                params.text_document.uri
            ))
            .await;
            debug(format!("Document text: {}", doc.text)).await;
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
        // 1. get the line prefix
        let line_number = params.text_document_position.position.line as usize;
        let character = params.text_document_position.position.character as usize;
        let url = &params.text_document_position.text_document.uri.to_string();
        let documents = self.documents.lock().await;
        let Some(doc) = documents.get(url) else {
            return Err(PathServerError::Unknown(format!("Document not found: {}", url)).into());
        };
        let line_prefix = doc.get_line(line_number, Some(character))?;

        // 2. parse the line
        let raw_path = parser::parse_line(&line_prefix);
        debug(format!("Completing for prefix: '{}'", raw_path)).await;

        // 3. completion
        let file_path = params.text_document_position.text_document.uri;
        let workspace_roots = self.workspace_roots.read().await;
        let completions = completion::complete(&raw_path, &workspace_roots, &file_path).await?;

        return Ok(Some(lsp_types::CompletionResponse::Array(completions)));
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
