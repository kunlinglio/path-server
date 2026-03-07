use tower_lsp::lsp_types;

use crate::common::*;
use crate::document::Document;
use crate::parser;

pub fn provide_document_links(doc: &Document) -> PathServerResult<Vec<lsp_types::DocumentLink>> {
    let strings = parser::parse_document(doc);
    let mut links = vec![];

    for s in strings {
        let start = doc.utf_16_pos(s.start_byte)?;
        let end = doc.utf_16_pos(s.end_byte)?;
        let range = lsp_types::Range::new(
            lsp_types::Position::new(start.0 as u32, start.1 as u32),
            lsp_types::Position::new(end.0 as u32, end.1 as u32),
        );

        links.push(lsp_types::DocumentLink {
            range,
            target: Some(lsp_types::Url::from_file_path("").unwrap()), // TODO: jump to actual url
            tooltip: Some("Follow path".into()),
            data: None,
        });
    }

    Ok(links)
}
