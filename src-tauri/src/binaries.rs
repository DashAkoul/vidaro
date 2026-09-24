use futures_util::StreamExt;
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

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

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    tool: String,   // "yt-dlp" | "ffmpeg" | "ffprobe"
    stage: String,  // "download" | "extract" | "done"
    received: u64,
    total: u64, // 0 = unknown
    message: String,
}

/// Returns the bundled binaries directory (from Tauri resource dir)
fn bundled_bin_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().resource_dir().ok().map(|dir| dir.join("binaries"))
}

/// Returns the fallback binaries directory (AppData/bin) for dev mode
pub fn bin_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("bin")
}

/// Returns the path to yt-dlp, checking bundled location first
pub fn ytdlp_path(app: &AppHandle) -> PathBuf {
    // 1. Check bundled location first (installer)
    if let Some(bundled_dir) = bundled_bin_dir(app) {
        let bundled = bundled_dir.join(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" });
        if bundled.exists() {
            return bundled;
        }
    }
    // 2. Fallback to AppData/bin (dev mode / old installs)
    bin_dir(app).join(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" })
}

/// Returns the path to ffmpeg, checking bundled location first
pub fn ffmpeg_path(app: &AppHandle) -> PathBuf {
    // 1. Check bundled location first (installer)
    if let Some(bundled_dir) = bundled_bin_dir(app) {
        let bundled = bundled_dir.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" });
        if bundled.exists() {
            return bundled;
        }
    }
    // 2. Fallback to AppData/bin
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
    let version = if ytdlp_found {
        ytdlp_version(&app)
    } else {
        None
    };
    let ffmpeg_found = ffmpeg_exists(&app);
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

/// Serializes concurrent calls: the second caller waits for the first to
/// finish instead of failing (React StrictMode fires effects twice in dev).
static ENSURE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Download any missing binaries. Emits "binaries-progress" events.
#[tauri::command]
pub async fn ensure_binaries(app: AppHandle) -> Result<BinariesStatus, String> {
    let _guard = ENSURE_LOCK.lock().await;
    // another call may have finished while we waited for the lock
    let status = binaries_status(app.clone());
    if status.ready {
        return Ok(status);
    }
    ensure_inner(&app).await
}

async fn ensure_inner(app: &AppHandle) -> Result<BinariesStatus, String> {
    let dir = bin_dir(app);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    // If bundled binaries exist, copy them to AppData/bin for future use
    if let Some(bundled_dir) = bundled_bin_dir(app) {
        for tool in ["yt-dlp", "ffmpeg", "ffprobe"] {
            let ext = if cfg!(windows) { ".exe" } else { "" };
            let bundled = bundled_dir.join(format!("{}{}", tool, ext));
            let dest = dir.join(format!("{}{}", tool, if cfg!(windows) { ".exe" } else { "" }));
            if bundled.exists() && !dest.exists() {
                let _ = std::fs::copy(&bundled, &dest);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
                }
            }
        }
    }

    if !ytdlp_exists(app) {
        let url = if cfg!(windows) {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
        } else {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
        };
        let dest_tmp = dir.join("yt-dlp.download");
        download_file(app, "yt-dlp", url, &dest_tmp).await?;
        std::fs::rename(&dest_tmp, ytdlp_path(app)).map_err(|e| e.to_string())?;
        finish_executable(app, &ytdlp_path(app))?;
    }

    if !ffmpeg_exists(app) {
        if cfg!(windows) {
            // BtbN static build (contains ffmpeg.exe + ffprobe.exe)
            let url = "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip";
            let zip_path = dir.join("ffmpeg.zip");
            download_file(app, "ffmpeg", url, &zip_path).await?;
            emit(app, "ffmpeg", "extract", 0, 0, "extracting");
            let zip_path_c = zip_path.clone();
            let dir_c = dir.clone();
            tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
                extract_from_zip(&zip_path_c, &dir_c, &["ffmpeg.exe", "ffprobe.exe"])
            })
            .await
            .map_err(|e| e.to_string())??;
            let _ = std::fs::remove_file(&zip_path);
        } else if cfg!(target_os = "macos") {
            for (tool, url) in [
                ("ffmpeg", "https://evermeet.cx/ffmpeg/get/ffmpeg/zip"),
                ("ffprobe", "https://evermeet.cx/ffmpeg/get/ffprobe/zip"),
            ] {
                let zip_path = dir.join(format!("{}.zip", tool));
                download_file(app, tool, url, &zip_path).await?;
                emit(app, tool, "extract", 0, 0, "extracting");
                let zip_path_c = zip_path.clone();
                let dir_c = dir.clone();
                let tool_s = tool.to_string();
                tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
                    extract_from_zip(&zip_path_c, &dir_c, &[&tool_s])
                })
                .await
                .map_err(|e| e.to_string())??;
                let _ = std::fs::remove_file(&zip_path);
            }
            finish_executable(app, &ffmpeg_path(app))?;
            finish_executable(app, &ffprobe_path(app))?;
        } else {
            return Err("unsupported-platform".to_string());
        }
    }

    emit(app, "done", "done", 0, 0, "done");
    Ok(binaries_status(app.clone()))
}

fn emit(app: &AppHandle, tool: &str, stage: &str, received: u64, total: u64, message: &str) {
    let _ = app.emit(
        "binaries-progress",
        ProgressPayload {
            tool: tool.to_string(),
            stage: stage.to_string(),
            received,
            total,
            message: message.to_string(),
        },
    );
}

/// Make the downloaded binary executable (unix) and drop the macOS quarantine flag.
fn finish_executable(_app: &AppHandle, path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
        let _ = std::process::Command::new("xattr")
            .args(["-d", "com.apple.quarantine"])
            .arg(path)
            .output();
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

async fn download_file(
    app: &AppHandle,
    tool: &str,
    url: &str,
    dest: &Path,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("{url}: {e}"))?;

    let total = resp.content_length().unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut received: u64 = 0;
    let mut last_emit = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("{url}: {e}"))?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        received += chunk.len() as u64;
        if last_emit.elapsed() >= std::time::Duration::from_millis(150) {
            last_emit = std::time::Instant::now();
            emit(app, tool, "download", received, total, "downloading");
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    emit(app, tool, "download", received, total, "downloading");
    Ok(())
}

/// Extract entries whose file name (at any depth) matches `names` into `dest`.
fn extract_from_zip(zip_path: &Path, dest: &Path, names: &[&str]) -> Result<(), String> {
    let f = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(fname) = entry.enclosed_name() else {
            continue;
        };
        let file_name = fname
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if names.iter().any(|n| file_name.eq_ignore_ascii_case(n)) {
            let out_path = dest.join(&file_name);
            let mut out = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(
                    &out_path,
                    std::fs::Permissions::from_mode(0o755),
                );
                let _ = std::process::Command::new("xattr")
                    .args(["-d", "com.apple.quarantine"])
                    .arg(&out_path)
                    .output();
            }
        }
    }
    // make sure we found everything
    for n in names {
        if !dest.join(n).exists() {
            return Err(format!("missing after extract: {n}"));
        }
    }
    Ok(())
}

/// Self-update yt-dlp (`yt-dlp -U`). Returns the tool output.
#[tauri::command]
pub async fn update_ytdlp(app: AppHandle) -> Result<String, String> {
    if !ytdlp_exists(&app) {
        return Err("yt-dlp not installed".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let out = std::process::Command::new(ytdlp_path(&app))
            .arg("-U")
            .output()
            .map_err(|e| e.to_string())?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        Ok(text.trim().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
