//! Executing a single queue item with yt-dlp.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager as _};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

use crate::core::config::ConfigStore;
use crate::core::events;
use crate::features::download::model::{Item, ItemStatus, Stage};
use crate::features::download::queue::{ItemInner, Queue};
use crate::features::download::{args, control, parser, scheduler};
use crate::features::history;
use crate::features::tools::ToolManager;

/// Shared across the two stream readers.
struct ParseState {
    errored: Option<String>,
    output_path: Option<String>,
    last_emit: Instant,
}

const EMIT_INTERVAL: Duration = Duration::from_millis(200);

pub async fn run(app: AppHandle, queue: Arc<Queue>, inner: Arc<ItemInner>) {
    // A second resume (double click, or the scheduler racing the user) must not
    // start a parallel yt-dlp process for the same item. A cancel immediately
    // followed by a retry can overlap by a few milliseconds, so give the
    // previous task a moment to release the item instead of dropping the run.
    let mut claim = None;
    for _ in 0..20 {
        if let Some(guard) = inner.begin_run() {
            claim = Some(guard);
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let Some(_run_guard) = claim else {
        return;
    };

    // 1. Honour a per-item schedule before competing for a slot.
    let (schedule_start, schedule_end) =
        inner.with_item(|item| (item.schedule_start.clone(), item.schedule_end.clone()));

    if schedule_start.is_some() {
        if inner.status() == ItemStatus::Queued {
            control::set_status(&app, &queue, &inner, ItemStatus::Scheduled, None);
        }
        if !scheduler::await_item_window(&inner, schedule_start.as_deref(), schedule_end.as_deref())
            .await
        {
            return;
        }
    }

    // 2. Wait for a concurrency slot. The permit is released on drop, so every
    //    early return below is automatically safe.
    let _permit = queue.gate.acquire().await;

    if inner.is_cancelled() || inner.is_paused() {
        return;
    }

    control::set_status(&app, &queue, &inner, ItemStatus::Active, None);

    // 3. Resolve tools and build the command.
    let settings = app
        .try_state::<ConfigStore>()
        .map(|c| c.get())
        .unwrap_or_default();

    let Some(tools) = app.try_state::<ToolManager>() else {
        fail(&app, &queue, &inner, "internal error: tools not initialised");
        return;
    };
    let tool_paths = tools.paths();

    if !tool_paths.ytdlp.is_file() {
        fail(
            &app,
            &queue,
            &inner,
            "yt-dlp is not installed. Open Settings and install the download tools.",
        );
        return;
    }

    let item = inner.snapshot();
    let cmd_args = args::build(&item, &settings, &tool_paths);

    let mut command = crate::core::process::async_command(&tool_paths.ytdlp);
    command
        .args(&cmd_args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    // yt-dlp passes its environment on to ffmpeg, so this covers both.
    if let Some(proxy) = settings.proxy_url() {
        for key in ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY"] {
            command.env(key, &proxy);
        }
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            fail(&app, &queue, &inner, &format!("could not start yt-dlp: {e}"));
            return;
        }
    };

    // 4. Stream both output channels.
    let state = Arc::new(Mutex::new(ParseState {
        errored: None,
        output_path: None,
        last_emit: Instant::now() - EMIT_INTERVAL,
    }));

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    inner.set_child(Some(child)).await;

    let mut readers = Vec::new();
    for stream in [stdout.map(Stream::Out), stderr.map(Stream::Err)]
        .into_iter()
        .flatten()
    {
        let app = app.clone();
        let inner = inner.clone();
        let state = state.clone();
        readers.push(tauri::async_runtime::spawn(async move {
            match stream {
                Stream::Out(s) => pump(s, &app, &inner, &state).await,
                Stream::Err(s) => pump(s, &app, &inner, &state).await,
            }
        }));
    }
    for handle in readers {
        let _ = handle.await;
    }

    let success = inner.wait_child().await;

    let (paused, cancelled) = {
        let control = inner.control.lock().unwrap_or_else(|p| p.into_inner());
        (control.paused, control.cancelled)
    };

    // The controller already wrote the final status; nothing left to do.
    if paused || cancelled {
        return;
    }

    if success {
        finish_ok(&app, &queue, &inner, &state).await;
    } else {
        let message = {
            let st = state.lock().await;
            st.errored.clone()
        }
        .unwrap_or_else(|| "yt-dlp exited with an error".to_string());
        fail(&app, &queue, &inner, &message);
    }
}

enum Stream {
    Out(tokio::process::ChildStdout),
    Err(tokio::process::ChildStderr),
}

async fn pump<R>(
    reader: R,
    app: &AppHandle,
    inner: &Arc<ItemInner>,
    state: &Arc<Mutex<ParseState>>,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        handle_line(app, inner, &line, state).await;
    }
}

async fn handle_line(
    app: &AppHandle,
    inner: &Arc<ItemInner>,
    raw: &str,
    state: &Arc<Mutex<ParseState>>,
) {
    let Some(event) = parser::classify(raw) else {
        return;
    };

    let mut changed = false;
    {
        let mut st = state.lock().await;
        match event {
            parser::LineEvent::Error(message) => {
                st.errored = Some(message);
                return;
            }
            parser::LineEvent::OutputPath { path, stage } => {
                st.output_path = Some(path);
                if let Some(stage) = stage {
                    inner.with_item(|item| item.stage = stage);
                    changed = true;
                }
            }
            parser::LineEvent::Stage(stage) => {
                inner.with_item(|item| item.stage = stage);
                changed = true;
            }
            parser::LineEvent::Progress(progress) => {
                inner.with_item(|item| {
                    item.percent = progress.percent.clamp(0.0, 100.0);
                    if let Some(speed) = progress.speed_bps {
                        item.speed_bps = speed;
                    }
                    if let Some(eta) = progress.eta_secs {
                        item.eta_secs = eta;
                    }
                    if let Some(total) = progress.total_bytes {
                        item.total_bytes = total;
                        item.downloaded_bytes =
                            (total as f64 * item.percent / 100.0).round() as u64;
                    }
                });
                changed = true;
            }
        }
    }

    if changed && take_emit_slot(state).await {
        events::emit(app, events::ITEM_UPDATED, inner.snapshot());
    }
}

/// Rate-limits progress events so the UI is not flooded.
async fn take_emit_slot(state: &Arc<Mutex<ParseState>>) -> bool {
    let mut st = state.lock().await;
    if st.last_emit.elapsed() >= EMIT_INTERVAL {
        st.last_emit = Instant::now();
        true
    } else {
        false
    }
}

async fn finish_ok(
    app: &AppHandle,
    queue: &Arc<Queue>,
    inner: &Arc<ItemInner>,
    state: &Arc<Mutex<ParseState>>,
) {
    // The user may have removed the item while it was finishing; emitting now
    // would make the UI add it back.
    if !still_queued(queue, inner) {
        return;
    }

    let output_path = {
        let st = state.lock().await;
        st.output_path.clone()
    };

    let snapshot = inner.with_item(|item| {
        item.status = ItemStatus::Completed;
        item.percent = 100.0;
        item.stage = Stage::Done;
        item.speed_bps = 0.0;
        item.eta_secs = 0.0;
        item.paused_by = None;
        if let Some(path) = output_path.clone() {
            item.output_path = Some(path);
        }
        item.clone()
    });

    events::emit(app, events::ITEM_UPDATED, &snapshot);
    control::persist(app, queue);
    record_history(app, &snapshot);
    events::emit_empty(app, events::QUEUE_CHANGED);
    events::emit_empty(app, events::HISTORY_CHANGED);
}

fn fail(app: &AppHandle, queue: &Arc<Queue>, inner: &Arc<ItemInner>, message: &str) {
    if !still_queued(queue, inner) {
        return;
    }

    let snapshot = inner.with_item(|item| {
        item.status = ItemStatus::Error;
        item.error = Some(message.to_string());
        item.speed_bps = 0.0;
        item.eta_secs = 0.0;
        item.clone()
    });
    events::emit(app, events::ITEM_UPDATED, &snapshot);
    control::persist(app, queue);
}

/// Is the item still part of the queue (i.e. not removed by the user)?
fn still_queued(queue: &Queue, inner: &ItemInner) -> bool {
    queue
        .get(&inner.with_item(|item| item.id.clone()))
        .is_some()
}

fn record_history(app: &AppHandle, item: &Item) {
    let path = item
        .output_path
        .clone()
        .unwrap_or_else(|| item.dir.clone());

    let size = item
        .output_path
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .unwrap_or(0);

    history::add(
        app,
        history::HistoryEntry {
            id: item.id.clone(),
            title: item.title.clone(),
            path,
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
}
