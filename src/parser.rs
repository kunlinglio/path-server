use regex::Regex;

pub fn parse_line(line: &str) -> String {
    // 1. parse by beginning
    //    e.g. "D:" or ".\" or "..\" for windows
    //    e.g. "/" or "~/" or "./" or "../" for unix
    // handle unix
    let beginning_unix = [r#"~/"#, r#"\.\./"#, r#"\./"#];
    for prefix in beginning_unix {
        if let Ok(re) = Regex::new(prefix) {
            if let Some(mat) = re.find_iter(line).last() {
                return line[mat.start()..].to_string();
            }
        }
    }
    // special case for unix root "/"
    let root_regex = Regex::new(r#"(?:^|[\s"'\[(])(/)"#).unwrap();
    if let Some(mat) = root_regex.find_iter(line).last() {
        if let Some(pos) = line[mat.start()..mat.end()].find('/') {
            return line[mat.start() + pos..].to_string();
        }
    }
    // handle windows
    let beginning_windows = [r#"[a-zA-Z]:\\"#, r#"\.\\"#, r#"\.\.\\ "#];
    for regex in beginning_windows {
        if let Ok(re) = Regex::new(regex) {
            if let Some(mat) = re.find_iter(line).last() {
                return line[mat.start()..].to_string();
            }
        }
    }
    // 2. parse by space
    if let Some(pos) = line.rfind(' ') {
        return line[pos + 1..].to_string();
    }
    line.to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_line() {
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
}
