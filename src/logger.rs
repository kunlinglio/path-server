#![allow(dead_code)]
use std::sync::OnceLock;

use tower_lsp::lsp_types;

static LSP_CLIENT: OnceLock<tower_lsp::Client> = OnceLock::new();

pub fn init(client: &tower_lsp::Client) {
    let _ = LSP_CLIENT.set(client.clone()); // ignore multi init error
}

pub async fn debug(message: String) {
    if !cfg!(debug_assertions) {
        return;
    }
    if cfg!(test) {
        println!("[DEBUG] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(lsp_types::MessageType::LOG, format!("[DEBUG] {}", message))
            .await;
    } else {
        panic!("Failed to log debug message: lopper is not initialized!")
    }
}

pub fn debug_sync(message: String) {
    tokio::spawn(async move {
        debug(message).await;
    });
}

pub async fn log(message: String) {
    if cfg!(test) {
        eprintln!("[LOG] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(lsp_types::MessageType::INFO, format!("[LOG] {}", message))
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

pub async fn warn(message: String) {
    if cfg!(test) {
        eprintln!("[WARN] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(
                lsp_types::MessageType::WARNING,
                format!("[WARN] {}", message),
            )
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

pub async fn error(message: String) {
    if cfg!(test) {
        eprintln!("[ERROR] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(
                lsp_types::MessageType::ERROR,
                format!("[ERROR] {}", message),
            )
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}
