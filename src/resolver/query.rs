use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::Config;
use crate::document::Document;
use crate::error::*;

use super::ResolvedPath;
use super::resolve_all;

pub async fn resolve_at_pos(
    document: &Document,
    config: &Config,
    workspace_roots: &HashSet<PathBuf>,
    doc_path: &Path,
    cursor: (usize, usize),
) -> PathServerResult<Option<ResolvedPath>> {
    let tokens = resolve_all(document, config, workspace_roots, doc_path).await?;

    let current_token: Vec<&ResolvedPath> = tokens
        .iter()
        .filter(|token| cursor_inside(cursor.0, cursor.1, token))
        .collect();

    if current_token.is_empty() {
        return Ok(None);
    }

    if current_token.len() != 1 {
        unreachable!("Expected exactly one token, found {}", current_token.len());
    }

    let current_token = current_token[0];
    Ok(Some(current_token.clone()))
}

fn cursor_inside(cursor_line: usize, cursor_character: usize, token: &ResolvedPath) -> bool {
    let (start_line, start_character) = token.start;
    let (end_line, end_character) = token.end;
    if cursor_line < start_line || cursor_line > end_line {
        // quick path: cursor do not in the token lines
        return false;
    };
    if start_line == end_line {
        // single line token, most frequent case
        return start_line == cursor_line
            && cursor_character >= start_character
            && cursor_character < end_character;
    };
    // multi-line token
    if cursor_line == start_line {
        return cursor_character >= start_character;
    }
    if cursor_line == end_line {
        return cursor_character < end_character;
    }
    true
}
