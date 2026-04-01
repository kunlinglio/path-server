//! Parsers for inline path parsing.

use std::vec::Vec;

use crate::{lsp_warn, to_sync};

use super::unescape::unescape;

const INIT_CONFIDENCE: usize = 16;

/// Parses a line of text and extracts the path from it.
/// Returns a series of candidates, from high priority to low priority.
pub fn parse_line(line: &str) -> Vec<String> {
    let without_escape = LineParser::new(line).parse();
    let with_escape = if let Some(unescaped) = unescape(line) {
        LineParser::new(&unescaped).parse()
    } else {
        vec![]
    };
    let mut sorted = [without_escape, with_escape]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    // deduplicate only by string
    sorted.sort_by(|a, b| a.1.cmp(&b.1));
    sorted.dedup_by(|a, b| a.1 == b.1);
    // sort
    sorted.sort_by(|x, y| {
        // confidence desc
        y.0.cmp(&x.0)
            // length desc
            .then_with(|| y.1.len().cmp(&x.1.len()))
    });
    sorted.into_iter().map(|(_, s)| s).collect()
}

struct LineParser {
    rev_content: Vec<(usize, char)>,
    cursor: usize,
    confidence: usize,
    in_disk_identifier: bool,
}

impl LineParser {
    pub fn new(line: &str) -> LineParser {
        let rev_content = line
            .chars()
            .rev()
            .collect::<String>()
            .char_indices()
            .collect();
        LineParser {
            rev_content,
            cursor: 0,
            confidence: INIT_CONFIDENCE,
            in_disk_identifier: false,
        }
    }

    fn peek(&self) -> Option<char> {
        self.rev_content.get(self.cursor).copied().map(|(_, c)| c)
    }

    fn peek_prev(&self) -> Option<char> {
        if self.cursor < 2 {
            None
        } else {
            self.rev_content
                .get(self.cursor - 2)
                .copied()
                .map(|(_, c)| c)
        }
    }

    fn peek_prev_prev(&self) -> Option<char> {
        if self.cursor < 3 {
            None
        } else {
            self.rev_content
                .get(self.cursor - 3)
                .copied()
                .map(|(_, c)| c)
        }
    }

    fn next(&mut self) -> Option<(usize, char)> {
        let res = self.rev_content.get(self.cursor).copied();
        if res.is_some() {
            self.cursor += 1;
        }
        res
    }

    fn construct_candidate(&self, end: usize) -> String {
        self.rev_content
            .iter()
            .take(end)
            .map(|(_, c)| *c)
            .rev()
            .collect()
    }

    pub fn parse(&mut self) -> Vec<(usize, String)> {
        let mut candidates = vec![];
        let mut break_ = false;
        while let Some((_, c)) = self.next() {
            if self.confidence == 0 {
                break_ = true;
                break;
            }
            if self.in_disk_identifier {
                if c.is_ascii_alphabetic() {
                    candidates.push((self.confidence, self.construct_candidate(self.cursor)));
                } else {
                    to_sync!(lsp_warn!(
                        "Unexpected character '{}' after disk identifier in path parsing",
                        c
                    ));
                }
                break_ = true;
                break;
            }
            match c {
                '\0' | '\t' | '\n' | '\r' => {
                    // terminals
                    break_ = true;
                    break;
                }
                ':' => {
                    if !matches!(self.peek(), Some(next_c) if next_c.is_ascii_alphabetic()) {
                        // next char is not a drive letter, treat ':' as normal char
                        continue;
                    } else if self.in_disk_identifier {
                        to_sync!(lsp_warn!(
                            "Unexpected ':' in path parsing after disk identifier"
                        ));
                        break_ = true;
                        break;
                    } else {
                        self.in_disk_identifier = true;
                    };
                }
                '\'' | '"' | ' ' | '[' | '(' | '<' | '>' | '|' | '?' | '*' => {
                    // decrease confidence
                    // exclude the terminal char, decrease confidence after pushing candidate
                    candidates.push((self.confidence, self.construct_candidate(self.cursor - 1)));
                    self.confidence -= 1;
                }
                '/' | '\\' => {
                    // increase confidence
                    // include slash, increase confidence before pushing candidate
                    self.confidence += 1;
                    candidates.push((self.confidence, self.construct_candidate(self.cursor)));
                }
                '.' => {
                    if matches!(self.peek_prev(), Some('/' | '\\'))
                        && !matches!(self.peek(), Some('.'))
                    {
                        // `./` or `.\`
                        candidates.push((self.confidence, self.construct_candidate(self.cursor)));
                    } else if matches!(self.peek_prev_prev(), Some('/' | '\\'))
                        && matches!(self.peek_prev(), Some('.'))
                    {
                        // `../` or `..\`
                        candidates.push((self.confidence, self.construct_candidate(self.cursor)));
                    }
                }
                _ => {
                    continue;
                }
            }
        }
        if !break_ {
            // end of line
            candidates.push((self.confidence, self.construct_candidate(self.cursor)));
        }
        candidates
    }
}

/// Separates an incomplete path (prefix) into a complete base directory and a partial name.
pub fn separate_prefix(mut prefix: String) -> (String, String) {
    let last_slash = prefix.rfind('/').or_else(|| prefix.rfind('\\'));

    if let Some(pos) = last_slash {
        let split_pos = pos + 1;
        let partial_name = prefix[split_pos..].to_string();
        prefix.truncate(split_pos);
        (prefix, partial_name)
    } else if prefix.is_empty() {
        ("./".to_string(), "".to_string())
    } else {
        ("./".to_string(), prefix)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_line_ascii() {
        // 1. unix home dir
        assert_eq!(
            parse_line("~/projects/rust/main.rs")[0],
            "~/projects/rust/main.rs".to_owned()
        );
        assert_eq!(
            parse_line("/etc/nginx/nginx.conf")[0],
            "/etc/nginx/nginx.conf".to_owned()
        );

        // 2. windows
        assert_eq!(
            parse_line(r"setting=C:\Windows\System32\")[0],
            r"C:\Windows\System32\".to_owned()
        );
        assert_eq!(
            parse_line(r"Look at .\local\file")[0],
            r".\local\file".to_owned()
        );

        // 3. quote
        assert_eq!(
            parse_line("import './components/Header")[0],
            "./components/Header".to_owned()
        );
        assert_eq!(
            parse_line("let p = \"../data/config.json")[0],
            "../data/config.json".to_owned()
        );

        // 4. markdown
        assert_eq!(
            parse_line("[link](./docs/README.md")[0],
            "./docs/README.md".to_owned()
        );
        assert_eq!(
            parse_line("![img](/assets/logo.png")[0],
            "/assets/logo.png".to_owned()
        );

        // 5. multi path in same line
        assert!(parse_line("from /tmp/a to /var/log/b").contains(&"/var/log/b".to_owned()));
    }

    #[test]
    fn test_parse_line_utf8() {
        assert_eq!(
            parse_line("import './中文文件夹/中文文件.js")[0],
            "./中文文件夹/中文文件.js".to_owned()
        );
        // unix absolute with chinese
        assert_eq!(
            parse_line("打开 /中文/文件.txt")[0],
            "/中文/文件.txt".to_owned()
        );
        // home directory with chinese
        assert_eq!(parse_line("~/项目/主要.rs")[0], "~/项目/主要.rs".to_owned());
        // relative current dir
        assert_eq!(parse_line("./中文/文件.js")[0], "./中文/文件.js".to_owned());
        // relative parent dir in a quoted string
        assert_eq!(
            parse_line("let s = \"../数据/配置.json")[0],
            "../数据/配置.json".to_owned()
        );
        // markdown link containing Chinese path
        assert_eq!(
            parse_line("[链接](./文档/说明.md")[0],
            "./文档/说明.md".to_owned()
        );
        // windows path with Chinese components (escaped backslashes)
        assert_eq!(
            parse_line("路径 C:\\项目\\子目录\\")[0],
            "C:\\项目\\子目录\\".to_owned()
        );
    }

    #[test]
    fn test_parse_line_empty() {
        assert!(parse_line("").contains(&"".to_owned()));
        assert!(parse_line("   ").contains(&"".to_owned()));
    }

    #[test]
    fn test_parse_line_mixed() {
        assert_eq!(
            parse_line("././../.././weird-file_name.v1.2")[0],
            "././../.././weird-file_name.v1.2".to_owned()
        );
    }

    #[test]
    fn test_separate_prefix() {
        // unix style
        let (base, partial) = separate_prefix("/home/user/file.txt".into());
        assert_eq!(base, "/home/user/");
        assert_eq!(partial, "file.txt");

        // Windows style
        let (base, partial) = separate_prefix(r"C:\Users\Admin\Doc".into());
        assert_eq!(base, r"C:\Users\Admin\");
        assert_eq!(partial, "Doc");

        // only filename
        let (base, partial) = separate_prefix("file.txt".into());
        assert_eq!(base, "./");
        assert_eq!(partial, "file.txt");

        // only dir
        let (base, partial) = separate_prefix("/usr/bin/".into());
        assert_eq!(base, "/usr/bin/");
        assert_eq!(partial, "");

        // hidden file
        let (base, partial) = separate_prefix("./.config".into());
        assert_eq!(base, "./");
        assert_eq!(partial, ".config");
    }

    #[test]
    fn test_hybrid_paths() {
        assert_eq!(
            parse_line(r"\\127.0.0.1\c$\temp\file.txt")[0],
            r"\\127.0.0.1\c$\temp\file.txt".to_owned()
        );
        assert_eq!(
            parse_line(r"//server/share/path")[0],
            r"//server/share/path".to_owned()
        );
    }

    #[test]
    fn test_others() {
        assert_eq!(
            parse_line(&"let f = \"./exclude_dir/".to_owned())[0],
            "./exclude_dir/".to_owned()
        );
    }
    #[test]
    fn test_near_paths() {
        assert!(parse_line("Error at/var/log/app.log").contains(&"/var/log/app.log".to_owned()));
        assert!(parse_line("See config at./config.yaml").contains(&"./config.yaml".to_owned()));
    }
}
