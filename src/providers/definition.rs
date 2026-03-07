use tower_lsp::lsp_types;

use crate::common::*;
use crate::document::Document;
use crate::parser;

pub fn provide_definition(
    doc: &Document,
    line: usize,
    character: usize,
) -> PathServerResult<Option<lsp_types::GotoDefinitionResponse>> {
    // find if the cursor is within a string
    let strings = parser::parse_document(doc);
    let current_str = strings.into_iter().find(|s| {
        let start = doc.utf_16_pos(s.start_byte).unwrap_or((0, 0));
        let end = doc.utf_16_pos(s.end_byte).unwrap_or((0, 0));
        line == start.0 && character >= start.1 && character <= end.1
    });

    let Some(_) = current_str else {
        return Ok(None);
    };

    Ok(Some(lsp_types::GotoDefinitionResponse::Scalar(
        lsp_types::Location::new(
            lsp_types::Url::from_file_path("").unwrap(), // TODO: jump to actual url
            lsp_types::Range::default(),                 // TODO: support actual range
        ),
    )))
}
