use std::process::Command;

use whatsername::tui::engine::App;
use whatsername::tui::view::AppView;

const SHELL_ENV_VAR: &str = "SHELL";
const DEFAULT_SHELL: &str = "/bin/sh";

fn run_in_shell(command: &str) -> i32 {
    let shell = std::env::var(SHELL_ENV_VAR).unwrap_or_else(|_| DEFAULT_SHELL.to_string());

    match Command::new(shell).arg("-c").arg(command).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("whatsername: failed to run command: {error}");
            1
        }
    }
}

fn main() {
    let mut app = match App::new(AppView::new) {
        Ok(app) => app,
        Err(error) => {
            eprintln!("whatsername: {error}");
            std::process::exit(1);
        }
    };

    let result = app.exec();
    let command = app.view().command().map(str::to_owned);
    drop(app);

    match result {
        Ok(status) => match command {
            Some(command) => std::process::exit(run_in_shell(&command)),
            None => std::process::exit(status),
        },
        Err(error) => {
            eprintln!("whatsername: {error}");
            std::process::exit(1);
        }
    }
}
