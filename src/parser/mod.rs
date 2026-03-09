mod extractor;
mod line;
mod path;

pub use extractor::update_tree;
pub use line::{parse_line, separate_prefix};
pub use path::parse_document;

/// Represents a parsed string in the source code with its range
#[derive(Debug, Clone)]
pub struct PathCandidate {
    pub content: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl PathCandidate {
    fn char_count(&self) -> usize {
        self.content.chars().count()
    }

    /// From utf-8 character index to byte index
    fn char_to_byte(&self, ch_idx: usize) -> usize {
        let total = self.char_count();
        if ch_idx == total {
            return self.content.len();
        }
        self.content
            .char_indices()
            .nth(ch_idx)
            .map(|(b, _)| b)
            .expect("char index out of bounds")
    }
    /// Return an owned PathCandidate for a char-based range
    fn slice(&self, start: usize, end: usize) -> PathCandidate {
        if start > end {
            panic!("range start greater than end");
        }
        let total = self.char_count();
        if end > total {
            panic!("range end out of bounds");
        }
        let bs = self.char_to_byte(start);
        let be = self.char_to_byte(end);
        let slice = &self.content[bs..be];
        PathCandidate {
            content: slice.to_string(),
            start_byte: self.start_byte + bs,
            end_byte: self.start_byte + be,
        }
    }
    pub fn len(&self) -> usize {
        self.content.len()
    }
}
