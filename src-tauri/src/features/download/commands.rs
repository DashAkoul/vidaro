//! Tauri commands for the download queue.

use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::core::config::{ConfigStore, SINGLE_VIDEOS_DIR};
use crate::core::error::{AppError, AppResult};
use crate::core::events;
use crate::core::filename::{is_plausible_dir, safe_join, sanitize_name};
use crate::features::download::control;
use crate::features::download::model::{
    EnqueuePayload, Item, ItemStatus, PausedBy, Stage,
};
use crate::features::download::queue::{self, ItemInner, Manager};

#[tauri::command]
pub async fn enqueue_batch(
    app: AppHandle,
    mgr: State<'_, Manager>,
    config: State<'_, ConfigStore>,
    payload: EnqueuePayload,
) -> AppResult<Vec<Item>> {
    let settings = config.get();
    let queue = mgr.queue.clone();
    queue.gate.set_max(settings.max_concurrent as usize);

    if !is_plausible_dir(&payload.dir) {
        return Err(AppError::Invalid(
            "destination folder must be an absolute path".into(),
        ));
    }
    if payload.items.is_empty() {
        return Err(AppError::Invalid("no videos selected".into()));
    }

    let root = PathBuf::from(&payload.dir);
    let batch = if payload.numbering {
        sanitize_name(&payload.batch_name)
    } else {
        String::new()
    };

    // Playlists/channels get their own folder; single videos share one.
    let destination = if payload.numbering && !batch.is_empty() {
        safe_join(&root, &batch)
            .ok_or_else(|| AppError::Invalid("invalid batch folder name".into()))?
    } else {
        root.join(SINGLE_VIDEOS_DIR)
    };
    std::fs::create_dir_all(&destination)?;
    let destination_str = destination.to_string_lossy().to_string();

    // Snapshot once instead of re-locking for every item.
    let existing: Vec<(String, String, ItemStatus)> = queue
        .all()
        .iter()
        .map(|inner| inner.with_item(|item| (item.video_id.clone(), item.dir.clone(), item.status)))
        .collect();

    let mut created: Vec<Item> = Vec::new();

    for entry in payload.items {
        let duplicate = existing.iter().any(|(video_id, dir, status)| {
            video_id == &entry.video_id
                && dir == &destination_str
                && !matches!(status, ItemStatus::Error | ItemStatus::Cancelled)
        });
        if duplicate {
            continue;
        }

        let prefix = match (payload.numbering, entry.index) {
            (true, Some(index)) => format!("{index:03} - "),
            _ => String::new(),
        };

        let scheduled = payload.schedule_start.is_some() && payload.schedule_end.is_some();

        let item = Item {
            id: uuid::Uuid::new_v4().to_string(),
            url: entry.url,
            video_id: entry.video_id,
            title: entry.title,
            index: if payload.numbering { entry.index } else { None },
            dir: destination_str.clone(),
            prefix,
            quality: payload.quality.clone(),
            audio_only: payload.audio_only,
            batch_name: batch.clone(),
            status: if scheduled {
                ItemStatus::Scheduled
            } else {
                ItemStatus::Queued
            },
            percent: 0.0,
            speed_bps: 0.0,
            eta_secs: 0.0,
            total_bytes: 0,
            downloaded_bytes: 0,
            stage: Stage::Download,
            error: None,
            output_path: None,
            created_at: chrono::Utc::now().timestamp_millis(),
            schedule_start: payload.schedule_start.clone(),
            schedule_end: payload.schedule_end.clone(),
            paused_by: None,
        };

        let inner = ItemInner::new(item.clone());
        queue.insert(inner.clone());
        created.push(item);

        control::spawn_run(&app, queue.clone(), inner);
    }

    queue::persist(&app, &queue);
    events::emit_empty(&app, events::QUEUE_CHANGED);
    Ok(created)
}

#[tauri::command]
pub fn get_queue(mgr: State<'_, Manager>) -> Vec<Item> {
    mgr.queue.snapshot()
}

#[tauri::command]
pub async fn pause_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> AppResult<()> {
    let inner = mgr.queue.get(&id).ok_or_else(|| AppError::NotFound(id.clone()))?;
    control::pause(&app, &mgr.queue, &inner, PausedBy::User).await;
    Ok(())
}

#[tauri::command]
pub async fn resume_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> AppResult<()> {
    let inner = mgr.queue.get(&id).ok_or_else(|| AppError::NotFound(id.clone()))?;
    if control::resume(&app, mgr.queue.clone(), inner) {
        Ok(())
    } else {
        Err(AppError::Invalid("this download was cancelled".into()))
    }
}

#[tauri::command]
pub async fn cancel_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> AppResult<()> {
    let inner = mgr.queue.get(&id).ok_or_else(|| AppError::NotFound(id.clone()))?;
    control::cancel(&app, &mgr.queue, &inner).await;
    Ok(())
}

#[tauri::command]
pub async fn retry_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> AppResult<()> {
    let inner = mgr.queue.get(&id).ok_or_else(|| AppError::NotFound(id.clone()))?;
    control::retry(&app, mgr.queue.clone(), inner).await;
    Ok(())
}

#[tauri::command]
pub async fn remove_item(app: AppHandle, mgr: State<'_, Manager>, id: String) -> AppResult<()> {
    if control::remove(&app, &mgr.queue, &id).await {
        Ok(())
    } else {
        Err(AppError::NotFound(id))
    }
}

#[tauri::command]
pub async fn pause_all(app: AppHandle, mgr: State<'_, Manager>) -> AppResult<()> {
    let queue = mgr.queue.clone();
    for inner in queue.all() {
        let status = inner.status();
        if matches!(status, ItemStatus::Active | ItemStatus::Queued | ItemStatus::Scheduled) {
            control::pause(&app, &queue, &inner, PausedBy::User).await;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn resume_all(app: AppHandle, mgr: State<'_, Manager>) -> AppResult<()> {
    let queue = mgr.queue.clone();
    for inner in queue.all() {
        if inner.status() == ItemStatus::Paused {
            control::resume(&app, queue.clone(), inner);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn clear_finished(app: AppHandle, mgr: State<'_, Manager>) -> AppResult<()> {
    let queue = mgr.queue.clone();
    let finished: Vec<String> = queue
        .all()
        .iter()
        .filter(|inner| inner.status().is_terminal())
        .map(|inner| inner.with_item(|item| item.id.clone()))
        .collect();

    for id in finished {
        queue.remove(&id);
    }

    queue::persist(&app, &queue);
    events::emit_empty(&app, events::QUEUE_CHANGED);
    Ok(())
}

#[tauri::command]
pub fn open_download_folder(mgr: State<'_, Manager>, id: String) -> AppResult<()> {
    let inner = mgr.queue.get(&id).ok_or_else(|| AppError::NotFound(id.clone()))?;
    let path = inner.with_item(|item| {
        item.output_path
            .clone()
            .unwrap_or_else(|| item.dir.clone())
    });
    crate::features::history::commands::reveal_path(path)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleUpdate {
    pub id: String,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[tauri::command]
pub fn update_item_schedule(
    app: AppHandle,
    mgr: State<'_, Manager>,
    payload: ScheduleUpdate,
) -> AppResult<()> {
    let inner = mgr
        .queue
        .get(&payload.id)
        .ok_or_else(|| AppError::NotFound(payload.id.clone()))?;

    let has_schedule = payload.start.is_some() && payload.end.is_some();

    let snapshot = inner.with_item(|item| {
        item.schedule_start = if has_schedule { payload.start.clone() } else { None };
        item.schedule_end = if has_schedule { payload.end.clone() } else { None };

        if has_schedule && item.status == ItemStatus::Queued {
            item.status = ItemStatus::Scheduled;
        } else if !has_schedule && item.status == ItemStatus::Scheduled {
            item.status = ItemStatus::Queued;
        }
        item.clone()
    });

    events::emit(&app, events::ITEM_UPDATED, &snapshot);
    queue::persist(&app, &mgr.queue);
    Ok(())
}
