use crate::tui::engine::{
    AppHandle, Frame, View,
    event::{CloseEvent, Event, KeyEvent},
};
use crate::tui::view::{CommandView, PromptView};

use std::boxed::Box;
use std::sync::mpsc::{self, Receiver};

pub struct AppView {
    app_handle: AppHandle,
    active_view: Box<dyn View>,
    command_rx: Receiver<String>,
    execution_rx: Option<Receiver<String>>,
    command: Option<String>,
}

impl AppView {
    pub fn new(app_handle: AppHandle) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let active_view = Box::new(PromptView::new(app_handle.clone(), command_tx));

        Self {
            app_handle,
            active_view,
            command_rx,
            execution_rx: None,
            command: None,
        }
    }

    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }
}

impl View for AppView {
    fn key_event(&mut self, event: KeyEvent) {
        self.active_view.key_event(event)
    }

    fn render(&self, width: usize, height: usize) -> Frame {
        self.active_view.render(width, height)
    }

    fn update(&mut self) {
        self.active_view.update();

        if let Ok(command) = self.command_rx.try_recv() {
            let (execution_tx, execution_rx) = mpsc::channel();
            self.execution_rx = Some(execution_rx);
            self.active_view = Box::new(CommandView::new(
                self.app_handle.clone(),
                command,
                execution_tx,
            ));
        }

        if let Some(execution_rx) = &self.execution_rx {
            if let Ok(command) = execution_rx.try_recv() {
                self.command = Some(command);
                self.execution_rx = None;
                self.app_handle.push(Event::Close(CloseEvent::new(0)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::engine::{
        View,
        event::{Key, KeyEvent},
    };
    use crate::tui::view::TextView;
    use std::sync::mpsc::{self, Receiver, Sender};

    fn app_view() -> (AppView, Sender<String>, Receiver<Event>) {
        let (command_tx, command_rx) = mpsc::channel();
        let (app_handle, app_rx) = AppHandle::new_mock_with_rx();
        let view = AppView {
            app_handle,
            active_view: Box::new(TextView::new(String::new())),
            command_rx,
            execution_rx: None,
            command: None,
        };

        (view, command_tx, app_rx)
    }

    #[test]
    fn command_enters_execution_state_and_closes_with_status_zero() {
        let (mut view, command_tx, app_rx) = app_view();

        command_tx.send(String::from("ls")).unwrap();
        view.update();
        assert_eq!(view.command(), None);

        view.key_event(KeyEvent::new(Key::Char(' ')));
        view.key_event(KeyEvent::new(Key::Char('l')));
        view.key_event(KeyEvent::new(Key::Enter));
        view.update();

        assert_eq!(view.command(), Some("ls l"));
        assert!(matches!(app_rx.try_recv(), Ok(Event::Update)));
        match app_rx.try_recv() {
            Ok(Event::Close(event)) => assert_eq!(event.status_code, 0),
            _ => panic!("expected close event"),
        }
    }

    #[test]
    fn empty_command_keeps_app_running() {
        let (mut view, command_tx, app_rx) = app_view();

        command_tx.send(String::from("  ")).unwrap();
        view.update();
        view.key_event(KeyEvent::new(Key::Enter));
        view.update();

        assert_eq!(view.command(), None);
        assert!(app_rx.try_recv().is_err());
    }
}
