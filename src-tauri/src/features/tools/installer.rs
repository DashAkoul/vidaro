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

/// Attempts per helper download; each one resumes where the last stopped.
const MAX_ATTEMPTS: u32 = 5;

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

    let mut builder = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(20));

    // GitHub is filtered on some networks while a proxy is available; without
    // this the tool download fails even though downloads would work.
    if let Some(config) = crate::core::state::config_opt(app) {
        match config.get().proxy_setting() {
            crate::core::config::ProxySetting::Url(url) => {
                if let Ok(proxy) = reqwest::Proxy::all(&url) {
                    builder = builder.proxy(proxy);
                }
            }
            crate::core::config::ProxySetting::Direct => {
                builder = builder.no_proxy();
            }
            crate::core::config::ProxySetting::Inherit => {}
        }
    }

    let client = builder.build()?;
    let tmp = dest.with_extension("download");

    // A 23 MB helper download over a flaky link used to leave a `.download`
    // file and a failed install, which looked like "the app is broken again".
    // Each attempt resumes from the bytes already on disk.
    let mut last_error: Option<AppError> = None;
    for attempt in 0..MAX_ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(2 * u64::from(attempt))).await;
        }
        match download_once(app, &client, tool, url, &tmp).await {
            Ok(received) => match replace_binary(&tmp, dest) {
                Ok(()) => {
                    report(app, tool, "done", received, received);
                    return Ok(());
                }
                // Windows refuses to replace a file that a running process
                // still has open (the version probe, or a download in flight),
                // so the finished file is kept and the swap retried shortly.
                Err(error) => {
                    crate::core::logging::log_startup(
                        "tools",
                        &format!("{tool}: could not replace the binary yet: {error}"),
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    last_error = Some(error);
                    continue;
                }
            },
            Err(error) => {
                crate::core::logging::log_startup(
                    "tools",
                    &format!("{tool} download attempt {} failed: {error}", attempt + 1),
                );
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AppError::Network(format!("{tool}: download failed"))))
}

/// Move a finished download into place, retrying briefly on Windows.
///
/// A helper binary may still be running (the version probe or a download in
/// flight), and Windows then refuses both the delete and the rename. Retrying
/// is enough: the handle is released within a second or two.
fn replace_binary(tmp: &Path, dest: &Path) -> AppResult<()> {
    let mut last: Option<String> = None;
    for _ in 0..5 {
        if dest.exists() {
            if let Err(error) = std::fs::remove_file(dest) {
                last = Some(error.to_string());
                std::thread::sleep(std::time::Duration::from_millis(600));
                continue;
            }
        }
        match std::fs::rename(tmp, dest) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        dest,
                        std::fs::Permissions::from_mode(0o755),
                    );
                }
                return Ok(());
            }
            Err(error) => {
                last = Some(error.to_string());
                std::thread::sleep(std::time::Duration::from_millis(600));
            }
        }
    }
    Err(AppError::Tool(format!(
        "could not replace {}: {}",
        dest.display(),
        last.unwrap_or_else(|| "unknown error".into())
    )))
}

/// One attempt, continuing from whatever the `.download` file already holds.
///
/// Returns the number of bytes on disk when the file is complete.
async fn download_once(
    app: &AppHandle,
    client: &reqwest::Client,
    tool: &str,
    url: &str,
    tmp: &Path,
) -> AppResult<u64> {
    let mut received: u64 = std::fs::metadata(tmp).map(|m| m.len()).unwrap_or(0);

    let mut request = client.get(url);
    if received > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={received}-"));
    }
    let response = request.send().await?;
    let status = response.status();

    // A `.download` that is already the full size makes a range request
    // unsatisfiable; that is success, not failure.
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && received > 0 {
        return Ok(received);
    }

    // 206 means the server honoured the range; anything else makes us restart.
    let resuming = status == reqwest::StatusCode::PARTIAL_CONTENT;
    if !resuming {
        if !status.is_success() {
            return Err(AppError::Network(format!(
                "{tool}: HTTP {status} from {url}"
            )));
        }
        received = 0;
    }

    let remaining = response.content_length().unwrap_or(0);
    let total = if resuming { received + remaining } else { remaining };

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(resuming)
        .truncate(!resuming)
        .open(tmp)?;

    let mut last_report = received;
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

    if total > 0 && received < total {
        return Err(AppError::Network(format!(
            "{tool}: connection closed at {received} of {total} bytes"
        )));
    }
    Ok(received)
}

/// Deno release archive for this platform.
///
/// Deno is yt-dlp's recommended JavaScript runtime: YouTube's player challenge
/// (`n`/`sig`) is JavaScript, and a runtime is what turns "No video formats
/// found" back into a real format list.
fn deno_archive_url() -> String {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        _ => "x86_64",
    };
    let target = if cfg!(windows) {
        format!("{arch}-pc-windows-msvc")
    } else if cfg!(target_os = "macos") {
        format!("{arch}-apple-darwin")
    } else {
        format!("{arch}-unknown-linux-gnu")
    };
    format!("https://github.com/denoland/deno/releases/latest/download/deno-{target}.zip")
}

/// Download Deno into the managed `bin` directory.
pub async fn install_deno(app: &AppHandle, bin_dir: &Path) -> AppResult<PathBuf> {
    let archive = bin_dir.join("deno-archive.zip");
    let dest = bin_dir.join(exe_name("deno"));

    report(app, "deno", "download", 0, 0);
    download(app, "deno", &deno_archive_url(), &archive).await?;
    report(app, "deno", "extract", 0, 0);

    let archive_owned = archive.clone();
    let dest_owned = dest.clone();
    tauri::async_runtime::spawn_blocking(move || -> AppResult<()> {
        extract_zip_member(&archive_owned, &dest_owned, &["deno.exe", "deno"])?;
        let _ = std::fs::remove_file(&archive_owned);
        Ok(())
    })
    .await
    .map_err(|e| AppError::Tool(format!("extraction task failed: {e}")))??;

    report(app, "deno", "done", 0, 0);
    Ok(dest)
}

/// Pull one named file out of a zip, wherever it sits inside the archive.
fn extract_zip_member(archive: &Path, dest: &Path, names: &[&str]) -> AppResult<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let base = entry
            .name()
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !names.iter().any(|name| base == name.to_ascii_lowercase()) {
            continue;
        }

        let mut buffer = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buffer)?;
        std::fs::write(dest, &buffer)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755));
        }
        return Ok(());
    }

    Err(AppError::Tool(
        "the Deno archive did not contain the runtime".into(),
    ))
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
