use crate::tui::engine::event::CloseEvent;
use crate::tui::engine::event::KeyEvent;

pub enum Event {
    Key(KeyEvent),     // User input
    Close(CloseEvent), // Exit event loop
    Render,            // Trigger view rerendering
    Update,            // Trigger view update + rerendering
}
