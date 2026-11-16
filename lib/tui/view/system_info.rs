use std::path::Path;
use std::process::Command;

use crate::tui::engine::{Frame, View, event::KeyEvent};
use crate::tui::view::TextView;

const PWD_ENV_VAR: &str = "PWD";
const SHELL_ENV_VAR: &str = "SHELL";
const USER_ENV_VAR: &str = "USER";
const LOGNAME_ENV_VAR: &str = "LOGNAME";
const UNAME_COMMAND: &str = "uname";
const ARCH_ARGUMENT: &str = "-m";
const OS_ARGUMENT: &str = "-s";
const ID_COMMAND: &str = "id";
const USER_ARGUMENT: &str = "-un";
const UID_ARGUMENT: &str = "-u";
const GIT_COMMAND: &str = "git";
const SW_VERS_COMMAND: &str = "sw_vers";
const OS_RELEASE_PATH: &str = "/etc/os-release";
const UNKNOWN: &str = "unknown";

struct GitInfo {
    root: String,
    branch: String,
    dirty: bool,
}

struct SystemInfo {
    pwd: String,
    arch: String,
    os: String,
    distro: Option<String>,
    shell: String,
    user: String,
    is_root: bool,
    git: Option<GitInfo>,
}

impl SystemInfo {
    fn resolve() -> Self {
        let pwd = resolve_pwd();
        let arch = resolve_arch();
        let os = resolve_os();
        let distro = resolve_distro(&os);
        let shell = resolve_shell();
        let user = resolve_user();
        let is_root = resolve_is_root();
        let git = resolve_git();

        Self {
            pwd,
            arch,
            os,
            distro,
            shell,
            user,
            is_root,
            git,
        }
    }

    fn render(&self) -> String {
        let mut info = format!(
            "System info:\nCurrent working directory: {}\nArchitecture: {}\nOperating system: {}",
            self.pwd, self.arch, self.os
        );

        if let Some(distro) = &self.distro {
            info.push_str(&format!("\nDistribution: {distro}"));
        }

        info.push_str(&format!(
            "\nShell: {}\nUser: {} (root: {})",
            self.shell, self.user, self.is_root
        ));

        if let Some(git) = &self.git {
            info.push_str(&format!(
                "\nGit repository: {} (branch: {}, dirty: {})",
                git.root, git.branch, git.dirty
            ));
        }

        info
    }

    #[cfg(test)]
    fn fixture(pwd: impl Into<String>, arch: impl Into<String>) -> Self {
        Self {
            pwd: pwd.into(),
            arch: arch.into(),
            os: String::from("linux"),
            distro: None,
            shell: String::from("bash"),
            user: String::from("user"),
            is_root: false,
            git: None,
        }
    }
}

pub struct SystemInfoView {
    info: SystemInfo,
    text_view: TextView,
}

impl SystemInfoView {
    pub fn new() -> Self {
        Self::from_system_info(SystemInfo::resolve())
    }

    fn from_system_info(info: SystemInfo) -> Self {
        let text_view = TextView::new(display_text(&info));

        Self { info, text_view }
    }

    #[cfg(test)]
    pub fn from_parts(pwd: impl Into<String>, arch: impl Into<String>) -> Self {
        Self::from_system_info(SystemInfo::fixture(pwd, arch))
    }

    pub fn system_info(&self) -> String {
        self.info.render()
    }
}

fn command_output(command: &str, args: &[&str]) -> Option<String> {
    Command::new(command)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|output| !output.is_empty())
}

fn resolve_pwd() -> String {
    std::env::var(PWD_ENV_VAR)
        .ok()
        .filter(|pwd| !pwd.is_empty())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|pwd| pwd.display().to_string())
        })
        .unwrap_or_else(|| UNKNOWN.to_string())
}

fn resolve_arch() -> String {
    command_output(UNAME_COMMAND, &[ARCH_ARGUMENT])
        .unwrap_or_else(|| std::env::consts::ARCH.to_string())
}

fn resolve_os() -> String {
    command_output(UNAME_COMMAND, &[OS_ARGUMENT])
        .unwrap_or_else(|| std::env::consts::OS.to_string())
}

fn resolve_distro(os: &str) -> Option<String> {
    if os.eq_ignore_ascii_case("darwin") || os.eq_ignore_ascii_case("macos") {
        resolve_macos_version()
    } else {
        std::fs::read_to_string(OS_RELEASE_PATH)
            .ok()
            .and_then(|contents| parse_os_release(&contents))
    }
}

fn resolve_macos_version() -> Option<String> {
    let name = command_output(SW_VERS_COMMAND, &["-productName"]);
    let version = command_output(SW_VERS_COMMAND, &["-productVersion"]);

    match (name, version) {
        (Some(name), Some(version)) => Some(format!("{name} {version}")),
        (Some(name), None) => Some(name),
        (None, Some(version)) => Some(version),
        (None, None) => None,
    }
}

fn parse_os_release(contents: &str) -> Option<String> {
    let mut name = None;
    let mut version = None;

    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            let value = unquote(value);
            if !value.is_empty() {
                return Some(value);
            }
        } else if let Some(value) = line.strip_prefix("NAME=") {
            let value = unquote(value);
            if !value.is_empty() {
                name = Some(value);
            }
        } else if let Some(value) = line.strip_prefix("VERSION_ID=") {
            let value = unquote(value);
            if !value.is_empty() {
                version = Some(value);
            }
        }
    }

    match (name, version) {
        (Some(name), Some(version)) => Some(format!("{name} {version}")),
        (Some(name), None) => Some(name),
        (None, Some(version)) => Some(version),
        (None, None) => None,
    }
}

fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn resolve_shell() -> String {
    std::env::var(SHELL_ENV_VAR)
        .ok()
        .filter(|shell| !shell.is_empty())
        .map(|shell| {
            Path::new(&shell)
                .file_name()
                .and_then(|name| name.to_str())
                .map(String::from)
                .unwrap_or(shell)
        })
        .unwrap_or_else(|| UNKNOWN.to_string())
}

fn resolve_user() -> String {
    std::env::var(USER_ENV_VAR)
        .ok()
        .filter(|user| !user.is_empty())
        .or_else(|| {
            std::env::var(LOGNAME_ENV_VAR)
                .ok()
                .filter(|user| !user.is_empty())
        })
        .or_else(|| command_output(ID_COMMAND, &[USER_ARGUMENT]))
        .unwrap_or_else(|| UNKNOWN.to_string())
}

fn resolve_is_root() -> bool {
    command_output(ID_COMMAND, &[UID_ARGUMENT]).is_some_and(|uid| uid == "0")
}

fn resolve_git() -> Option<GitInfo> {
    let root = command_output(GIT_COMMAND, &["rev-parse", "--show-toplevel"])?;
    let branch = command_output(GIT_COMMAND, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|| UNKNOWN.to_string());
    let dirty = Command::new(GIT_COMMAND)
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| !output.stdout.is_empty());

    Some(GitInfo {
        root,
        branch,
        dirty,
    })
}

fn display_text(info: &SystemInfo) -> String {
    let mut entries = vec![
        (String::from("PWD:"), info.pwd.clone()),
        (String::from("OS:"), info.os.clone()),
        (String::from("Arch:"), info.arch.clone()),
        (String::from("Shell:"), info.shell.clone()),
    ];

    if let Some(git) = &info.git {
        entries.push((String::from("Git:"), git.branch.clone()));
    }

    let label_width = entries
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);

    entries
        .iter()
        .map(|(label, value)| format!("{label:<label_width$} {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

impl View for SystemInfoView {
    fn key_event(&mut self, _event: KeyEvent) {
        // Do nothing
    }

    fn render(&self, width: usize, height: usize) -> Frame {
        self.text_view.render(width, height)
    }

    fn update(&mut self) {
        // Do nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_info_uses_full_labels() {
        let view = SystemInfoView::from_parts("/home/user", "x86_64");

        assert_eq!(
            view.system_info(),
            "System info:\nCurrent working directory: /home/user\nArchitecture: x86_64\n\
             Operating system: linux\nShell: bash\nUser: user (root: false)"
        );
    }

    #[test]
    fn system_info_includes_distribution_and_git_when_available() {
        let mut info = SystemInfo::fixture("/repo", "arm64");
        info.distro = Some(String::from("Ubuntu 24.04 LTS"));
        info.git = Some(GitInfo {
            root: String::from("/repo"),
            branch: String::from("main"),
            dirty: true,
        });
        let view = SystemInfoView::from_system_info(info);

        assert_eq!(
            view.system_info(),
            "System info:\nCurrent working directory: /repo\nArchitecture: arm64\n\
             Operating system: linux\nDistribution: Ubuntu 24.04 LTS\nShell: bash\n\
             User: user (root: false)\n\
             Git repository: /repo (branch: main, dirty: true)"
        );
    }

    #[test]
    fn renders_fields_aligned_by_value_column() {
        let view = SystemInfoView::from_parts("/home/user", "x86_64");

        let frame = view.render(40, 10);

        assert_eq!(
            frame.rows(),
            &[
                "PWD:   /home/user",
                "OS:    linux",
                "Arch:  x86_64",
                "Shell: bash",
            ]
        );
        assert_eq!(frame.cursor(), None);
    }

    #[test]
    fn renders_git_branch_when_present() {
        let mut info = SystemInfo::fixture("/repo", "arm64");
        info.git = Some(GitInfo {
            root: String::from("/repo"),
            branch: String::from("main"),
            dirty: false,
        });
        let view = SystemInfoView::from_system_info(info);

        let frame = view.render(40, 10);

        assert_eq!(
            frame.rows(),
            &[
                "PWD:   /repo",
                "OS:    linux",
                "Arch:  arm64",
                "Shell: bash",
                "Git:   main",
            ]
        );
    }

    #[test]
    fn parse_os_release_prefers_pretty_name() {
        let contents = "NAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\n";

        assert_eq!(
            parse_os_release(contents).as_deref(),
            Some("Ubuntu 24.04 LTS")
        );
    }

    #[test]
    fn parse_os_release_falls_back_to_name_and_version() {
        let contents = "NAME=Alpine\nVERSION_ID=3.20\n";

        assert_eq!(parse_os_release(contents).as_deref(), Some("Alpine 3.20"));
    }

    #[test]
    fn parse_os_release_returns_none_without_names() {
        assert_eq!(parse_os_release("ID=linux\n"), None);
    }

    #[test]
    fn new_resolves_system_info_from_the_system() {
        let view = SystemInfoView::new();

        assert!(!view.info.pwd.is_empty());
        assert!(!view.info.arch.is_empty());
        assert!(!view.info.os.is_empty());
        assert!(!view.info.shell.is_empty());
        assert!(!view.info.user.is_empty());
        assert_eq!(view.system_info(), view.info.render());
    }
}
