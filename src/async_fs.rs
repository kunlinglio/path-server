//! Async concurrent file system access api wrapper

use std::path::Path;
use tokio::fs;

use crate::common::PathServerResult;

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
