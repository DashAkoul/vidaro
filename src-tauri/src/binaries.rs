use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BinariesStatus {
    pub ready: bool,
    pub ytdlp_found: bool,
    pub ytdlp_version: Option<String>,
    pub ffmpeg_found: bool,
    pub ytdlp_path: String,
    pub ffmpeg_path: String,
    pub platform: String,
}

#[allow(dead_code)]
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    tool: String,
    stage: String,
    received: u64,
    total: u64,
    message: String,
}

static BINARIES_PATHS: OnceLock<BinariesPaths> = OnceLock::new();

#[allow(dead_code)]
#[derive(Clone)]
struct BinariesPaths {
    ytdlp: PathBuf,
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
}

pub fn init_binaries_paths(app: &AppHandle) {
    let _ = BINARIES_PATHS.get_or_init(|| {
        let ytdlp = ytdlp_path(app);
        let ffmpeg = ffmpeg_path(app);
        let ffprobe = ffprobe_path(app);
        BinariesPaths { ytdlp, ffmpeg, ffprobe }
    });
}

fn get_binaries_paths(app: &AppHandle) -> &'static BinariesPaths {
    BINARIES_PATHS.get_or_init(|| {
        let ytdlp = ytdlp_path(app);
        let ffmpeg = ffmpeg_path(app);
        let ffprobe = ffprobe_path(app);
        BinariesPaths { ytdlp, ffmpeg, ffprobe }
    })
}

fn bundled_bin_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().resource_dir().ok().map(|dir| dir.join("binaries"))
}

pub fn bin_dir(app: &AppHandle) -> &Path {
    if let Some(parent) = get_binaries_paths(app).ytdlp.parent() {
        parent
    } else {
        // Fallback to app data dir/bin
        static FALLBACK: OnceLock<PathBuf> = OnceLock::new();
        FALLBACK.get_or_init(|| {
            app.path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("bin")
        }).as_path()
    }
}

pub fn ytdlp_path(app: &AppHandle) -> PathBuf {
    if let Some(bundled_dir) = bundled_bin_dir(app) {
        let bundled = bundled_dir.join(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" });
        if bundled.exists() {
            return bundled;
        }
    }
    bin_dir(app).join(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" })
}

pub fn ffmpeg_path(app: &AppHandle) -> PathBuf {
    if let Some(bundled_dir) = bundled_bin_dir(app) {
        let bundled = bundled_dir.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" });
        if bundled.exists() {
            return bundled;
        }
    }
    bin_dir(app).join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
}

fn ffprobe_path(app: &AppHandle) -> PathBuf {
    if let Some(bundled_dir) = bundled_bin_dir(app) {
        let bundled = bundled_dir.join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
        if bundled.exists() {
            return bundled;
        }
    }
    bin_dir(app).join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" })
}

pub fn ytdlp_exists(app: &AppHandle) -> bool {
    ytdlp_path(app).exists()
}

pub fn ffmpeg_exists(app: &AppHandle) -> bool {
    ffmpeg_path(app).exists()
}

fn ytdlp_version(app: &AppHandle) -> Option<String> {
    let out = std::process::Command::new(ytdlp_path(app))
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[tauri::command]
pub fn binaries_status(app: AppHandle) -> BinariesStatus {
    let ytdlp_found = ytdlp_exists(&app);
    let ffmpeg_found = ffmpeg_exists(&app);
    let version = if ytdlp_found { ytdlp_version(&app) } else { None };
    BinariesStatus {
        ready: ytdlp_found && ffmpeg_found,
        ytdlp_found,
        ytdlp_version: version,
        ffmpeg_found,
        ytdlp_path: ytdlp_path(&app).to_string_lossy().to_string(),
        ffmpeg_path: ffmpeg_path(&app).to_string_lossy().to_string(),
        platform: std::env::consts::OS.to_string(),
    }
}

#[tauri::command]
pub fn ensure_binaries(app: AppHandle) -> Result<(), String> {
    // Just check if binaries exist - they should be bundled
    if !ytdlp_exists(&app) || !ffmpeg_exists(&app) {
        return Err("Binaries not found. Please reinstall the application.".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn update_ytdlp(_app: AppHandle) -> Result<String, String> {
    Err("Auto-update not implemented yet".to_string())
}