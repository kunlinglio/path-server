use std::collections::HashSet;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use tower_lsp::lsp_types;

use crate::common::*;
use crate::config;
use crate::logger::*;
use crate::parser;
use crate::utils;

/// The wrapper struct inside this module to store additional information.
struct CompletionItemInner {
    completion: lsp_types::CompletionItem,
    full_path: PathBuf,
}

pub async fn complete(
    prefix: &str,
    workspace_roots: &HashSet<PathBuf>,
    current_file: &Path,
    completion_config: &config::Completion,
) -> PathServerResult<Vec<lsp_types::CompletionItem>> {
    let (base_dir, partial_name) = parser::separate_prefix(prefix);
    debug(format!(
        "Detected base_dir: '{}', partial_name: '{}'",
        base_dir, partial_name
    ))
    .await;
    let base_dir = expand_tilde(&base_dir)?;
    let base_dir = PathBuf::from(base_dir);

    let mut completions: Vec<CompletionItemInner> = vec![];
    if base_dir.is_absolute() {
        // absolute path
        let absolute_completions = complete_absolute(
            &base_dir,
            &partial_name,
            completion_config.show_hidden_files,
        )
        .await?;
        completions.extend(absolute_completions);
    } else if base_dir.is_relative() {
        // relative path
        let workspace_folders = workspace_roots
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let parent = current_file
            .parent()
            .map(|p| p.to_string_lossy().into_owned());
        let home = std::env::var("HOME").ok();
        let base_paths = completion_config.iter_base_path(&workspace_folders, &parent, &home);
        let mut rel_completions = vec![];
        for base_path in base_paths {
            let completions = complete_relative(
                &base_dir,
                &partial_name,
                &base_path,
                completion_config.show_hidden_files,
            )
            .await?;
            rel_completions.extend(completions);
        }
        completions.extend(rel_completions);
    } else {
        unreachable!()
    };
    Ok(filter(
        completions,
        completion_config.max_results,
        &completion_config.exclude,
    )
    .await)
}

/// Expand "~" to the user's home directory
fn expand_tilde(path: &str) -> PathServerResult<String> {
    let path = if path.starts_with("~/") {
        let home = std::env::var("HOME").map_err(|e| {
            PathServerError::Unknown(format!("Failed to get HOME environment variable: {}", e))
        })?;
        format!("{}{}", home, &path[1..])
    } else {
        path.to_string()
    };
    Ok(path)
}

/// Filter duplicated and ignored completions
async fn filter(
    completions: Vec<CompletionItemInner>,
    max_completions: usize,
    exclude_patterns: &[String],
) -> Vec<lsp_types::CompletionItem> {
    let mut builder = GlobSetBuilder::new();
    for pattern in exclude_patterns {
        let Ok(glob) = Glob::new(pattern) else {
            info(format!(
                "Invalid glob pattern in config.completion.exclude: {}, ignoring",
                pattern
            ))
            .await;
            continue;
        };
        builder.add(glob);
    }
    let exclude_set = match builder.build() {
        Ok(set) => set,
        Err(e) => {
            info(format!(
                "Failed to build exclude set: {}, ignoring exclusions",
                e
            ))
            .await;
            GlobSet::new(vec![Glob::new("").unwrap()]).unwrap()
        }
    };

    let mut seen_labels: HashSet<String> = HashSet::new();
    let max_completions = if max_completions == 0 {
        usize::MAX
    } else {
        max_completions
    };

    completions
        .into_iter()
        .filter(|item| seen_labels.insert(item.completion.label.clone()))
        .filter(|item| !exclude_set.is_match(&item.full_path))
        .take(max_completions)
        .map(|item| item.completion)
        .collect()
}

async fn complete_absolute(
    base_dir: &Path,
    partial_name: &str,
    show_hidden_files: bool,
) -> PathServerResult<Vec<CompletionItemInner>> {
    let mut completions = vec![];
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
        if !filename.starts_with(partial_name) {
            continue;
        }
        if !show_hidden_files && utils::is_hidden_file(&file.path())? {
            continue;
        }
        if file.path().is_dir() {
            completions.push(CompletionItemInner {
                completion: lsp_types::CompletionItem {
                    label: filename,
                    kind: Some(lsp_types::CompletionItemKind::FOLDER),
                    ..Default::default()
                },
                full_path: file.path(),
            });
        } else {
            completions.push(CompletionItemInner {
                completion: lsp_types::CompletionItem {
                    label: filename,
                    kind: Some(lsp_types::CompletionItemKind::FILE),
                    ..Default::default()
                },
                full_path: file.path(),
            });
        }
    }
    Ok(completions)
}

async fn complete_relative(
    base_dir: &Path,
    partial_name: &str,
    root: &Path,
    show_hidden_files: bool,
) -> PathServerResult<Vec<CompletionItemInner>> {
    let mut completions = vec![];
    let dir = root.join(base_dir);
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
        if !filename.starts_with(partial_name) {
            continue;
        }
        if !show_hidden_files && utils::is_hidden_file(&file.path())? {
            continue;
        }
        if file.path().is_dir() {
            completions.push(CompletionItemInner {
                completion: lsp_types::CompletionItem {
                    label: filename.clone(),
                    kind: Some(lsp_types::CompletionItemKind::FOLDER),
                    ..Default::default()
                },
                full_path: file.path(),
            });
        } else {
            completions.push(CompletionItemInner {
                completion: lsp_types::CompletionItem {
                    label: filename.clone(),
                    kind: Some(lsp_types::CompletionItemKind::FILE),
                    ..Default::default()
                },
                full_path: file.path(),
            });
        }
    }
    Ok(completions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_complete() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // workspace + files
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::File::create(root.join("data").join("a.txt")).unwrap();
        std::fs::File::create(root.join("data").join("b.log")).unwrap();

        let mut roots = HashSet::new();
        roots.insert(root.to_path_buf());
        let current_file = root.join("src").join("main.rs");
        std::fs::create_dir_all(current_file.parent().unwrap()).unwrap();
        std::fs::File::create(&current_file).unwrap();

        let completion_config = crate::config::Completion {
            max_results: 1,
            show_hidden_files: true,
            exclude: vec!["*.log".into()],
            base_path: vec!["${workspaceFolder}".into()],
        };

        let items = complete("./data/a", &roots, &current_file, &completion_config)
            .await
            .unwrap();

        // only "a.txt": "b.log" is excluded and max_results = 1
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "a.txt");
    }

    #[tokio::test]
    async fn test_filter() {
        // test filter duplicate and exclude
        let items = vec![
            CompletionItemInner {
                completion: lsp_types::CompletionItem {
                    label: "keep.txt".into(),
                    ..Default::default()
                },
                full_path: std::path::PathBuf::from("/some/path/to/keep.txt"),
            },
            CompletionItemInner {
                completion: lsp_types::CompletionItem {
                    label: "ignore.log".into(),
                    ..Default::default()
                },
                full_path: std::path::PathBuf::from("/some/path/to/ignore.log"),
            },
            CompletionItemInner {
                completion: lsp_types::CompletionItem {
                    label: "keep.txt".into(),
                    ..Default::default()
                },
                full_path: std::path::PathBuf::from("/some/path/to/keep.txt"),
            }, // duplicate
        ];
        let filtered = filter(items, 0, &vec!["*.log".into()]).await;
        // should drop ".log" and deduplicate
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].label, "keep.txt");

        // test cap at max results
        let items = vec![
            lsp_types::CompletionItem {
                label: "1.txt".into(),
                ..Default::default()
            },
            lsp_types::CompletionItem {
                label: "2.log".into(),
                ..Default::default()
            },
            lsp_types::CompletionItem {
                label: "3.txt".into(),
                ..Default::default()
            }, // duplicate
        ]
        .iter()
        .map(|completion| CompletionItemInner {
            completion: completion.clone(),
            full_path: std::path::PathBuf::new(),
        })
        .collect();
        let filtered = filter(items, 1, &vec![]).await;
        // should cap at 1
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].label, "1.txt");
    }

    #[tokio::test]
    async fn test_expand_tilde() {
        // test with HOME env
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            env::set_var("HOME", dir.path());
        }

        let result = expand_tilde("~/projects").unwrap();
        assert_eq!(result, format!("{}/projects", dir.path().display()));
        let result = expand_tilde("/path/without/tilde");
        assert_eq!(result.unwrap(), "/path/without/tilde".to_string());

        // test without HOME env
        unsafe {
            env::remove_var("HOME");
        }
        let result = expand_tilde("~/projects");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_complete_absolute() {
        // prepare a temporary directory structure
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        // files and dirs
        std::fs::create_dir(base.join("app_dir")).unwrap();
        std::fs::File::create(base.join("apple.txt")).unwrap();
        std::fs::File::create(base.join("banana.txt")).unwrap();

        // complete_absolute with partial "app"
        let abs_results = complete_absolute(&base.to_path_buf(), "app", true)
            .await
            .unwrap();
        let labels: Vec<String> = abs_results
            .into_iter()
            .map(|c| c.completion.label)
            .collect();
        assert!(labels.contains(&"apple.txt".to_string()));
        assert!(labels.contains(&"app_dir".to_string()));
    }

    #[tokio::test]
    async fn test_complete_absolute_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        std::fs::create_dir(base.join("a_dir")).unwrap();
        std::fs::File::create(base.join("a_dir").join("visible_file.txt")).unwrap();
        std::fs::File::create(base.join("a_dir").join("hidden_file.txt")).unwrap();
        hf::hide(base.join("a_dir").join("hidden_file.txt")).unwrap();
        let hidden_filepath = {
            #[cfg(unix)]
            {
                hf::unix::hidden_file_name(base.join("a_dir").join("hidden_file.txt"))
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            }
            #[cfg(not(unix))]
            {
                "hidden_file.txt".to_string()
            }
        };
        let hidden_filename = std::path::PathBuf::from(&hidden_filepath)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(base.join("a_dir").join(&hidden_filepath).exists());
        assert!(hf::is_hidden(base.join("a_dir").join(&hidden_filepath)).unwrap());

        // complete without showing hidden files
        let abs_results = complete_absolute(&base.to_path_buf().join("a_dir"), "", false)
            .await
            .unwrap();
        let labels: Vec<String> = abs_results
            .into_iter()
            .map(|c| c.completion.label)
            .collect();
        assert!(labels.contains(&"visible_file.txt".to_string()));
        assert!(!labels.contains(&hidden_filepath));

        // complete with showing hidden files
        let abs_results = complete_absolute(&base.to_path_buf().join("a_dir"), "", true)
            .await
            .unwrap();
        let labels: Vec<String> = abs_results
            .into_iter()
            .map(|c| c.completion.label)
            .collect();
        assert!(labels.contains(&"visible_file.txt".to_string()));
        assert!(labels.contains(&hidden_filename));
    }

    #[tokio::test]
    async fn test_complete_relative() {
        // prepare workspace root with a subdir
        let ws = tempfile::tempdir().unwrap();
        let root = ws.path();
        std::fs::create_dir(root.join("subdir")).unwrap();
        std::fs::File::create(root.join("subdir").join("part.txt")).unwrap();
        std::fs::create_dir(root.join("subdir").join("parcel")).unwrap();

        // complete_relative for base_dir "subdir/" and partial "par"
        let rel_results = complete_relative(&PathBuf::from("subdir/"), "par", root, true)
            .await
            .unwrap();
        let mut found_file = false;
        let mut found_dir = false;
        for item in rel_results {
            if item.completion.label == "part.txt" {
                assert_eq!(item.completion.label, "part.txt".to_string());
                found_file = true;
            }
            if item.completion.label == "parcel" {
                assert_eq!(item.completion.label, "parcel".to_string());
                found_dir = true;
            }
        }
        assert!(found_file && found_dir);
    }

    #[tokio::test]
    async fn test_complete_relative_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        std::fs::create_dir(base.join("a_dir")).unwrap();
        std::fs::File::create(base.join("a_dir").join("visible_file.txt")).unwrap();
        std::fs::File::create(base.join("a_dir").join("hidden_file.txt")).unwrap();
        hf::hide(base.join("a_dir").join("hidden_file.txt")).unwrap();
        let hidden_filepath = {
            #[cfg(unix)]
            {
                hf::unix::hidden_file_name(base.join("a_dir").join("hidden_file.txt"))
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            }
            #[cfg(not(unix))]
            {
                "hidden_file.txt".to_string()
            }
        };
        let hidden_filename = std::path::PathBuf::from(&hidden_filepath)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(base.join("a_dir").join(&hidden_filepath).exists());
        assert!(hf::is_hidden(base.join("a_dir").join(&hidden_filepath)).unwrap());

        // complete without showing hidden files
        let abs_results = complete_relative(&PathBuf::from("./a_dir"), "", base, false)
            .await
            .unwrap();
        let labels: Vec<String> = abs_results
            .into_iter()
            .map(|c| c.completion.label)
            .collect();
        assert!(labels.contains(&"visible_file.txt".to_string()));
        assert!(!labels.contains(&hidden_filepath));

        // complete with showing hidden files
        let abs_results = complete_relative(&PathBuf::from("./a_dir"), "", base, true)
            .await
            .unwrap();
        let labels: Vec<String> = abs_results
            .into_iter()
            .map(|c| c.completion.label)
            .collect();
        assert!(labels.contains(&"visible_file.txt".to_string()));
        assert!(labels.contains(&hidden_filename));
    }
}
