use crate::tui::engine::{Frame, View, event::KeyEvent};

use std::boxed::Box;

pub struct VerticalBoxView {
    views: Vec<Box<dyn View>>,
    spacing: usize,
    focus: usize,
}

impl VerticalBoxView {
    pub fn new(views: Vec<Box<dyn View>>, spacing: usize, focus: usize) -> Self {
        Self {
            views,
            spacing,
            focus,
        }
    }
}

impl View for VerticalBoxView {
    fn key_event(&mut self, event: KeyEvent) {
        // KeyEvent is propogated only to focused view
        if self.focus < self.views.len() {
            self.views[self.focus].key_event(event);
        }
    }

    fn render(&self, width: usize, height: usize) -> Frame {
        if self.views.is_empty() {
            return Frame::new(vec![String::new(); height.max(1)], None);
        }

        let total_spacing = self.spacing.saturating_mul(self.views.len() - 1);
        let section_height = height.saturating_sub(total_spacing) / self.views.len();
        if section_height < 1 {
            return Frame::new(vec![String::new(); height.max(1)], None);
        }

        let frames: Vec<_> = self
            .views
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

    fn update(&mut self) {
        for view in &mut self.views {
            view.update();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    struct TestView {
        rows: Vec<String>,
        cursor: Option<(usize, usize)>,
        last_request: Rc<Cell<Option<(usize, usize)>>>,
    }

    impl TestView {
        fn new(rows: &[&str], cursor: Option<(usize, usize)>) -> Self {
            Self {
                rows: rows.iter().map(|row| row.to_string()).collect(),
                cursor,
                last_request: Rc::new(Cell::new(None)),
            }
        }
    }

    impl View for TestView {
        fn key_event(&mut self, _event: KeyEvent) {}

        fn render(&self, width: usize, height: usize) -> Frame {
            self.last_request.set(Some((width, height)));
            Frame::new(self.rows.clone(), self.cursor)
        }

        fn update(&mut self) {}
    }

    fn boxed(rows: &[&str], cursor: Option<(usize, usize)>) -> Box<dyn View> {
        Box::new(TestView::new(rows, cursor))
    }

    fn assert_frame(frame: &Frame, rows: &[&str], cursor: Option<(usize, usize)>) {
        let actual: Vec<&str> = frame.rows().iter().map(String::as_str).collect();
        assert_eq!(actual, rows);
        assert_eq!(frame.cursor(), cursor);
    }

    #[test]
    fn splits_height_equally_and_joins_with_spacing() {
        let first = boxed(&["a1", "a2"], None);
        let second = boxed(&["b1", "b2"], None);
        let view = VerticalBoxView::new(vec![first, second], 1, 0);

        let frame = view.render(10, 5);

        assert_frame(&frame, &["a1", "a2", "", "b1", "b2"], None);
    }

    #[test]
    fn requests_equal_section_height_from_children() {
        let first = TestView::new(&["a"], None);
        let second = TestView::new(&["b"], None);
        let first_request = Rc::clone(&first.last_request);
        let second_request = Rc::clone(&second.last_request);
        let view = VerticalBoxView::new(vec![Box::new(first), Box::new(second)], 1, 0);

        view.render(20, 7);

        assert_eq!(first_request.get(), Some((20, 3)));
        assert_eq!(second_request.get(), Some((20, 3)));
    }

    #[test]
    fn offsets_cursor_by_focused_view_position() {
        let first = boxed(&["abc", "def"], Some((0, 1)));
        let second = boxed(&["ghi", "jkl"], Some((1, 2)));
        let view = VerticalBoxView::new(vec![first, second], 1, 1);

        let frame = view.render(10, 5);

        assert_frame(&frame, &["abc", "def", "", "ghi", "jkl"], Some((4, 2)));
    }

    #[test]
    fn ignores_cursor_of_unfocused_view() {
        let first = boxed(&["abc", "def"], Some((0, 1)));
        let second = boxed(&["ghi", "jkl"], None);
        let view = VerticalBoxView::new(vec![first, second], 1, 1);

        let frame = view.render(10, 5);

        assert_eq!(frame.cursor(), None);
    }

    #[test]
    fn uses_actual_child_heights_for_cursor_offset() {
        let first = boxed(&["only"], None);
        let second = boxed(&["abc", "def"], Some((1, 1)));
        let view = VerticalBoxView::new(vec![first, second], 2, 1);

        let frame = view.render(10, 6);

        assert_frame(&frame, &["only", "", "", "abc", "def"], Some((4, 1)));
    }

    #[test]
    fn returns_blank_frame_when_children_do_not_fit() {
        let view = VerticalBoxView::new(
            vec![
                boxed(&["a"], None),
                boxed(&["b"], None),
                boxed(&["c"], None),
            ],
            1,
            0,
        );

        let frame = view.render(10, 2);

        assert_frame(&frame, &["", ""], None);
    }

    #[test]
    fn returns_blank_frame_without_views() {
        let view = VerticalBoxView::new(Vec::new(), 0, 0);

        let frame = view.render(10, 3);

        assert_frame(&frame, &["", "", ""], None);
    }
}
