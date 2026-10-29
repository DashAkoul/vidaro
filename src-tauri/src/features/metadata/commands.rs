//! Fetching video / playlist / channel metadata without downloading.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::{AppHandle, Manager as _};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

use crate::core::config::ConfigStore;
use crate::core::error::{AppError, AppResult};
use crate::core::events;
use crate::features::metadata::model::{InfoProgress, InfoResult, VideoEntry};
use crate::features::metadata::url::{classify_url, normalize_channel_url, UrlKind};
use crate::features::tools::ToolManager;

/// Hard cap on entries returned for very large channels.
const MAX_ENTRIES: usize = 5000;

static FETCHING: AtomicBool = AtomicBool::new(false);
static CANCELLED: AtomicBool = AtomicBool::new(false);

fn child_slot() -> &'static Mutex<Option<tokio::process::Child>> {
    static SLOT: OnceLock<Mutex<Option<tokio::process::Child>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Resets the "fetch in progress" flag even on early return or panic.
struct FetchGuard;
impl Drop for FetchGuard {
    fn drop(&mut self) {
        FETCHING.store(false, Ordering::SeqCst);
    }
}

fn emit_progress(app: &AppHandle, message: impl Into<String>) {
    events::emit(
        app,
        events::INFO_PROGRESS,
        InfoProgress {
            message: message.into(),
        },
    );
}

#[tauri::command]
pub async fn fetch_info(
    app: AppHandle,
    url: String,
    kind: Option<String>,
) -> AppResult<InfoResult> {
    if FETCHING.swap(true, Ordering::SeqCst) {
        return Err(AppError::Invalid("another fetch is already running".into()));
    }
    let _guard = FetchGuard;
    CANCELLED.store(false, Ordering::SeqCst);

    // Also the security boundary: an option-looking string never reaches yt-dlp.
    let Some(trimmed) = crate::features::metadata::url::normalize_url(&url) else {
        return Err(AppError::Invalid(format!("not a valid URL: {url}")));
    };

    let kind = match kind.as_deref() {
        Some("video") => UrlKind::Video,
        Some("playlist") => UrlKind::Playlist,
        Some("channel") => UrlKind::Channel,
        _ => classify_url(&trimmed),
    };

    let target = match kind {
        UrlKind::Channel => normalize_channel_url(&trimmed),
        _ => trimmed,
    };

    let Some(tools) = app.try_state::<ToolManager>() else {
        return Err(AppError::Tool("tools not initialised".into()));
    };
    let ytdlp = tools.paths().ytdlp;
    if !ytdlp.is_file() {
        return Err(AppError::Tool("yt-dlp is not installed".into()));
    }

    let mut args: Vec<String> = [
        "--no-warnings",
        "--no-colors",
        "--socket-timeout",
        "30",
        "--retries",
        "3",
        "--extractor-retries",
        "2",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    if kind != UrlKind::Video {
        args.push("--flat-playlist".into());
    }
    args.push("--dump-single-json".into());

    if let Some(proxy) = app
        .try_state::<ConfigStore>()
        .and_then(|c| c.get().proxy_url())
    {
        args.push("--proxy".into());
        args.push(proxy);
    }
    args.push(target);

    let mut command = crate::core::process::async_command(&ytdlp);
    command
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let mut child = command
        .spawn()
        .map_err(|e| AppError::Tool(format!("could not start yt-dlp: {e}")))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    *child_slot().lock().await = Some(child);

    // stdout carries the JSON document.
    let json_task = stdout.map(|out| {
        tauri::async_runtime::spawn(async move {
            let mut buffer = String::new();
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                buffer.push_str(&line);
                buffer.push('\n');
            }
            buffer
        })
    });

    // stderr carries progress and error messages.
    let app_for_stderr = app.clone();
    let err_task = stderr.map(|err| {
        tauri::async_runtime::spawn(async move {
            let mut last_emit = Instant::now() - Duration::from_secs(1);
            let mut last_message = String::new();
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(raw)) = lines.next_line().await {
                let cleaned = crate::features::download::parser::strip_ansi(&raw);
                let line = cleaned.trim();
                if line.is_empty() || line.starts_with("WARNING:") {
                    continue;
                }
                if line.starts_with('[') || line.starts_with("ERROR:") {
                    if last_emit.elapsed() >= Duration::from_millis(250) {
                        last_emit = Instant::now();
                        last_message = line.to_string();
                        emit_progress(&app_for_stderr, last_message.clone());
                    }
                }
            }
            last_message
        })
    });

    let json_text = match json_task {
        Some(task) => task.await.unwrap_or_default(),
        None => String::new(),
    };
    let last_error = match err_task {
        Some(task) => task.await.unwrap_or_default(),
        None => String::new(),
    };

    let success = {
        let mut slot = child_slot().lock().await;
        match slot.take() {
            Some(mut child) => child.wait().await.map(|s| s.success()).unwrap_or(false),
            None => false,
        }
    };

    if CANCELLED.load(Ordering::SeqCst) {
        return Err(AppError::Cancelled);
    }

    if json_text.trim().is_empty() {
        let detail = if last_error.is_empty() {
            "could not read video information".to_string()
        } else {
            last_error
        };
        return Err(AppError::Invalid(detail));
    }

    let json: Value = serde_json::from_str(json_text.trim())
        .map_err(|_| AppError::Invalid("could not parse metadata".into()))?;

    if !success && json.is_null() {
        return Err(AppError::Invalid("yt-dlp failed".into()));
    }

    let result = parse_result(&json, kind)?;
    emit_progress(&app, "done");
    Ok(result)
}

#[tauri::command]
pub async fn cancel_fetch() {
    CANCELLED.store(true, Ordering::SeqCst);
    let mut slot = child_slot().lock().await;
    if let Some(child) = slot.as_mut() {
        let _ = child.start_kill();
    }
}

fn parse_result(json: &Value, kind: UrlKind) -> AppResult<InfoResult> {
    match kind {
        UrlKind::Video => parse_video(json),
        UrlKind::Playlist | UrlKind::Channel => parse_collection(json, kind),
    }
}

fn parse_video(json: &Value) -> AppResult<InfoResult> {
    let id = json["id"].as_str().unwrap_or_default().to_string();
    if id.is_empty() {
        return Err(AppError::Invalid("no video found at this URL".into()));
    }

    let mut heights: Vec<u32> = Vec::new();
    if let Some(formats) = json["formats"].as_array() {
        for format in formats {
            let has_video = format["vcodec"]
                .as_str()
                .map(|codec| codec != "none")
                .unwrap_or(false);
            if let Some(height) = format["height"].as_u64() {
                let height = height as u32;
                if has_video && height >= 144 && !heights.contains(&height) {
                    heights.push(height);
                }
            }
        }
    }
    heights.sort_unstable_by(|a, b| b.cmp(a));

    Ok(InfoResult::Video {
        id,
        title: json["title"].as_str().unwrap_or("Untitled").to_string(),
        duration: json["duration"].as_f64(),
        thumbnail: json["thumbnail"].as_str().map(str::to_string),
        uploader: json["uploader"]
            .as_str()
            .or_else(|| json["channel"].as_str())
            .map(str::to_string),
        heights,
    })
}

fn parse_collection(json: &Value, kind: UrlKind) -> AppResult<InfoResult> {
    let raw_entries = json["entries"].as_array().cloned().unwrap_or_default();
    let truncated = raw_entries.len() > MAX_ENTRIES;

    let mut entries: Vec<VideoEntry> = raw_entries
        .into_iter()
        .take(MAX_ENTRIES)
        .filter_map(|entry| {
            let id = entry["id"].as_str().unwrap_or_default().to_string();
            if id.is_empty() {
                return None;
            }
            let url = entry["url"]
                .as_str()
                .map(|url| {
                    if url.starts_with("http") {
                        url.to_string()
                    } else {
                        format!("https://www.youtube.com/watch?v={url}")
                    }
                })
                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}"));

            Some(VideoEntry {
                index: 0,
                id,
                title: entry["title"].as_str().unwrap_or("[unavailable]").to_string(),
                duration: entry["duration"].as_f64(),
                thumbnail: entry["thumbnails"]
                    .as_array()
                    .and_then(|list| list.last())
                    .and_then(|thumb| thumb["url"].as_str())
                    .map(str::to_string),
                url,
            })
        })
        .collect();

    if entries.is_empty() {
        return Err(AppError::Invalid("no videos found".into()));
    }

    // yt-dlp lists channels newest-first; number them oldest-first instead.
    if kind == UrlKind::Channel {
        entries.reverse();
    }
    for (position, entry) in entries.iter_mut().enumerate() {
        entry.index = (position + 1) as u32;
    }

    Ok(InfoResult::Collection {
        coll_type: kind.as_str().to_string(),
        id: json["id"].as_str().unwrap_or_default().to_string(),
        title: json["title"]
            .as_str()
            .or_else(|| json["uploader"].as_str())
            .or_else(|| json["channel"].as_str())
            .unwrap_or("Collection")
            .to_string(),
        uploader: json["uploader"]
            .as_str()
            .or_else(|| json["channel"].as_str())
            .map(str::to_string),
        thumbnail: json["thumbnails"]
            .as_array()
            .and_then(|list| list.last())
            .and_then(|thumb| thumb["url"].as_str())
            .map(str::to_string),
        entries,
        truncated,
    })
}
