use crate::tui::engine::{
    AppHandle, Frame, View,
    event::{CloseEvent, Event, Key, KeyEvent},
};
use crate::tui::view::TextEditView;

use std::sync::mpsc::Sender;

pub struct CommandView {
    app_handle: AppHandle,
    editor: TextEditView,
    execution_tx: Sender<String>,
}

impl CommandView {
    pub fn new(app_handle: AppHandle, command: String, execution_tx: Sender<String>) -> Self {
        Self {
            app_handle,
            editor: TextEditView::new("Command", command),
            execution_tx,
        }
    }

    fn close(&self) {
        self.app_handle.push(Event::Close(CloseEvent::new(130)));
    }

    fn execute(&self) {
        let command = self.editor.text();
        if command.trim().is_empty() {
            return;
        }

        let _ = self.execution_tx.send(command.to_string());
        self.app_handle.push(Event::Update);
    }
}

impl View for CommandView {
    fn key_event(&mut self, event: KeyEvent) {
        match event.key {
            Key::Escape | Key::CtrlC | Key::CtrlD => self.close(),
            Key::Enter => self.execute(),
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
    use std::sync::mpsc::{self, Receiver};

    fn command_view(command: &str) -> (CommandView, Receiver<String>, Receiver<Event>) {
        let (execution_tx, execution_rx) = mpsc::channel();
        let (app_handle, app_rx) = AppHandle::new_mock_with_rx();
        let view = CommandView::new(app_handle, command.to_string(), execution_tx);

        (view, execution_rx, app_rx)
    }

    #[test]
    fn renders_received_command_for_editing() {
        let (view, _execution_rx, _app_rx) = command_view("ls");

        assert_eq!(
            view.render(12, 3).rows(),
            &["╭─Command──╮", "│ > ls     │", "╰──────────╯"],
        );
    }

    #[test]
    fn received_command_is_editable() {
        let (mut view, _execution_rx, _app_rx) = command_view("ls");

        view.key_event(KeyEvent::new(Key::Char(' ')));
        view.key_event(KeyEvent::new(Key::Char('-')));
        view.key_event(KeyEvent::new(Key::Char('l')));

        assert_eq!(view.editor.text(), "ls -l");
    }

    #[test]
    fn enter_sends_edited_command_and_requests_update() {
        let (mut view, execution_rx, app_rx) = command_view("ls");

        view.key_event(KeyEvent::new(Key::Char(' ')));
        view.key_event(KeyEvent::new(Key::Char('-')));
        view.key_event(KeyEvent::new(Key::Char('l')));
        view.key_event(KeyEvent::new(Key::Enter));

        assert_eq!(execution_rx.try_recv(), Ok(String::from("ls -l")));
        assert!(matches!(app_rx.try_recv(), Ok(Event::Update)));
        assert_eq!(view.editor.text(), "ls -l");
    }

    #[test]
    fn enter_preserves_trailing_whitespace() {
        let (mut view, execution_rx, _app_rx) = command_view("ls");

        view.key_event(KeyEvent::new(Key::Char(' ')));
        view.key_event(KeyEvent::new(Key::Enter));

        assert_eq!(execution_rx.try_recv(), Ok(String::from("ls ")));
    }

    #[test]
    fn enter_ignores_empty_or_whitespace_only_command() {
        for command in ["", "   "] {
            let (mut view, execution_rx, app_rx) = command_view(command);

            view.key_event(KeyEvent::new(Key::Enter));

            assert!(execution_rx.try_recv().is_err());
            assert!(app_rx.try_recv().is_err());
        }
    }

    #[test]
    fn escape_closes_with_status_130() {
        let (mut view, _execution_rx, app_rx) = command_view("ls");

        view.key_event(KeyEvent::new(Key::Escape));

        match app_rx.try_recv() {
            Ok(Event::Close(event)) => assert_eq!(event.status_code, 130),
            _ => panic!("expected close event"),
        }
    }
}
