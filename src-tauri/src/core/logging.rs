//! Minimal file logging for startup and panics.
//!
//! A `windows_subsystem = "windows"` app has no console: a panic during
//! setup, a failed plugin, or a window that never shows are all completely
//! invisible. This module appends a small, time-stamped trace to
//! `<app data>/startup.log` so a launch problem can be diagnosed from disk
//! afterwards. The log is intentionally tiny (32 KiB cap) and costs nothing
//! after startup.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::Manager as _;

/// Absolute path of the log file, set once during setup.
static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Cap: older lines are dropped once the file exceeds this size.
const MAX_BYTES: u64 = 32 * 1024;

/// Point the logger at `<app data dir>/startup.log` and record the process
/// identity. Must be called first in `setup`.
pub fn init(app: &tauri::AppHandle) {
    let dir = app.path().app_data_dir().ok();
    let Some(dir) = dir else { return };
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("startup.log");

    // Trim: keep the file bounded by rewriting the tail.
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > MAX_BYTES {
            if let Ok(old) = std::fs::read_to_string(&path) {
                let tail: String = old
                    .lines()
                    .rev()
                    .take(200)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n");
                let _ = std::fs::write(&path, tail);
            }
        }
    }

    if let Ok(mut guard) = LOG_PATH.lock() {
        *guard = Some(path.clone());
    }

    log_startup(
        "boot",
        &format!(
            "pid={} exe={} args={:?} version={}",
            std::process::id(),
            std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            std::env::args().skip(1).collect::<Vec<_>>(),
            app.package_info().version,
        ),
    );
}

/// Append one line to the startup log. Never panics, never blocks long.
pub fn log_startup(stage: &str, message: &str) {
    let Ok(guard) = LOG_PATH.lock() else { return };
    let Some(path) = guard.as_ref() else { return };
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let _ = writeln!(file, "[{now}] [{stage}] {message}");
}

/// Install a panic hook that writes panics to the startup log.
///
/// A panic before the window is up is otherwise completely silent in a
/// `windows_subsystem` app — the process just vanishes, which the user
/// reports as "the app does not start".
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log_startup("panic", &format!("{info}"));
        default(info);
    }));
}
