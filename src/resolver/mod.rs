mod collect;
mod query;

pub use collect::{ResolvedPath, ResolvedPathCache, resolve_all};
pub use query::resolve_at_pos;
