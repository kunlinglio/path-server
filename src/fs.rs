//! Async concurrent file system access api wrapper

use std::path::{Path, PathBuf};
use tokio::fs;
use tower_lsp::lsp_types;

use crate::error::*;

/// Check if a path exists
pub async fn exists(path: impl AsRef<Path>) -> bool {
    fs::try_exists(path).await.unwrap_or(false)
}

/// Check if dir
pub async fn is_dir(path: impl AsRef<Path>) -> bool {
    if let Ok(metadata) = fs::metadata(path).await {
        metadata.is_dir()
    } else {
        false
    }
}

/// Return entries for a dir
pub async fn read_dir(path: impl AsRef<Path>) -> PathServerResult<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path).await?;
    let mut files = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        files.push(entry);
    }

    Ok(files)
}

pub fn url_to_path(url: &lsp_types::Url) -> PathServerResult<PathBuf> {
    if url.scheme() != "file" {
        return Err(PathServerError::Unsupported(format!(
            "Non-local url is not supported: {}",
            url
        )));
    }
    url.to_file_path().map_err(|_| {
        PathServerError::InvalidPath(format!("Failed to convert URL to file path: {}", url))
    })
}

pub fn is_hidden_file(path: &Path) -> PathServerResult<bool> {
    let Some(is_unix_hidden) = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
    else {
        return Err(PathServerError::InvalidPath(format!(
            "{} do not contained file name, cannot check hidden or not",
            path.display()
        )));
    };
    if is_unix_hidden {
        return Ok(true);
    }
    #[cfg(windows)]
    {
        if hf::is_hidden(path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

// pub fn path_to_url(path: &PathBuf) -> PathServerResult<lsp_types::Url> {
//     lsp_types::Url::from_file_path(path).map_err(|_| {
//         PathServerError::InvalidPath(format!(
//             "Failed to convert file path to URL: {}",
//             path.display()
//         ))
//     })
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_to_path() {
        // valid file url
        #[cfg(not(windows))]
        let url_str = "file:///tmp";
        #[cfg(windows)]
        let url_str = "file:///C:/tmp";
        let url = lsp_types::Url::parse(url_str).unwrap();
        let path = url_to_path(&url).unwrap();
        assert!(path.ends_with("tmp"));

        // non-file scheme should error
        let url = lsp_types::Url::parse("http://example.com").unwrap();
        let err = url_to_path(&url).unwrap_err();
        match err {
            PathServerError::Unsupported(_) => {}
            _ => assert!(false, "expected Unsupported error, got: {}", err),
        }
    }
}
