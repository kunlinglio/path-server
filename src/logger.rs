#![allow(dead_code)]
use std::sync::OnceLock;

use tower_lsp_server::ls_types;

static LSP_CLIENT: OnceLock<tower_lsp_server::Client> = OnceLock::new();

pub fn init(client: &tower_lsp_server::Client) {
    let _ = LSP_CLIENT.set(client.clone()); // ignore multi init error
}

#[doc(hidden)]
#[macro_export]
macro_rules! lsp_debug {
    ($($arg:tt)*) => {
        {
            #[cfg(debug_assertions)]
            {
                let fn_name = $crate::__function_name!();
                $crate::logger::__debug(format!(
                    "{}() {} ({}:{})",
                    fn_name,
                    format_args!($($arg)*),
                    file!(),
                    line!(),
                ))
            }
            #[cfg(not(debug_assertions))]
            {
                async { () }
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! lsp_info {
    ($($arg:tt)*) => {
        $crate::logger::__info(format!($($arg)*)) // do not print extra information for clarity
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! lsp_warn {
    ($($arg:tt)*) => {
        {
            let fn_name = $crate::__function_name!();
            $crate::logger::__warn(format!("{}() {} ({}:{})", fn_name, format!($($arg)*), file!(), line!()))
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! lsp_error {
    ($($arg:tt)*) => {
        {
            let fn_name = $crate::__function_name!();
            $crate::logger::__error(format!("{}() {} ({}:{})", fn_name, format!($($arg)*), file!(), line!()))
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! to_sync {
    ($log_future:expr) => {
        tokio::spawn($log_future);
    };
}

#[doc(hidden)]
pub async fn __debug(message: String) {
    if cfg!(test) {
        println!("[DEBUG] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(ls_types::MessageType::LOG, format!("[DEBUG] {}", message))
            .await;
    } else {
        panic!("Failed to log debug message: lopper is not initialized!")
    }
}

#[doc(hidden)]
pub async fn __info(message: String) {
    if cfg!(test) {
        eprintln!("[INFO] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(ls_types::MessageType::INFO, message)
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

#[doc(hidden)]
pub async fn __warn(message: String) {
    if cfg!(test) {
        eprintln!("[WARN] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(ls_types::MessageType::WARNING, message)
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

#[doc(hidden)]
pub async fn __error(message: String) {
    if cfg!(test) {
        eprintln!("[ERROR] {}", message);
        return;
    }
    if let Some(client) = LSP_CLIENT.get() {
        client
            .log_message(ls_types::MessageType::ERROR, message)
            .await;
    } else {
        panic!("Failed to log: lopper is not initialized!")
    }
}

#[doc(hidden)]
pub fn __type_name_of<T>(_: T) -> &'static str {
    std::any::type_name::<T>()
}

#[doc(hidden)]
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
