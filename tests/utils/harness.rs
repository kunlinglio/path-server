#![allow(dead_code)]

use path_server::Config;
use path_server::PathServer;
use std::fs::{self, File};
use std::path::PathBuf;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService};

pub struct TestHarness {
    _temp_dir: tempfile::TempDir,
    root_uri: Url,
    root_path: PathBuf,
    service: Option<LspService<PathServer>>,
}

impl TestHarness {
    /// create workspace folder and init language server
    pub async fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_path = temp_dir.path().to_path_buf();
        let root_uri = Url::from_directory_path(&root_path).unwrap();

        let (service, _) = LspService::new(|client| PathServer::new(client));
        let harness = Self {
            _temp_dir: temp_dir,
            root_uri: root_uri.clone(),
            root_path,
            service: Some(service),
        };

        // initialize language server
        harness
            .get_server()
            .initialize(InitializeParams {
                root_uri: Some(root_uri),
                ..Default::default()
            })
            .await
            .unwrap();
        harness
    }

    pub fn get_server(&self) -> &PathServer {
        self.service.as_ref().unwrap().inner()
    }

    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    /// override server config for testing
    pub async fn set_config(&self, config: Config) {
        self.get_server().set_test_config(config).await;
    }

    /// quick create file in workspace
    pub fn create_file(&self, rel_path: &str) {
        let full_path = self.root_path.join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        File::create(full_path).unwrap();
    }

    /// emulate open a file
    pub async fn open_doc(&self, rel_path: &str, content: &str) -> Url {
        let uri = self.root_uri.join(rel_path).unwrap();
        self.get_server()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "".into(),
                    version: 1,
                    text: content.into(),
                },
            })
            .await;
        uri
    }

    /// get completion items
    pub async fn completion_items(
        &self,
        uri: &Url,
        line: u32,
        character: u32,
    ) -> Vec<CompletionItem> {
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };

        let result = self.get_server().completion(params).await.unwrap();

        match result {
            Some(CompletionResponse::Array(items)) => items,
            Some(CompletionResponse::List(list)) => list.items,
            None => vec![],
        }
    }

    /// test completion
    pub async fn assert_completion_contains(
        &self,
        uri: &Url,
        line: u32,
        character: u32,
        expected_label: &str,
    ) {
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };

        let result = self.get_server().completion(params).await.unwrap();

        // match result
        let items = match result {
            Some(CompletionResponse::Array(items)) => items,
            Some(CompletionResponse::List(list)) => list.items,
            None => vec![],
        };

        let found = items.iter().any(|item| item.label == expected_label);

        if !found {
            let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
            panic!(
                "Expected completion '{}' not found in: {:?}",
                expected_label, labels
            );
        }
    }

    /// get document links
    pub async fn document_links(&self, uri: &Url) -> Vec<DocumentLink> {
        let params = DocumentLinkParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self.get_server().document_link(params).await.unwrap();
        result.unwrap_or_default()
    }
}
