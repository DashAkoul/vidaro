//! Scheduling helpers: time-window maths and the global scheduler loop.

use std::sync::Arc;
use std::time::Duration;

use chrono::Timelike;
use tauri::{AppHandle, Manager as _};

use crate::core::config::{parse_hhmm, ConfigStore};
use crate::core::plugin::AppContext;
use crate::features::download::control;
use crate::features::download::model::{ItemStatus, PausedBy};
use crate::features::download::queue::{ItemInner, Queue};

pub fn now_minutes() -> u32 {
    let now = chrono::Local::now();
    now.hour() * 60 + now.minute()
}

/// Is `now` inside `[start, end)`? Handles windows that cross midnight.
pub fn in_window(now: u32, start: u32, end: u32) -> bool {
    if start == end {
        // Degenerate window: treat as "always open" rather than "never".
        return true;
    }
    if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

/// Block until the window opens, giving up if the item is paused or cancelled.
async fn wait_for_window(inner: &ItemInner, start: u32, end: u32) -> bool {
    loop {
        if inner.is_cancelled() || inner.is_paused() {
            return false;
        }
        if in_window(now_minutes(), start, end) {
            return true;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

pub async fn await_item_window(inner: &ItemInner, start: Option<&str>, end: Option<&str>) -> bool {
    let (Some(start), Some(end)) = (start, end) else {
        return true;
    };
    let (Some(start), Some(end)) = (parse_hhmm(start), parse_hhmm(end)) else {
        return true;
    };
    wait_for_window(inner, start, end).await
}

/// Background task: pause/resume unscheduled downloads to honour the global
/// window. Runs every 20 seconds and only touches items it paused itself.
pub fn spawn_global(app: AppHandle, queue: Arc<Queue>) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(20)).await;
            tick(&app, &queue).await;
        }
    });
}

async fn tick(app: &AppHandle, queue: &Arc<Queue>) {
    let Some(config) = app.try_state::<ConfigStore>() else {
        return;
    };
    let settings = config.get();
    if !settings.scheduler_enabled {
        return;
    }

    let (Some(start), Some(end)) = (
        parse_hhmm(&settings.scheduler_start),
        parse_hhmm(&settings.scheduler_end),
    ) else {
        return;
    };

    let inside = in_window(now_minutes(), start, end);

    // Items with their own schedule manage themselves.
    let candidates: Vec<Arc<ItemInner>> = queue
        .all()
        .into_iter()
        .filter(|inner| {
            inner.with_item(|item| item.schedule_start.is_none() && item.schedule_end.is_none())
        })
        .collect();

    if inside {
        for inner in candidates {
            let should_resume = inner.with_item(|item| {
                item.status == ItemStatus::Paused && item.paused_by == Some(PausedBy::Scheduler)
            });
            if should_resume {
                control::resume(app, queue.clone(), inner);
            }
        }
    } else {
        for inner in candidates {
            let status = inner.status();
            if matches!(status, ItemStatus::Active | ItemStatus::Queued) {
                control::pause(app, queue, &inner, PausedBy::Scheduler).await;
            }
        }
    }
}

pub struct SchedulerPlugin;

impl crate::core::plugin::Plugin for SchedulerPlugin {
    fn name(&self) -> &'static str {
        "scheduler"
    }

    fn startup(&self, ctx: &AppContext) -> crate::core::error::AppResult<()> {
        let queue = ctx
            .app
            .state::<crate::features::download::queue::Manager>()
            .queue
            .clone();
        spawn_global(ctx.app.clone(), queue);
        Ok(())
    }
}
