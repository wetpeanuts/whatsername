use crate::tui::engine::{
    AppHandle, Frame, View,
    event::{CloseEvent, Event, Key, KeyEvent},
};
use crate::tui::view::TextEditView;

pub struct CommandView {
    app_handle: AppHandle,
    editor: TextEditView,
}

impl CommandView {
    pub fn new(app_handle: AppHandle, command: String) -> Self {
        Self {
            app_handle,
            editor: TextEditView::new("Command", command),
        }
    }

    fn close(&self) {
        self.app_handle.push(Event::Close(CloseEvent::new(130)));
    }
}

impl View for CommandView {
    fn key_event(&mut self, event: KeyEvent) {
        match event.key {
            Key::Escape | Key::CtrlC | Key::CtrlD => self.close(),
            _ => self.editor.key_event(event),
        }
    }

    fn render(&self, width: usize, height: usize) -> Frame {
        self.editor.render(width, height)
    }

    fn update(&mut self) {
        self.editor.update();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_received_command_for_editing() {
        let view = CommandView::new(AppHandle::new_mock(), String::from("ls"));

        assert_eq!(
            view.render(12, 3).rows(),
            &["╭─Command──╮", "│ > ls     │", "╰──────────╯"],
        );
    }

    #[test]
    fn received_command_is_editable() {
        let mut view = CommandView::new(AppHandle::new_mock(), String::from("ls"));

        view.key_event(KeyEvent::new(Key::Char(' ')));
        view.key_event(KeyEvent::new(Key::Char('-')));
        view.key_event(KeyEvent::new(Key::Char('l')));

        assert_eq!(view.editor.text(), "ls -l");
    }
}
