use crate::tui::engine::{Frame, View, event::KeyEvent};

pub struct TextView {
    text: String,
}

impl TextView {
    pub fn new(text: String) -> Self {
        TextView { text }
    }
}

impl View for TextView {
    fn key_event(&mut self, _event: KeyEvent) {
        // Do nothing
    }

    fn render(&self, _width: usize, _height: usize) -> Frame {
        let mut rows: Vec<String> = self.text.lines().map(String::from).collect();
        if rows.is_empty() {
            rows.push(String::new());
        }

        Frame::new(rows, None)
    }

    fn update(&mut self) {
        // Do nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_multiple_lines_as_rows() {
        let view = TextView::new(String::from("hello\nworld"));

        let frame = view.render(10, 10);

        assert_eq!(frame.rows(), &["hello", "world"]);
        assert_eq!(frame.cursor(), None);
    }

    #[test]
    fn renders_empty_text_as_single_empty_row() {
        let view = TextView::new(String::new());

        let frame = view.render(10, 10);

        assert_eq!(frame.rows(), &[""]);
    }
}
