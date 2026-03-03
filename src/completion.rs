use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tower_lsp::lsp_types;

use crate::logger::*;

pub async fn complete(
    prefix: &str,
    workspace_roots: &HashSet<lsp_types::Url>,
    current_file: &lsp_types::Url,
) -> Vec<lsp_types::CompletionItem> {
    // 1. separate prefix into finished and remains
    let (base_dir, partial_name) = separate_prefix(&prefix);
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

    // 2. fs access
    let mut completions: Vec<lsp_types::CompletionItem> = vec![];
    if base_dir.is_absolute() {
        // a. absolute path
        let absolute_completions = complete_absolute(&base_dir, &partial_name).await;
        completions.extend(absolute_completions);
    } else if base_dir.is_relative() {
        // b. relative path
        // base on workspace roots
        for root in workspace_roots.iter() {
            let Ok(root_path) = root.to_file_path() else {
                info(format!(
                    "Failed to convert workspace root to file path: {root}"
                ))
                .await;
                continue;
            };
            let rel_workspace_completions =
                complete_relative(&base_dir, &partial_name, &root_path).await;
            completions.extend(rel_workspace_completions);
        }
        // base on current file url
        if let Ok(file_path) = current_file.to_file_path() {
            if let Some(parent) = file_path.parent() {
                let rel_current_file_completions =
                    complete_relative(&base_dir, &partial_name, &parent).await;
                completions.extend(rel_current_file_completions);
            }
        }
    } else {
        panic!("Unreachable!")
    };
    return completions;
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

async fn complete_absolute(
    base_dir: &PathBuf,
    partial_name: &str,
) -> Vec<lsp_types::CompletionItem> {
    let mut completions: Vec<lsp_types::CompletionItem> = vec![];
    if !base_dir.exists() {
        info(format!(
            "Base directory does not exist: {}",
            base_dir.display()
        ))
        .await;
        return vec![];
    }
    if !base_dir.is_dir() {
        info(format!(
            "Base directory is not a directory: {}",
            base_dir.display()
        ))
        .await;
        return vec![];
    }
    let Ok(files) = base_dir.read_dir() else {
        info(format!(
            "Failed to read base directory: {}",
            base_dir.display()
        ))
        .await;
        return vec![];
    };
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
            completions.push(lsp_types::CompletionItem {
                label: filename,
                kind: Some(lsp_types::CompletionItemKind::FOLDER),
                ..Default::default()
            });
        } else {
            completions.push(lsp_types::CompletionItem {
                label: filename,
                kind: Some(lsp_types::CompletionItemKind::FILE),
                ..Default::default()
            });
        }
    }
    return completions;
}

async fn complete_relative(
    base_dir: &PathBuf,
    partial_name: &str,
    root: &Path,
) -> Vec<lsp_types::CompletionItem> {
    let mut completions: Vec<lsp_types::CompletionItem> = vec![];
    let dir = root.join(&base_dir);
    if !dir.exists() {
        info(format!("Base directory does not exist: {}", dir.display())).await;
        return vec![];
    }
    if !dir.is_dir() {
        info(format!(
            "Base directory is not a directory: {}",
            dir.display()
        ))
        .await;
        return vec![];
    }
    let Ok(files) = dir.read_dir() else {
        info(format!("Failed to read base directory: {}", dir.display())).await;
        return vec![];
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
            completions.push(lsp_types::CompletionItem {
                label: filename.clone(),
                kind: Some(lsp_types::CompletionItemKind::FOLDER),
                detail: Some("From Workspace".to_string()),
                insert_text: Some(filename),
                ..Default::default()
            });
        } else {
            completions.push(lsp_types::CompletionItem {
                label: filename.clone(),
                kind: Some(lsp_types::CompletionItemKind::FILE),
                detail: Some("From Workspace".to_string()),
                insert_text: Some(filename),
                ..Default::default()
            });
        }
    }
    return completions;
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
}
