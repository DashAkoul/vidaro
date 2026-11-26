//! Process spawning helpers.
//!
//! Every child process in Vidaro goes through these helpers so that Windows
//! never flashes a console window. `CREATE_NO_WINDOW` is mandatory here: the
//! bundled `yt-dlp.exe` and `ffmpeg.exe` are console applications, and without
//! the flag Windows opens (and immediately closes) a terminal window.

use std::ffi::OsStr;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Force a child process to read and write UTF-8 on its stdio.
///
/// `yt-dlp` is a PyInstaller bundle. On Windows it picks its stdio encoding
/// from the process code page, and a GUI app started with `CREATE_NO_WINDOW`
/// gives it a hidden console whose code page is the machine's OEM/ANSI one.
/// Every character outside that code page is then dropped or mangled on the
/// way to us: a Persian title such as `وقتی همسرت ADHD داره` arrived as
/// `  ADHD  ` and was stored as the file path of a finished download. The
/// flags below make Python ignore the locale entirely, so what yt-dlp prints
/// is byte-for-byte what it meant to print. They are harmless for any other
/// helper that does not read them.
pub fn force_utf8(cmd: &mut tokio::process::Command) {
    cmd.env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONLEGACYWINDOWSSTDIO", "0");
}

/// Async command with a hidden console window on Windows and UTF-8 stdio.
pub fn async_command<S: AsRef<OsStr>>(program: S) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    force_utf8(&mut cmd);
    cmd
}

/// Blocking command with a hidden console window on Windows and UTF-8 stdio.
pub fn sync_command<S: AsRef<OsStr>>(program: S) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONLEGACYWINDOWSSTDIO", "0");
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
