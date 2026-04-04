use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tower_lsp_server::jsonrpc;
use tower_lsp_server::ls_types;

use crate::client::{ClientMetadata, Editor, get_client, set_client};
use crate::config;
use crate::document::Document;
use crate::error::*;
use crate::fs;
use crate::logger::{self};
use crate::parser::tree_sitter_supported;
use crate::providers;
use crate::{lsp_debug, lsp_error, lsp_info};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub struct PathServer {
    client: tower_lsp_server::Client,
    workspace_roots: RwLock<HashSet<ls_types::Uri>>,
    /// file path -> document
    documents: RwLock<HashMap<ls_types::Uri, Document>>,
    config_cache: RwLock<Option<Arc<config::Config>>>,
}

impl PathServer {
    pub fn new(client: tower_lsp_server::Client) -> Self {
        logger::init(&client);
        Self {
            client,
            workspace_roots: RwLock::new(HashSet::new()),
            documents: RwLock::new(HashMap::new()),
            config_cache: RwLock::new(None),
        }
    }

    async fn get_config(&self) -> Arc<config::Config> {
        if let Some(cfg) = self.config_cache.read().await.clone() {
            return cfg;
        }
        let cfg = Arc::new(config::get(&self.client).await);
        *self.config_cache.write().await = Some(cfg.clone());
        cfg
    }

    pub async fn set_test_config(&self, cfg: config::Config) {
        // a hacky way to make test config effect - set it into cache
        let mut guard = self.config_cache.write().await;
        *guard = Some(Arc::new(cfg));
    }

    pub fn parse_editor_env(&self, params: &ls_types::InitializeParams) -> Editor {
        let Some(options) = &params.initialization_options else {
            return Editor::Unknown("unknown".into());
        };
        let Some(editor) = options.get("editor") else {
            return Editor::Unknown("unknown".into());
        };
        if let Some(editor_str) = editor.as_str() {
            return Editor::from(editor_str);
        }
        Editor::Unknown("unknown".into())
    }

    pub fn parse_client_env(&self, params: &ls_types::InitializeParams) -> ClientMetadata {
        let editor = self.parse_editor_env(params);
        let support_document_link = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.document_link.clone())
            .is_some();
        ClientMetadata {
            editor,
            support_document_link,
        }
    }

    pub async fn workspace_paths(&self) -> Vec<String> {
        let lock_guard = self.workspace_roots.read().await;
        lock_guard
            .iter()
            .filter_map(|url| fs::url_to_path(url).ok().flatten())
            .map(|p: PathBuf| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    }

    pub fn doc_parent(doc_url: &ls_types::Uri) -> Option<String> {
        fs::url_to_path(doc_url)
            .ok()
            .flatten()
            .and_then(|p| p.parent().map(|p| p.to_string_lossy().into_owned()))
    }
}

impl tower_lsp_server::LanguageServer for PathServer {
    async fn initialize(
        &self,
        params: ls_types::InitializeParams,
    ) -> jsonrpc::Result<ls_types::InitializeResult> {
        lsp_info!("Initializing Path Server...").await;
        // set editor env
        let client_env = self.parse_client_env(&params);
        lsp_info!("Client Env: {}", client_env).await;
        set_client(client_env).await;
        // get workspace roots
        // for backward compatibility
        #[allow(deprecated)]
        if let Some(uri) = params.root_uri {
            let mut roots = self.workspace_roots.write().await;
            roots.insert(uri);
        }
        if let Some(folders) = params.workspace_folders {
            let mut roots = self.workspace_roots.write().await;
            for folder in folders {
                lsp_info!("Adding workspace root: {}", folder.uri.as_str()).await;
                roots.insert(folder.uri);
            }
        }
        Ok(ls_types::InitializeResult {
            capabilities: ls_types::ServerCapabilities {
                // for path completion
                completion_provider: Some(ls_types::CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        "/".to_string(),
                        "\\".to_string(), // For windows paths
                        ":".to_string(),
                    ]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                // for path highlighting
                document_link_provider: Some(ls_types::DocumentLinkOptions {
                    resolve_provider: Some(false),
                    work_done_progress_options: Default::default(),
                }),
                // for path jumping
                definition_provider: Some(ls_types::OneOf::Left(true)),
                // for hover hint
                hover_provider: Some(ls_types::HoverProviderCapability::Simple(true)),
                // detectors
                text_document_sync: Some(ls_types::TextDocumentSyncCapability::Kind(
                    ls_types::TextDocumentSyncKind::INCREMENTAL,
                )),
                workspace: Some(ls_types::WorkspaceServerCapabilities {
                    workspace_folders: Some(ls_types::WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(ls_types::OneOf::Left(true)),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: ls_types::InitializedParams) {
        lsp_info!("Path Server initialized").await;
        lsp_info!("Path Server version: {}", VERSION).await;
        if cfg!(debug_assertions) {
            lsp_info!("Running in debug mode").await;
        } else {
            lsp_info!("Running in release mode").await;
        }
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        lsp_info!("Shutting down Path Server").await;
        Ok(())
    }

    async fn did_change_configuration(&self, _: ls_types::DidChangeConfigurationParams) {
        let cfg = Arc::new(config::get(&self.client).await);
        *self.config_cache.write().await = Some(cfg);
        lsp_info!(
            "[Config] Configuration changed, update to: {}",
            self.config_cache.read().await.as_ref().unwrap()
        )
        .await;
    }

    async fn did_change_workspace_folders(
        &self,
        params: ls_types::DidChangeWorkspaceFoldersParams,
    ) {
        for folder in params.event.added {
            lsp_info!("Adding workspace folder: {}", folder.uri.as_str()).await;
            let mut roots = self.workspace_roots.write().await;
            roots.insert(folder.uri);
        }
        for folder in params.event.removed {
            lsp_info!("Removing workspace folder: {}", folder.uri.as_str()).await;
            let mut roots = self.workspace_roots.write().await;
            roots.remove(&folder.uri);
        }
    }

    async fn did_open(&self, params: ls_types::DidOpenTextDocumentParams) {
        let start = Instant::now();
        lsp_info!(
            "[Document Sync] Opening document: {}, language: {}, tree-sitter: {}",
            params.text_document.uri.as_str(),
            params.text_document.language_id,
            if tree_sitter_supported(&params.text_document.language_id) {
                "supported"
            } else {
                "unsupported"
            }
        )
        .await;
        let mut documents = self.documents.write().await;
        let doc_res = Document::new(params.text_document.text, &params.text_document.language_id);
        let Ok(doc) = doc_res else {
            lsp_error!(
                "Failed to create document for: {}, error: {}",
                params.text_document.uri.as_str(),
                doc_res.unwrap_err()
            )
            .await;
            return;
        };
        documents.insert(params.text_document.uri.clone(), doc);
        lsp_info!(
            "[Document Sync] Successfully opened document: {} in {:?}",
            params.text_document.uri.as_str(),
            start.elapsed()
        )
        .await;
    }

    async fn did_change(&self, params: ls_types::DidChangeTextDocumentParams) {
        let start = Instant::now();
        lsp_info!(
            "[Document Sync] Changing document: {}",
            params.text_document.uri.as_str()
        )
        .await;
        let mut docs = self.documents.write().await;
        let doc = docs
            .entry(params.text_document.uri.clone())
            .or_insert_with(Document::default);
        // apply each change in order
        for change in params.content_changes.into_iter() {
            let result = doc.apply_change(change);
            if let Err(e) = result {
                lsp_error!(
                    "Failed to apply change to document {}: {}",
                    params.text_document.uri.as_str(),
                    e
                )
                .await;
                return;
            }
        }
        lsp_info!(
            "[Document Sync] Successfully applied changes to document: {} in {:?}",
            params.text_document.uri.as_str(),
            start.elapsed()
        )
        .await;
    }

    async fn did_close(&self, params: ls_types::DidCloseTextDocumentParams) {
        lsp_info!(
            "[Document Sync] Closing document: {}",
            params.text_document.uri.as_str()
        )
        .await;
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
    }

    async fn completion(
        &self,
        params: ls_types::CompletionParams,
    ) -> jsonrpc::Result<Option<ls_types::CompletionResponse>> {
        let start = Instant::now();
        // get the line prefix
        let line_number = params.text_document_position.position.line as usize;
        let character = params.text_document_position.position.character as usize;
        let documents = self.documents.read().await;
        let doc = documents
            .get(&params.text_document_position.text_document.uri)
            .ok_or(PathServerError::Unknown(format!(
                "Document {} not found, please open it before completion",
                params.text_document_position.text_document.uri.as_str()
            )))?;

        // completion
        let config = self.get_config().await;
        let workspace_roots = self.workspace_paths().await;
        let parent = Self::doc_parent(&params.text_document_position.text_document.uri);
        let completions = providers::provide_completion(
            doc,
            (line_number, character),
            &workspace_roots,
            &parent,
            &config,
        )
        .await?;
        lsp_info!(
            "[Completion] Generated {} completions in {:?}",
            completions.len(),
            start.elapsed()
        )
        .await;
        lsp_debug!(
            "{:?}",
            completions
                .iter()
                .map(|c| c.label.to_owned())
                .collect::<Vec<_>>()
        )
        .await;
        Ok(Some(ls_types::CompletionResponse::Array(completions)))
    }

    async fn document_link(
        &self,
        params: ls_types::DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<ls_types::DocumentLink>>> {
        let start = Instant::now();
        let config = self.get_config().await;
        let client = get_client().await;
        if !client.support_document_link {
            lsp_info!("[Document Link] Client does not support document link").await;
            return Ok(None);
        };
        if !config.highlight.enable {
            lsp_info!("[Document Link] Highlighting is disabled").await;
            return Ok(None);
        }
        lsp_info!(
            "[Document Link] Processing document link request for: {}",
            params.text_document.uri.as_str()
        )
        .await;
        let documents = self.documents.read().await;
        let doc = documents
            .get(&params.text_document.uri)
            .ok_or(PathServerError::Unknown(format!(
                "Document {} not found, please open it before providing document links",
                params.text_document.uri.as_str()
            )))?;

        let workspace_roots = self.workspace_paths().await;
        let parent = Self::doc_parent(&params.text_document.uri);
        let links =
            providers::provide_document_links(doc, &parent, &config, &workspace_roots).await?;
        lsp_info!(
            "[Document Link] Generated {} document links in {:?}",
            links.len(),
            start.elapsed()
        )
        .await;
        lsp_debug!(
            "{:?}",
            links
                .iter()
                .map(|l| l.target.to_owned())
                .collect::<Vec<_>>()
        )
        .await;
        Ok(Some(links))
    }

    async fn goto_definition(
        &self,
        params: ls_types::GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<ls_types::GotoDefinitionResponse>> {
        let start = Instant::now();
        lsp_info!(
            "[Goto Definition] Processing goto definition request for: {} {}:{}",
            params
                .text_document_position_params
                .text_document
                .uri
                .as_str(),
            params.text_document_position_params.position.line,
            params.text_document_position_params.position.character
        )
        .await;
        let line = params.text_document_position_params.position.line as usize;
        let character = params.text_document_position_params.position.character as usize;
        let parent = Self::doc_parent(&params.text_document_position_params.text_document.uri);

        let documents = self.documents.read().await;
        let doc = documents
            .get(&params.text_document_position_params.text_document.uri)
            .ok_or(PathServerError::Unknown(format!(
                "Document {} not found, please open it before providing goto definition",
                params
                    .text_document_position_params
                    .text_document
                    .uri
                    .as_str()
            )))?;
        let config = self.get_config().await;
        let workspace_roots = self.workspace_paths().await;

        let definition =
            providers::provide_definition(doc, &parent, line, character, &config, &workspace_roots)
                .await?;
        if let Some(definition) = &definition {
            let ls_types::GotoDefinitionResponse::Link(definition) = &definition else {
                unreachable!("Definition is not a link");
            };
            lsp_info!(
                "[Goto Definition] Generated definition to: {} in {:?}",
                definition[0].target_uri.as_str(),
                start.elapsed()
            )
            .await;
            lsp_debug!("[Goto Definition] Definition details: {:?}", definition).await;
        } else {
            lsp_info!(
                "[Goto Definition] No definition found in {:?}",
                start.elapsed()
            )
            .await;
        }
        Ok(definition)
    }

    async fn hover(
        &self,
        params: ls_types::HoverParams,
    ) -> jsonrpc::Result<Option<ls_types::Hover>> {
        let start = Instant::now();
        lsp_info!(
            "[Hover] Processing hover request for: {} {}:{}",
            params
                .text_document_position_params
                .text_document
                .uri
                .as_str(),
            params.text_document_position_params.position.line,
            params.text_document_position_params.position.character
        )
        .await;
        let client = get_client().await;
        let config = self.get_config().await;
        if client.support_document_link && config.highlight.enable {
            lsp_info!("[Hover] Client support document link and highlight is enabled, provide nothing to avoid duplicated hover item").await;
            return Ok(None);
        };
        let line = params.text_document_position_params.position.line as usize;
        let character = params.text_document_position_params.position.character as usize;
        let documents = self.documents.read().await;
        let parent = Self::doc_parent(&params.text_document_position_params.text_document.uri);
        let doc = documents
            .get(&params.text_document_position_params.text_document.uri)
            .ok_or(PathServerError::Unknown(format!(
                "Document {} not found, please open it before hover information",
                params
                    .text_document_position_params
                    .text_document
                    .uri
                    .as_str()
            )))?;
        let workspace_roots = self.workspace_paths().await;

        let hover =
            providers::provide_hover(doc, &parent, line, character, &config, &workspace_roots)
                .await?;
        if let Some(hover) = &hover {
            lsp_info!(
                "[Hover] Generated hover content: {:?} in {:?}",
                hover.contents,
                start.elapsed()
            )
            .await;
        } else {
            lsp_info!("[Hover] No hover content found in {:?}", start.elapsed()).await;
        };
        Ok(hover)
    }
}
