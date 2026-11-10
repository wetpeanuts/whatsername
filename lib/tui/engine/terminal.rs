use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::process::Command;

const DEFAULT_TERM_WIDTH: usize = 80;
const DEFAULT_TERM_HEIGHT: usize = 24;
const ENTER_ALT_SCREEN: &str = "\x1b[?1049h\x1b[H\x1b[2J";
const LEAVE_ALT_SCREEN: &str = "\x1b[0m\x1b[?25h\x1b[?1049l";

pub struct Terminal {
    tty: File,
    saved_settings: String,
}

impl Terminal {
    pub fn new() -> Result<Self> {
        let tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
        let output = Command::new("stty")
            .args(["-f", "/dev/tty", "-g"])
            .output()?;
        let saved = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let mut terminal = Terminal {
            tty,
            saved_settings: saved,
        };
        Command::new("stty")
            .args(["-f", "/dev/tty", "raw", "-echo", "min", "0", "time", "1"])
            .status()?;
        terminal.write_str(ENTER_ALT_SCREEN)?;
        Ok(terminal)
    }

    pub fn write_str(&mut self, s: &str) -> Result<()> {
        self.tty.write_all(s.as_bytes())?;
        self.tty.flush()?;

        Ok(())
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let bytes_read = self.tty.read(buf)?;

        Ok(bytes_read)
    }

    /// Returns (width, height) in chars for the current terminal window
    pub fn dim(&self) -> (usize, usize) {
        let output = Command::new("stty")
            .args(["-f", "/dev/tty", "size"])
            .output();
        let text = output
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        let mut parts = text.split_whitespace();
        let rows = parts
            .next()
            .and_then(|c| c.parse().ok())
            .unwrap_or(DEFAULT_TERM_HEIGHT);
        let cols = parts
            .next()
            .and_then(|c| c.parse().ok())
            .unwrap_or(DEFAULT_TERM_WIDTH);
        (cols, rows)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.tty.write_all(LEAVE_ALT_SCREEN.as_bytes());
        let _ = self.tty.flush();
        let _ = Command::new("stty")
            .arg("-f")
            .arg("/dev/tty")
            .arg(&self.saved_settings)
            .status();
    }
}
