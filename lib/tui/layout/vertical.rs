use crate::tui::engine::{Frame, View, event::KeyEvent};

/// Non owning vertical layout implementation.
/// Allows any number of views (non owning), and only 1 focused view:
/// Key events are delivered only to the focused view.
pub struct VerticalLayout {
    spacing: usize,
    focus: usize,
}

impl VerticalLayout {
    pub fn new(spacing: usize, focus: usize) -> Self {
        Self { spacing, focus }
    }

    pub fn render(&self, views: &[&dyn View], width: usize, height: usize) -> Frame {
        if views.is_empty() {
            return Frame::new(vec![String::new(); height.max(1)], None);
        }

        let total_spacing = self.spacing.saturating_mul(views.len() - 1);
        let section_height = height.saturating_sub(total_spacing) / views.len();
        if section_height < 1 {
            return Frame::new(vec![String::new(); height.max(1)], None);
        }

        let frames: Vec<_> = views
            .iter()
            .map(|view| view.render(width, section_height))
            .collect();

        let mut rows = Vec::new();
        let mut cursor = None;

        for (index, frame) in frames.iter().enumerate() {
            if index > 0 {
                rows.extend(std::iter::repeat_n(String::new(), self.spacing));
            }

            let start_row = rows.len();
            rows.extend(frame.rows().iter().cloned());

            if index == self.focus {
                cursor = frame
                    .cursor()
                    .map(|(row, column)| (start_row + row, column));
            }
        }

        Frame::new(rows, cursor)
    }

    pub fn key_event(&self, views: &mut [&mut dyn View], event: KeyEvent) {
        // KeyEvent is propogated only to focused view
        if self.focus < views.len() {
            views[self.focus].key_event(event);
        }
    }

    pub fn update(&self, views: &mut [&mut dyn View]) {
        for view in views {
            view.update();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::engine::event::Key;
    use std::cell::Cell;

    struct TestView {
        rows: Vec<String>,
        cursor: Option<(usize, usize)>,
        last_render_request: Cell<Option<(usize, usize)>>,
    }

    impl TestView {
        fn new(rows: &[&str], cursor: Option<(usize, usize)>) -> Self {
            Self {
                rows: rows.iter().map(|row| row.to_string()).collect(),
                cursor,
                last_render_request: Cell::new(None),
            }
        }
    }

    impl View for TestView {
        fn key_event(&mut self, _event: KeyEvent) {}

        fn render(&self, width: usize, height: usize) -> Frame {
            self.last_render_request.set(Some((width, height)));
            Frame::new(self.rows.clone(), self.cursor)
        }

        fn update(&mut self) {}
    }

    fn assert_frame(frame: &Frame, rows: &[&str], cursor: Option<(usize, usize)>) {
        let actual: Vec<&str> = frame.rows().iter().map(String::as_str).collect();
        assert_eq!(actual, rows);
        assert_eq!(frame.cursor(), cursor);
    }

    #[test]
    fn splits_height_equally_and_joins_with_spacing() {
        let first = TestView::new(&["a1", "a2"], None);
        let second = TestView::new(&["b1", "b2"], None);
        let layout = VerticalLayout::new(1, 0);

        let frame = layout.render(&[&first, &second], 10, 5);

        assert_frame(&frame, &["a1", "a2", "", "b1", "b2"], None);
    }

    #[test]
    fn requests_equal_section_height_from_children() {
        let first = TestView::new(&["a"], None);
        let second = TestView::new(&["b"], None);
        let layout = VerticalLayout::new(1, 0);

        layout.render(&[&first, &second], 20, 7);

        assert_eq!(first.last_render_request.get(), Some((20, 3)));
        assert_eq!(second.last_render_request.get(), Some((20, 3)));
    }

    #[test]
    fn offsets_cursor_by_focused_view_position() {
        let first = TestView::new(&["abc", "def"], Some((0, 1)));
        let second = TestView::new(&["ghi", "jkl"], Some((1, 2)));
        let layout = VerticalLayout::new(1, 1);

        let frame = layout.render(&[&first, &second], 10, 5);

        assert_frame(&frame, &["abc", "def", "", "ghi", "jkl"], Some((4, 2)));
    }

    #[test]
    fn ignores_cursor_of_unfocused_view() {
        let first = TestView::new(&["abc", "def"], Some((0, 1)));
        let second = TestView::new(&["ghi", "jkl"], None);
        let layout = VerticalLayout::new(1, 1);

        let frame = layout.render(&[&first, &second], 10, 5);

        assert_eq!(frame.cursor(), None);
    }

    #[test]
    fn uses_actual_child_heights_for_cursor_offset() {
        let first = TestView::new(&["only"], None);
        let second = TestView::new(&["abc", "def"], Some((1, 1)));
        let layout = VerticalLayout::new(2, 1);

        let frame = layout.render(&[&first, &second], 10, 6);

        assert_frame(&frame, &["only", "", "", "abc", "def"], Some((4, 1)));
    }

    #[test]
    fn returns_blank_frame_when_children_do_not_fit() {
        let view_a = TestView::new(&["a"], None);
        let view_b = TestView::new(&["b"], None);
        let view_c = TestView::new(&["c"], None);
        let layout = VerticalLayout::new(1, 0);

        let frame = layout.render(&[&view_a, &view_b, &view_c], 10, 2);

        assert_frame(&frame, &["", ""], None);
    }

    #[test]
    fn returns_blank_frame_without_views() {
        let layout = VerticalLayout::new(0, 0);

        let frame = layout.render(&[], 10, 3);

        assert_frame(&frame, &["", "", ""], None);
    }

    #[test]
    fn delivers_key_event_only_to_focused_view() {
        #[derive(Default)]
        struct RecordingView {
            events: Vec<Key>,
        }

        impl View for RecordingView {
            fn key_event(&mut self, event: KeyEvent) {
                self.events.push(event.key);
            }

            fn render(&self, _width: usize, _height: usize) -> Frame {
                Frame::new(vec![String::new()], None)
            }

            fn update(&mut self) {}
        }

        let mut first = RecordingView::default();
        let mut second = RecordingView::default();
        let layout = VerticalLayout::new(0, 1);

        layout.key_event(
            &mut [&mut first, &mut second],
            KeyEvent::new(Key::Char('x')),
        );

        assert!(first.events.is_empty());
        assert_eq!(second.events, vec![Key::Char('x')]);
    }
}
