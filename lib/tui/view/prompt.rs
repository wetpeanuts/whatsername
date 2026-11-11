use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use crate::tui::engine::{
    AppHandle, Frame, View,
    event::{CloseEvent, Event, Key, KeyEvent},
};
use crate::tui::view::{TextEditView, TextView, VerticalBoxView};

const TEXT: &str = "hello\nworld";
const SPACING: usize = 1;
const FOCUS: usize = 1;
const AGENT_ENV_VAR: &str = "WHATSERNAME_AGENT";
const AGENT_DEFAULT_VALUE: &str = "opencode run --print-logs=false";

struct EditorView {
    editor: Rc<RefCell<TextEditView>>,
}

impl View for EditorView {
    fn key_event(&mut self, event: KeyEvent) {
        self.editor.borrow_mut().key_event(event);
    }

    fn render(&self, width: usize, height: usize) -> Frame {
        self.editor.borrow().render(width, height)
    }

    fn update(&mut self) {
        self.editor.borrow_mut().update();
    }
}

pub struct PromptView {
    app_handle: AppHandle,
    editor: Rc<RefCell<TextEditView>>,
    content: VerticalBoxView,
    command_tx: Sender<String>,
    agent_cmd: String,
}

impl PromptView {
    pub fn new(app_handle: AppHandle, command_tx: Sender<String>) -> Self {
        let editor = Rc::new(RefCell::new(TextEditView::new("Prompt", String::new())));
        let content = VerticalBoxView::new(
            vec![
                Box::new(TextView::new(String::from(TEXT))),
                Box::new(EditorView {
                    editor: Rc::clone(&editor),
                }),
            ],
            SPACING,
            FOCUS,
        );

        let agent_cmd = std::env::var(AGENT_ENV_VAR)
            .unwrap_or_else(|_| AGENT_DEFAULT_VALUE.to_string());

        Self {
            app_handle,
            editor,
            content,
            command_tx,
            agent_cmd,
        }
    }

    fn close(&self) {
        self.app_handle.push(Event::Close(CloseEvent::new(130)));
    }

    fn accept_command(&mut self) {
        let text = self.editor.borrow().text().to_string();
        if text.is_empty() {
            return;
        }

        let _ = self.command_tx.send(text);
        self.app_handle.push(Event::Update);
    }
}

impl View for PromptView {
    fn key_event(&mut self, event: KeyEvent) {
        match event.key {
            Key::Enter => self.accept_command(),
            Key::Escape | Key::CtrlC | Key::CtrlD => self.close(),
            _ => self.content.key_event(event),
        }
    }

    fn render(&self, width: usize, height: usize) -> Frame {
        self.content.render(width, height)
    }

    fn update(&mut self) {
        self.content.update();
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

    #[test]
    fn renders_text_above_editor_and_offsets_cursor() {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let view = PromptView::new(AppHandle::new_mock(), command_tx);

        let frame = view.render(12, 10);

        assert_eq!(
            frame.rows(),
            &[
                "hello",
                "world",
                "",
                "╭─Prompt───╮",
                "│ >        │",
                "╰──────────╯",
            ],
        );
        assert_eq!(frame.cursor(), Some((4, 4)));
    }
}
