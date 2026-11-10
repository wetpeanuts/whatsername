pub struct Frame {
    rows: Vec<String>,
    cursor: Option<(usize, usize)>,
}

impl Frame {
    pub fn new(rows: Vec<String>, cursor: Option<(usize, usize)>) -> Self {
        assert!(!rows.is_empty(), "frame must contain at least one row");

        if let Some((row, col)) = cursor {
            assert!(row < rows.len(), "cursor row is outside the frame");
            assert!(
                col <= rows[row].chars().count(),
                "cursor column is outside the frame row"
            );
        }

        Self { rows, cursor }
    }

    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    pub fn cursor(&self) -> Option<(usize, usize)> {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_rows_and_cursor() {
        let frame = Frame::new(vec![String::from("abc"), String::from("def")], Some((1, 2)));

        assert_eq!(frame.rows(), &["abc", "def"]);
        assert_eq!(frame.height(), 2);
        assert_eq!(frame.cursor(), Some((1, 2)));
    }

    #[test]
    #[should_panic]
    fn rejects_empty_frames() {
        Frame::new(Vec::new(), None);
    }

    #[test]
    #[should_panic]
    fn rejects_cursor_row_outside_frame() {
        Frame::new(vec![String::from("abc")], Some((1, 0)));
    }

    #[test]
    #[should_panic]
    fn rejects_cursor_column_outside_row() {
        Frame::new(vec![String::from("abc")], Some((0, 4)));
    }
}
