//! Locating the bundled / downloaded helper binaries.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::core::paths::AppPaths;

#[derive(Debug, Clone)]
pub struct ToolPaths {
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    /// JavaScript runtime for YouTube's `n`/`sig` challenge. Optional: Node,
    /// Bun or QuickJS already on the PATH are just as good as a bundled Deno.
    pub javascript_runtime: Option<JsRuntime>,
}

/// A JavaScript engine yt-dlp may use to solve YouTube's player challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsRuntime {
    /// Name yt-dlp knows: `deno`, `node`, `bun` or `quickjs`.
    pub name: String,
    pub path: PathBuf,
}

impl ToolPaths {
    /// Directory handed to yt-dlp via `--ffmpeg-location`.
    pub fn ffmpeg_dir(&self) -> PathBuf {
        self.ffmpeg
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

pub fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Directories searched for helper binaries, most specific first.
fn search_dirs(app: &AppHandle, paths: &AppPaths) -> Vec<PathBuf> {
    let mut dirs = vec![paths.bin_dir.clone()];

    if let Ok(resource) = app.path().resource_dir() {
        dirs.push(resource.join("binaries"));
        dirs.push(resource);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("binaries"));
            dirs.push(parent.to_path_buf());
        }
    }

    // In `tauri dev` the binary lives deep inside target/debug, so also look
    // next to the crate sources where developers keep the tools.
    #[cfg(debug_assertions)]
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        dirs.push(PathBuf::from(manifest).join("binaries"));
    }

    dirs
}

fn find(app: &AppHandle, paths: &AppPaths, base: &str) -> PathBuf {
    let file = exe_name(base);
    let dirs = search_dirs(app, paths);
    for dir in &dirs {
        let candidate = dir.join(&file);
        if candidate.is_file() {
            return candidate;
        }
    }
    // Nothing found: report the canonical install location.
    paths.bin_dir.join(file)
}

pub fn resolve(app: &AppHandle, paths: &AppPaths) -> ToolPaths {
    ToolPaths {
        ytdlp: find(app, paths, "yt-dlp"),
        ffmpeg: find(app, paths, "ffmpeg"),
        ffprobe: find(app, paths, "ffprobe"),
        javascript_runtime: find_javascript_runtime(app, paths),
    }
}

impl ToolPaths {
    /// The runtime yt-dlp should be told about, if any.
    pub fn javascript(&self) -> Option<&JsRuntime> {
        self.javascript_runtime.as_ref()
    }
}

/// Locate a JavaScript runtime, preferring the managed one.
///
/// YouTube increasingly serves formats only when its player challenge can be
/// evaluated, and yt-dlp refuses to invent a JS engine: without one it warns
/// "n challenge solving failed" and returns no formats at all. A managed Deno
/// is tried first, then whatever the machine already has on its PATH so we do
/// not download 100 MB next to an existing Node install.
fn find_javascript_runtime(app: &AppHandle, paths: &AppPaths) -> Option<JsRuntime> {
    let managed = find(app, paths, "deno");
    if managed.is_file() {
        return Some(JsRuntime {
            name: "deno".to_string(),
            path: managed,
        });
    }

    for name in ["deno", "node", "bun", "qjs"] {
        if let Some(path) = which(&exe_name(name)) {
            return Some(JsRuntime {
                name: name.to_string(),
                path,
            });
        }
    }
    None
}

/// Look a program up on `PATH` (plus the usual Windows install locations).
fn which(program: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default();

    #[cfg(windows)]
    {
        // A GUI app does not always inherit the shell's PATH.
        if let Some(program_files) = std::env::var_os("ProgramFiles") {
            candidates.push(PathBuf::from(program_files).join("nodejs"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local).join("Programs").join("nodejs"));
            candidates.push(PathBuf::from(local).join("deno").join("bin"));
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            candidates.push(PathBuf::from(home).join(".deno").join("bin"));
        }
    }

    #[cfg(not(windows))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(PathBuf::from(&home).join(".deno").join("bin"));
            candidates.push(PathBuf::from(&home).join(".bun").join("bin"));
            candidates.push(PathBuf::from(&home).join(".local").join("bin"));
        }
    }

    candidates
        .into_iter()
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}
