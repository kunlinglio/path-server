#![allow(dead_code)]
use std::sync::OnceLock;

use tower_lsp::lsp_types;

static LSP_CLIENT: OnceLock<tower_lsp::Client> = OnceLock::new();

pub fn init(client: &tower_lsp::Client) {
    let _ = LSP_CLIENT.set(client.clone()); // ignore multi init error
}

pub async fn __debug(message: String) {
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

pub async fn __info(message: String) {
    if cfg!(test) {
        eprintln!("[INFO] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(lsp_types::MessageType::INFO, message)
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

pub async fn __warn(message: String) {
    if cfg!(test) {
        eprintln!("[WARN] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(lsp_types::MessageType::WARNING, message)
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

pub async fn __error(message: String) {
    if cfg!(test) {
        eprintln!("[ERROR] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(lsp_types::MessageType::ERROR, message)
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

pub fn __type_name_of<T>(_: T) -> &'static str {
    std::any::type_name::<T>()
}

#[macro_export]
macro_rules! __function_name {
    () => {{
        fn f() {}
        $crate::logger::__type_name_of(f)
            .rsplit("::")
            .find(|&part| part != "f" && part != "{{closure}}")
            .expect("Short function name")
    }};
}

#[macro_export]
macro_rules! lsp_debug {
    ($($arg:tt)*) => {
        {
            let fn_name = $crate::__function_name!();
            $crate::logger::__debug(format!("{}() {} ({}:{})", fn_name, format!($($arg)*), file!(), line!()))
        }
    };
}

#[macro_export]
macro_rules! lsp_info {
    ($($arg:tt)*) => {
        $crate::logger::__info(format!($($arg)*)) // do not print extra information for clarity
    };
}

#[macro_export]
macro_rules! lsp_warn {
    ($($arg:tt)*) => {
        {
            let fn_name = $crate::__function_name!();
            $crate::logger::__warn(format!("{}() {} ({}:{})", fn_name, format!($($arg)*), file!(), line!()))
        }
    };
}

#[macro_export]
macro_rules! lsp_error {
    ($($arg:tt)*) => {
        {
            let fn_name = $crate::__function_name!();
            $crate::logger::__error(format!("{}() {} ({}:{})", fn_name, format!($($arg)*), file!(), line!()))
        }
    };
}

#[macro_export]
macro_rules! to_sync {
    ($log_future:expr) => {
        tokio::spawn($log_future);
    };
}
