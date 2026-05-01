//! **Path Server** is an LSP server for path completion.
//!
//! **⚠️ WARNING: Internal API**
//!
//! This crate is primarily designed to be distributed as a standalone binary. And has no intention to maintain as a library dependency for other projects for now.

mod client;
mod config;
mod document;
mod error;
mod fs;
mod logger;
mod parser;
mod providers;
mod resolver;
mod server;
#[doc(hidden)]
pub use crate::server::PathServer;
#[doc(hidden)]
pub use config::{Completion, Config, Highlight};
