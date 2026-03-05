use std::convert::TryFrom;

use serde::Deserialize;
use tower_lsp::lsp_types;

use crate::logger::*;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Max results showed in completion, 0 indicate no limit
    pub max_results: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self { max_results: 0 }
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
