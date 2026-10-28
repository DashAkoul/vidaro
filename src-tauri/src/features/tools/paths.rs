//! Locating the bundled / downloaded helper binaries.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::core::paths::AppPaths;

#[derive(Debug, Clone)]
pub struct ToolPaths {
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
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
    }
}
