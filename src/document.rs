#[derive(Debug, Clone)]
pub struct Document {
    pub text: String,
}

impl Document {
    pub fn new(text: String) -> Self {
        Self { text }
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn update_text(&mut self, new_text: String) {
        self.text = new_text;
    }

    pub fn get_line(&self, line_number: usize) -> Option<&str> {
        self.text.lines().nth(line_number)
    }
}
