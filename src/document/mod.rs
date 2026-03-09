mod language;
pub use language::Language;

use line_index::{LineIndex, TextSize, WideEncoding, WideLineCol};
use tower_lsp::lsp_types;
use tree_sitter::Tree;

use crate::error::*;
use crate::parser::{new_tree, update_tree};

#[derive(Debug, Clone)]
pub struct Document {
    /// Raw text
    pub text: String,
    /// Language if from lsp client
    pub language: Language,
    /// Index for line/column -> offset calculations
    index: LineIndex,
    /// Tree-sitter AST tree for incremental parsing
    tree: Option<Tree>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            text: String::new(),
            index: LineIndex::new(""),
            language: Language::Unknown("".into()),
            tree: None,
        }
    }
}

impl Document {
    pub fn new(text: String, language_id: &str) -> PathServerResult<Self> {
        let mut doc = Self {
            text: text.clone(),
            index: LineIndex::new(&text),
            language: Language::from_id(language_id),
            tree: None,
        };
        doc.tree = new_tree(&doc)?;
        Ok(doc)
    }

    pub fn apply_change(
        &mut self,
        change: &lsp_types::TextDocumentContentChangeEvent,
    ) -> PathServerResult<()> {
        // TODO: optimize performance by lazy refresh index and tree
        if change.range.is_none() {
            *self = Self::new(change.text.clone(), &self.language.to_string())?;
            return Ok(());
        }
        let range = change.range.as_ref().unwrap();
        let start_byte =
            self.utf16_pos_to_offset(range.start.line as usize, range.start.character as usize)?;
        let old_end_byte =
            self.utf16_pos_to_offset(range.end.line as usize, range.end.character as usize)?;
        let new_end_byte = start_byte + change.text.len();
        let mut old_document = std::mem::take(self);

        // construct new self with updated text and index, but old tree
        let mut new_text = std::mem::take(&mut old_document.text);
        new_text.replace_range(start_byte..old_end_byte, &change.text);
        let new_index = LineIndex::new(&new_text);
        *self = Self {
            text: new_text,
            index: new_index,
            tree: None,
            language: old_document.language.clone(),
        };

        // update tree
        let old_tree = old_document.tree.take();
        self.tree = update_tree(
            &old_document,
            old_tree,
            self,
            start_byte,
            old_end_byte,
            new_end_byte,
        )?;
        Ok(())
    }

    pub fn get_line(
        &self,
        line_number: usize,
        end_char: Option<usize>,
    ) -> PathServerResult<String> {
        let line_start = self.utf16_pos_to_offset(line_number, 0)?;
        let line_end = if let Some(end_char) = end_char {
            self.utf16_pos_to_offset(line_number, end_char)?
        } else {
            self.utf16_pos_to_offset(line_number + 1, 0)?
        };

        Ok(self.text[line_start..line_end].to_string())
    }

    pub fn offset_to_utf16_pos(&self, offset: usize) -> PathServerResult<(usize, usize)> {
        offset_to_utf16_position(&self.index, offset)
    }

    pub fn utf16_pos_to_offset(&self, line: usize, character: usize) -> PathServerResult<usize> {
        utf16_position_to_offset(&self.index, line, character)
    }

    pub fn offset_to_utf8_pos(&self, offset: usize) -> PathServerResult<(usize, usize)> {
        offset_to_utf8_position(&self.index, offset)
    }

    pub fn get_tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }
}

/// Convert UTF-16 line/column to byte offset
/// - "column" in (line, column) is the the "utf-16 code unit" offset, in which a emoji/Chinese character may span 2 units.
/// - (line, column) in UTF-8 is the all "byte offset" based
fn utf16_position_to_offset(
    index: &LineIndex,
    line: usize,
    character: usize,
) -> PathServerResult<usize> {
    let wide_line_col = WideLineCol {
        line: line as u32,
        col: character as u32,
    };
    // convert from "code unit based" to "byte based"
    let Some(line_col) = index.to_utf8(WideEncoding::Utf16, wide_line_col) else {
        return Err(PathServerError::EncodingError(format!(
            "Failed to convert wide line/column to UTF-8 for line {}, column {}",
            line, character
        )));
    };
    // calculate offset: offset = starts[line] + col
    let Some(char_offset) = index.offset(line_col) else {
        return Err(PathServerError::EncodingError(format!(
            "Failed to calculate character offset for line {}, column {}",
            line, character
        )));
    };
    Ok(char_offset.into())
}

fn offset_to_utf16_position(index: &LineIndex, offset: usize) -> PathServerResult<(usize, usize)> {
    let text_offset = TextSize::new(offset as u32);
    let line_col = index.line_col(text_offset);
    let Some(wide_offset) = index.to_wide(WideEncoding::Utf16, line_col) else {
        return Err(PathServerError::EncodingError(format!(
            "Failed to convert offset to wide position for offset {}",
            offset
        )));
    };
    Ok((wide_offset.line as usize, wide_offset.col as usize))
}

fn offset_to_utf8_position(index: &LineIndex, offset: usize) -> PathServerResult<(usize, usize)> {
    let text_offset = TextSize::new(offset as u32);
    let line_col = index.line_col(text_offset);
    Ok((line_col.line as usize, line_col.col as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_to_offset_ascii() {
        let text = r#"Hello
World"#;
        let index = LineIndex::new(text);
        assert_eq!(utf16_position_to_offset(&index, 0, 0).unwrap(), 0);
        assert_eq!(utf16_position_to_offset(&index, 0, 5).unwrap(), 5);
        assert_eq!(utf16_position_to_offset(&index, 1, 0).unwrap(), 6);
        assert_eq!(utf16_position_to_offset(&index, 1, 5).unwrap(), 11);
    }

    #[test]
    fn test_position_to_offset_utf8() {
        let text = [
            "这是一个UTF-8字符测试。\n",
            "這是一個 UTF-8 字元測試。\n",
            "これはUTF-8文字のテストです。\n",
            "이것은 UTF-8 문자 테스트입니다。\n",
        ];
        let index = LineIndex::new(&text.concat());
        // test start of line
        assert_eq!(utf16_position_to_offset(&index, 0, 0).unwrap(), 0);
        assert_eq!(
            utf16_position_to_offset(&index, 1, 0).unwrap(),
            0 + text[0].len()
        );
        assert_eq!(
            utf16_position_to_offset(&index, 2, 0).unwrap(),
            0 + text[0].len() + text[1].len()
        );
        assert_eq!(
            utf16_position_to_offset(&index, 3, 0).unwrap(),
            0 + text[0].len() + text[1].len() + text[2].len()
        );
        assert_eq!(
            utf16_position_to_offset(&index, 4, 0).unwrap(),
            0 + text[0].len() + text[1].len() + text[2].len() + text[3].len()
        );
        // test middle of line
        assert_eq!(
            utf16_position_to_offset(&index, 0, 4).unwrap(),
            "这是一个".len()
        );
        assert_eq!(
            utf16_position_to_offset(&index, 1, 1).unwrap(),
            0 + text[0].len() + "這".len()
        );
        assert_eq!(
            utf16_position_to_offset(&index, 2, 10).unwrap(),
            0 + text[0].len() + text[1].len() + "これはUTF-8文字".len()
        );
        assert_eq!(
            utf16_position_to_offset(&index, 3, 20).unwrap(),
            0 + text[0].len()
                + text[1].len()
                + text[2].len()
                + "이것은 UTF-8 문자 테스트입니다。".len()
        );
    }

    #[test]
    fn test_get_line_utf8() {
        let text = [
            "第一行内容\n",
            "第二行-包含中文 and ASCII characters\n",
            "第三行结束\n",
        ];
        let doc = Document::new(text.concat(), &Language::plain_text.to_string()).unwrap();

        // get full lines
        assert_eq!(doc.get_line(0, None).unwrap(), text[0]);
        assert_eq!(doc.get_line(1, None).unwrap(), text[1]);
        assert_eq!(doc.get_line(2, None).unwrap(), text[2]);
        // get line with end
        assert_eq!(doc.get_line(0, Some(3)).unwrap(), "第一行");
        assert_eq!(
            doc.get_line(1, Some(18)).unwrap(),
            "第二行-包含中文 and ASCII"
        );
        assert_eq!(doc.get_line(2, Some(1)).unwrap(), "第");
    }

    #[test]
    fn test_apply_change_range() {
        let text = ["First line\n", "Second line: 包含中文\n", "Third line\n"];
        let mut doc = Document::new(text.concat(), &Language::plain_text.to_string()).unwrap();
        assert_eq!(doc.text, text.concat());

        // replace second line by range (line 1 start -> line 2 start)
        let change = lsp_types::TextDocumentContentChangeEvent {
            range: Some(lsp_types::Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 2,
                    character: 0,
                },
            }),
            range_length: None,
            text: "New second line: 也包含中文\n".to_string(),
        };

        doc.apply_change(&change).unwrap();
        assert_eq!(
            doc.get_line(1, None).unwrap(),
            "New second line: 也包含中文\n"
        );
    }
    #[test]
    fn test_apply_change_full() {
        let text = ["First line\n", "Second line: 包含中文\n", "Third line\n"];
        let mut doc = Document::new(text.concat(), &Language::plain_text.to_string()).unwrap();
        assert_eq!(doc.text, text.concat());
        // full document replace when range is None
        let full = lsp_types::TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "New beginning\nAnother line\n".to_string(),
        };
        doc.apply_change(&full).unwrap();
        assert_eq!(doc.text, "New beginning\nAnother line\n");
    }
}
