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
            start_byte: 0,
            end_byte: s.len(),
        };
        let sub = pc.slice(1, 3); // chars at indices 1..3 -> "é𝄞"
        assert_eq!(sub.content, "é𝄞");
        let expected_start = s.char_indices().nth(1).unwrap().0;
        let expected_end = s.char_indices().nth(3).unwrap().0;
        assert_eq!(sub.start_byte, expected_start);
        assert_eq!(sub.end_byte, expected_end);
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
