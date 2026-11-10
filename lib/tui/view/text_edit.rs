use std::cell::Cell;

use crate::tui::engine::{
    Frame, View,
    event::{Key, KeyEvent},
};

const MIN_WIDTH: usize = 10;
const MIN_HEIGHT: usize = 3;
const BORDER_COLUMNS: usize = 2;
const PREFIX_COLUMNS: usize = 3;
const RIGHT_PADDING: usize = 1;
const TEXT_CAPACITY_OFFSET: usize = BORDER_COLUMNS + PREFIX_COLUMNS + RIGHT_PADDING;
const DEFAULT_WIDTH: usize = 80;
const BORDER_TITLE_OFFSET: usize = 3;

pub struct TextEditView {
    title: String,
    text: String,
    cursor: usize,
    last_width: Cell<usize>,
}

impl TextEditView {
    pub fn new(title: impl Into<String>, initial_text: impl Into<String>) -> Self {
        let text = initial_text.into();
        let cursor = text.len();

        Self {
            title: title.into(),
            text,
            cursor,
            last_width: Cell::new(DEFAULT_WIDTH),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    fn set_char_index(&mut self, index: usize) {
        match self.text.char_indices().nth(index) {
            Some((byte, _)) => self.cursor = byte,
            None => self.cursor = self.text.len(),
        }
    }

    fn move_vertical(&mut self, up: bool) {
        let capacity = self.last_width.get() - TEXT_CAPACITY_OFFSET;
        let char_index = self.text[..self.cursor].chars().count();
        let row = char_index / capacity;
        let col = char_index % capacity;

        if up {
            if row == 0 {
                return;
            }

            self.set_char_index((row - 1) * capacity + col);
            return;
        }

        let char_len = self.text.chars().count();
        let target_row = row + 1;
        if target_row * capacity > char_len {
            return;
        }

        self.set_char_index((target_row * capacity + col).min(char_len));
    }

    fn layout(&self, width: usize, height: usize) -> Frame {
        let capacity = width - TEXT_CAPACITY_OFFSET;
        let chars: Vec<char> = self.text.chars().collect();
        let char_cursor = self.text[..self.cursor].chars().count();

        let mut text_rows = if chars.is_empty() {
            1
        } else {
            chars.len().div_ceil(capacity)
        };

        let cursor_text_row = char_cursor / capacity;
        if cursor_text_row + 1 > text_rows {
            text_rows = cursor_text_row + 1;
        }

        let visible_rows = height - BORDER_COLUMNS;
        let window_rows = text_rows.min(visible_rows);
        let offset = if text_rows > visible_rows {
            cursor_text_row
                .saturating_sub(visible_rows.saturating_sub(1))
                .min(text_rows - visible_rows)
        } else {
            0
        };

        let title_fill = width
            .checked_sub(BORDER_TITLE_OFFSET + self.title.chars().count())
            .expect("title does not fit in the frame width");

        let mut rows = Vec::with_capacity(window_rows + 2);
        rows.push(format!("╭─{}{}╮", self.title, "─".repeat(title_fill)));

        for row in offset..offset + window_rows {
            let start = row * capacity;
            let end = (start + capacity).min(chars.len());
            let text_len = end.saturating_sub(start);
            let prefix = if row == 0 { " > " } else { "   " };

            let mut line = String::with_capacity(width);
            line.push('│');
            line.push_str(prefix);
            if start < chars.len() {
                line.extend(chars[start..end].iter());
            }
            line.extend(std::iter::repeat_n(
                ' ',
                width - BORDER_COLUMNS - PREFIX_COLUMNS - text_len,
            ));
            line.push('│');

            debug_assert_eq!(line.chars().count(), width);
            rows.push(line);
        }

        rows.push(format!("╰{}╯", "─".repeat(width - 2)));

        Frame::new(
            rows,
            Some((1 + cursor_text_row - offset, 4 + (char_cursor % capacity))),
        )
    }
}

impl View for TextEditView {
    fn key_event(&mut self, event: KeyEvent) {
        match event.key {
            Key::Char(c) => {
                self.text.insert(self.cursor, c);
                self.cursor += c.len_utf8();
            }
            Key::Backspace => {
                if let Some((start, _)) = self.text[..self.cursor].char_indices().next_back() {
                    self.text.remove(start);
                    self.cursor = start;
                }
            }
            Key::Delete => {
                if self.text[self.cursor..].chars().next().is_some() {
                    self.text.remove(self.cursor);
                }
            }
            Key::Left => {
                if let Some((start, _)) = self.text[..self.cursor].char_indices().next_back() {
                    self.cursor = start;
                }
            }
            Key::Right => {
                if let Some(c) = self.text[self.cursor..].chars().next() {
                    self.cursor += c.len_utf8();
                }
            }
            Key::Up => self.move_vertical(true),
            Key::Down => self.move_vertical(false),
            Key::Home | Key::CtrlA => self.cursor = 0,
            Key::End | Key::CtrlE => self.cursor = self.text.len(),
            Key::CtrlU => {
                self.text.drain(..self.cursor);
                self.cursor = 0;
            }
            Key::CtrlK => self.text.truncate(self.cursor),
            Key::Enter | Key::Escape | Key::CtrlC | Key::CtrlD => {}
        }
    }

    fn render(&self, width: usize, height: usize) -> Frame {
        assert!(
            width >= MIN_WIDTH,
            "terminal width must be at least {MIN_WIDTH}, got {width}"
        );
        assert!(
            height >= MIN_HEIGHT,
            "terminal height must be at least {MIN_HEIGHT}, got {height}"
        );

        self.last_width.set(width);
        self.layout(width, height)
    }

    fn update(&mut self) {
        // Do nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_getter_returns_current_text() {
        let view = TextEditView::new("Prompt", "ls");

        assert_eq!(view.text(), "ls");
    }

    #[test]
    fn initial_text_places_cursor_at_end() {
        let mut view = TextEditView::new("Prompt", "abcd");

        view.key_event(KeyEvent::new(Key::Char('e')));

        assert_eq!(view.text(), "abcde");
    }

    #[test]
    fn typing_inserts_at_cursor() {
        let mut view = TextEditView::new("Prompt", "");
        view.key_event(KeyEvent::new(Key::Char('a')));
        view.key_event(KeyEvent::new(Key::Char('b')));
        view.key_event(KeyEvent::new(Key::Left));
        view.key_event(KeyEvent::new(Key::Char('c')));

        assert_eq!(view.text(), "acb");
    }

    fn assert_frame(frame: &Frame, rows: &[&str], cursor: Option<(usize, usize)>) {
        let actual: Vec<&str> = frame.rows().iter().map(String::as_str).collect();
        assert_eq!(actual, rows);
        assert_eq!(frame.cursor(), cursor);
    }

    #[test]
    fn empty_prompt_first_render() {
        let view = TextEditView::new("Prompt", "");

        assert_frame(
            &view.render(12, 5),
            &["╭─Prompt───╮", "│ >        │", "╰──────────╯"],
            Some((1, 4)),
        );
    }

    #[test]
    fn short_prompt_stays_on_one_line() {
        let view = TextEditView::new("Prompt", "ls");

        assert_frame(
            &view.render(12, 3),
            &["╭─Prompt───╮", "│ > ls     │", "╰──────────╯"],
            Some((1, 6)),
        );
    }

    #[test]
    fn cursor_in_the_middle_of_the_prompt() {
        let mut view = TextEditView::new("Prompt", "abcd");
        view.key_event(KeyEvent::new(Key::Home));
        view.key_event(KeyEvent::new(Key::Right));
        view.key_event(KeyEvent::new(Key::Right));

        assert_frame(
            &view.render(12, 3),
            &["╭─Prompt───╮", "│ > abcd   │", "╰──────────╯"],
            Some((1, 6)),
        );
    }

    #[test]
    fn long_prompt_wraps_to_next_line() {
        let view = TextEditView::new("Prompt", "abcdefgh");

        assert_frame(
            &view.render(10, 10),
            &[
                "╭─Prompt─╮",
                "│ > abcd │",
                "│   efgh │",
                "│        │",
                "╰────────╯",
            ],
            Some((3, 4)),
        );
    }

    #[test]
    fn partial_wrapped_line_is_padded() {
        let view = TextEditView::new("Prompt", "abcdefg");

        assert_frame(
            &view.render(10, 10),
            &["╭─Prompt─╮", "│ > abcd │", "│   efg  │", "╰────────╯"],
            Some((2, 7)),
        );
    }

    #[test]
    fn cursor_at_exact_width_adds_empty_line() {
        let view = TextEditView::new("Prompt", "abcd");

        assert_frame(
            &view.render(10, 10),
            &["╭─Prompt─╮", "│ > abcd │", "│        │", "╰────────╯"],
            Some((2, 4)),
        );
    }

    #[test]
    fn viewport_keeps_top_cursor_visible() {
        let mut view = TextEditView::new("Prompt", "abcdefghijklmnopqrstuv");
        view.key_event(KeyEvent::new(Key::Home));

        assert_frame(
            &view.render(10, 5),
            &[
                "╭─Prompt─╮",
                "│ > abcd │",
                "│   efgh │",
                "│   ijkl │",
                "╰────────╯",
            ],
            Some((1, 4)),
        );
    }

    #[test]
    fn viewport_keeps_middle_cursor_visible() {
        let mut view = TextEditView::new("Prompt", "abcdefghijklmnopqrstuv");
        view.key_event(KeyEvent::new(Key::Home));
        for _ in 0..12 {
            view.key_event(KeyEvent::new(Key::Right));
        }

        assert_frame(
            &view.render(10, 5),
            &[
                "╭─Prompt─╮",
                "│   efgh │",
                "│   ijkl │",
                "│   mnop │",
                "╰────────╯",
            ],
            Some((3, 4)),
        );
    }

    #[test]
    fn viewport_keeps_bottom_cursor_visible() {
        let view = TextEditView::new("Prompt", "abcdefghijklmnopqrstuv");

        assert_frame(
            &view.render(10, 5),
            &[
                "╭─Prompt─╮",
                "│   mnop │",
                "│   qrst │",
                "│   uv   │",
                "╰────────╯",
            ],
            Some((3, 6)),
        );
    }

    #[test]
    fn all_rows_have_requested_width() {
        let view = TextEditView::new("Prompt", "abcdefghijklmnopqrstuv");

        let frame = view.render(13, 5);

        assert!(frame.rows().iter().all(|row| row.chars().count() == 13));
    }

    #[test]
    fn renders_custom_title() {
        let view = TextEditView::new("Command", "ls");

        assert_frame(
            &view.render(12, 3),
            &["╭─Command──╮", "│ > ls     │", "╰──────────╯"],
            Some((1, 6)),
        );
    }

    #[test]
    #[should_panic]
    fn too_narrow_width_panics() {
        let view = TextEditView::new("Prompt", "");
        view.render(9, 10);
    }

    #[test]
    #[should_panic]
    fn too_short_height_panics() {
        let view = TextEditView::new("Prompt", "");
        view.render(10, 2);
    }
}
