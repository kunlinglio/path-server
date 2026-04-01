pub fn unescape(escaped: &str) -> Option<String> {
    let line = format!("\"{}\"", escaped);
    let Ok(unescaped) = serde_json::from_str::<String>(&line) else {
        return None;
    };
    Some(unescaped)
}
