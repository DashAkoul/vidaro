//! Queue control primitives shared by the UI commands and the scheduler.

use std::sync::Arc;

use tauri::AppHandle;

use crate::core::events;
use crate::features::download::model::{ItemStatus, PausedBy};
use crate::features::download::queue::{self, ItemInner, Queue};
use crate::features::download::runner;

pub fn persist(app: &AppHandle, queue: &Queue) {
    queue::persist(app, queue);
}

/// Update status, emit an event, and persist — in that order.
pub fn set_status(
    app: &AppHandle,
    queue: &Queue,
    inner: &ItemInner,
    status: ItemStatus,
    paused_by: Option<PausedBy>,
) {
    // A control command may finish after the user removed the item; emitting
    // then would make the UI show it again.
    let id = inner.with_item(|item| item.id.clone());
    if queue.get(&id).is_none() {
        return;
    }

    let snapshot = inner.with_item(|item| {
        item.status = status;
        item.paused_by = paused_by;
        // Progress is deliberately *not* reset when an item returns to the
        // queue: resuming continues from the `.part` file, so zeroing here
        // made a 40%-done download flash back to 0% until yt-dlp reported.
        if status != ItemStatus::Paused {
            item.speed_bps = 0.0;
            item.eta_secs = 0.0;
        }
        item.clone()
    });

    events::emit(app, events::ITEM_UPDATED, &snapshot);
    persist(app, queue);
}

/// Spawn a background task that drives one item.
pub fn spawn_run(app: &AppHandle, queue: Arc<Queue>, inner: Arc<ItemInner>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        runner::run(app, queue, inner).await;
    });
}

pub async fn pause(app: &AppHandle, queue: &Queue, inner: &Arc<ItemInner>, by: PausedBy) {
    {
        let mut control = inner
            .control
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        control.paused = true;
    }
    inner.kill_child().await;
    set_status(app, queue, inner, ItemStatus::Paused, Some(by));
}

/// Returns false when the item was cancelled and cannot be resumed.
pub fn resume(app: &AppHandle, queue: Arc<Queue>, inner: Arc<ItemInner>) -> bool {
    {
        let mut control = inner
            .control
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if control.cancelled {
            return false;
        }
        control.paused = false;
    }
    set_status(app, &queue, &inner, ItemStatus::Queued, None);
    spawn_run(app, queue, inner);
    true
}

pub async fn cancel(app: &AppHandle, queue: &Queue, inner: &Arc<ItemInner>) {
    {
        let mut control = inner
            .control
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        control.cancelled = true;
        control.paused = false;
    }
    inner.kill_child().await;
    // A cancelled download is abandoned, not parked: drop the `.part` files so
    // they cannot sit in the folder forever. Pausing keeps them for resume.
    inner.remove_partials();
    set_status(app, queue, inner, ItemStatus::Cancelled, None);
}

pub async fn retry(app: &AppHandle, queue: Arc<Queue>, inner: Arc<ItemInner>) {
    {
        let mut control = inner
            .control
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        control.cancelled = false;
        control.paused = false;
    }
    // A new attempt reports its own destinations.
    inner.clear_destinations();
    inner.with_item(|item| {
        item.output_path = None;
        item.error = None;
    });
    set_status(app, &queue, &inner, ItemStatus::Queued, None);
    spawn_run(app, queue, inner);
}

pub async fn remove(app: &AppHandle, queue: &Arc<Queue>, id: &str) -> bool {
    let Some(inner) = queue.get(id) else {
        return false;
    };
    {
        let mut control = inner
            .control
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        control.cancelled = true;
    }
    inner.kill_child().await;
    // Removing an unfinished item discards its partial download; leaving the
    // `.part` behind would orphan it with nothing in the queue pointing at it.
    inner.remove_partials();
    queue.remove(id);
    persist(app, queue);
    events::emit_empty(app, events::QUEUE_CHANGED);
    true
}
