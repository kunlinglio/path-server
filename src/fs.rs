use std::fs;
use std::path::PathBuf;

use crate::logger::*;

// Abstraction over the filesystem.

pub fn exists(path: &PathBuf) -> bool {
    path.exists()
}

pub async fn ls(path: &PathBuf) -> Result<Vec<PathBuf>, std::io::Error> {
    if !path.exists() {
        error(format!("Path does not exist: {}", path.display())).await;
        return Ok(vec![]);
    };
    if !path.is_dir() {
        error(format!("Path is not a directory: {}", path.display())).await;
        return Ok(vec![]);
    };
    let mut entries = vec![];
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        entries.push(entry.path());
    }
    Ok(entries)
}
