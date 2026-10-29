//! Helper-binary management: yt-dlp and ffmpeg.

pub mod commands;
pub mod installer;
pub mod paths;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::core::error::{AppError, AppResult};
use crate::core::paths::AppPaths;
use crate::core::plugin::{AppContext, Plugin};
use crate::core::process::capture_stdout;
use crate::features::tools::paths::ToolPaths;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub ready: bool,
    pub ytdlp_found: bool,
    pub ytdlp_version: Option<String>,
    pub ffmpeg_found: bool,
    pub ytdlp_path: String,
    pub ffmpeg_path: String,
    pub platform: String,
}

/// Owns binary locations and caches the (expensive) yt-dlp version probe.
///
/// Cloning is cheap (one `Arc`), which lets a command move the manager into a
/// blocking task instead of stalling the UI thread on the probe.
#[derive(Clone)]
pub struct ToolManager {
    inner: Arc<ToolInner>,
}

struct ToolInner {
    paths: RwLock<ToolPaths>,
    version: RwLock<Option<String>>,
    probe_done: RwLock<bool>,
    app_paths: AppPaths,
    /// Guards against two concurrent installs writing the same temp file.
    installing: AtomicBool,
}

/// Clears the install flag however the operation ends.
struct InstallGuard<'a>(&'a AtomicBool);

impl Drop for InstallGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl ToolManager {
    pub fn new(app: &AppHandle, app_paths: AppPaths) -> Self {
        let resolved = paths::resolve(app, &app_paths);
        Self {
            inner: Arc::new(ToolInner {
                paths: RwLock::new(resolved),
                version: RwLock::new(None),
                probe_done: RwLock::new(false),
                app_paths,
                installing: AtomicBool::new(false),
            }),
        }
    }

    fn begin_install(&self) -> AppResult<InstallGuard<'_>> {
        if self.inner.installing.swap(true, Ordering::SeqCst) {
            return Err(AppError::Invalid(
                "an installation is already in progress".into(),
            ));
        }
        Ok(InstallGuard(&self.inner.installing))
    }

    pub fn paths(&self) -> ToolPaths {
        self.inner
            .paths
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Re-resolve after installing/updating a binary.
    pub fn refresh(&self, app: &AppHandle) {
        let resolved = paths::resolve(app, &self.inner.app_paths);
        if let Ok(mut guard) = self.inner.paths.write() {
            *guard = resolved;
        }
        if let Ok(mut guard) = self.inner.version.write() {
            *guard = None;
        }
        if let Ok(mut guard) = self.inner.probe_done.write() {
            *guard = false;
        }
    }

    /// yt-dlp version, probed at most once per process lifetime.
    ///
    /// The probe runs with a hidden console window (see `core::process`), which
    /// is what used to cause a terminal to flash on every app launch.
    pub fn ytdlp_version(&self) -> Option<String> {
        if let Ok(done) = self.inner.probe_done.read() {
            if *done {
                return self.inner.version.read().ok().and_then(|g| g.clone());
            }
        }

        let path = self.paths().ytdlp;
        let version = if path.is_file() {
            capture_stdout(&path, &["--version"])
        } else {
            None
        };

        if let Ok(mut guard) = self.inner.version.write() {
            *guard = version.clone();
        }
        if let Ok(mut guard) = self.inner.probe_done.write() {
            *guard = true;
        }
        version
    }

    pub fn status(&self) -> ToolStatus {
        let p = self.paths();
        let ytdlp_found = p.ytdlp.is_file();
        let ffmpeg_found = p.ffmpeg.is_file();
        ToolStatus {
            ready: ytdlp_found && ffmpeg_found,
            ytdlp_found,
            ytdlp_version: if ytdlp_found {
                self.ytdlp_version()
            } else {
                None
            },
            ffmpeg_found,
            ytdlp_path: p.ytdlp.to_string_lossy().to_string(),
            ffmpeg_path: p.ffmpeg.to_string_lossy().to_string(),
            platform: std::env::consts::OS.to_string(),
        }
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.inner.app_paths.bin_dir.clone()
    }

    /// Install whatever is missing.
    pub async fn ensure(&self, app: &AppHandle) -> AppResult<ToolStatus> {
        let _guard = self.begin_install()?;
        let p = self.paths();

        if !p.ytdlp.is_file() {
            installer::install_ytdlp(app, &self.inner.app_paths.bin_dir).await?;
        }
        if !p.ffmpeg.is_file() || !p.ffprobe.is_file() {
            installer::install_ffmpeg(app, &self.inner.app_paths.bin_dir).await?;
        }

        self.refresh(app);
        let status = self.status();
        if !status.ready {
            return Err(AppError::Tool(
                "helper binaries are still missing after install".into(),
            ));
        }
        Ok(status)
    }

    pub async fn update_ytdlp(&self, app: &AppHandle) -> AppResult<String> {
        let _guard = self.begin_install()?;
        installer::install_ytdlp(app, &self.inner.app_paths.bin_dir).await?;
        self.refresh(app);
        Ok(self
            .ytdlp_version()
            .unwrap_or_else(|| "unknown version".to_string()))
    }
}

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn name(&self) -> &'static str {
        "tools"
    }

    fn startup(&self, ctx: &AppContext) -> AppResult<()> {
        // Warm the cache off the UI thread so the first `binaries_status` call
        // returns instantly.
        let app = ctx.app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            if let Some(mgr) = app.try_state::<ToolManager>() {
                let _ = mgr.ytdlp_version();
            }
        });
        Ok(())
    }
}
