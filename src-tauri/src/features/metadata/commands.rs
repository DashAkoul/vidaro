//! Fetching video / playlist / channel metadata without downloading.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager as _};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::Mutex;

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

    // YouTube intermittently answers with a bot check or a degraded player
    // response. Those clear up on their own, so retry a couple of times before
    // reporting a failure.
    let mut attempt = 0u8;
    loop {
        attempt += 1;
        CANCELLED.store(false, Ordering::SeqCst);

        let error = match fetch_once(&app, &url, kind.as_deref()).await {
            Ok(result) => return Ok(result),
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(error) => error,
        };

        if attempt >= 3 || !is_transient(&error.to_string()) {
            return Err(error);
        }
        emit_progress(&app, format!("retrying after: {error}"));
        // A bot check can last a few seconds; a fixed 1.2 s delay retried into
        // the same block every time. Back off progressively.
        tokio::time::sleep(Duration::from_millis(2000 * attempt as u64)).await;
    }
}

/// Errors that are worth retrying: YouTube throttles and bot-checks
/// intermittently, so the same request usually succeeds a moment later.
fn is_transient(message: &str) -> bool {
    const MARKERS: [&str; 6] = [
        "Sign in to confirm",
        "not a bot",
        "Requested format is not available",
        "page needs to be reloaded",
        "No video formats found",
        "Unable to extract",
    ];
    MARKERS.iter().any(|marker| message.contains(marker))
}

/// Strip yt-dlp's decorations so the UI shows a readable sentence.
fn clean_error(raw: &str) -> String {
    let mut text = raw.trim();
    if let Some(rest) = text.strip_prefix("ERROR: ") {
        text = rest;
    }
    if text.starts_with('[') {
        if let Some(bracket) = text.find("] ") {
            let rest = &text[bracket + 2..];
            text = match rest.find(": ") {
                Some(colon) if !rest[..colon].contains(' ') => &rest[colon + 2..],
                _ => rest,
            };
        }
    }
    text.trim().to_string()
}

/// Pick the most useful diagnostic out of yt-dlp's stderr tail.
///
/// An `ERROR:` always wins; a `WARNING:` that mentions a bot challenge wins
/// over unrelated progress chatter; otherwise the last meaningful line is
/// better than a generic "could not read this URL".
fn best_error_line(lines: &[String]) -> String {
    let is_block = |line: &str| {
        line.contains("not a bot")
            || line.contains("Sign in to confirm")
            || line.contains("unavailable")
            || line.contains("Unable to extract")
    };

    if let Some(line) = lines
        .iter()
        .rev()
        .find(|line| line.starts_with("ERROR:") || is_block(line))
    {
        return line.clone();
    }
    if let Some(line) = lines.iter().rev().find(|line| line.starts_with("WARNING:")) {
        return line.clone();
    }
    lines
        .iter()
        .rev()
        .find(|line| !line.starts_with("[download]"))
        .cloned()
        .unwrap_or_default()
}

/// Turn the two failure modes users actually hit into actionable sentences.
fn friendly_error(raw: &str) -> String {
    let message = clean_error(raw);

    if message.contains("not a bot")
        || message.contains("Sign in to confirm")
        || message.contains("confirm you’re not a bot")
    {
        return "YouTube asked to confirm you are not a bot. This is a temporary block on your network/IP, not on the video — switch to another VPN or proxy server (Settings → Proxy), or wait a few minutes and try again."
            .to_string();
    }
    if message.contains("Video unavailable") || message.contains("This video is unavailable") {
        return "This video is unavailable — it may be deleted, private, or blocked in your region."
            .to_string();
    }
    message
}

async fn fetch_once(app: &AppHandle, url: &str, kind: Option<&str>) -> AppResult<InfoResult> {

    // Also the security boundary: an option-looking string never reaches yt-dlp.
    let Some(trimmed) = crate::features::metadata::url::normalize_url(url) else {
        return Err(AppError::Invalid(format!("not a valid URL: {url}")));
    };

    let kind = match kind {
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

    // yt-dlp inherits the process/registry proxy unless told otherwise, so
    // "off" needs an explicit empty value.
    match crate::core::state::config(&app).get().proxy_setting() {
        crate::core::config::ProxySetting::Url(proxy) => {
            args.push("--proxy".into());
            args.push(proxy);
        }
        crate::core::config::ProxySetting::Direct => {
            args.push("--proxy".into());
            args.push(String::new());
        }
        crate::core::config::ProxySetting::Inherit => {}
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
    //
    // Rate limiting belongs to the *progress events*, not to the diagnostics:
    // dropping every line that arrives within 250 ms of the previous one used
    // to swallow the final `ERROR:` line, leaving the UI with a meaningless
    // "yt-dlp could not read this URL". Keep the tail of the stream and pick
    // the most informative line afterwards.
    let app_for_stderr = app.clone();
    let err_task = stderr.map(|err| {
        tauri::async_runtime::spawn(async move {
            let mut last_emit = Instant::now() - Duration::from_secs(1);
            let mut lines: Vec<String> = Vec::new();
            let mut reader = BufReader::new(err).lines();
            while let Ok(Some(raw)) = reader.next_line().await {
                let cleaned = crate::features::download::parser::strip_ansi(&raw);
                let line = cleaned.trim();
                if line.is_empty() {
                    continue;
                }
                // Bound memory for pathological output while keeping the tail.
                if lines.len() >= 200 {
                    lines.remove(0);
                }
                lines.push(line.to_string());

                if line.starts_with('[') || line.starts_with("ERROR:") {
                    if last_emit.elapsed() >= Duration::from_millis(250) {
                        last_emit = Instant::now();
                        emit_progress(&app_for_stderr, line.to_string());
                    }
                }
            }
            best_error_line(&lines)
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

    let (success, exit_code) = {
        let mut slot = child_slot().lock().await;
        match slot.take() {
            Some(mut child) => match child.wait().await {
                Ok(status) => (status.success(), status.code()),
                Err(_) => (false, None),
            },
            None => (false, None),
        }
    };
    let exit_note = exit_code
        .map(|code| format!(" (exit code {code})"))
        .unwrap_or_default();

    if CANCELLED.load(Ordering::SeqCst) {
        return Err(AppError::Cancelled);
    }

    if json_bytes.iter().all(|b| b.is_ascii_whitespace()) {
        let detail = if !last_error.is_empty() {
            friendly_error(&last_error)
        } else if !success {
            format!("yt-dlp exited with an error{exit_note}")
        } else {
            "could not read video information".to_string()
        };
        return Err(AppError::Invalid(detail));
    }

    // Parsing a large channel listing is CPU-bound, so keep it off the runtime.
    // `Option<RawInfo>` accepts the bare `null` yt-dlp prints when it could not
    // extract anything, instead of failing with a serde type error.
    let parsed = tauri::async_runtime::spawn_blocking(move || {
        serde_json::from_slice::<Option<RawInfo>>(&json_bytes)
    })
    .await
    .map_err(|e| AppError::Tool(format!("parse task failed: {e}")))?;

    let raw = match parsed {
        Ok(Some(raw)) => raw,
        // No document: report what yt-dlp actually said.
        Ok(None) => {
            return Err(AppError::Invalid(if !last_error.is_empty() {
                friendly_error(&last_error)
            } else if !success {
                format!(
                    "yt-dlp returned no video information{exit_note}. Check the download tools and your network/proxy settings."
                )
            } else {
                "yt-dlp returned no video information".to_string()
            }));
        }
        Err(e) => {
            let detail = if last_error.is_empty() {
                e.to_string()
            } else {
                format!("{} ({})", e, friendly_error(&last_error))
            };
            return Err(AppError::Invalid(format!("could not parse metadata: {detail}")));
        }
    };

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
    fn bare_null_is_accepted() {
        // yt-dlp prints `null` when it could not extract anything.
        let parsed: Option<RawInfo> = serde_json::from_slice(b"null").unwrap();
        assert!(parsed.is_none());
    }

    #[test]
    fn error_messages_are_cleaned() {
        assert_eq!(
            clean_error("ERROR: [youtube] cXeLjuzQglg: This video is unavailable"),
            "This video is unavailable"
        );
        assert_eq!(clean_error("ERROR: something broke"), "something broke");
        assert_eq!(clean_error("plain message"), "plain message");
    }

    #[test]
    fn unavailable_video_is_not_retried() {
        assert!(!is_transient("This video is unavailable"));
        assert!(is_transient("Sign in to confirm you are not a bot"));
        assert!(is_transient("Requested format is not available"));
    }

    #[test]
    fn bot_check_becomes_actionable() {
        let message = friendly_error(
            "ERROR: [youtube] abc: Sign in to confirm you’re not a bot. Use --cookies",
        );
        assert!(message.contains("switch to another VPN"), "{message}");
        // The friendly text must still be recognised as transient, otherwise
        // the retry loop would stop after the first failure.
        assert!(is_transient(&message), "{message}");
    }

    /// Regression: the final bot-check line used to be dropped by the progress
    /// rate limiter (and WARNING lines were discarded outright), so the UI
    /// showed "yt-dlp could not read this URL" instead of the real reason.
    #[test]
    fn stderr_tail_keeps_the_bot_check() {
        let lines: Vec<String> = vec![
            "[youtube] Extracting URL: https://www.youtube.com/watch?v=x".into(),
            "[youtube] x: Downloading webpage".into(),
            "[youtube] x: Downloading player API JSON".into(),
            "ERROR: [youtube] x: Sign in to confirm you’re not a bot. Use --cookies".into(),
        ];
        let picked = best_error_line(&lines);
        assert!(picked.contains("not a bot"), "{picked}");
        assert!(friendly_error(&picked).contains("switch to another VPN"));
    }

    #[test]
    fn warning_only_bot_check_is_not_lost() {
        let lines: Vec<String> = vec![
            "[youtube] Extracting URL: https://youtu.be/x".into(),
            "WARNING: [youtube] Sign in to confirm you’re not a bot".into(),
        ];
        let picked = best_error_line(&lines);
        assert!(picked.contains("not a bot"), "{picked}");
    }

    #[test]
    fn empty_stderr_returns_empty() {
        assert_eq!(best_error_line(&[]), "");
    }

    #[test]
    fn empty_collection_is_an_error() {
        let json = r#"{"id": "x", "entries": []}"#;
        assert!(parse_collection(&raw(json), UrlKind::Playlist).is_err());
    }
}
