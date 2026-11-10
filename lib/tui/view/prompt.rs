use std::sync::mpsc::Sender;

use crate::tui::engine::{
    AppHandle, Frame, View,
    event::{CloseEvent, Event, Key, KeyEvent},
};
use crate::tui::view::TextEditView;

pub struct PromptView {
    app_handle: AppHandle,
    editor: TextEditView,
    command_tx: Sender<String>,
}

impl PromptView {
    pub fn new(app_handle: AppHandle, command_tx: Sender<String>) -> Self {
        Self {
            app_handle,
            editor: TextEditView::new("Prompt", String::new()),
            command_tx,
        }
    }

    fn close(&self) {
        self.app_handle.push(Event::Close(CloseEvent::new(130)));
    }

    fn accept_command(&mut self) {
        if self.editor.text().is_empty() {
            return;
        }

        let _ = self.command_tx.send(self.editor.text().to_string());
        self.app_handle.push(Event::Update);
    }
}

impl View for PromptView {
    fn key_event(&mut self, event: KeyEvent) {
        match event.key {
            Key::Enter => self.accept_command(),
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

    fn type_text(view: &mut PromptView, text: &str) {
        for c in text.chars() {
            view.key_event(KeyEvent::new(Key::Char(c)));
        }
    }

    #[test]
    fn enter_sends_prompt_to_command_channel() {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let mut view = PromptView::new(AppHandle::new_mock(), command_tx);
        type_text(&mut view, "ls");

        view.key_event(KeyEvent::new(Key::Enter));

        assert_eq!(command_rx.try_recv(), Ok(String::from("ls")));
    }

    #[test]
    fn enter_ignores_empty_prompt() {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let mut view = PromptView::new(AppHandle::new_mock(), command_tx);

        view.key_event(KeyEvent::new(Key::Enter));

        assert!(command_rx.try_recv().is_err());
    }
}
