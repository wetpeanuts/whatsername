use crate::tui::engine::{AppHandle, Frame, View, event::KeyEvent};
use crate::tui::view::{CommandView, PromptView};

use std::boxed::Box;
use std::sync::mpsc::{self, Receiver};

pub struct AppView {
    app_handle: AppHandle,
    active_view: Box<dyn View>,
    command_rx: Receiver<String>,
}

impl AppView {
    pub fn new(app_handle: AppHandle) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let active_view = Box::new(PromptView::new(app_handle.clone(), command_tx));

        Self {
            app_handle,
            active_view,
            command_rx,
        }
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
            self.active_view = Box::new(CommandView::new(self.app_handle.clone(), command));
        }
    }
}
