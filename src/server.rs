use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types;

use regex::Regex;
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
        let line_prefix = self
            .documents
            .lock()
            .await
            .get(&params.text_document_position.text_document.uri.to_string())
            .and_then(|doc| doc.get_line(line_number))
            .map(|line| {
                let end = std::cmp::min(character, line.len());
                line[..end].to_string()
            })
            .unwrap_or("".into());
        let raw_path = parse_path(&line_prefix);
        let file_path = params.text_document_position.text_document.uri;
        info(format!("Completing for prefix: '{}'", raw_path)).await;

        // 2. parse prefix into finished and remains
        let (base_dir, partial_name) = separate_prefix(&raw_path);
        info(format!(
            "Detected base_dir: '{}', partial_name: '{}'",
            base_dir, partial_name
        ))
        .await;
        // manual unfold "~"
        let base_dir = if base_dir.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_else(|_| "".into());
            format!("{}{}", home, &base_dir[1..])
        } else {
            base_dir
        };
        let base_dir = PathBuf::from(base_dir);
        // 3. fs access
        let mut completion_filenames: Vec<lsp_types::CompletionItem> = vec![];
        if base_dir.is_absolute() {
            // a. absolute path
            if !base_dir.exists() {
                info(format!(
                    "Base directory does not exist: {}",
                    base_dir.display()
                ))
                .await;
            } else if !base_dir.is_dir() {
                info(format!(
                    "Base directory is not a directory: {}",
                    base_dir.display()
                ))
                .await;
            } else {
                if let Ok(files) = base_dir.read_dir() {
                    for file in files {
                        let Ok(file) = file else {
                            info(format!(
                                "Failed to read file in base directory: {}",
                                base_dir.display()
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
                        if !filename.starts_with(&partial_name) {
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
                } else {
                    info(format!(
                        "Failed to read base directory: {}",
                        base_dir.display()
                    ))
                    .await;
                };
            }
        } else if base_dir.is_relative() {
            // b. relative path
            // base on workspace roots
            let roots = self.workspace_root.read().await;
            for root in roots.iter() {
                let Ok(root_path) = root.to_file_path() else {
                    info(format!(
                        "Failed to convert workspace root to file path: {root}"
                    ))
                    .await;
                    continue;
                };
                let dir = root_path.join(&base_dir);
                if !dir.exists() {
                    info(format!("Base directory does not exist: {}", dir.display())).await;
                    continue;
                }
                if !dir.is_dir() {
                    info(format!(
                        "Base directory is not a directory: {}",
                        dir.display()
                    ))
                    .await;
                    continue;
                }
                let Ok(files) = dir.read_dir() else {
                    info(format!("Failed to read base directory: {}", dir.display())).await;
                    continue;
                };
                for file in files {
                    let Ok(file) = file else {
                        info(format!(
                            "Failed to read file in base directory: {}",
                            dir.display()
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
                    if !filename.starts_with(&partial_name) {
                        continue;
                    }
                    if file.path().is_dir() {
                        completion_filenames.push(lsp_types::CompletionItem {
                            label: filename.clone(),
                            kind: Some(lsp_types::CompletionItemKind::FOLDER),
                            detail: Some("From Workspace".to_string()),
                            insert_text: Some(filename),
                            ..Default::default()
                        });
                    } else {
                        completion_filenames.push(lsp_types::CompletionItem {
                            label: filename.clone(),
                            kind: Some(lsp_types::CompletionItemKind::FILE),
                            detail: Some("From Workspace".to_string()),
                            insert_text: Some(filename),
                            ..Default::default()
                        });
                    }
                }
            }
            // base on current file url
            if let Ok(file_path) = file_path.to_file_path() {
                if let Some(parent) = file_path.parent() {
                    let dir = parent.join(base_dir);
                    if !dir.exists() {
                        info(format!("Directory does not exist: {}", dir.display())).await;
                    } else if !dir.is_dir() {
                        info(format!("Directory is not a directory: {}", dir.display())).await;
                    } else {
                        if let Ok(files) = dir.read_dir() {
                            for file in files {
                                let Ok(file) = file else {
                                    info(format!(
                                        "Failed to read file in parent directory: {}",
                                        dir.display()
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
                                if !filename.starts_with(&partial_name) {
                                    continue;
                                }
                                if file.path().is_dir() {
                                    completion_filenames.push(lsp_types::CompletionItem {
                                        label: filename.clone(),
                                        kind: Some(lsp_types::CompletionItemKind::FOLDER),
                                        detail: Some("From document".to_string()),
                                        insert_text: Some(filename),
                                        ..Default::default()
                                    });
                                } else {
                                    completion_filenames.push(lsp_types::CompletionItem {
                                        label: filename.clone(),
                                        kind: Some(lsp_types::CompletionItemKind::FILE),
                                        detail: Some("From document".to_string()),
                                        insert_text: Some(filename),
                                        ..Default::default()
                                    });
                                }
                            }
                        } else {
                            info(format!(
                                "Failed to read parent directory: {}",
                                dir.display()
                            ))
                            .await;
                        }
                    }
                }
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
    let prefix = prefix.to_string();
    let last_slash = prefix.rfind('/');
    let last_backslash = prefix.rfind('\\');
    let (mut base_dir, partial_name) = if let Some(pos) = last_slash {
        (prefix[..pos + 1].to_string(), prefix[pos + 1..].to_string())
    } else if let Some(pos) = last_backslash {
        (prefix[..pos + 1].to_string(), prefix[pos + 1..].to_string())
    } else {
        // no slash, e.g. index.htm
        ("".to_string(), prefix)
    };
    if base_dir.is_empty() {
        base_dir = "./".to_string();
    }
    return (base_dir, partial_name);
}

fn parse_path(line: &str) -> String {
    // 1. parse by beginning
    //    e.g. "D:" or ".\" or "..\" for windows
    //    e.g. "/" or "~/" or "./" or "../" for unix
    // handle unix
    let beginning_unix = [r#"~/"#, r#"\.\./"#, r#"\./"#];
    for prefix in beginning_unix {
        if let Ok(re) = Regex::new(prefix) {
            if let Some(mat) = re.find_iter(line).last() {
                return line[mat.start()..].to_string();
            }
        }
    }
    // special case for unix root "/"
    let root_regex = Regex::new(r#"(?:^|[\s"'\[(])(/)"#).unwrap();
    if let Some(mat) = root_regex.find_iter(line).last() {
        if let Some(pos) = line[mat.start()..mat.end()].find('/') {
            return line[mat.start() + pos..].to_string();
        }
    }
    // handle windows
    let beginning_windows = [r#"[a-zA-Z]:\\"#, r#"\.\\"#, r#"\.\.\\ "#];
    for regex in beginning_windows {
        if let Ok(re) = Regex::new(regex) {
            if let Some(mat) = re.find_iter(line).last() {
                return line[mat.start()..].to_string();
            }
        }
    }
    // 2. parse by space
    if let Some(pos) = line.rfind(' ') {
        return line[pos + 1..].to_string();
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separate_prefix() {
        // unix style
        let (base, partial) = separate_prefix("/home/user/file.txt");
        assert_eq!(base, "/home/user/");
        assert_eq!(partial, "file.txt");

        // Windows style
        let (base, partial) = separate_prefix(r"C:\Users\Admin\Doc");
        assert_eq!(base, r"C:\Users\Admin\");
        assert_eq!(partial, "Doc");

        // only filename
        let (base, partial) = separate_prefix("file.txt");
        assert_eq!(base, "./");
        assert_eq!(partial, "file.txt");

        // only dir
        let (base, partial) = separate_prefix("/usr/bin/");
        assert_eq!(base, "/usr/bin/");
        assert_eq!(partial, "");

        // hidden file
        let (base, partial) = separate_prefix("./.config");
        assert_eq!(base, "./");
        assert_eq!(partial, ".config");
    }

    #[test]
    fn test_parse_path() {
        // 1. unix home dir
        assert_eq!(
            parse_path("~/projects/rust/main.rs"),
            "~/projects/rust/main.rs"
        );
        assert_eq!(parse_path("/etc/nginx/nginx.conf"), "/etc/nginx/nginx.conf");

        // 2. windows
        assert_eq!(
            parse_path(r"setting=C:\Windows\System32\"),
            r"C:\Windows\System32\"
        );
        assert_eq!(parse_path(r"Look at .\local\file"), r".\local\file");

        // 3. quote
        assert_eq!(
            parse_path("import './components/Header"),
            "./components/Header"
        );
        assert_eq!(
            parse_path("let p = \"../data/config.json"),
            "../data/config.json"
        );

        // 4. markdown
        assert_eq!(parse_path("[link](./docs/README.md"), "./docs/README.md");
        assert_eq!(parse_path("![img](/assets/logo.png"), "/assets/logo.png");

        // 5. multi path in same line
        assert_eq!(parse_path("from /tmp/a to /var/log/b"), "/var/log/b");
    }
}
