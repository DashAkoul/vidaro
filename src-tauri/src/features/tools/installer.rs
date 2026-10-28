//! Downloading and installing yt-dlp / ffmpeg on first run.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::Serialize;
use tauri::AppHandle;

use crate::core::error::{AppError, AppResult};
use crate::core::events;
use crate::features::tools::paths::exe_name;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolsProgress {
    pub tool: String,
    pub stage: String, // "download" | "extract" | "done"
    pub received: u64,
    pub total: u64,
}

fn report(app: &AppHandle, tool: &str, stage: &str, received: u64, total: u64) {
    events::emit(
        app,
        events::TOOLS_PROGRESS,
        ToolsProgress {
            tool: tool.to_string(),
            stage: stage.to_string(),
            received,
            total,
        },
    );
}

const USER_AGENT: &str = "Vidaro/2.0 (+https://github.com/DashAkoul/vidaro)";

fn ytdlp_url() -> &'static str {
    if cfg!(windows) {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
    }
}

/// Stream a URL to `dest`, reporting progress as we go.
async fn download(app: &AppHandle, tool: &str, url: &str, dest: &Path) -> AppResult<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "{tool}: HTTP {} from {url}",
            response.status()
        )));
    }

    let total = response.content_length().unwrap_or(0);
    let tmp = dest.with_extension("download");
    let mut file = std::fs::File::create(&tmp)?;

    let mut received: u64 = 0;
    let mut last_report: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        received += chunk.len() as u64;
        if received - last_report > 256 * 1024 {
            last_report = received;
            report(app, tool, "download", received, total);
        }
    }

    file.flush()?;
    drop(file);

    // Atomic replace so a half-written binary is never picked up.
    if dest.exists() {
        let _ = std::fs::remove_file(dest);
    }
    std::fs::rename(&tmp, dest)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755));
    }

    report(app, tool, "done", received, total);
    Ok(())
}

pub async fn install_ytdlp(app: &AppHandle, bin_dir: &Path) -> AppResult<PathBuf> {
    let dest = bin_dir.join(exe_name("yt-dlp"));
    download(app, "yt-dlp", ytdlp_url(), &dest).await?;
    Ok(dest)
}

/// ffmpeg archive URL plus the member paths we want out of it.
fn ffmpeg_archive() -> (&'static str, &'static str) {
    if cfg!(windows) {
        (
            "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip",
            "zip",
        )
    } else if cfg!(target_os = "macos") {
        ("https://evermeet.cx/ffmpeg/getrelease/zip", "zip")
    } else {
        (
            "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-linux64-gpl.tar.xz",
            "tar",
        )
    }
}

pub async fn install_ffmpeg(app: &AppHandle, bin_dir: &Path) -> AppResult<()> {
    let (url, kind) = ffmpeg_archive();
    let archive = bin_dir.join(if kind == "zip" {
        "ffmpeg-archive.zip"
    } else {
        "ffmpeg-archive.tar.xz"
    });

    report(app, "ffmpeg", "download", 0, 0);
    download(app, "ffmpeg", url, &archive).await?;
    report(app, "ffmpeg", "extract", 0, 0);

    let bin_dir_owned = bin_dir.to_path_buf();
    let archive_owned = archive.clone();

    tauri::async_runtime::spawn_blocking(move || -> AppResult<()> {
        if kind == "zip" {
            extract_zip_members(&archive_owned, &bin_dir_owned)?;
        } else {
            extract_with_tar(&archive_owned, &bin_dir_owned)?;
        }
        let _ = std::fs::remove_file(&archive_owned);
        Ok(())
    })
    .await
    .map_err(|e| AppError::Tool(format!("extraction task failed: {e}")))??;

    report(app, "ffmpeg", "done", 0, 0);
    Ok(())
}

/// Pull `ffmpeg` and `ffprobe` out of an archive, wherever they sit inside it.
fn extract_zip_members(archive: &Path, dest_dir: &Path) -> AppResult<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;

    let mut found = 0usize;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let base = name.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
        let target = match base.as_str() {
            "ffmpeg.exe" | "ffmpeg" => dest_dir.join(exe_name("ffmpeg")),
            "ffprobe.exe" | "ffprobe" => dest_dir.join(exe_name("ffprobe")),
            _ => continue,
        };

        let mut buffer = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buffer)?;
        std::fs::write(&target, &buffer)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
        }
        found += 1;
    }

    if found < 2 {
        return Err(AppError::Tool(
            "ffmpeg archive did not contain ffmpeg and ffprobe".into(),
        ));
    }
    Ok(())
}

/// Linux builds ship as .tar.xz; shelling out to `tar` avoids an xz dependency.
fn extract_with_tar(archive: &Path, dest_dir: &Path) -> AppResult<()> {
    let staging = dest_dir.join("_ffmpeg_extract");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)?;

    let status = crate::core::process::sync_command("tar")
        .arg("-xJf")
        .arg(archive)
        .arg("-C")
        .arg(&staging)
        .status()?;

    if !status.success() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(AppError::Tool(
            "could not extract ffmpeg archive (is xz installed?)".into(),
        ));
    }

    let mut copied = 0;
    for base in ["ffmpeg", "ffprobe"] {
        if let Some(found) = find_file(&staging, base) {
            let target = dest_dir.join(base);
            std::fs::copy(&found, &target)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
            }
            copied += 1;
        }
    }
    let _ = std::fs::remove_dir_all(&staging);

    if copied < 2 {
        return Err(AppError::Tool(
            "ffmpeg archive did not contain ffmpeg and ffprobe".into(),
        ));
    }
    Ok(())
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
                return Some(path);
            }
        }
    }
    None
}
