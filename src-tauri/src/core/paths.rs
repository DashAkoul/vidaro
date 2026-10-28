//! Filesystem layout used by the application.

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Every path the app writes to, resolved once at startup.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub config_file: PathBuf,
    pub queue_file: PathBuf,
    pub history_file: PathBuf,
    /// Where downloaded yt-dlp / ffmpeg live.
    pub bin_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &AppHandle) -> Self {
        let data_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));

        Self {
            config_file: data_dir.join("settings.json"),
            queue_file: data_dir.join("queue.json"),
            history_file: data_dir.join("history.json"),
            bin_dir: data_dir.join("bin"),
            data_dir,
        }
    }

    /// Create every directory the app needs. Idempotent.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.bin_dir)?;
        Ok(())
    }
}
