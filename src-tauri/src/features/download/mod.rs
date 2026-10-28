//! The download queue feature.

pub mod args;
pub mod commands;
pub mod control;
pub mod model;
pub mod parser;
pub mod queue;
pub mod runner;
pub mod scheduler;

pub use queue::Manager;

use std::path::PathBuf;

use tauri::Manager as _;

use crate::core::config::SINGLE_VIDEOS_DIR;
use crate::core::error::{AppError, AppResult};
use crate::core::events;
use crate::core::filename::{is_plausible_dir, sanitize_name};
use crate::core::plugin::{AppContext, Plugin};
use crate::features::download::model::{Item, ItemStatus, Stage};
use crate::features::download::queue::ItemInner;

/// Load the persisted queue into memory.
pub struct DownloadPlugin;

impl Plugin for DownloadPlugin {
    fn name(&self) -> &'static str {
        "download"
    }

    fn startup(&self, ctx: &AppContext) -> AppResult<()> {
        let manager = ctx.app.state::<Manager>();
        queue::load_from(&ctx.paths.queue_file, &manager.queue);
        Ok(())
    }
}

/// Add one video to the queue. Used by the browser-extension bridge.
pub fn enqueue_single(
    app: &tauri::AppHandle,
    url: &str,
    title: &str,
    quality: &str,
    audio_only: bool,
) -> AppResult<Item> {
    let manager = app
        .try_state::<Manager>()
        .ok_or_else(|| AppError::Tool("queue not initialised".into()))?;
    let settings = app
        .try_state::<crate::core::config::ConfigStore>()
        .map(|c| c.get())
        .unwrap_or_default();

    let root = if settings.default_dir.trim().is_empty() {
        PathBuf::from(crate::core::config::default_download_root(app))
    } else {
        PathBuf::from(&settings.default_dir)
    };
    let root_str = root.to_string_lossy().to_string();
    if !is_plausible_dir(&root_str) {
        return Err(AppError::Invalid("no valid download folder".into()));
    }

    let destination = root.join(sanitize_name(SINGLE_VIDEOS_DIR));
    std::fs::create_dir_all(&destination)?;
    let destination_str = destination.to_string_lossy().to_string();

    let video_id = extract_video_id(url);
    let duplicate = manager.queue.all().iter().any(|inner| {
        inner.with_item(|item| {
            item.video_id == video_id
                && item.dir == destination_str
                && !matches!(item.status, ItemStatus::Error | ItemStatus::Cancelled)
        })
    });
    if duplicate {
        return Err(AppError::Invalid("already in the queue".into()));
    }

    let item = Item {
        id: uuid::Uuid::new_v4().to_string(),
        url: url.to_string(),
        video_id,
        title: if title.trim().is_empty() {
            "Untitled".to_string()
        } else {
            title.to_string()
        },
        index: None,
        dir: destination_str,
        prefix: String::new(),
        quality: quality.to_string(),
        audio_only,
        batch_name: String::new(),
        status: ItemStatus::Queued,
        percent: 0.0,
        speed_bps: 0.0,
        eta_secs: 0.0,
        total_bytes: 0,
        downloaded_bytes: 0,
        stage: Stage::Download,
        error: None,
        output_path: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        schedule_start: None,
        schedule_end: None,
        paused_by: None,
    };

    let inner = ItemInner::new(item.clone());
    manager.queue.insert(inner.clone());
    control::spawn_run(app, manager.queue.clone(), inner);

    queue::persist(app, &manager.queue);
    events::emit_empty(app, events::QUEUE_CHANGED);
    Ok(item)
}

/// Best-effort extraction of the YouTube video id from any URL shape.
fn extract_video_id(url: &str) -> String {
    if let Some(rest) = url.split("v=").nth(1) {
        return rest.split(['&', '#']).next().unwrap_or(rest).to_string();
    }
    if let Some(rest) = url.split("youtu.be/").nth(1) {
        return rest.split(['?', '&', '#']).next().unwrap_or(rest).to_string();
    }
    url.to_string()
}
