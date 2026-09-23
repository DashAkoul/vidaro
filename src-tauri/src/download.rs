use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager as _, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::sleep;
use chrono::Timelike;

use crate::binaries::{bin_dir, ytdlp_path};
use crate::history::{self, HistoryEntry};
use crate::settings;
use crate::util::sanitize_name;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub url: String,
    pub video_id: String,
    pub title: String,
    pub index: Option<u32>, // None = single video (no numbering)
    pub dir: String,        // destination folder for this item
    pub prefix: String,     // e.g. "003 - " ("" for single videos)
    pub quality: String,    // "best" | "2160" | "1080" ...
    pub audio_only: bool,
    pub batch_name: String,
    pub status: String, // queued active paused completed error cancelled scheduled
    pub percent: f64,
    pub speed_bps: f64,
    pub eta_secs: f64,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub stage: String, // download | merge | audio | done
    pub error: Option<String>,
    pub output_path: Option<String>,
    pub created_at: i64,
    // Scheduler (per-item)
    pub schedule_start: Option<String>, // "HH:MM" 24h format
    pub schedule_end: Option<String>,   // "HH:MM" 24h format
}

pub struct Control {
    pub paused: bool,
    pub cancelled: bool,
}

pub struct ItemInner {
    pub item: tokio::sync::Mutex<Item>,
    pub child: tokio::sync::Mutex<Option<tokio::process::Child>>,
    pub control: Mutex<Control>,
}

impl ItemInner {
    pub fn new(item: Item) -> Arc<Self> {
        Arc::new(Self {
            item: tokio::sync::Mutex::new(item),
            child: tokio::sync::Mutex::new(None),
            control: Mutex::new(Control {
                paused: false,
                cancelled: false,
            }),
        })
    }
}

/// Resizable concurrency gate: running downloads hold a slot.
pub struct Gate {
    state: Mutex<GateState>,
    notify: tokio::sync::Notify,
}

struct GateState {
    active: usize,
    max: usize,
}

impl Gate {
    pub fn new(max: usize) -> Self {
        Self {
            state: Mutex::new(GateState { active: 0, max }),
            notify: tokio::sync::Notify::new(),
        }
    }

    pub async fn acquire(&self) {
        loop {
            {
                let mut g = self.state.lock().unwrap();
                if g.active < g.max {
                    g.active += 1;
                    return;
                }
            }
            let notified = self.notify.notified();
            notified.await;
        }
    }

    pub fn release(&self) {
        let mut g = self.state.lock().unwrap();
        g.active = g.active.saturating_sub(1);
        drop(g);
        self.notify.notify_one();
    }

    pub fn set_max(&self, max: usize) {
        let mut g = self.state.lock().unwrap();
        g.max = max.max(1);
        drop(g);
        self.notify.notify_one();
    }
}

pub struct Inner {
    pub items: Mutex<BTreeMap<String, Arc<ItemInner>>>,
    pub gate: Gate,
}

pub struct Manager {
    pub inner: Arc<Inner>,
}

impl Manager {
    pub fn new(max: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                items: Mutex::new(BTreeMap::new()),
                gate: Gate::new(max.max(1)),
            }),
        }
    }
}

/// Wrapper so spawned tasks can share the manager's gate without a State.
pub struct GateProxy(pub Arc<Inner>);

impl GateProxy {
    pub async fn acquire(&self) {
        self.0.gate.acquire().await;
    }
    pub fn release(&self) {
        self.0.gate.release();
    }
}

// ---------------------------------------------------------------------------
// Events & persistence
// ---------------------------------------------------------------------------

/// Shared state for parsing yt-dlp output across both streams.
pub struct ParseState {
    pub errored: Option<String>,
    pub output_path: Option<String>,
    pub last_emit: Instant,
}

pub fn emit_item(app: &AppHandle, item: &Item) {
    let _ = app.emit("item-updated", item);
}

pub fn emit_queue_changed(app: &AppHandle) {
    let _ = app.emit("queue-changed", ());
}

pub fn queue_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("queue.json")
}

pub fn persist_queue(app: &AppHandle, inner: &Inner) {
    let items: Vec<Item> = {
        let map = inner.items.lock().unwrap();
        let mut v = Vec::new();
        for k in map.values() {
            if let Ok(item) = k.item.try_lock() {
                v.push(item.clone());
            }
        }
        v
    };
    if let Ok(json) = serde_json::to_string_pretty(&items) {
        if let Some(parent) = queue_path(app).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(queue_path(app), json);
    }
}

/// Called once at startup: previously active/queued downloads become paused
/// so the user can resume them (numbering manifest lives in queue.json).
pub fn restore_queue(app: &AppHandle) {
    let Ok(text) = std::fs::read_to_string(queue_path(app)) else {
        return;
    };
    let Ok(items) = serde_json::from_str::<Vec<Item>>(&text) else {
        return;
    };
    let mgr = app.state::<Manager>();
    for mut item in items {
        if item.status == "active" || item.status == "queued" {
            item.status = "paused".to_string();
            item.percent = 0.0;
            item.speed_bps = 0.0;
            item.eta_secs = 0.0;
        }
        let id = item.id.clone();
        let inner = ItemInner::new(item);
        mgr.inner.items.lock().unwrap().insert(id, inner);
    }
}

/// Start background task to monitor global scheduler settings
pub fn start_global_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            
            let settings = settings::load(&app);
            if !settings.scheduler_enabled {
                continue;
            }
            
            let (start_min, end_min) = match (parse_time(&settings.scheduler_start), parse_time(&settings.scheduler_end)) {
                (Ok(s), Ok(e)) => (s, e),
                _ => continue,
            };
            
            let now = now_minutes();
            let in_window = if start_min <= end_min {
                now >= start_min && now < end_min
            } else {
                now >= start_min || now < end_min
            };
            
            let mgr = app.state::<Manager>();
            let items_to_update: Vec<(String, String)> = {
                let map = mgr.inner.items.lock().unwrap();
                map.values()
                    .filter_map(|inner| {
                        inner.item.try_lock().ok().map(|item| {
                            let status = item.status.clone();
                            let id = item.id.clone();
                            let has_schedule = item.schedule_start.is_some() && item.schedule_end.is_some();
                            (id, status, has_schedule)
                        })
                    })
                    .filter(|(_, status, has_schedule)| {
                        // Only auto-pause/resume items without per-item schedule
                        !has_schedule && matches!(status.as_str(), "queued" | "active" | "paused")
                    })
                    .map(|(id, status, _)| {
                        let new_status: String = if in_window {
                            if status == "paused" { "queued".to_string() } else { status }
                        } else {
                            if status == "queued" || status == "active" { "paused".to_string() } else { status }
                        };
                        (id, new_status)
                    })
                    .filter(|(id, new_status)| {
                        // Only include if status actually changes
                        let map = mgr.inner.items.lock().unwrap();
                        map.get(id)
                            .and_then(|inner| inner.item.try_lock().ok())
                            .map(|item| item.status != *new_status)
                            .unwrap_or(false)
                    })
                    .collect()
            };
            
            for (id, new_status) in items_to_update {
                if let Some(inner) = find_inner(&mgr, &id) {
                    let _ = set_status(&app, &inner, &new_status).await;
                    if new_status == "queued" {
                        let gate = GateProxy(mgr.inner.clone());
                        let app2 = app.clone();
                        let inner2 = inner.clone();
                        tauri::async_runtime::spawn(async move {
                            run_item(app2, inner2, gate).await;
                        });
                    }
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewItem {
    pub video_id: String,
    pub title: String,
    pub url: String,
    pub index: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueuePayload {
    pub items: Vec<NewItem>,
    pub dir: String,
    pub batch_name: String,
    pub quality: String,
    pub audio_only: bool,
    pub numbering: bool,
    // Per-item scheduler (optional)
    pub schedule_start: Option<String>,
    pub schedule_end: Option<String>,
}

#[tauri::command]
pub async fn enqueue_batch(
    app: AppHandle,
    mgr: State<'_, Manager>,
    payload: EnqueuePayload,
) -> Result<Vec<Item>, String> {
    let setts = settings::load(&app);
    mgr.inner.gate.set_max(setts.max_concurrent.max(1) as usize);

    // destination folder:
    // - numbered batches (playlist/channel) -> <dir>/<batch name>
    // - single videos                      -> <dir>/Single Videos
    let batch = if payload.numbering {
        sanitize_name(&payload.batch_name)
    } else {
        String::new()
    };
    let root = PathBuf::from(&payload.dir);
    let dest = if payload.numbering && !batch.is_empty() {
        root.join(&batch)
    } else {
        root.join(settings::SINGLE_VIDEOS_DIR)
    };
    std::fs::create_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;

    let mut created: Vec<Item> = Vec::new();
    for ni in &payload.items {
        // de-duplicate: same video into the same folder already in the queue
        {
            let map = mgr.inner.items.lock().unwrap();
            let dest_str = dest.to_string_lossy().to_string();
            let dup = map.values().any(|inner| {
                if let Ok(item) = inner.item.try_lock() {
                    item.video_id == ni.video_id
                        && item.dir == dest_str
                        && !matches!(item.status.as_str(), "error" | "cancelled")
                } else {
                    false
                }
            });
            if dup {
                continue;
            }
        }

        let prefix = match (payload.numbering, ni.index) {
            (true, Some(i)) => format!("{:03} - ", i),
            _ => String::new(),
        };
        let item = Item {
            id: uuid::Uuid::new_v4().to_string(),
            url: ni.url.clone(),
            video_id: ni.video_id.clone(),
            title: ni.title.clone(),
            index: if payload.numbering { ni.index } else { None },
            dir: dest.to_string_lossy().to_string(),
            prefix,
            quality: payload.quality.clone(),
            audio_only: payload.audio_only,
            batch_name: batch.clone(),
            status: if payload.schedule_start.is_some() { "scheduled".to_string() } else { "queued".to_string() },
            percent: 0.0,
            speed_bps: 0.0,
            eta_secs: 0.0,
            total_bytes: 0,
            downloaded_bytes: 0,
            stage: "download".to_string(),
            error: None,
            output_path: None,
            created_at: chrono::Utc::now().timestamp_millis(),
            schedule_start: payload.schedule_start.clone(),
            schedule_end: payload.schedule_end.clone(),
        };
        let inner = ItemInner::new(item.clone());
        mgr.inner
            .items
            .lock()
            .unwrap()
            .insert(item.id.clone(), inner.clone());
        created.push(item);

        let app2 = app.clone();
        let gate = GateProxy(mgr.inner.clone());
        tauri::async_runtime::spawn(run_item(app2, inner, gate));
    }

    persist_queue(&app, &mgr.inner);
    emit_queue_changed(&app);
    Ok(created)
}

async fn set_status(app: &AppHandle, inner: &Arc<ItemInner>, status: &str) -> Item {
    let item = {
        let mut guard = inner.item.lock().await;
        guard.status = status.to_string();
        if status == "queued" {
            guard.percent = 0.0;
            guard.speed_bps = 0.0;
            guard.eta_secs = 0.0;
            guard.stage = "download".to_string();
            guard.error = None;
        }
        guard.clone()
    };
    emit_item(app, &item);
    persist_queue(app, &app.state::<Manager>().inner);
    item
}

pub async fn run_item(app: AppHandle, inner: Arc<ItemInner>, gate: GateProxy) {
    // Check scheduler before acquiring gate
    {
        let item = inner.item.lock().await;
        if let (Some(start_str), Some(end_str)) = (&item.schedule_start, &item.schedule_end) {
            if let (Ok(start), Ok(end)) = (parse_time(start_str), parse_time(end_str)) {
                drop(item);
                wait_for_schedule(start, end).await;
                // Re-check after wait
                let _item = inner.item.lock().await;
                let skip = {
                    let c = inner.control.lock().unwrap();
                    c.paused || c.cancelled
                };
                if skip {
                    return;
                }
            }
        }
    }

    gate.acquire().await;

    // while waiting for a free slot the user may have paused/cancelled
    let skip = {
        let c = inner.control.lock().unwrap();
        c.paused || c.cancelled
    };
    if skip {
        gate.release();
        return;
    }

    let _ = set_status(&app, &inner, "active").await;

    let setts = settings::load(&app);
    let (ytdlp, args) = {
        let item = inner.item.lock().await;
        let ffmpeg_dir = bin_dir(&app);
        (ytdlp_path(&app), build_args(&item, &setts, &ffmpeg_dir))
    };

    let mut cmd = tokio::process::Command::new(&ytdlp);
    cmd.args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW (tokio's own method)
    }

    match cmd.spawn() {
        Ok(mut child) => {
            // Newer yt-dlp builds write progress info to stdout; older ones to
            // stderr — read both streams with shared parse state.
            let state = Arc::new(tokio::sync::Mutex::new(ParseState {
                errored: None,
                output_path: None,
                last_emit: Instant::now() - std::time::Duration::from_secs(1),
            }));

            let stdout: Option<tokio::process::ChildStdout> = child.stdout.take();
            let stderr: Option<tokio::process::ChildStderr> = child.stderr.take();
            {
                let mut guard = inner.child.lock().await;
                *guard = Some(child);
            }

            let mut readers = Vec::new();
            if let Some(s) = stdout {
                let app2 = app.clone();
                let inner2 = inner.clone();
                let state2 = state.clone();
                readers.push(tauri::async_runtime::spawn(async move {
                    let mut reader = BufReader::new(s).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        handle_line(&app2, &inner2, &line, &state2).await;
                    }
                }));
            }
            if let Some(s) = stderr {
                let app2 = app.clone();
                let inner2 = inner.clone();
                let state2 = state.clone();
                readers.push(tauri::async_runtime::spawn(async move {
                    let mut reader = BufReader::new(s).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        handle_line(&app2, &inner2, &line, &state2).await;
                    }
                }));
            }
            for h in readers {
                let _ = h.await;
            }

            let success = {
                let mut guard = inner.child.lock().await;
                if let Some(c) = guard.as_mut() {
                    matches!(c.wait().await, Ok(s) if s.success())
                } else {
                    false
                }
            };
            let _ = inner.child.lock().await.take();

            let (paused, cancelled) = {
                let c = inner.control.lock().unwrap();
                (c.paused, c.cancelled)
            };

            if paused || cancelled {
                // controller already set the status; just release the slot
            } else if success {
                let parsed = {
                    let st = state.lock().await;
                    (st.output_path.clone(), st.errored.clone())
                };
                {
                    let mut guard = inner.item.lock().await;
                    guard.status = "completed".to_string();
                    guard.percent = 100.0;
                    guard.stage = "done".to_string();
                    guard.speed_bps = 0.0;
                    guard.eta_secs = 0.0;
                    if let Some(p) = parsed.0 {
                        guard.output_path = Some(p);
                    }
                }
                let snapshot = inner.item.lock().await.clone();
                emit_item(&app, &snapshot);
                persist_queue(&app, &app.state::<Manager>().inner);

                let item = snapshot;
                let size = item
                    .output_path
                    .as_ref()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .map(|m| m.len())
                    .unwrap_or(0);
                history::add_entry(
                    &app,
                    HistoryEntry {
                        id: item.id.clone(),
                        title: item.title.clone(),
                        path: item.output_path.clone().unwrap_or_else(|| item.dir.clone()),
                        size_bytes: size,
                        finished_at: chrono::Utc::now().timestamp_millis(),
                        kind: if item.audio_only { "audio" } else { "video" }.to_string(),
                        quality: if item.audio_only {
                            "MP3".to_string()
                        } else {
                            item.quality.clone()
                        },
                        batch_name: item.batch_name.clone(),
                    },
                );
                emit_queue_changed(&app);
                maybe_release_finished_batch(&app, &item);
            } else {
                let msg = {
                    let st = state.lock().await;
                    st.errored
                        .clone()
                        .unwrap_or_else(|| "yt-dlp exited with an error".to_string())
                };
                let mut guard = inner.item.lock().await;
                guard.status = "error".to_string();
                guard.error = Some(msg);
                guard.speed_bps = 0.0;
                guard.eta_secs = 0.0;
                let snapshot = guard.clone();
                drop(guard);
                emit_item(&app, &snapshot);
                persist_queue(&app, &app.state::<Manager>().inner);
            }
        }
        Err(e) => {
            let mut guard = inner.item.lock().await;
            guard.status = "error".to_string();
            guard.error = Some(format!("failed to start yt-dlp: {e}"));
            let snapshot = guard.clone();
            drop(guard);
            emit_item(&app, &snapshot);
            persist_queue(&app, &app.state::<Manager>().inner);
        }
    }

    gate.release();
}

async fn handle_line(
    app: &AppHandle,
    inner: &Arc<ItemInner>,
    raw: &str,
    state: &Arc<tokio::sync::Mutex<ParseState>>,
) {
    let line = crate::util::strip_ansi(raw);
    let line = line.trim();
    if line.is_empty() {
        return;
    }

    if let Some(msg) = line.strip_prefix("ERROR:") {
        state.lock().await.errored = Some(msg.trim().to_string());
        return;
    }

    let mut changed = false;
    {
        let mut st = state.lock().await;

        if line.contains("has already been downloaded") {
            if let Some(path) = line.strip_prefix("[download] ") {
                let path = path.replace(" has already been downloaded", "");
                st.output_path = Some(path);
            }
        } else if let Some(rest) = line.strip_prefix("[download] Destination:") {
            st.output_path = Some(rest.trim().to_string());
        } else if line.starts_with("[Merger]") {
            if let Some(p) = parse_quoted_path(line) {
                st.output_path = Some(p);
            }
            let mut guard = inner.item.lock().await;
            guard.stage = "merge".to_string();
            changed = true;
        } else if line.starts_with("[ExtractAudio]") {
            if let Some(rest) = line.strip_prefix("[ExtractAudio] Destination:") {
                st.output_path = Some(rest.trim().to_string());
            }
            let mut guard = inner.item.lock().await;
            guard.stage = "audio".to_string();
            changed = true;
        } else if line.starts_with("[download]") {
            if let Some(prog) = parse_progress(line) {
                let mut guard = inner.item.lock().await;
                guard.percent = prog.percent;
                if let Some(s) = prog.speed_bps {
                    guard.speed_bps = s;
                }
                if let Some(e) = prog.eta_secs {
                    guard.eta_secs = e;
                }
                if let Some(t) = prog.total_bytes {
                    guard.total_bytes = t;
                    guard.downloaded_bytes = (t as f64 * prog.percent / 100.0) as u64;
                }
                drop(guard);
                changed = true;
            }
        }
    }

    if changed && state.lock().await.last_emit.elapsed() >= std::time::Duration::from_millis(200) {
        state.lock().await.last_emit = Instant::now();
        let snapshot = inner.item.lock().await.clone();
        emit_item(app, &snapshot);
    }
}

fn parse_quoted_path(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line.rfind('"')?;
    if end > start {
        Some(line[start + 1..end].to_string())
    } else {
        None
    }
}

struct ParsedProgress {
    percent: f64,
    speed_bps: Option<f64>,
    eta_secs: Option<f64>,
    total_bytes: Option<u64>,
}

/// Parse lines like:
/// "[download]  45.3% of    5.83MiB at    2.34MiB/s ETA 00:12"
/// "[download]   2.1% of ~  101.83MiB at  411.53KiB/s ETA 04:12"
fn parse_progress(line: &str) -> Option<ParsedProgress> {
    let pct_idx = line.find('%')?;
    let before = &line[..pct_idx];
    let percent: f64 = before
        .rsplit(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?
        .parse()
        .ok()?;

    let mut total_bytes = None;
    let mut speed_bps = None;
    let mut eta_secs = None;

    if let Some(rest) = line[pct_idx..].strip_prefix("% of") {
        let rest = rest.trim_start().trim_start_matches('~').trim_start();
        let (size_str, after) = split_token(rest);
        if let Some((v, unit)) = parse_size_token(&size_str) {
            total_bytes = Some(crate::util::parse_size(v, unit) as u64);
        }
        let after = after.trim_start();
        if let Some(rest) = after.strip_prefix("at") {
            let rest = rest.trim_start();
            let (speed_str, after) = split_token(rest);
            speed_bps = parse_speed_token(&speed_str);
            let after = after.trim_start();
            if let Some(rest) = after.strip_prefix("ETA") {
                let (eta_str, _) = split_token(rest.trim_start());
                eta_secs = Some(crate::util::parse_eta(&eta_str));
            }
        }
    }

    Some(ParsedProgress {
        percent,
        speed_bps,
        eta_secs,
        total_bytes,
    })
}

fn split_token(s: &str) -> (String, &str) {
    match s.find(char::is_whitespace) {
        Some(i) => (s[..i].to_string(), &s[i..]),
        None => (s.to_string(), ""),
    }
}

fn parse_size_token(token: &str) -> Option<(f64, &'static str)> {
    for unit in ["TiB", "GiB", "MiB", "KiB", "GB", "MB", "KB"] {
        if let Some(num) = token.strip_suffix(unit) {
            return num.trim().parse::<f64>().ok().map(|v| (v, unit));
        }
    }
    None
}

fn parse_speed_token(token: &str) -> Option<f64> {
    // "2.34MiB/s" -> bytes per second
    let token = token.trim_end_matches("/s");
    for (suffix, mult) in [
        ("TiB", 1024.0f64.powi(4)),
        ("GiB", 1024.0f64.powi(3)),
        ("MiB", 1024.0f64.powi(2)),
        ("KiB", 1024.0),
        ("GB", 1e9),
        ("MB", 1e6),
        ("KB", 1e3),
    ] {
        if let Some(num) = token.strip_suffix(suffix) {
            return num.trim().parse::<f64>().ok().map(|v| v * mult);
        }
    }
    token.trim().parse::<f64>().ok()
}

// ---------------------------------------------------------------------------
// Control commands
// ---------------------------------------------------------------------------

/// When every item of a batch (same folder) has reached a terminal state
/// (completed/cancelled) and none are still queued/active/paused/error,
/// move the completed ones out of the queue — they already live in history.
fn maybe_release_finished_batch(app: &AppHandle, finished: &Item) {
    let mgr = app.state::<Manager>();
    let map = mgr.inner.items.lock().unwrap();
    let same_batch: Vec<(String, String)> = map
        .values()
        .filter_map(|inner| inner.item.try_lock().ok().map(|i| i))
        .filter(|i| i.dir == finished.dir)
        .map(|i| (i.id.clone(), i.status.clone()))
        .collect();
    let still_working = same_batch
        .iter()
        .any(|(_, s)| matches!(s.as_str(), "queued" | "active" | "paused" | "error"));
    if still_working {
        return;
    }
    // drop completed/cancelled items of this batch from the queue
    let mut map = map;
    for (id, status) in same_batch {
        if matches!(status.as_str(), "completed" | "cancelled") {
            map.remove(&id);
        }
    }
    drop(map);
    persist_queue(app, &mgr.inner);
    emit_queue_changed(app);
}

#[tauri::command]
pub fn get_queue(mgr: State<'_, Manager>) -> Vec<Item> {
    let map = mgr.inner.items.lock().unwrap();
    let mut out = Vec::new();
    for v in map.values() {
        if let Ok(item) = v.item.try_lock() {
            out.push(item.clone());
        }
    }
    out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    out
}

fn find_inner(mgr: &Manager, id: &str) -> Option<Arc<ItemInner>> {
    mgr.inner.items.lock().unwrap().get(id).cloned()
}

async fn kill_child(inner: &ItemInner) {
    let mut guard = inner.child.lock().await;
    if let Some(child) = guard.as_mut() {
        let _ = child.kill().await;
    }
}

#[tauri::command]
pub async fn pause_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> Result<(), String> {
    let Some(inner) = find_inner(&mgr, &id) else {
        return Err("not-found".to_string());
    };
    {
        let mut c = inner.control.lock().unwrap();
        c.paused = true;
    }
    kill_child(&inner).await;
    set_status(&app, &inner, "paused").await;
    Ok(())
}

#[tauri::command]
pub async fn resume_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> Result<(), String> {
    let Some(inner) = find_inner(&mgr, &id) else {
        return Err("not-found".to_string());
    };
    {
        let mut c = inner.control.lock().unwrap();
        if c.cancelled {
            return Err("cancelled".to_string());
        }
        c.paused = false;
    }
    set_status(&app, &inner, "queued").await;
    let gate = GateProxy(mgr.inner.clone());
    tauri::async_runtime::spawn(run_item(app, inner, gate));
    Ok(())
}

#[tauri::command]
pub async fn cancel_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> Result<(), String> {
    let Some(inner) = find_inner(&mgr, &id) else {
        return Err("not-found".to_string());
    };
    {
        let mut c = inner.control.lock().unwrap();
        c.cancelled = true;
        c.paused = false;
    }
    kill_child(&inner).await;
    set_status(&app, &inner, "cancelled").await;
    Ok(())
}

#[tauri::command]
pub async fn retry_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> Result<(), String> {
    let Some(inner) = find_inner(&mgr, &id) else {
        return Err("not-found".to_string());
    };
    {
        let mut c = inner.control.lock().unwrap();
        c.cancelled = false;
        c.paused = false;
    }
    {
        let mut guard = inner.item.lock().await;
        guard.output_path = None;
    }
    set_status(&app, &inner, "queued").await;
    let gate = GateProxy(mgr.inner.clone());
    tauri::async_runtime::spawn(run_item(app, inner, gate));
    Ok(())
}

#[tauri::command]
pub async fn remove_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> Result<(), String> {
    let Some(inner) = find_inner(&mgr, &id) else {
        return Err("not-found".to_string());
    };
    {
        let mut c = inner.control.lock().unwrap();
        c.cancelled = true;
    }
    kill_child(&inner).await;
    mgr.inner.items.lock().unwrap().remove(&id);
    persist_queue(&app, &mgr.inner);
    emit_queue_changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn update_item_schedule(
    app: AppHandle,
    mgr: State<'_, Manager>,
    id: String,
    schedule_start: Option<String>,
    schedule_end: Option<String>,
) -> Result<(), String> {
    let Some(inner) = find_inner(&mgr, &id) else {
        return Err("not-found".to_string());
    };
    {
        let mut guard = inner.item.lock().await;
        guard.schedule_start = schedule_start.clone();
        guard.schedule_end = schedule_end.clone();
        // If setting schedule, change status to scheduled if currently queued
        if schedule_start.is_some() && schedule_end.is_some() && guard.status == "queued" {
            guard.status = "scheduled".to_string();
        } else if schedule_start.is_none() && schedule_end.is_none() && guard.status == "scheduled" {
            guard.status = "queued".to_string();
        }
        let snapshot = guard.clone();
        drop(guard);
        emit_item(&app, &snapshot);
        persist_queue(&app, &mgr.inner);
    }
    Ok(())
}

#[tauri::command]
pub async fn pause_all(app: AppHandle, mgr: State<'_, Manager>) -> Result<(), String> {
    pause_all_inner(app, mgr.inner.clone()).await
}

pub async fn pause_all_inner(app: AppHandle, inner: Arc<Inner>) -> Result<(), String> {
    let targets: Vec<Arc<ItemInner>> = {
        let map = inner.items.lock().unwrap();
        map.values()
            .filter(|inner| {
                inner
                    .item
                    .try_lock()
                    .map(|i| i.status == "active" || i.status == "queued")
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    };
    for item in &targets {
        item.control.lock().unwrap().paused = true;
    }
    for item in &targets {
        kill_child(item).await;
        set_status(&app, item, "paused").await;
    }
    Ok(())
}

#[tauri::command]
pub async fn resume_all(app: AppHandle, mgr: State<'_, Manager>) -> Result<(), String> {
    resume_all_inner(app, mgr.inner.clone()).await
}

pub async fn resume_all_inner(app: AppHandle, inner: Arc<Inner>) -> Result<(), String> {
    let targets: Vec<Arc<ItemInner>> = {
        let map = inner.items.lock().unwrap();
        map.values()
            .filter(|inner| {
                inner
                    .item
                    .try_lock()
                    .map(|i| i.status == "paused")
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    };
    for item in &targets {
        item.control.lock().unwrap().paused = false;
    }
    for item in targets {
        set_status(&app, &item, "queued").await;
        let app2 = app.clone();
        let gate = GateProxy(inner.clone());
        tauri::async_runtime::spawn(run_item(app2, item, gate));
    }
    Ok(())
}

#[tauri::command]
pub async fn clear_finished(app: AppHandle, mgr: State<'_, Manager>) -> Result<(), String> {
    {
        let mut map = mgr.inner.items.lock().unwrap();
        let remove: Vec<String> = map
            .iter()
            .filter(|(_, v)| {
                v.item
                    .try_lock()
                    .map(|i| matches!(i.status.as_str(), "completed" | "cancelled"))
                    .unwrap_or(false)
            })
            .map(|(k, _)| k.clone())
            .collect();
        for id in remove {
            map.remove(&id);
        }
    }
    persist_queue(&app, &mgr.inner);
    emit_queue_changed(&app);
    Ok(())
}

/// Open the folder that contains a finished download.
#[tauri::command]
pub fn open_download_folder(app: AppHandle, id: String) -> Result<(), String> {
    let mgr = app.state::<Manager>();
    let Some(inner) = find_inner(&mgr, &id) else {
        return Err("not-found".to_string());
    };
    let item = inner.item.blocking_lock();
    let path = item.output_path.clone().unwrap_or_else(|| item.dir.clone());
    drop(item);
    crate::history::reveal_path(path)
}

// ---------------------------------------------------------------------------
// Scheduler helpers
// ---------------------------------------------------------------------------

/// Parse "HH:MM" to minutes since midnight
fn parse_time(s: &str) -> Result<u32, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("invalid time format".to_string());
    }
    let h: u32 = parts[0].parse().map_err(|_| "invalid hour")?;
    let m: u32 = parts[1].parse().map_err(|_| "invalid minute")?;
    if h > 23 || m > 59 {
        return Err("time out of range".to_string());
    }
    Ok(h * 60 + m)
}

/// Current time in minutes since midnight (local time)
fn now_minutes() -> u32 {
    let now = chrono::Local::now();
    now.hour() * 60 + now.minute()
}

/// Wait until the schedule window opens.
/// If current time is within [start, end), returns immediately.
/// If current time is before start, sleeps until start.
/// If current time is after end, sleeps until next day's start.
async fn wait_for_schedule(start_min: u32, end_min: u32) {
    loop {
        let now = now_minutes();
        if start_min <= end_min {
            // Same day window (e.g., 02:00 - 06:00)
            if now >= start_min && now < end_min {
                return; // Within window
            }
            if now < start_min {
                let wait = (start_min - now) as u64 * 60;
                sleep(Duration::from_secs(wait)).await;
                return;
            }
            // now >= end_min, wait until next day start
            let wait = (24 * 60 - now + start_min) as u64 * 60;
            sleep(Duration::from_secs(wait)).await;
        } else {
            // Overnight window (e.g., 22:00 - 04:00)
            if now >= start_min || now < end_min {
                return; // Within window
            }
            // now is between end_min and start_min (daytime)
            let wait = (start_min - now) as u64 * 60;
            sleep(Duration::from_secs(wait)).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Args builder
// ---------------------------------------------------------------------------

fn build_args(item: &Item, setts: &settings::Settings, ffmpeg_dir: &PathBuf) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "--newline".into(),
        "--no-playlist".into(),
        "--continue".into(),
        "--no-overwrites".into(),
        "--retries".into(),
        "3".into(),
        "--fragment-retries".into(),
        "3".into(),
        "--socket-timeout".into(),
        "30".into(),
        "--ffmpeg-location".into(),
        ffmpeg_dir.to_string_lossy().to_string(),
    ];

    // Proxy support
    if !setts.proxy.trim().is_empty() {
        a.push("--proxy".into());
        a.push(setts.proxy.trim().to_string());
    }

    if item.audio_only {
        a.push("-x".into());
        a.push("--audio-format".into());
        a.push("mp3".into());
        a.push("--audio-quality".into());
        a.push("0".into());
        if setts.embed_thumbnail {
            a.push("--embed-thumbnail".into());
        }
        if setts.embed_metadata {
            a.push("--add-metadata".into());
        }
    } else {
        let filter = if item.quality == "best" {
            "bv*+ba/b".to_string()
        } else if let Ok(h) = item.quality.parse::<u32>() {
            format!("bv*[height<={h}]+ba/b[height<={h}]")
        } else {
            "bv*+ba/b".to_string()
        };
        a.push("-f".into());
        a.push(filter);
        a.push("--merge-output-format".into());
        a.push("mp4".into());
        if setts.write_subs {
            a.push("--write-subs".into());
            a.push("--write-auto-subs".into());
            a.push("--sub-langs".into());
            a.push(setts.sub_langs.clone());
            a.push("--convert-subs".into());
            a.push("srt".into());
        }
    }

    if !setts.rate_limit.trim().is_empty() {
        a.push("--limit-rate".into());
        a.push(setts.rate_limit.trim().to_string());
    }

    // "<dir>/<prefix>%(title)s.%(ext)s" — prefix already contains "NNN - "
    let sep = std::path::MAIN_SEPARATOR;
    a.push("-o".into());
    a.push(format!(
        "{dir}{sep}{prefix}%(title)s.%(ext)s",
        dir = item.dir,
        prefix = item.prefix
    ));

    a.push(item.url.clone());
    a
}
