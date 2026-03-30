mod collect;
mod query;

pub use collect::resolve_all;
pub use query::resolve_at_pos;

use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ResolvedPath {
    pub start: (usize, usize), // (line, character) in utf16
    pub end: (usize, usize),   // (line, character) in utf16
    pub target: PathBuf,
    pub is_dir: bool,
}

impl ResolvedPath {
    fn pos_compare(a: (usize, usize), b: (usize, usize)) -> bool {
        if a.0 == b.0 { a.1 < b.1 } else { a.0 < b.0 }
    }
    pub fn intersects(&self, other: &ResolvedPath) -> bool {
        Self::pos_compare(self.start, other.end) && Self::pos_compare(other.start, self.end)
    }
}

#[derive(Debug)]
pub struct ResolvedPathCache {
    tokens: Option<Arc<Vec<ResolvedPath>>>,
    config_signature: String,
}

impl ResolvedPathCache {
    pub fn new() -> Self {
        Self {
            tokens: None,
            config_signature: String::new(),
        }
    }
}
