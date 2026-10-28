//! Process spawning helpers.
//!
//! Every child process in Vidaro goes through these helpers so that Windows
//! never flashes a console window. `CREATE_NO_WINDOW` is mandatory here: the
//! bundled `yt-dlp.exe` and `ffmpeg.exe` are console applications, and without
//! the flag Windows opens (and immediately closes) a terminal window.

use std::ffi::OsStr;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Async command with a hidden console window on Windows.
pub fn async_command<S: AsRef<OsStr>>(program: S) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Blocking command with a hidden console window on Windows.
pub fn sync_command<S: AsRef<OsStr>>(program: S) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Run a command to completion and return trimmed stdout.
///
/// Never panics: a missing or failing binary simply yields `None`.
pub fn capture_stdout<S: AsRef<OsStr>>(program: S, args: &[&str]) -> Option<String> {
    let out = sync_command(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}
