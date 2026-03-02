#![allow(dead_code)]
use std::sync::OnceLock;

use tower_lsp::lsp_types;

static LSP_CLIENT: OnceLock<tower_lsp::Client> = OnceLock::new();

pub fn init(client: &tower_lsp::Client) {
    LSP_CLIENT.set(client.clone()).unwrap();
}

pub async fn log(message: String) {
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(lsp_types::MessageType::LOG, message)
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

pub async fn info(message: String) {
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(lsp_types::MessageType::INFO, message)
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

pub async fn error(message: String) {
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(lsp_types::MessageType::ERROR, message)
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}
