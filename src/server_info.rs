use std::fmt::Display;

use strum_macros::{Display, EnumString};

#[derive(EnumString, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
pub enum ServerMode {
    Debug,
    Release,
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    version: String,
    mode: ServerMode,
}

impl Display for ServerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n    version: {}\n    mode: {}",
            self.version, self.mode
        )
    }
}

impl ServerInfo {
    pub fn new() -> ServerInfo {
        let version = env!("CARGO_PKG_VERSION").to_string();
        let mode = if cfg!(debug_assertions) {
            ServerMode::Debug
        } else {
            ServerMode::Release
        };
        ServerInfo { version, mode }
    }
}
