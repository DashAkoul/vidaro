use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

use crate::binaries::ytdlp_path;
use crate::util::{classify_url, normalize_channel_url, UrlKind};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoEntry {
    pub index: u32,
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub url: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum InfoResult {
    Video {
        id: String,
        title: String,
        duration: Option<f64>,
        thumbnail: Option<String>,
        uploader: Option<String>,
        /// available video heights, descending
        heights: Vec<u32>,
    },
    Collection {
        coll_type: String, // "playlist" | "channel"
        id: String,
        title: String,
        uploader: Option<String>,
        thumbnail: Option<String>,
        /// numbered 1..N; playlist order, or oldest→newest for channels
        entries: Vec<VideoEntry>,
        truncated: bool,
    },
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InfoProgress {
    message: String,
}

static FETCHING: AtomicBool = AtomicBool::new(false);

/// Fetch metadata / video list without downloading anything.
/// `kind`: "auto" | "video" | "playlist" | "channel"
#[tauri::command]
pub async fn fetch_info(
    app: AppHandle,
    url: String,
    kind: Option<String>,
) -> Result<InfoResult, String> {
    if FETCHING.swap(true, Ordering::SeqCst) {
        return Err("already-fetching".to_string());
    }
    let result = fetch_inner(&app, &url, kind).await;
    FETCHING.store(false, Ordering::SeqCst);
    result
}

#[tauri::command]
pub fn cancel_fetch() {
    FETCHING.store(false, Ordering::SeqCst);
    if let Some(child) = FETCH_CHILD.lock().unwrap().as_mut() {
        let _ = child.kill();
    }
}

static FETCH_CHILD: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

async fn fetch_inner(
    app: &AppHandle,
    url: &str,
    kind: Option<String>,
) -> Result<InfoResult, String> {
    if !ytdlp_path(app).exists() {
        return Err("ytdlp-missing".to_string());
    }

    let kind = match kind.as_deref() {
        Some("video") => UrlKind::Video,
        Some("playlist") => UrlKind::Playlist,
        Some("channel") => UrlKind::Channel,
        _ => classify_url(url),
    };

    let fetch_url = match kind {
        UrlKind::Channel => normalize_channel_url(url),
        _ => url.to_string(),
    };

    let mut args: Vec<String> = vec![
        "--no-warnings".to_string(),
        "--socket-timeout".to_string(),
        "30".to_string(),
        "--retries".to_string(),
        "3".to_string(),
        "--extractor-retries".to_string(),
        "2".to_string(),
    ];
    match kind {
        UrlKind::Video => {
            args.push("--dump-single-json".to_string());
        }
        _ => {
            args.push("--flat-playlist".to_string());
            args.push("--dump-single-json".to_string());
        }
    }
    args.push(fetch_url);

    let mut command = Command::new(ytdlp_path(app));
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    {
        let mut guard = FETCH_CHILD.lock().unwrap();
        *guard = Some(child);
    }

    // Newer yt-dlp builds write everything (JSON, progress lines) to stdout;
    // older ones put progress on stderr. Read both: lines starting with "["
    // are progress messages, everything else accumulates as the JSON result.
    let json_buf = Arc::new(std::sync::Mutex::new(String::new()));
    let progress_app = app.clone();
    let buf1 = json_buf.clone();
    let h1 = std::thread::spawn(move || {
        read_stream(stdout, &progress_app, &buf1);
    });
    let progress_app2 = app.clone();
    let h2 = std::thread::spawn(move || {
        let sink = Arc::new(Mutex::new(String::new()));
        read_stream(stderr, &progress_app2, &sink);
    });

    let _ = h1.join();
    let _ = h2.join();
    if let Some(mut child) = FETCH_CHILD.lock().unwrap().take() {
        let _ = child.wait();
        if !FETCHING.load(Ordering::SeqCst) {
            // cancelled while running
            return Err("cancelled".to_string());
        }
    }

    let output = json_buf.lock().unwrap().clone();
    let json: Value = serde_json::from_str(output.trim())
        .map_err(|_| "Could not parse metadata (bad URL or network error?)".to_string())?;

    let result = parse_result(&json, kind).map_err(|e| e.to_string())?;
    // hint the UI that listing is done
    let _ = app.emit(
        "info-progress",
        InfoProgress {
            message: "done".to_string(),
        },
    );
    Ok(result)
}

fn read_stream<R: std::io::Read + Send>(
    reader: R,
    app: &AppHandle,
    json_buf: &Arc<Mutex<String>>,
) {
    let reader = std::io::BufReader::new(reader);
    let mut last_emit = std::time::Instant::now() - std::time::Duration::from_secs(1);
    for line in reader.lines().map_while(Result::ok) {
        let line = crate::util::strip_ansi(&line);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("ERROR:") {
            let _ = app.emit("info-progress", InfoProgress { message: trimmed.to_string() });
        } else if trimmed.starts_with("WARNING:") {
            // ignore
        } else if trimmed.starts_with('[') {
            if last_emit.elapsed() >= std::time::Duration::from_millis(250) {
                last_emit = std::time::Instant::now();
                let _ = app.emit("info-progress", InfoProgress { message: trimmed.to_string() });
            }
        } else {
            let mut buf = json_buf.lock().unwrap();
            buf.push_str(&line);
            buf.push('\n');
        }
    }
}

fn parse_result(json: &Value, kind: UrlKind) -> Result<InfoResult, &'static str> {
    match kind {
        UrlKind::Video => {
            let id = json["id"].as_str().unwrap_or_default().to_string();
            let title = json["title"]
                .as_str()
                .unwrap_or("Untitled")
                .to_string();
            let duration = json["duration"].as_f64();
            let thumbnail = json["thumbnail"].as_str().map(|s| s.to_string());
            let uploader = json["uploader"]
                .as_str()
                .or_else(|| json["channel"].as_str())
                .map(|s| s.to_string());
            let mut heights: Vec<u32> = Vec::new();
            if let Some(formats) = json["formats"].as_array() {
                for f in formats {
                    let has_video = f["vcodec"].as_str().map(|v| v != "none").unwrap_or(false);
                    if let Some(h) = f["height"].as_u64() {
                        if has_video && h >= 144 && !heights.contains(&(h as u32)) {
                            heights.push(h as u32);
                        }
                    }
                }
            }
            heights.sort_unstable_by(|a, b| b.cmp(a));
            Ok(InfoResult::Video {
                id,
                title,
                duration,
                thumbnail,
                uploader,
                heights,
            })
        }
        UrlKind::Playlist | UrlKind::Channel => {
            let id = json["id"].as_str().unwrap_or_default().to_string();
            let coll_type = if kind == UrlKind::Channel {
                "channel".to_string()
            } else {
                "playlist".to_string()
            };
            let title = json["title"]
                .as_str()
                .or_else(|| json["uploader"].as_str())
                .or_else(|| json["channel"].as_str())
                .unwrap_or("Collection")
                .to_string();
            let uploader = json["uploader"]
                .as_str()
                .or_else(|| json["channel"].as_str())
                .map(|s| s.to_string());
            let thumbnail = json["thumbnails"]
                .as_array()
                .and_then(|a| a.last())
                .and_then(|t| t["url"].as_str())
                .map(|s| s.to_string());

            let raw_entries = json["entries"].as_array().cloned().unwrap_or_default();
            let truncated = raw_entries.len() > 5000;
            let mut entries: Vec<VideoEntry> = Vec::new();
            for v in raw_entries.into_iter().take(5000) {
                let vid = v["id"].as_str().unwrap_or_default().to_string();
                if vid.is_empty() {
                    continue;
                }
                let title = v["title"]
                    .as_str()
                    .unwrap_or("[unavailable]")
                    .to_string();
                let duration = v["duration"].as_f64();
                let thumbnail = v["thumbnails"]
                    .as_array()
                    .and_then(|a| a.last())
                    .and_then(|t| t["url"].as_str())
                    .map(|s| s.to_string());
                let url = v["url"]
                    .as_str()
                    .map(|s| {
                        if s.starts_with("http") {
                            s.to_string()
                        } else {
                            format!("https://www.youtube.com/watch?v={}", s)
                        }
                    })
                    .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={vid}"));
                entries.push(VideoEntry {
                    index: 0,
                    id: vid,
                    title,
                    duration,
                    thumbnail,
                    url,
                });
            }
            if entries.is_empty() {
                return Err("no-videos-found");
            }

            if kind == UrlKind::Channel {
                // yt-dlp lists channels newest-first; reverse so numbering
                // runs from the oldest video to the newest.
                entries.reverse();
            }
            for (i, e) in entries.iter_mut().enumerate() {
                e.index = (i + 1) as u32;
            }
            Ok(InfoResult::Collection {
                coll_type,
                id,
                title,
                uploader,
                thumbnail,
                entries,
                truncated,
            })
        }
    }
}
