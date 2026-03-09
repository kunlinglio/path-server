use line_index::{LineIndex, TextSize, WideEncoding, WideLineCol};
use tower_lsp::lsp_types;
use tree_sitter::Tree;

use crate::common::*;
use crate::languages::Language;
use crate::parser::update_tree;

#[derive(Debug, Clone)]
pub struct Document {
    /// Raw text
    pub text: String,
    /// Index for line/column -> offset calculations
    index: LineIndex,
    /// Language if from lsp client
    pub language: Language,
    /// Tree-sitter AST tree for incremental parsing
    pub tree: Option<Tree>,
}

impl Document {
    pub fn new(text: String, language_id: &str) -> PathServerResult<Self> {
        let mut doc = Self {
            text: text.clone(),
            index: LineIndex::new(&text),
            language: Language::from_id(language_id),
            tree: None,
        };
        doc.tree = update_tree(&doc)?;
        Ok(doc)
    }

    pub fn apply_change(
        &mut self,
        change: &lsp_types::TextDocumentContentChangeEvent,
    ) -> PathServerResult<()> {
        if change.range.is_none() {
            self.text = change.text.clone();
            self.tree = update_tree(self)?;
            return Ok(());
        }
        let range = change.range.as_ref().unwrap();
        let start = position_to_offset(
            &self.index,
            range.start.line as usize,
            range.start.character as usize,
        )?;
        let end = position_to_offset(
            &self.index,
            range.end.line as usize,
            range.end.character as usize,
        )?;

        self.text.replace_range(start..end, &change.text);
        self.index = LineIndex::new(&self.text);
        self.tree = update_tree(self)?;
        Ok(())
    }

    pub fn get_line(
        &self,
        line_number: usize,
        end_char: Option<usize>,
    ) -> PathServerResult<String> {
        let line_start = position_to_offset(&self.index, line_number, 0)?;
        let line_end = if let Some(end_char) = end_char {
            position_to_offset(&self.index, line_number, end_char)?
        } else {
            position_to_offset(&self.index, line_number + 1, 0).unwrap_or(self.text.len())
        };

        Ok(self.text[line_start..line_end].to_string())
    }

    pub fn offset_to_utf16_pos(&self, offset: usize) -> PathServerResult<(usize, usize)> {
        offset_to_position(&self.index, offset)
    }

    pub fn utf16_pos_to_offset(&self, line: usize, character: usize) -> PathServerResult<usize> {
        position_to_offset(&self.index, line, character)
    }
}

/// Convert UTF-16 line/column to byte offset
/// - "column" in (line, column) is the the "utf-16 code unit" offset, in which a emoji/Chinese character may span 2 units.
/// - (line, column) in UTF-8 is the all "byte offset" based
fn position_to_offset(index: &LineIndex, line: usize, character: usize) -> PathServerResult<usize> {
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

fn offset_to_position(index: &LineIndex, offset: usize) -> PathServerResult<(usize, usize)> {
    let line_col = index.line_col(TextSize::new(offset as u32));
    let Some(wide_offset) = index.to_wide(WideEncoding::Utf16, line_col) else {
        return Err(PathServerError::EncodingError(format!(
            "Failed to convert offset to wide position for offset {}",
            offset
        )));
    };
    Ok((wide_offset.line as usize, wide_offset.col as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_to_offset_ascii() {
        let text = r#"Hello
World"#;
        let index = LineIndex::new(text);
        assert_eq!(position_to_offset(&index, 0, 0).unwrap(), 0);
        assert_eq!(position_to_offset(&index, 0, 5).unwrap(), 5);
        assert_eq!(position_to_offset(&index, 1, 0).unwrap(), 6);
        assert_eq!(position_to_offset(&index, 1, 5).unwrap(), 11);
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
        assert_eq!(position_to_offset(&index, 0, 0).unwrap(), 0);
        assert_eq!(position_to_offset(&index, 1, 0).unwrap(), 0 + text[0].len());
        assert_eq!(
            position_to_offset(&index, 2, 0).unwrap(),
            0 + text[0].len() + text[1].len()
        );
        assert_eq!(
            position_to_offset(&index, 3, 0).unwrap(),
            0 + text[0].len() + text[1].len() + text[2].len()
        );
        assert_eq!(
            position_to_offset(&index, 4, 0).unwrap(),
            0 + text[0].len() + text[1].len() + text[2].len() + text[3].len()
        );
        // test middle of line
        assert_eq!(position_to_offset(&index, 0, 4).unwrap(), "这是一个".len());
        assert_eq!(
            position_to_offset(&index, 1, 1).unwrap(),
            0 + text[0].len() + "這".len()
        );
        assert_eq!(
            position_to_offset(&index, 2, 10).unwrap(),
            0 + text[0].len() + text[1].len() + "これはUTF-8文字".len()
        );
        assert_eq!(
            position_to_offset(&index, 3, 20).unwrap(),
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
