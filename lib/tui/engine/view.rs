use crate::tui::engine::{Frame, event::KeyEvent};

pub trait View {
    fn render(&self, width: usize, height: usize) -> Frame;
    fn key_event(&mut self, event: KeyEvent);
    fn update(&mut self);
}
