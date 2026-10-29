//! Fetching video / playlist / channel metadata without downloading.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager as _};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::Mutex;

use crate::core::config::ConfigStore;
use crate::core::error::{AppError, AppResult};
use crate::core::events;
use crate::features::metadata::model::{InfoProgress, InfoResult, RawInfo, RawThumbnail, VideoEntry};
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

    // stdout carries the JSON document. Read it as raw bytes: decoding to a
    // `String` first would mean validating and buffering the whole document a
    // second time before the parser sees it.
    let json_task = stdout.map(|out| {
        tauri::async_runtime::spawn(async move {
            let mut buffer: Vec<u8> = Vec::new();
            let mut reader = BufReader::new(out);
            let _ = reader.read_to_end(&mut buffer).await;
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

    let json_bytes = match json_task {
        Some(task) => task.await.unwrap_or_default(),
        None => Vec::new(),
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

    if json_bytes.iter().all(|b| b.is_ascii_whitespace()) {
        let detail = if !last_error.is_empty() {
            last_error
        } else if !success {
            "yt-dlp exited with an error".to_string()
        } else {
            "could not read video information".to_string()
        };
        return Err(AppError::Invalid(detail));
    }

    // Parsing a large channel listing is CPU-bound, so keep it off the runtime.
    let raw: RawInfo = tauri::async_runtime::spawn_blocking(move || {
        serde_json::from_slice::<RawInfo>(&json_bytes)
    })
    .await
    .map_err(|e| AppError::Tool(format!("parse task failed: {e}")))?
    .map_err(|e| AppError::Invalid(format!("could not parse metadata: {e}")))?;

    let result = parse_result(&raw, kind)?;
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

fn parse_result(raw: &RawInfo, kind: UrlKind) -> AppResult<InfoResult> {
    match kind {
        UrlKind::Video => parse_video(raw),
        UrlKind::Playlist | UrlKind::Channel => parse_collection(raw, kind),
    }
}

/// Last thumbnail in the list is the highest resolution one.
fn best_thumbnail(list: Option<&[RawThumbnail]>) -> Option<String> {
    list.and_then(|items| items.last())
        .and_then(|thumb| thumb.url.clone())
}

fn parse_video(raw: &RawInfo) -> AppResult<InfoResult> {
    let id = raw.id.clone().unwrap_or_default();
    if id.is_empty() {
        return Err(AppError::Invalid("no video found at this URL".into()));
    }

    let mut heights: Vec<u32> = Vec::new();
    for format in raw.formats.iter().flatten() {
        let has_video = format
            .vcodec
            .as_deref()
            .map(|codec| codec != "none")
            .unwrap_or(false);
        if let Some(height) = format.height {
            if has_video && height >= 144 && !heights.contains(&height) {
                heights.push(height);
            }
        }
    }
    heights.sort_unstable_by(|a, b| b.cmp(a));

    Ok(InfoResult::Video {
        id,
        title: raw.title.clone().unwrap_or_else(|| "Untitled".to_string()),
        duration: raw.duration,
        thumbnail: raw.thumbnail.clone(),
        uploader: raw.uploader.clone().or_else(|| raw.channel.clone()),
        heights,
    })
}

fn parse_collection(raw: &RawInfo, kind: UrlKind) -> AppResult<InfoResult> {
    let listed = raw.entries.as_deref().unwrap_or_default();
    let truncated = listed.len() > MAX_ENTRIES;

    let mut entries: Vec<VideoEntry> = listed
        .iter()
        .take(MAX_ENTRIES)
        // yt-dlp emits `null` for videos that became unavailable.
        .flatten()
        .filter_map(|entry| {
            let id = entry.id.clone().unwrap_or_default();
            if id.is_empty() {
                return None;
            }

            let url = match entry.url.as_deref() {
                Some(url) if url.starts_with("http") => url.to_string(),
                Some(url) => format!("https://www.youtube.com/watch?v={url}"),
                None => format!("https://www.youtube.com/watch?v={id}"),
            };

            Some(VideoEntry {
                index: 0,
                id,
                title: entry
                    .title
                    .clone()
                    .unwrap_or_else(|| "[unavailable]".to_string()),
                duration: entry.duration,
                thumbnail: best_thumbnail(entry.thumbnails.as_deref()),
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
        id: raw.id.clone().unwrap_or_default(),
        title: raw
            .title
            .clone()
            .or_else(|| raw.uploader.clone())
            .or_else(|| raw.channel.clone())
            .unwrap_or_else(|| "Collection".to_string()),
        uploader: raw.uploader.clone().or_else(|| raw.channel.clone()),
        thumbnail: best_thumbnail(raw.thumbnails.as_deref()),
        entries,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIDEO_JSON: &str = r#"{
        "id": "abc123",
        "title": "A video",
        "duration": 95,
        "thumbnail": "https://i.ytimg.com/vi/abc123/maxresdefault.jpg",
        "uploader": "Someone",
        "formats": [
            {"height": 720, "vcodec": "avc1"},
            {"height": 1080, "vcodec": "avc1"},
            {"height": 1080, "vcodec": "none"},
            {"height": 144, "vcodec": "avc1"}
        ]
    }"#;

    const CHANNEL_JSON: &str = r#"{
        "id": "UC123",
        "title": "A channel",
        "uploader": "Someone",
        "thumbnails": [{"url": "https://i.ytimg.com/a.jpg"}, {"url": "https://i.ytimg.com/b.jpg"}],
        "entries": [
            {"id": "newest", "title": "Newest", "duration": 10},
            {"id": "older", "title": "Older", "duration": 20},
            null,
            {"id": "oldest", "title": "Oldest"}
        ]
    }"#;

    fn raw(json: &str) -> RawInfo {
        serde_json::from_str(json).expect("sample json must parse")
    }

    #[test]
    fn video_heights_are_unique_and_sorted() {
        let result = parse_video(&raw(VIDEO_JSON)).unwrap();
        match result {
            InfoResult::Video { heights, title, .. } => {
                assert_eq!(heights, vec![1080, 720, 144]);
                assert_eq!(title, "A video");
            }
            _ => panic!("expected a video"),
        }
    }

    #[test]
    fn channel_is_reversed_and_numbered() {
        let result = parse_collection(&raw(CHANNEL_JSON), UrlKind::Channel).unwrap();
        match result {
            InfoResult::Collection { entries, truncated, thumbnail, .. } => {
                assert!(!truncated);
                let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
                // Oldest first, with the null entry dropped.
                assert_eq!(ids, vec!["oldest", "older", "newest"]);
                assert_eq!(entries[0].index, 1);
                assert_eq!(entries[2].index, 3);
                // Missing duration is tolerated.
                assert_eq!(entries[0].duration, None);
                assert_eq!(thumbnail.as_deref(), Some("https://i.ytimg.com/b.jpg"));
            }
            _ => panic!("expected a collection"),
        }
    }

    #[test]
    fn empty_collection_is_an_error() {
        let json = r#"{"id": "x", "entries": []}"#;
        assert!(parse_collection(&raw(json), UrlKind::Playlist).is_err());
    }
}
