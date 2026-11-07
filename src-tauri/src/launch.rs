//! "Open in terminal / editor / file manager". Programs are started directly (never through a
//! shell), with the repository folder as an argument or working directory.
use std::path::Path;
use std::process::{Command, Stdio};

const TERMINALS: &[&str] = &[
    "x-terminal-emulator",
    "gnome-terminal",
    "kgx",
    "konsole",
    "xfce4-terminal",
    "tilix",
    "alacritty",
    "kitty",
    "foot",
    "xterm",
];
const EDITORS: &[&str] = &["code", "codium", "zed", "subl", "gedit", "kate", "xdg-open"];

/// First candidate for which `exists` is true.
fn first_available<'a>(candidates: &[&'a str], exists: impl Fn(&str) -> bool) -> Option<&'a str> {
    candidates.iter().copied().find(|c| exists(c))
}

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(program).is_file()))
        .unwrap_or(false)
}

/// Splits a user-typed command ("code --new-window") into program and arguments.
/// Deliberately simple: whitespace-separated, no quoting or shell syntax.
pub fn split_command(cmd: &str) -> Option<(String, Vec<String>)> {
    let mut parts = cmd.split_whitespace().map(str::to_string);
    let program = parts.next()?;
    Some((program, parts.collect()))
}

/// Arguments that make `terminal` open in `dir`. Terminals without such a flag rely on the
/// working directory of the spawned process.
fn terminal_dir_args(terminal: &str, dir: &Path) -> Vec<String> {
    let d = dir.to_string_lossy().into_owned();
    match terminal {
        "gnome-terminal" | "xfce4-terminal" | "tilix" => vec![format!("--working-directory={d}")],
        "kgx" => vec![format!("--working-directory={d}")],
        "konsole" => vec!["--workdir".into(), d],
        "alacritty" | "foot" => vec!["--working-directory".into(), d],
        "kitty" => vec!["--directory".into(), d],
        _ => vec![],
    }
}

/// The command that opens a terminal in `dir`: the user's own (`custom`) or the first one found.
pub fn terminal_command(
    custom: Option<&str>,
    dir: &Path,
    exists: impl Fn(&str) -> bool,
) -> Result<Command, String> {
    let (program, mut args) = match custom.and_then(split_command) {
        Some((p, a)) => (p, a),
        None => {
            let t = first_available(TERMINALS, exists)
                .ok_or("No terminal program found. Set one in Settings.")?;
            (t.to_string(), vec![])
        }
    };
    args.extend(terminal_dir_args(&program, dir));
    let mut c = Command::new(program);
    c.args(args).current_dir(dir);
    Ok(c)
}

/// The command that opens `dir` in an editor: the user's own or the first one found.
pub fn editor_command(
    custom: Option<&str>,
    dir: &Path,
    exists: impl Fn(&str) -> bool,
) -> Result<Command, String> {
    let (program, mut args) = match custom.and_then(split_command) {
        Some((p, a)) => (p, a),
        None => {
            let e =
                first_available(EDITORS, exists).ok_or("No editor found. Set one in Settings.")?;
            (e.to_string(), vec![])
        }
    };
    args.push(dir.to_string_lossy().into_owned());
    let mut c = Command::new(program);
    c.args(args).current_dir(dir);
    Ok(c)
}

fn spawn_detached(mut c: Command) -> Result<(), String> {
    c.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not start {}: {e}", c.get_program().to_string_lossy()))
}

pub fn open_terminal(custom: Option<&str>, dir: &Path) -> Result<(), String> {
    spawn_detached(terminal_command(custom, dir, on_path)?)
}

pub fn open_editor(custom: Option<&str>, dir: &Path) -> Result<(), String> {
    spawn_detached(editor_command(custom, dir, on_path)?)
}

/// Opens a web link in the default browser. Only plain http(s) links are allowed.
pub fn open_url(url: &str) -> Result<(), String> {
    let ok = (url.starts_with("https://") || url.starts_with("http://"))
        && !url.chars().any(|c| c.is_whitespace() || c.is_control());
    if !ok {
        return Err("Only http and https links can be opened.".into());
    }
    let mut c = Command::new("xdg-open");
    c.arg(url);
    spawn_detached(c)
}

pub fn open_folder(dir: &Path) -> Result<(), String> {
    let mut c = Command::new("xdg-open");
    c.arg(dir);
    spawn_detached(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(c: &Command) -> Vec<String> {
        c.get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn splits_commands_on_whitespace() {
        assert_eq!(
            split_command("code --new-window"),
            Some(("code".into(), vec!["--new-window".into()]))
        );
        assert_eq!(split_command("  subl  "), Some(("subl".into(), vec![])));
        assert_eq!(split_command("   "), None);
    }

    #[test]
    fn picks_the_first_available_program() {
        let have = |p: &str| p == "konsole" || p == "xterm";
        let c = terminal_command(None, Path::new("/work/repo"), have).unwrap();
        assert_eq!(c.get_program(), "konsole");
        assert_eq!(args(&c), ["--workdir", "/work/repo"]);
        assert_eq!(c.get_current_dir(), Some(Path::new("/work/repo")));

        assert!(terminal_command(None, Path::new("/x"), |_| false).is_err());
        assert!(editor_command(None, Path::new("/x"), |_| false).is_err());
        let e = editor_command(None, Path::new("/work/repo"), |p| {
            p == "xdg-open" || p == "gedit"
        })
        .unwrap();
        assert_eq!(e.get_program(), "gedit", "earlier candidates win");
    }

    #[test]
    fn custom_commands_get_the_folder_and_are_not_run_through_a_shell() {
        let e = editor_command(
            Some("code --new-window"),
            Path::new("/work/my repo"),
            |_| false,
        )
        .unwrap();
        assert_eq!(e.get_program(), "code");
        assert_eq!(
            args(&e),
            ["--new-window", "/work/my repo"],
            "paths with spaces stay one argument"
        );

        // shell syntax is just text: it becomes (harmless) arguments, never executed
        let t = terminal_command(Some("xterm; rm -rf /"), Path::new("/w"), |_| false).unwrap();
        assert_eq!(t.get_program(), "xterm;");

        let g = terminal_command(Some("gnome-terminal"), Path::new("/w/r"), |_| false).unwrap();
        assert_eq!(args(&g), ["--working-directory=/w/r"]);
    }

    #[test]
    fn only_web_links_are_opened() {
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ftp://x",
            "/etc/passwd",
            "https://a b",
            "https://x\n--evil",
            "-flag",
        ] {
            assert!(open_url(bad).is_err(), "{bad:?}");
        }
    }
}
