use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use crate::tui::engine::{
    AppHandle, Frame, View,
    event::{CloseEvent, Event, Key, KeyEvent},
};
use crate::tui::layout::VerticalLayout;
use crate::tui::view::{SystemInfoView, TextEditView};

const SPACING: usize = 1;
const FOCUS: usize = 1;
const AGENT_ENV_VAR: &str = "WHATSERNAME_AGENT";
const AGENT_DEFAULT_VALUE: &str = "opencode run --print-logs=false";
const AGENT_PROMPT_PREFIX: &str = "a request to generate a shell command that does the following: ";
const AGENT_PROMPT_SUFFIX: &str =
    ". the output should be in the format ```command_here``` and no other output.";

trait AgentRunner: Send + Sync {
    fn run(&self, agent_cmd: &str, prompt: &str, pid: &AtomicI32) -> Result<String, String>;
}

struct CommandAgent;

impl AgentRunner for CommandAgent {
    fn run(&self, agent_cmd: &str, prompt: &str, pid: &AtomicI32) -> Result<String, String> {
        let child = Command::new("sh")
            .arg("-c")
            .arg(format!("exec {agent_cmd} \"$1\""))
            .arg("sh")
            .arg(prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to start agent: {error}"))?;

        pid.store(child.id() as i32, Ordering::SeqCst);

        let output = child
            .wait_with_output()
            .map_err(|error| format!("failed to wait for agent: {error}"))?;

        pid.store(0, Ordering::SeqCst);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "agent exited with {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

fn build_agent_prompt(text: &str, system_info: &str) -> String {
    format!("{AGENT_PROMPT_PREFIX}{text}{AGENT_PROMPT_SUFFIX}\n\n{system_info}")
}

fn extract_command(stdout: &str) -> Option<String> {
    let block = stdout.split("```").nth(1)?;
    let block = block
        .strip_prefix("bash\n")
        .or_else(|| block.strip_prefix("sh\n"))
        .unwrap_or(block)
        .trim();

    if block.is_empty() {
        None
    } else {
        Some(block.to_string())
    }
}

struct PromptContent {
    system_info_view: SystemInfoView,
    prompt_editor_view: TextEditView,
}

impl PromptContent {
    fn views(&self) -> [&dyn View; 2] {
        [&self.system_info_view, &self.prompt_editor_view]
    }

    fn views_mut(&mut self) -> [&mut dyn View; 2] {
        [&mut self.system_info_view, &mut self.prompt_editor_view]
    }
}

pub struct PromptView {
    app_handle: AppHandle,
    content: PromptContent,
    layout: VerticalLayout,
    command_tx: Sender<String>,
    agent_cmd: String,
    agent_runner: Arc<dyn AgentRunner>,
    agent_pid: Arc<AtomicI32>,
    agent_rx: Option<Receiver<Result<String, String>>>,
    running: bool,
}

impl PromptView {
    pub fn new(app_handle: AppHandle, command_tx: Sender<String>) -> Self {
        let agent_cmd =
            std::env::var(AGENT_ENV_VAR).unwrap_or_else(|_| AGENT_DEFAULT_VALUE.to_string());

        Self::with_agent(app_handle, command_tx, agent_cmd, Arc::new(CommandAgent))
    }

    fn with_agent(
        app_handle: AppHandle,
        command_tx: Sender<String>,
        agent_cmd: String,
        agent_runner: Arc<dyn AgentRunner>,
    ) -> Self {
        let content = PromptContent {
            system_info_view: SystemInfoView::new(),
            prompt_editor_view: TextEditView::new("Prompt", String::new()),
        };
        let layout = VerticalLayout::new(SPACING, FOCUS);

        Self {
            app_handle,
            content,
            layout,
            command_tx,
            agent_cmd,
            agent_runner,
            agent_pid: Arc::new(AtomicI32::new(0)),
            agent_rx: None,
            running: false,
        }
    }

    fn close(&self) {
        self.kill_agent();
        self.app_handle.push(Event::Close(CloseEvent::new(130)));
    }

    fn kill_agent(&self) {
        let pid = self.agent_pid.load(Ordering::SeqCst);
        if pid > 0 {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
    }

    fn start_agent(&mut self) {
        let text = self.content.prompt_editor_view.text().to_string();
        if text.is_empty() {
            return;
        }

        let (tx, rx) = mpsc::channel();
        let app_handle = self.app_handle.clone();
        let agent_cmd = self.agent_cmd.clone();
        let agent_runner = Arc::clone(&self.agent_runner);
        let agent_pid = Arc::clone(&self.agent_pid);
        let system_info = self.content.system_info_view.system_info();
        let prompt = build_agent_prompt(&text, &system_info);

        self.running = true;
        self.agent_rx = Some(rx);
        self.content.prompt_editor_view.set_disabled(true);
        self.app_handle.push(Event::Update);

        thread::spawn(move || {
            let result = agent_runner.run(&agent_cmd, &prompt, &agent_pid);
            let _ = tx.send(result);
            let _ = app_handle.push(Event::Update);
        });
    }

    fn poll_agent(&mut self) {
        let result = match &self.agent_rx {
            Some(rx) => match rx.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    Some(Err(String::from("agent task ended unexpectedly")))
                }
            },
            None => None,
        };

        if let Some(result) = result {
            self.finish_agent(result);
        }
    }

    fn finish_agent(&mut self, result: Result<String, String>) {
        self.running = false;
        self.agent_rx = None;
        self.content.prompt_editor_view.set_disabled(false);

        match result {
            Ok(stdout) => match extract_command(&stdout) {
                Some(command) => {
                    let _ = self.command_tx.send(command);
                }
                None => self.app_handle.push(Event::Error(format!(
                    "agent output did not contain a ```command``` block: {}",
                    stdout.trim()
                ))),
            },
            Err(message) => self.app_handle.push(Event::Error(message)),
        }
    }
}

impl View for PromptView {
    fn key_event(&mut self, event: KeyEvent) {
        match event.key {
            Key::Escape | Key::CtrlC | Key::CtrlD => self.close(),
            _ if self.running => {}
            Key::Enter => self.start_agent(),
            _ => self.layout.key_event(&mut self.content.views_mut(), event),
        }
    }

    fn render(&self, width: usize, height: usize) -> Frame {
        self.layout.render(&self.content.views(), width, height)
    }

    fn update(&mut self) {
        self.poll_agent();
        self.layout.update(&mut self.content.views_mut());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Condvar, Mutex};
    use std::time::{Duration, Instant};

    struct FakeAgent {
        output: Result<String, String>,
        gate: Option<Arc<(Mutex<bool>, Condvar)>>,
    }

    impl AgentRunner for FakeAgent {
        fn run(&self, _agent_cmd: &str, _prompt: &str, _pid: &AtomicI32) -> Result<String, String> {
            if let Some(gate) = &self.gate {
                let (lock, condvar) = &**gate;
                let mut released = lock.lock().unwrap();
                while !*released {
                    released = condvar.wait(released).unwrap();
                }
            }

            self.output.clone()
        }
    }

    fn prompt_view(
        output: Result<String, String>,
        gate: Option<Arc<(Mutex<bool>, Condvar)>>,
    ) -> (PromptView, Receiver<String>, Receiver<Event>) {
        let (command_tx, command_rx) = mpsc::channel();
        let (app_handle, app_rx) = AppHandle::new_mock_with_rx();
        let runner: Arc<dyn AgentRunner> = Arc::new(FakeAgent { output, gate });
        let view = PromptView::with_agent(app_handle, command_tx, String::from("agent"), runner);

        (view, command_rx, app_rx)
    }

    fn type_text(view: &mut PromptView, text: &str) {
        for c in text.chars() {
            view.key_event(KeyEvent::new(Key::Char(c)));
        }
    }

    fn wait_for_command(view: &mut PromptView, rx: &Receiver<String>) -> Option<String> {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            view.update();
            if let Ok(command) = rx.try_recv() {
                return Some(command);
            }
            thread::sleep(Duration::from_millis(1));
        }

        None
    }

    fn wait_for_error(view: &mut PromptView, rx: &Receiver<Event>) -> Option<String> {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            view.update();
            while let Ok(event) = rx.try_recv() {
                if let Event::Error(message) = event {
                    return Some(message);
                }
            }
            thread::sleep(Duration::from_millis(1));
        }

        None
    }

    #[test]
    fn enter_runs_agent_and_sends_extracted_command() {
        let (mut view, command_rx, _app_rx) = prompt_view(Ok(String::from("```ls -la```")), None);
        type_text(&mut view, "list files");

        view.key_event(KeyEvent::new(Key::Enter));

        assert_eq!(
            wait_for_command(&mut view, &command_rx),
            Some(String::from("ls -la"))
        );
    }

    #[test]
    fn enter_ignores_empty_prompt() {
        let (mut view, command_rx, _app_rx) = prompt_view(Ok(String::from("```ls```")), None);

        view.key_event(KeyEvent::new(Key::Enter));

        assert!(!view.running);
        assert!(command_rx.try_recv().is_err());
    }

    #[test]
    fn running_view_ignores_editing_keys_and_hides_cursor() {
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let (mut view, command_rx, _app_rx) =
            prompt_view(Ok(String::from("```ls```")), Some(Arc::clone(&gate)));
        type_text(&mut view, "ls");

        view.key_event(KeyEvent::new(Key::Enter));
        view.key_event(KeyEvent::new(Key::Char('x')));

        assert!(view.running);
        assert_eq!(view.content.prompt_editor_view.text(), "ls");
        let frame = view.render(12, 10);
        assert_eq!(frame.cursor(), None);
        assert!(frame.rows().iter().any(|row| row.contains("\x1b[90m")));

        let (lock, condvar) = &*gate;
        *lock.lock().unwrap() = true;
        condvar.notify_all();

        assert_eq!(
            wait_for_command(&mut view, &command_rx),
            Some(String::from("ls"))
        );
    }

    #[test]
    fn escape_while_running_closes_the_app() {
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let (mut view, _command_rx, app_rx) =
            prompt_view(Ok(String::from("```ls```")), Some(Arc::clone(&gate)));
        type_text(&mut view, "ls");
        view.key_event(KeyEvent::new(Key::Enter));

        view.key_event(KeyEvent::new(Key::Escape));

        let status = loop {
            match app_rx.try_recv() {
                Ok(Event::Close(event)) => break Some(event.status_code),
                Ok(_) => continue,
                Err(_) => break None,
            }
        };

        assert_eq!(status, Some(130));

        let (lock, condvar) = &*gate;
        *lock.lock().unwrap() = true;
        condvar.notify_all();
    }

    #[test]
    fn agent_failure_propagates_error() {
        let (mut view, _command_rx, app_rx) =
            prompt_view(Err(String::from("agent exploded")), None);
        type_text(&mut view, "ls");

        view.key_event(KeyEvent::new(Key::Enter));

        assert_eq!(
            wait_for_error(&mut view, &app_rx),
            Some(String::from("agent exploded"))
        );
    }

    #[test]
    fn missing_fenced_block_propagates_error() {
        let (mut view, _command_rx, app_rx) = prompt_view(Ok(String::from("ls")), None);
        type_text(&mut view, "ls");

        view.key_event(KeyEvent::new(Key::Enter));

        assert!(
            wait_for_error(&mut view, &app_rx)
                .is_some_and(|message| message.contains("did not contain"))
        );
    }

    #[test]
    fn renders_system_info_above_editor_and_offsets_cursor() {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let mut view = PromptView::new(AppHandle::new_mock(), command_tx);
        view.content.system_info_view = SystemInfoView::from_parts("/home/user", "x86_64");

        let frame = view.render(12, 10);

        assert_eq!(
            frame.rows(),
            &[
                "PWD:   /home/user",
                "OS:    linux",
                "Arch:  x86_64",
                "Shell: bash",
                "",
                "╭─Prompt───╮",
                "│ >        │",
                "╰──────────╯",
            ],
        );
        assert_eq!(frame.cursor(), Some((6, 4)));
    }

    #[test]
    fn builds_agent_prompt_with_request_format_and_system_info() {
        let system_info = SystemInfoView::from_parts("/home/user", "x86_64").system_info();
        let prompt = build_agent_prompt("list files", &system_info);

        assert_eq!(
            prompt,
            "a request to generate a shell command that does the following: list files. \
             the output should be in the format ```command_here``` and no other output.\n\n\
             System info:\nCurrent working directory: /home/user\nArchitecture: x86_64\n\
             Operating system: linux\nShell: bash\nUser: user (root: false)"
        );
    }

    #[test]
    fn extracts_command_from_fenced_block() {
        assert_eq!(
            extract_command("```ls -la```"),
            Some(String::from("ls -la"))
        );
    }

    #[test]
    fn strips_language_tag_from_fenced_block() {
        assert_eq!(
            extract_command("```bash\nls -la\n```"),
            Some(String::from("ls -la"))
        );
        assert_eq!(
            extract_command("```sh\nls -la\n```"),
            Some(String::from("ls -la"))
        );
    }

    #[test]
    fn ignores_output_without_fenced_block() {
        assert_eq!(extract_command("ls -la"), None);
        assert_eq!(extract_command("```   ```"), None);
    }
}
