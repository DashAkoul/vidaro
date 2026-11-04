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

/// Bundle identifier, also the data-directory name Tauri uses.
pub const APP_ID: &str = "com.akoul.yt-downloader";

impl AppPaths {
    pub fn resolve(app: &AppHandle) -> Self {
        let data_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));

        Self::with_data_dir(data_dir)
    }

    /// Same layout without a Tauri handle, for the headless native-messaging
    /// relay that runs before the app is built.
    pub fn standalone() -> Self {
        Self::with_data_dir(default_data_dir())
    }

    fn with_data_dir(data_dir: PathBuf) -> Self {
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

/// Resolve the per-user data directory the way Tauri does, without an app handle.
fn default_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join(APP_ID);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_ID);
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(data).join(APP_ID);
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".local").join("share").join(APP_ID);
        }
    }

    PathBuf::from(APP_ID)
}
