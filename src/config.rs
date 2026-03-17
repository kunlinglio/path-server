use std::convert::TryFrom;
use std::fmt::Display;
use std::path::PathBuf;

use config::{Config as ConfigLoader, File, FileFormat};
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types;

use crate::error::*;
use crate::lsp_error;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    /// Base paths for relative path completion/highlight/jump.
    /// Supports `${workspaceFolder}`, `${document}`, `${userHome}` as placeholders.
    /// The order determines the priority in suggestions.
    #[serde(alias = "basePath")]
    pub base_path: Vec<String>,

    pub completion: Completion,
    pub highlight: Highlight,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct Completion {
    /// Max results shown in completion; 0 indicates no limit.
    #[serde(alias = "maxResults")]
    pub max_results: usize,

    /// Whether to show hidden files in completion.
    #[serde(alias = "showHiddenFiles")]
    pub show_hidden_files: bool,

    /// List of paths to exclude from completion.
    /// Supports glob patterns.
    pub exclude: Vec<String>,

    /// Whether to automatically trigger next completion after selecting an item.
    #[serde(alias = "triggerNextCompletion")]
    pub trigger_next_completion: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct Highlight {
    /// Whether to highlight paths in the editor with underlines.
    pub enable: bool,

    /// Whether to highlight directory paths. (Jump behavior may vary by editor/OS).
    #[serde(alias = "highlightDirectory")]
    pub highlight_directory: bool,
}

impl Config {
    /// Iter all base path patterns in config and convert them into a vector.
    /// Return (path, original_pattern, order)
    pub fn base_paths(
        &self,
        workspace_folders: &[String],
        document_parent: Option<&String>,
        user_home: Option<&String>,
    ) -> Vec<(PathBuf, String, usize)> {
        self.base_path
            .iter()
            .enumerate()
            .filter_map(|(index, path)| {
                if path.contains("${workspaceFolder}") {
                    Some(
                        workspace_folders
                            .iter()
                            .map(|workspace_folder| {
                                let expanded = path.replace("${workspaceFolder}", workspace_folder);
                                (PathBuf::from(expanded), path.clone(), index)
                            })
                            .collect(),
                    )
                } else if path.contains("${document}") {
                    match document_parent {
                        Some(parent) => {
                            let expanded = path.replace("${document}", parent);
                            Some(vec![(PathBuf::from(expanded), path.clone(), index)])
                        }
                        None => None,
                    }
                } else if path.contains("${userHome}") {
                    match user_home {
                        Some(home) => {
                            let expanded = path.replace("${userHome}", home);
                            Some(vec![(PathBuf::from(expanded), path.clone(), index)])
                        }
                        None => None,
                    }
                } else {
                    Some(vec![(PathBuf::from(path), path.clone(), index)])
                }
            })
            .flatten()
            .collect()
    }

    pub fn signature(&self) -> PathServerResult<String> {
        let bytes = serde_json::to_vec(self).map_err(|e| {
            PathServerError::Unknown(format!(
                "Failed to calculate config signature, failed to serialize config: {}",
                e
            ))
        })?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_path: vec!["${document}".into(), "${workspaceFolder}".into()],
            completion: Completion {
                max_results: 0,
                show_hidden_files: true,
                exclude: vec![
                    "**/node_modules".into(),
                    "**/.git".into(),
                    "**/.DS_Store".into(),
                ],
                trigger_next_completion: true,
            },
            highlight: Highlight {
                enable: true,
                highlight_directory: true,
            },
        }
    }
}

impl TryFrom<serde_json::Value> for Config {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string_pretty(self)
                .unwrap_or_else(|_| "Failed to serialize config".into())
        )
    }
}

pub async fn get(client: &tower_lsp::Client) -> Config {
    let user_configs = client
        .configuration(vec![lsp_types::ConfigurationItem {
            scope_uri: None,
            section: Some("path-server".to_string()),
        }])
        .await;
    let Ok(user_configs) = user_configs else {
        lsp_error!(
            "Failed to get user configs: {}, use default config",
            user_configs.unwrap_err()
        )
        .await;
        return Config::default();
    };
    if user_configs.is_empty() || user_configs[0].is_null() {
        return Config::default();
    }

    let merge_res = merge_configs(Config::default(), user_configs);
    let Ok(config) = merge_res else {
        lsp_error!(
            "Failed to merge configs: {}, use default config",
            merge_res.unwrap_err()
        )
        .await;
        return Config::default();
    };
    config
}

fn merge_configs(default: Config, user: Vec<serde_json::Value>) -> PathServerResult<Config> {
    let mut builder = ConfigLoader::builder();
    let default_json = serde_json::to_string(&default).unwrap();
    builder = builder.add_source(File::from_str(&default_json, FileFormat::Json));

    let normalized_user = normalize_keys(user[0].clone());
    let user_json = normalized_user.to_string();
    builder = builder.add_source(File::from_str(&user_json, FileFormat::Json));

    match builder.build() {
        Ok(c) => match c.try_deserialize::<Config>() {
            Ok(config) => Ok(config),
            Err(e) => Err(PathServerError::UserConfigError(format!(
                "Failed to deserialize merged config: {}",
                e
            ))),
        },
        Err(e) => Err(PathServerError::UserConfigError(format!(
            "Failed to build config: {}",
            e
        ))),
    }
}

/// Convert input json's keys into snake case
fn normalize_keys(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                let snake_key = to_snake_case(&k);
                new_map.insert(snake_key, normalize_keys(v));
            }
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(normalize_keys).collect())
        }
        other => other,
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_config() {
        let default_json = r#"
        {
            "base_path": ["${document}", "${workspaceFolder}"],
            "completion": {
                "max_results": 0,
                "show_hidden_files": true,
                "exclude": ["**/node_modules", "**/.git", "**/.DS_Store"],
                "trigger_next_completion": true
            },
            "highlight": {
                "enable": true,
                "highlight_directory": true
            }
        }"#;
        let v: serde_json::Value = serde_json::from_str(default_json).unwrap();
        let default_cfg = Config::try_from(v).unwrap();
        assert_eq!(default_cfg, Config::default());
    }

    #[test]
    fn test_base_paths_expands_workspace_and_document() {
        let config = Config {
            base_path: vec![
                "${workspaceFolder}/src".into(),
                "${document}".into(),
                "/absolute/path".into(),
            ],
            completion: Completion {
                max_results: 0,
                show_hidden_files: true,
                exclude: vec![],
                trigger_next_completion: true,
            },
            highlight: Highlight {
                enable: true,
                highlight_directory: true,
            },
        };

        let workspace_folders = vec!["/ws1".to_string(), "/ws2".to_string()];
        let document_parent = Some(&"/ws1/project".to_string());
        let user_home = None;

        let result = config.base_paths(&workspace_folders, document_parent, user_home);

        let expected: Vec<PathBuf> = vec![
            "/ws1/src".into(),
            "/ws2/src".into(),
            "/ws1/project".into(),
            "/absolute/path".into(),
        ];

        assert_eq!(
            result
                .into_iter()
                .map(|(path, _, _)| path)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn test_base_paths_skips_missing_document_or_user_home() {
        let config = Config {
            base_path: vec!["${document}".into(), "${userHome}/foo".into()],
            completion: Completion {
                max_results: 0,
                show_hidden_files: true,
                exclude: vec![],
                trigger_next_completion: true,
            },
            highlight: Highlight {
                enable: true,
                highlight_directory: true,
            },
        };

        let workspace_folders = vec![];
        let document_parent = None;
        let user_home = Some(&"/home/user".to_string());

        let result = config.base_paths(&workspace_folders, document_parent, user_home);

        let expected: Vec<PathBuf> = vec!["/home/user/foo".into()];

        assert_eq!(
            result
                .into_iter()
                .map(|(path, _, _)| path)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn test_merge_highlight_partial() {
        let default = Config::default();
        let user_json = serde_json::json!({"highlight": {"enable": false}});
        let res = merge_configs(default.clone(), vec![user_json]);
        assert!(res.is_ok());
        let cfg = res.unwrap();
        assert_eq!(cfg.highlight.enable, false);
        assert_eq!(
            cfg.highlight.highlight_directory,
            default.highlight.highlight_directory
        );
        assert_eq!(cfg.completion, default.completion);
        assert_eq!(cfg.base_path, default.base_path);
    }

    #[test]
    fn test_merge_camel_case() {
        let default = Config::default();
        let user_json = serde_json::json!({"completion": {"maxResults": 10}});
        let res = merge_configs(default.clone(), vec![user_json]);
        assert!(res.is_ok());
        let cfg = res.unwrap();
        assert_eq!(cfg.completion.max_results, 10);
    }

    #[test]
    fn test_merge_partial_completion() {
        let default = Config::default();
        let user_json = serde_json::json!({
            "completion": {"max_results": 10, "exclude": ["/tmp"]}
        });
        let res = merge_configs(default.clone(), vec![user_json]);
        assert!(res.is_ok());
        let cfg = res.unwrap();
        assert_eq!(cfg.completion.max_results, 10);
        assert_eq!(cfg.completion.exclude, vec!["/tmp".to_string()]);
        assert_eq!(cfg.highlight, default.highlight);
        assert_eq!(
            cfg.completion.show_hidden_files,
            default.completion.show_hidden_files
        );
        assert_eq!(cfg.base_path, default.base_path);
    }

    #[test]
    fn test_merge_self() {
        let default = Config::default();
        let users = serde_json::to_value(&default).unwrap();
        let res = merge_configs(default.clone(), vec![users]);
        assert!(res.is_ok());
        let cfg = res.unwrap();
        assert_eq!(cfg, default);
    }
}
