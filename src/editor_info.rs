use std::fmt::Display;

use tower_lsp_server::ls_types;

use strum_macros::{Display, EnumString};

#[derive(EnumString, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
pub enum Editor {
    Zed,
    VSCode,
    #[strum(default)]
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct EditorInfo {
    pub editor: Editor,
    pub support_document_link: bool,
}

impl Display for EditorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n    editor: {}\n    support_document_link: {}",
            self.editor, self.support_document_link
        )
    }
}

impl EditorInfo {
    pub fn from_initialize_params(params: &ls_types::InitializeParams) -> EditorInfo {
        let editor = params
            .initialization_options
            .as_ref()
            .and_then(|options| options.get("editor"))
            .and_then(|editor| editor.as_str())
            .map(Editor::from)
            .unwrap_or_else(|| Editor::Unknown("unknown".into()));
        let support_document_link = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.document_link.clone())
            .is_some();
        EditorInfo {
            editor,
            support_document_link,
        }
    }
}
