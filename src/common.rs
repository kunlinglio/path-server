use serde_json::Value;
use thiserror::Error;
use tower_lsp::jsonrpc;

pub type PathServerResult<T> = Result<T, PathServerError>;

#[derive(Debug, Error)]
pub enum PathServerError {
    // code 1000
    #[error("Encoding error: {0}")]
    EncodingError(String), // UTF-8/UTF-16 encoding/decoding error
    // code 1001
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    // code 1002
    #[error("Unsupported: {0}")]
    Unsupported(String),
    // code 2000
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<PathServerError> for tower_lsp::jsonrpc::Error {
    fn from(err: PathServerError) -> Self {
        match err {
            PathServerError::EncodingError(msg) => jsonrpc::Error {
                code: jsonrpc::ErrorCode::ServerError(1000),
                message: std::borrow::Cow::Borrowed("Encoding error"),
                data: Some(Value::String(msg)),
            },
            PathServerError::IoError(e) => jsonrpc::Error {
                code: jsonrpc::ErrorCode::ServerError(1001),
                message: std::borrow::Cow::Borrowed("IO error"),
                data: Some(Value::String(e.to_string())),
            },
            PathServerError::Unsupported(msg) => jsonrpc::Error {
                code: jsonrpc::ErrorCode::ServerError(1002),
                message: std::borrow::Cow::Borrowed("Unsupported"),
                data: Some(Value::String(msg)),
            },
            PathServerError::Unknown(msg) => jsonrpc::Error {
                code: jsonrpc::ErrorCode::ServerError(2000),
                message: std::borrow::Cow::Borrowed("Unknown error"),
                data: Some(Value::String(msg)),
            },
        }
    }
}
