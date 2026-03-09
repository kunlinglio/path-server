//! Parsers for document path parsing.
use std::vec::Vec;

use crate::document::Document;

use super::PathCandidate;
use super::extractor::extract_string;

pub fn parse_document(document: &Document) -> Vec<Vec<PathCandidate>> {
    extract_string(document)
        .into_iter()
        .map(extract_paths_from_string)
        .collect()
}

/// Try to extract paths from a string token,
/// return candidates, from high priority to low priority
fn extract_paths_from_string(path_ref: PathCandidate) -> Vec<PathCandidate> {
    let mut results = Vec::new();
    let content = &path_ref.content;

    // 1. 如果整个字符串包含路径分隔符，直接作为最高优先级候选
    if content.contains('/') || content.contains('\\') {
        results.push(path_ref.clone());
    }

    // 2. try to trim quote
    results.push(path_ref.clone().slice(1, path_ref.len() - 1));

    // 2. 将内部按空格或特定符号拆分，进一步提取子路径并作为次备选
    // 这里可以结合你之前 inline.rs 的逻辑
    if let Some(pos) = content.rfind(' ') {
        let sub_content = &content[pos + 1..];
        if sub_content.contains('/') || sub_content.contains('\\') {
            results.push(PathCandidate {
                content: sub_content.to_string(),
                start_byte: path_ref.start_byte + pos + 1,
                end_byte: path_ref.end_byte,
            });
        }
    }

    results
}
