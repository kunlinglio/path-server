use std::convert::TryFrom;
use std::path::PathBuf;

use serde::Deserialize;
use tower_lsp::lsp_types;

use crate::logger::*;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub completion: Completion,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Completion {
    /// Max results shown in completion, 0 indicate no limit
    #[serde(alias = "maxResults")]
    pub max_results: usize,

    /// Whether to show hidden files in completion
    #[serde(alias = "showHiddenFiles")]
    pub show_hidden_files: bool,

    /// List of paths to exclude from completion
    /// Supports glob patterns
    pub exclude: Vec<String>,

    /// Base paths for relative path completion
    /// Supports `${workspaceFolder}`, `${document}`, `${userHome}` as placeholders
    #[serde(alias = "basePath")]
    pub base_path: Vec<String>,
}

impl Completion {
    pub fn iter_base_path(
        &self,
        workspace_folders: &[String],
        document_parent: &Option<String>,
        user_home: &Option<String>,
    ) -> Vec<PathBuf> {
        let mut expanded_paths = vec![];
        for path in &self.base_path {
            if path.contains("${workspaceFolder}") {
                for workspace_folder in workspace_folders {
                    let expanded = path.replace("${workspaceFolder}", workspace_folder);
                    expanded_paths.push(PathBuf::from(expanded));
                }
            } else if path.contains("${document}") {
                if document_parent.is_some() {
                    let expanded = path.replace("${document}", document_parent.as_deref().unwrap());
                    expanded_paths.push(PathBuf::from(expanded));
                }
            } else if path.contains("${userHome}") {
                if user_home.is_some() {
                    let expanded = path.replace("${userHome}", user_home.as_deref().unwrap());
                    expanded_paths.push(PathBuf::from(expanded));
                }
            } else {
                expanded_paths.push(PathBuf::from(path));
            }
        }
        expanded_paths
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            completion: Completion {
                max_results: 0,
                show_hidden_files: true,
                exclude: vec!["**/node_modules/**".into(), "**/.git/**".into()],
                base_path: vec!["${workspaceFolder}".into(), "${document}".into()],
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

pub async fn get(client: &tower_lsp::Client) -> Config {
    let configs = client
        .configuration(vec![lsp_types::ConfigurationItem {
            scope_uri: None,
            section: Some("path-server".to_string()),
        }])
        .await;
    let Ok(configs) = configs else {
        info(format!(
            "Failed to get configuration:{}, use default",
            configs.unwrap_err()
        ))
        .await;
        return Default::default();
    };
    assert!(configs.len() == 1);
    let Ok(config) = Config::try_from(configs[0].clone()) else {
        info(format!(
            "Failed to parse configuration:{}, use default",
            configs[0].clone()
        ))
        .await;
        return Default::default();
    };
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.completion.max_results, 0);
        assert!(cfg.completion.show_hidden_files);
        assert_eq!(
            cfg.completion.exclude,
            vec!["**/node_modules/**", "**/.git/**"]
        );
        assert_eq!(
            cfg.completion.base_path,
            vec!["${workspaceFolder}", "${document}"]
        );
    }

    #[test]
    fn test_iter_base_path_expands_workspace_and_document() {
        let completion = Completion {
            max_results: 0,
            show_hidden_files: true,
            exclude: vec![],
            base_path: vec![
                "${workspaceFolder}/src".into(),
                "${document}".into(),
                "/absolute/path".into(),
            ],
        };

        let workspace_folders = vec!["/ws1".to_string(), "/ws2".to_string()];
        let document_parent = Some("/ws1/project".to_string());
        let user_home = None;

        let result = completion.iter_base_path(&workspace_folders, &document_parent, &user_home);

        let expected: Vec<PathBuf> = vec![
            "/ws1/src".into(),
            "/ws2/src".into(),
            "/ws1/project".into(),
            "/absolute/path".into(),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_iter_base_path_skips_missing_document_or_user_home() {
        let completion = Completion {
            max_results: 0,
            show_hidden_files: true,
            exclude: vec![],
            base_path: vec!["${document}".into(), "${userHome}/foo".into()],
        };

        let workspace_folders = vec![];
        let document_parent = None;
        let user_home = Some("/home/user".to_string());

        let result = completion.iter_base_path(&workspace_folders, &document_parent, &user_home);

        let expected: Vec<PathBuf> = vec!["/home/user/foo".into()];

        assert_eq!(result, expected);
    }
}
