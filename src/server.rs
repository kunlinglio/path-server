use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types;

use crate::client::{ClientMetadata, Editor, get_client, set_client};
use crate::config;
use crate::document::Document;
use crate::error::*;
use crate::fs::url_to_path;
use crate::logger::{self};
use crate::parser;
use crate::providers;
use crate::{lsp_debug, lsp_error, lsp_info};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub struct PathServer {
    client: tower_lsp::Client,
    workspace_roots: RwLock<HashSet<PathBuf>>,
    /// file path -> document
    documents: RwLock<HashMap<PathBuf, Document>>,
    config_cache: RwLock<Option<Arc<config::Config>>>,
}

impl PathServer {
    pub fn new(client: tower_lsp::Client) -> Self {
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

    pub fn parse_editor_env(&self, params: &lsp_types::InitializeParams) -> Editor {
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

    pub fn parse_client_env(&self, params: &lsp_types::InitializeParams) -> ClientMetadata {
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
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for PathServer {
    async fn initialize(
        &self,
        params: lsp_types::InitializeParams,
    ) -> jsonrpc::Result<lsp_types::InitializeResult> {
        lsp_info!("Initializing Path Server").await;
        // set editor env
        let client_env = self.parse_client_env(&params);
        lsp_info!("Client Env: {}", client_env).await;
        set_client(client_env).await;
        // for backward compatibility
        if let Some(uri) = params.root_uri {
            let root = url_to_path(&uri).map_err(|e| {
                PathServerError::InvalidPath(format!(
                    "Invalid workspace root URI: {}, error: {}",
                    uri, e
                ))
            })?;
            let mut roots = self.workspace_roots.write().await;
            roots.insert(root);
        }
        if let Some(folders) = params.workspace_folders {
            let mut roots = self.workspace_roots.write().await;
            for folder in folders {
                lsp_info!("Adding workspace root: {}", folder.uri).await;
                let root = url_to_path(&folder.uri).map_err(|e| {
                    PathServerError::InvalidPath(format!(
                        "Invalid workspace folder URI: {}, error: {}",
                        folder.uri, e
                    ))
                })?;
                roots.insert(root);
            }
        }
        Ok(lsp_types::InitializeResult {
            capabilities: lsp_types::ServerCapabilities {
                // for path completion
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
                // for path highlighting
                document_link_provider: Some(lsp_types::DocumentLinkOptions {
                    resolve_provider: Some(false),
                    work_done_progress_options: Default::default(),
                }),
                // for path jumping
                definition_provider: Some(lsp_types::OneOf::Left(true)),
                // for hover hint
                hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
                // detectors
                text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Kind(
                    lsp_types::TextDocumentSyncKind::INCREMENTAL,
                )),
                workspace: Some(lsp_types::WorkspaceServerCapabilities {
                    workspace_folders: Some(lsp_types::WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(lsp_types::OneOf::Left(true)),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: lsp_types::InitializedParams) {
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

    async fn did_change_configuration(&self, _: lsp_types::DidChangeConfigurationParams) {
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
        params: lsp_types::DidChangeWorkspaceFoldersParams,
    ) {
        for folder in params.event.added {
            lsp_info!("Adding workspace folder: {}", folder.uri).await;
            let mut roots = self.workspace_roots.write().await;
            let root_result = url_to_path(&folder.uri);
            let Ok(root) = root_result else {
                lsp_error!(
                    "Failed to convert URI to file path: {}, error: {}",
                    folder.uri,
                    root_result.unwrap_err()
                )
                .await;
                continue;
            };
            roots.insert(root);
        }
        for folder in params.event.removed {
            lsp_info!("Removing workspace folder: {}", folder.uri).await;
            let mut roots = self.workspace_roots.write().await;
            let root_result = url_to_path(&folder.uri);
            let Ok(root) = root_result else {
                lsp_error!(
                    "Failed to convert URI to file path: {}, error: {}",
                    folder.uri,
                    root_result.unwrap_err()
                )
                .await;
                continue;
            };
            roots.remove(&root);
        }
    }

    async fn did_open(&self, params: lsp_types::DidOpenTextDocumentParams) {
        lsp_info!(
            "Opening document: {}, language: {}",
            params.text_document.uri,
            params.text_document.language_id
        )
        .await;
        let mut documents = self.documents.write().await;
        let path_res = url_to_path(&params.text_document.uri);
        let Ok(path) = path_res else {
            lsp_error!(
                "Failed to convert document URI to file path: {}, error: {}",
                params.text_document.uri,
                path_res.unwrap_err()
            )
            .await;
            return;
        };
        let doc_res = Document::new(params.text_document.text, &params.text_document.language_id);
        let Ok(doc) = doc_res else {
            lsp_error!(
                "Failed to create document for: {}, error: {}",
                params.text_document.uri,
                doc_res.unwrap_err()
            )
            .await;
            return;
        };
        documents.insert(path, doc);
    }

    async fn did_change(&self, params: lsp_types::DidChangeTextDocumentParams) {
        lsp_info!(
            "[Document Sync] Changing document: {}",
            params.text_document.uri
        )
        .await;
        let path_res = url_to_path(&params.text_document.uri);
        let Ok(path) = path_res else {
            lsp_error!(
                "Failed to convert document URI to file path: {}, error: {}",
                params.text_document.uri,
                path_res.unwrap_err()
            )
            .await;
            return;
        };
        let mut docs = self.documents.write().await;
        let doc = docs.entry(path).or_insert_with(Document::default);
        // apply each change in order
        for change in params.content_changes.into_iter() {
            let result = doc.apply_change(change);
            if let Err(e) = result {
                lsp_error!(
                    "Failed to apply change to document {}: {}",
                    params.text_document.uri,
                    e
                )
                .await;
                return;
            }
        }
    }

    async fn did_close(&self, params: lsp_types::DidCloseTextDocumentParams) {
        lsp_info!(
            "[Document Sync] Closing document: {}",
            params.text_document.uri
        )
        .await;
        let path_res = url_to_path(&params.text_document.uri);
        let Ok(path) = path_res else {
            lsp_error!(
                "Failed to convert document URI to file path: {}",
                params.text_document.uri
            )
            .await;
            return;
        };
        self.documents.write().await.remove(&path);
    }

    async fn completion(
        &self,
        params: lsp_types::CompletionParams,
    ) -> jsonrpc::Result<Option<lsp_types::CompletionResponse>> {
        // get the line prefix
        let line_number = params.text_document_position.position.line as usize;
        let character = params.text_document_position.position.character as usize;
        let path = url_to_path(&params.text_document_position.text_document.uri).map_err(|e| {
            PathServerError::InvalidPath(format!(
                "Failed to convert document URI to file path: {}, error: {}",
                params.text_document_position.text_document.uri, e
            ))
        })?;
        let documents = self.documents.read().await;
        let doc = documents
            .get(&path)
            .ok_or(PathServerError::Unknown(format!(
                "Document {} not found, please open it before completion",
                path.display()
            )))?;
        let line_prefix = doc.get_line(line_number, Some(character))?;

        // parse the line
        let raw_path = parser::parse_line(&line_prefix);
        lsp_info!("[Completion] Completing for prefix: '{}'", raw_path).await;

        // completion
        let config = self.get_config().await;
        let file_path =
            url_to_path(&params.text_document_position.text_document.uri).map_err(|e| {
                PathServerError::InvalidPath(format!(
                    "Failed to convert document URI to file path: {}, error: {}",
                    params.text_document_position.text_document.uri, e
                ))
            })?;
        let workspace_roots = self.workspace_roots.read().await;
        let completions =
            providers::provide_completion(&raw_path, &workspace_roots, &file_path, &config).await?;
        lsp_info!("[Completion] Generated {} completions", completions.len()).await;
        lsp_debug!(
            "{:?}",
            completions
                .iter()
                .map(|c| c.label.to_owned())
                .collect::<Vec<_>>()
        )
        .await;
        return Ok(Some(lsp_types::CompletionResponse::Array(completions)));
    }

    async fn document_link(
        &self,
        params: lsp_types::DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<lsp_types::DocumentLink>>> {
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
            params.text_document.uri
        )
        .await;
        let path = url_to_path(&params.text_document.uri).map_err(|e| {
            PathServerError::InvalidPath(format!(
                "Failed to convert document URI to file path: {}, error: {}",
                params.text_document.uri, e
            ))
        })?;
        let documents = self.documents.read().await;
        let doc = documents
            .get(&path)
            .ok_or(PathServerError::Unknown(format!(
                "Document {} not found, please open it before providing document links",
                path.display()
            )))?;

        let workspace_roots = self.workspace_roots.read().await;
        let links =
            providers::provide_document_links(doc, &path, &config, &workspace_roots).await?;
        lsp_info!("[Document Link] Generated {} document links", links.len()).await;
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
        params: lsp_types::GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<lsp_types::GotoDefinitionResponse>> {
        lsp_info!(
            "[Goto Definition] Processing goto definition request for: {} {}:{}",
            params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position.line,
            params.text_document_position_params.position.character
        )
        .await;
        let line = params.text_document_position_params.position.line as usize;
        let character = params.text_document_position_params.position.character as usize;
        let path =
            url_to_path(&params.text_document_position_params.text_document.uri).map_err(|e| {
                PathServerError::InvalidPath(format!(
                    "Failed to convert document URI to file path: {}, error: {}",
                    params.text_document_position_params.text_document.uri, e
                ))
            })?;

        let documents = self.documents.read().await;
        let doc = documents
            .get(&path)
            .ok_or(PathServerError::Unknown(format!(
                "Document {} not found, please open it before providing goto definition",
                path.display()
            )))?;
        let config = self.get_config().await;
        let workspace_roots = self.workspace_roots.read().await;

        let definition =
            providers::provide_definition(doc, &path, line, character, &config, &workspace_roots)
                .await?;
        if let Some(definition) = &definition {
            let lsp_types::GotoDefinitionResponse::Link(definition) = &definition else {
                unreachable!("Definition is not a link");
            };
            lsp_info!(
                "[Goto Definition] Generated definition to: {}",
                definition[0].target_uri
            )
            .await;
            lsp_debug!("[Goto Definition] Definition details: {:?}", definition).await;
        } else {
            lsp_info!("[Goto Definition] No definition found").await;
        }
        Ok(definition)
    }

    async fn hover(
        &self,
        params: lsp_types::HoverParams,
    ) -> jsonrpc::Result<Option<lsp_types::Hover>> {
        lsp_info!(
            "[Hover] Processing hover request for: {} {}:{}",
            params.text_document_position_params.text_document.uri,
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
        let path =
            url_to_path(&params.text_document_position_params.text_document.uri).map_err(|e| {
                PathServerError::InvalidPath(format!(
                    "Failed to convert document URI to file path: {}, error: {}",
                    params.text_document_position_params.text_document.uri, e
                ))
            })?;
        let documents = self.documents.read().await;
        let doc = documents
            .get(&path)
            .ok_or(PathServerError::Unknown(format!(
                "Document {} not found, please open it before hover information",
                path.display()
            )))?;
        let workspace_roots = self.workspace_roots.read().await;

        let hover =
            providers::provide_hover(doc, &path, line, character, &config, &workspace_roots)
                .await?;
        if let Some(hover) = &hover {
            lsp_info!("[Hover] Generated hover content: {:?}", hover.contents).await;
        } else {
            lsp_info!("[Hover] No hover content found").await;
        };
        Ok(hover)
    }
}
