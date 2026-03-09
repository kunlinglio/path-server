//! Parsers for inline path parsing.

use regex::Regex;

/// Parses a line of text and extracts the path from it.
pub fn parse_line(line: &str) -> String {
    // 1. parse by beginning
    //    e.g. "D:" or ".\" or "..\" for windows
    //    e.g. "/" or "~/" or "./" or "../" for unix
    // handle unix
    let beginning_unix = [r#"~/"#, r#"\.\./"#, r#"\./"#];
    for prefix in beginning_unix {
        if let Ok(re) = Regex::new(prefix)
            && let Some(mat) = re.find_iter(line).last()
        {
            return line[mat.start()..].to_string();
        }
    }
    // special case for unix root "/"
    let root_regex = Regex::new(r#"(?:^|[\s"'\[(])(/)"#).unwrap();
    if let Some(mat) = root_regex.find_iter(line).last()
        && let Some(pos) = line[mat.start()..mat.end()].find('/')
    {
        return line[mat.start() + pos..].to_string();
    }
    // handle windows
    let beginning_windows = [r#"[a-zA-Z]:\\"#, r#"\.\\"#, r#"\.\.\\ "#];
    for regex in beginning_windows {
        if let Ok(re) = Regex::new(regex)
            && let Some(mat) = re.find_iter(line).last()
        {
            return line[mat.start()..].to_string();
        }
    }
    // 2. parse by space
    if let Some(pos) = line.rfind(' ') {
        return line[pos + 1..].to_string();
    }
    line.to_string()
}

/// Separates an incomplete path (prefix) into a complete base directory and a partial name.
pub fn separate_prefix(prefix: &str) -> (String, String) {
    let prefix = prefix.to_string();
    let last_slash = prefix.rfind('/');
    let last_backslash = prefix.rfind('\\');
    let (mut base_dir, partial_name) = if let Some(pos) = last_slash {
        (prefix[..pos + 1].to_string(), prefix[pos + 1..].to_string())
    } else if let Some(pos) = last_backslash {
        (prefix[..pos + 1].to_string(), prefix[pos + 1..].to_string())
    } else {
        // no slash, e.g. index.htm
        ("".to_string(), prefix)
    };
    if base_dir.is_empty() {
        base_dir = "./".to_string();
    }
    (base_dir, partial_name)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_line_ascii() {
        // 1. unix home dir
        assert_eq!(
            parse_line("~/projects/rust/main.rs"),
            "~/projects/rust/main.rs"
        );
        assert_eq!(parse_line("/etc/nginx/nginx.conf"), "/etc/nginx/nginx.conf");

        // 2. windows
        assert_eq!(
            parse_line(r"setting=C:\Windows\System32\"),
            r"C:\Windows\System32\"
        );
        assert_eq!(parse_line(r"Look at .\local\file"), r".\local\file");

        // 3. quote
        assert_eq!(
            parse_line("import './components/Header"),
            "./components/Header"
        );
        assert_eq!(
            parse_line("let p = \"../data/config.json"),
            "../data/config.json"
        );

        // 4. markdown
        assert_eq!(parse_line("[link](./docs/README.md"), "./docs/README.md");
        assert_eq!(parse_line("![img](/assets/logo.png"), "/assets/logo.png");

        // 5. multi path in same line
        assert_eq!(parse_line("from /tmp/a to /var/log/b"), "/var/log/b");
    }

    #[test]
    fn test_parse_line_utf8() {
        assert_eq!(
            parse_line("import './中文文件夹/中文文件.js"),
            "./中文文件夹/中文文件.js"
        );
        // unix absolute with chinese
        assert_eq!(parse_line("打开 /中文/文件.txt"), "/中文/文件.txt");
        // home directory with chinese
        assert_eq!(parse_line("~/项目/主要.rs"), "~/项目/主要.rs");
        // relative current dir
        assert_eq!(parse_line("./中文/文件.js"), "./中文/文件.js");
        // relative parent dir in a quoted string
        assert_eq!(
            parse_line("let s = \"../数据/配置.json"),
            "../数据/配置.json"
        );
        // markdown link containing Chinese path
        assert_eq!(parse_line("[链接](./文档/说明.md"), "./文档/说明.md");
        // windows path with Chinese components (escaped backslashes)
        assert_eq!(parse_line("路径 C:\\项目\\子目录\\"), "C:\\项目\\子目录\\");
    }

    #[test]
    fn test_parse_line_empty() {
        assert_eq!(parse_line(""), "");
        assert_eq!(parse_line("   "), "");
    }

    // TODO: pass this test
    // #[test]
    // fn test_parse_line_mixed() {
    //     assert_eq!(
    //         parse_line("././../.././weird-file_name.v1.2"),
    //         "././../.././weird-file_name.v1.2"
    //     );
    // }

    #[test]
    fn test_parse_line_network() {
        // network URL
        assert_eq!(
            parse_line("see http://example.com/path/to/res"),
            "http://example.com/path/to/res"
        );
        // Windows-style network path
        assert_eq!(
            parse_line("copy \\\\server\\share\\file.txt"),
            "\\\\server\\share\\file.txt"
        );
    }

    #[test]
    fn test_separate_prefix() {
        // unix style
        let (base, partial) = separate_prefix("/home/user/file.txt");
        assert_eq!(base, "/home/user/");
        assert_eq!(partial, "file.txt");

        // Windows style
        let (base, partial) = separate_prefix(r"C:\Users\Admin\Doc");
        assert_eq!(base, r"C:\Users\Admin\");
        assert_eq!(partial, "Doc");

        // only filename
        let (base, partial) = separate_prefix("file.txt");
        assert_eq!(base, "./");
        assert_eq!(partial, "file.txt");

        // only dir
        let (base, partial) = separate_prefix("/usr/bin/");
        assert_eq!(base, "/usr/bin/");
        assert_eq!(partial, "");

        // hidden file
        let (base, partial) = separate_prefix("./.config");
        assert_eq!(base, "./");
        assert_eq!(partial, ".config");
    }
}
