//! Module to manage environment about client
use std::fmt::Display;
use std::sync::OnceLock;

use tokio::sync::RwLock;

use strum_macros::{Display, EnumString};

#[derive(EnumString, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
pub enum Editor {
    Zed,
    VSCode,
    #[strum(default)]
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct ClientMetadata {
    pub editor: Editor,
    pub support_document_link: bool,
}

impl Default for ClientMetadata {
    fn default() -> Self {
        Self {
            editor: Editor::Unknown("unknown".into()),
            support_document_link: true,
        }
    }
}

impl Display for ClientMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n    editor: {}\n    support_document_link: {}",
            self.editor, self.support_document_link
        )
    }
}

static ENV: OnceLock<RwLock<ClientMetadata>> = OnceLock::new();

pub async fn set_client(client: ClientMetadata) {
    let lock = ENV.get_or_init(|| RwLock::new(ClientMetadata::default()));
    let mut guard = lock.write().await;
    *guard = client;
}

pub async fn get_client() -> ClientMetadata {
    let lock = ENV.get_or_init(|| RwLock::new(ClientMetadata::default()));
    lock.read().await.clone()
}
