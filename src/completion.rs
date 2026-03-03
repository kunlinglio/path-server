use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tower_lsp::lsp_types;

use crate::common::*;
use crate::logger::*;

pub async fn complete(
    prefix: &str,
    workspace_roots: &HashSet<lsp_types::Url>,
    current_file: &lsp_types::Url,
) -> PathServerResult<Vec<lsp_types::CompletionItem>> {
    // 1. separate prefix into finished and remains
    let (base_dir, partial_name) = separate_prefix(&prefix);
    debug(format!(
        "Detected base_dir: '{}', partial_name: '{}'",
        base_dir, partial_name
    ))
    .await;
    // manual unfold "~"
    let base_dir = if base_dir.starts_with("~/") {
        let home = std::env::var("HOME").map_err(|e| {
            PathServerError::Unknown(format!("Failed to get HOME environment variable: {}", e))
        })?;
        format!("{}{}", home, &base_dir[1..])
    } else {
        base_dir
    };
    let base_dir = PathBuf::from(base_dir);

    // 2. fs access
    let mut completions: Vec<lsp_types::CompletionItem> = vec![];
    if base_dir.is_absolute() {
        // a. absolute path
        let absolute_completions = complete_absolute(&base_dir, &partial_name).await?;
        completions.extend(absolute_completions);
    } else if base_dir.is_relative() {
        // b. relative path
        // base on workspace roots
        for root in workspace_roots.iter() {
            let root_path = url_to_path(root)?;
            let rel_workspace_completions =
                complete_relative(&base_dir, &partial_name, &root_path).await?;
            completions.extend(rel_workspace_completions);
        }
        // base on current file url
        if let Ok(file_path) = url_to_path(current_file) {
            let Some(parent) = file_path.parent() else {
                return Err(PathServerError::Unknown(format!(
                    "Failed to get parent directory of current file: {}",
                    current_file
                )));
            };
            let rel_current_file_completions =
                complete_relative(&base_dir, &partial_name, &parent).await?;
            completions.extend(rel_current_file_completions);
        }
    } else {
        assert!(false, "Unreachable!");
    };
    return Ok(completions);
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

fn url_to_path(url: &lsp_types::Url) -> PathServerResult<PathBuf> {
    if url.scheme() != "file" {
        return Err(PathServerError::Unsupported(format!(
            "Non-local url is not supported: {}",
            url
        )));
    }
    url.to_file_path().map_err(|_| {
        PathServerError::Unknown(format!("Failed to convert URL to file path: {}", url))
    })
}

async fn complete_absolute(
    base_dir: &PathBuf,
    partial_name: &str,
) -> PathServerResult<Vec<lsp_types::CompletionItem>> {
    let mut completions: Vec<lsp_types::CompletionItem> = vec![];
    if !base_dir.exists() {
        debug(format!(
            "Base directory does not exist: {}",
            base_dir.display()
        ))
        .await;
        return Ok(vec![]);
    }
    if !base_dir.is_dir() {
        debug(format!(
            "Base directory is not a directory: {}",
            base_dir.display()
        ))
        .await;
        return Ok(vec![]);
    }
    let files = base_dir.read_dir()?;
    for file in files {
        let file = file?;
        let filename = file.file_name().into_string().map_err(|os_str| {
            PathServerError::EncodingError(format!(
                "Failed to convert file name to string: {}",
                os_str.to_string_lossy()
            ))
        })?;
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
    return Ok(completions);
}

async fn complete_relative(
    base_dir: &PathBuf,
    partial_name: &str,
    root: &Path,
) -> PathServerResult<Vec<lsp_types::CompletionItem>> {
    let mut completions: Vec<lsp_types::CompletionItem> = vec![];
    let dir = root.join(&base_dir);
    if !dir.exists() {
        debug(format!("Base directory does not exist: {}", dir.display())).await;
        return Ok(vec![]);
    }
    if !dir.is_dir() {
        debug(format!(
            "Base directory is not a directory: {}",
            dir.display()
        ))
        .await;
        return Ok(vec![]);
    }
    let files = dir.read_dir()?;
    for file in files {
        let file = file?;
        let filename = file.file_name().into_string().map_err(|os_str| {
            PathServerError::EncodingError(format!(
                "Failed to convert file name to string: {}",
                os_str.to_string_lossy()
            ))
        })?;
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
    return Ok(completions);
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
