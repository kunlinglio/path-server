mod line;
mod path;
mod tree_sitter;

pub use line::{parse_line, separate_prefix};
pub use path::parse_document;
pub use tree_sitter::{new_tree, update_tree};

/// Represents a parsed string in the source code with its range
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathCandidate {
    pub content: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[allow(dead_code)]
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

    /// Slice by absolute byte positions in the document
    pub fn slice_bytes(&self, start_byte: usize, end_byte: usize) -> PathCandidate {
        if start_byte > end_byte {
            panic!("byte range start greater than end");
        }
        if start_byte < self.start_byte || end_byte > self.end_byte {
            panic!("byte range out of bounds of this PathCandidate");
        }

        // Convert absolute positions to relative offsets in self.content
        let rel_start = start_byte - self.start_byte;
        let rel_end = end_byte - self.start_byte;

        let slice = &self.content[rel_start..rel_end];
        PathCandidate {
            content: slice.to_string(),
            start_byte,
            end_byte,
        }
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Trim the space from both begin and end
    pub fn trim(&self) -> PathCandidate {
        let s = &self.content;
        let mut first: Option<usize> = None;
        let mut last: Option<usize> = None;

        for (i, ch) in s.char_indices() {
            if !ch.is_whitespace() {
                first = Some(i);
                break;
            }
        }

        for (i, ch) in s.char_indices().rev() {
            if !ch.is_whitespace() {
                last = Some(i + ch.len_utf8());
                break;
            }
        }

        match (first, last) {
            (Some(f), Some(l)) => {
                let slice = &s[f..l];
                PathCandidate {
                    content: slice.to_string(),
                    start_byte: self.start_byte + f,
                    end_byte: self.start_byte + l,
                }
            }
            _ => {
                // all whitespace -> empty slice at end
                let pos = s.len();
                PathCandidate {
                    content: String::new(),
                    start_byte: self.start_byte + pos,
                    end_byte: self.start_byte + pos,
                }
            }
        }
    }

    pub fn split(&self, content: &str, delimiter: &[char]) -> Vec<PathCandidate> {
        let mut results = Vec::new();
        let mut last_pos = 0;

        while let Some(pos) = content[last_pos..].find(|c| delimiter.contains(&c)) {
            let end = last_pos + pos;
            if end > last_pos {
                let sub_content = &content[last_pos..end];
                if sub_content.contains('/') || sub_content.contains('\\') {
                    let trimmed = PathCandidate {
                        content: sub_content.to_string(),
                        start_byte: self.start_byte + last_pos,
                        end_byte: self.start_byte + end,
                    }
                    .trim();
                    if !trimmed.content.is_empty() {
                        results.push(trimmed);
                    }
                }
            }
            last_pos = end + 1;
        }

        // process last part
        if last_pos < content.len() {
            let sub_content = &content[last_pos..];
            if sub_content.contains('/') || sub_content.contains('\\') {
                let trimmed = PathCandidate {
                    content: sub_content.to_string(),
                    start_byte: self.start_byte + last_pos,
                    end_byte: self.start_byte + content.len(),
                }
                .trim();
                if !trimmed.content.is_empty() {
                    results.push(trimmed);
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_candidate_slice() {
        // "a" (1 byte), "é" (2 bytes), "𝄞" (4 bytes), "b" (1 byte)
        let s = "aé𝄞b".to_string();
        let pc = PathCandidate {
            content: s.clone(),
            start_byte: 1,
            end_byte: s.len(),
        };
        let sub = pc.slice_bytes(1 + 1, 1 + 3); // bytes at indices 1..3 -> "é𝄞"
        assert_eq!(sub.content, "é");
        assert_eq!(sub.start_byte, 2);
        assert_eq!(sub.end_byte, 4);
        let sub = pc.slice_bytes(1 + 3, 1 + 7); // bytes at indices 3..7 -> "𝄞b"
        assert_eq!(sub.content, "𝄞");
        assert_eq!(sub.start_byte, 4);
        assert_eq!(sub.end_byte, 8);
    }
    #[test]
    fn path_candidate_trim_basic_unicode() {
        let s = "  \t aé𝄞b \n".to_string();
        let pc = PathCandidate {
            content: s.clone(),
            start_byte: 5,
            end_byte: 5 + s.len(),
        };
        let first = s
            .char_indices()
            .find(|&(_, ch)| !ch.is_whitespace())
            .unwrap()
            .0;
        let (last_i, last_ch) = s
            .char_indices()
            .rev()
            .find(|&(_, ch)| !ch.is_whitespace())
            .unwrap();
        let last = last_i + last_ch.len_utf8();

        let trimmed = pc.trim();
        assert_eq!(trimmed.content, &s[first..last]);
        assert_eq!(trimmed.start_byte, pc.start_byte + first);
        assert_eq!(trimmed.end_byte, pc.start_byte + last);
    }

    #[test]
    fn path_candidate_trim_all_whitespace() {
        let s = " \t\n".to_string();
        let pc = PathCandidate {
            content: s.clone(),
            start_byte: 3,
            end_byte: 3 + s.len(),
        };
        let trimmed = pc.trim();
        assert_eq!(trimmed.content, "");
        assert_eq!(trimmed.start_byte, pc.start_byte + s.len());
        assert_eq!(trimmed.end_byte, pc.start_byte + s.len());
    }

    #[test]
    fn path_candidate_trim_no_whitespace() {
        let s = "a bc".to_string();
        let pc = PathCandidate {
            content: s.clone(),
            start_byte: 0,
            end_byte: 0 + s.len(),
        };
        let trimmed = pc.trim();
        assert_eq!(trimmed.content, s);
        assert_eq!(trimmed.start_byte, pc.start_byte);
        assert_eq!(trimmed.end_byte, pc.end_byte);
    }
}
