use super::PathCandidate;

pub fn split(content: &str, path_ref: &PathCandidate, delimiter: &[char]) -> Vec<PathCandidate> {
    let mut results = Vec::new();
    let mut last_pos = 0;

    while let Some(pos) = content[last_pos..].find(|c| delimiter.contains(&c)) {
        let end = last_pos + pos;
        if end > last_pos {
            let sub_content = &content[last_pos..end];
            if sub_content.contains('/') || sub_content.contains('\\') {
                let trimmed = PathCandidate {
                    content: sub_content.to_string(),
                    start_byte: path_ref.start_byte + last_pos,
                    end_byte: path_ref.start_byte + end,
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
                start_byte: path_ref.start_byte + last_pos,
                end_byte: path_ref.start_byte + content.len(),
            }
            .trim();
            if !trimmed.content.is_empty() {
                results.push(trimmed);
            }
        }
    }
    results
}
