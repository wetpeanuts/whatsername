use crate::tui::engine::{
    Frame, Terminal, View,
    event::{Event, KeyParser},
};

use anyhow::Result;
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};

const INITIAL_EVENT_QUEUE_CAPACITY: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameLayout {
    top: usize,
    height: usize,
}

pub struct App<V: View> {
    term: Terminal,
    event_queue: VecDeque<Event>,
    event_rx: Receiver<Event>,
    key_parser: KeyParser,
    view: V,
    previous_layout: Option<FrameLayout>,
}

#[derive(Clone)]
pub struct AppHandle {
    tx: Sender<Event>,
}

impl AppHandle {
    fn new(tx: Sender<Event>) -> Self {
        Self { tx }
    }

    #[cfg(test)]
    pub(crate) fn new_mock() -> Self {
        let (tx, _) = mpsc::channel();
        Self { tx }
    }

    #[cfg(test)]
    pub(crate) fn new_mock_with_rx() -> (Self, Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        (Self { tx }, rx)
    }

    pub fn push(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}

impl<V: View> App<V> {
    pub fn new<F>(make_view: F) -> Result<Self>
    where
        F: FnOnce(AppHandle) -> V,
    {
        let (tx, rx) = mpsc::channel();
        let view = make_view(AppHandle::new(tx));

        Ok(Self {
            term: Terminal::new()?,
            event_queue: VecDeque::with_capacity(INITIAL_EVENT_QUEUE_CAPACITY),
            event_rx: rx,
            key_parser: KeyParser::new(),
            view: view,
            previous_layout: None,
        })
    }

    pub fn view(&self) -> &V {
        &self.view
    }

    pub fn exec(&mut self) -> Result<i32> {
        self.render_view()?;

        loop {
            self.parse_key()?;
            self.drain_external_events();

            while let Some(event) = self.event_queue.pop_front() {
                match event {
                    Event::Close(close_event) => return Ok(close_event.status_code),
                    Event::Error(message) => return Err(anyhow::anyhow!(message)),
                    Event::Key(key_event) => {
                        self.view.key_event(key_event);
                        self.render_view()?;
                    }
                    Event::Render => self.render_view()?,
                    Event::Update => {
                        self.view.update();
                        self.render_view()?
                    }
                }

                self.drain_external_events();
            }
        }
    }

    fn render_view(&mut self) -> Result<()> {
        let (width, height) = self.term.dim();
        let frame = self.view.render(width, height);
        let (rendered, layout) = compose_frame(self.previous_layout, &frame, height);

        self.term.write_str(&rendered)?;
        self.previous_layout = Some(layout);

        Ok(())
    }

    fn drain_external_events(&mut self) {
        // Fetch events produced by views and other sources through AppHandler
        self.event_queue.extend(self.event_rx.try_iter());
    }

    fn parse_key(&mut self) -> Result<()> {
        let mut buf = [0u8; 256];
        let read = self.term.read(&mut buf)?;

        let key_events = if read == 0 {
            self.key_parser.flush()
        } else {
            self.key_parser.feed(&buf[..read])
        };

        self.event_queue
            .extend(key_events.into_iter().map(Event::Key));

        Ok(())
    }
}

fn compose_frame(
    previous: Option<FrameLayout>,
    frame: &Frame,
    height: usize,
) -> (String, FrameLayout) {
    debug_assert!(
        frame.height() <= height,
        "frame height {} exceeds terminal height {}",
        frame.height(),
        height
    );

    let mut out = String::new();

    if let Some(previous) = previous {
        let clear_top = previous.top.min(height.saturating_sub(1));
        out.push_str(&format!("\x1b[{};1H\x1b[J", clear_top + 1));
    }

    let top = height.saturating_sub(frame.height());
    for (index, row) in frame.rows().iter().enumerate() {
        out.push_str(&format!("\x1b[{};1H", top + index + 1));
        out.push_str(row);
    }

    if let Some((row, col)) = frame.cursor() {
        out.push_str(&format!("\x1b[{};{}H", top + row + 1, col + 1));
    }

    (
        out,
        FrameLayout {
            top,
            height: frame.height(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_initial_frame() {
        let frame = Frame::new(vec![String::from("abc"), String::from("def")], Some((0, 1)));

        let (rendered, layout) = compose_frame(None, &frame, 10);

        assert_eq!(rendered, "\x1b[9;1Habc\x1b[10;1Hdef\x1b[9;2H");
        assert_eq!(layout, FrameLayout { top: 8, height: 2 });
    }

    #[test]
    fn clears_previous_frame_before_drawing() {
        let frame = Frame::new(vec![String::from("xy")], None);

        let (rendered, layout) = compose_frame(Some(FrameLayout { top: 8, height: 2 }), &frame, 10);

        assert_eq!(rendered, "\x1b[9;1H\x1b[J\x1b[10;1Hxy");
        assert_eq!(layout, FrameLayout { top: 9, height: 1 });
    }

    #[test]
    fn composes_view_switch_with_different_height() {
        let frame = Frame::new(
            vec![
                String::from("one"),
                String::from("two"),
                String::from("three"),
            ],
            Some((2, 2)),
        );

        let (rendered, layout) = compose_frame(Some(FrameLayout { top: 9, height: 1 }), &frame, 10);

        assert_eq!(
            rendered,
            "\x1b[10;1H\x1b[J\x1b[8;1Hone\x1b[9;1Htwo\x1b[10;1Hthree\x1b[10;3H"
        );
        assert_eq!(layout, FrameLayout { top: 7, height: 3 });
    }

    #[test]
    fn clamps_previous_layout_on_resize() {
        let frame = Frame::new(vec![String::from("z")], None);

        let (rendered, layout) = compose_frame(Some(FrameLayout { top: 8, height: 2 }), &frame, 5);

        assert_eq!(rendered, "\x1b[5;1H\x1b[J\x1b[5;1Hz");
        assert_eq!(layout, FrameLayout { top: 4, height: 1 });
    }
}
