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
    let snapshot = inner.with_item(|item| {
        item.status = status;
        item.paused_by = paused_by;
        if status == ItemStatus::Queued {
            item.reset_progress();
        }
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
    queue.remove(id);
    persist(app, queue);
    events::emit_empty(app, events::QUEUE_CHANGED);
    true
}
