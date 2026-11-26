//! Automatic retries for transient YouTube failures.
//!
//! The single biggest complaint about Vidaro was that it "works sometimes":
//! YouTube answers with a bot check or a degraded player response for a while,
//! then starts working again. Nothing in the app could survive that window —
//! every affected item sat in the error tab until the user pressed Retry
//! manually, often long after the block had lifted.
//!
//! An item that fails for a transient reason is therefore put back into the
//! queue with a `retry_at` timestamp and the attempt counter incremented. This
//! module wakes it up when the time comes. The state lives in `queue.json`, so
//! a retry still happens after the app is closed and reopened.

use std::sync::Arc;

use tauri::AppHandle;

use crate::core::config::Settings;
use crate::features::download::model::ItemStatus;
use crate::features::download::queue::{ItemInner, Queue};

/// How long to wait before the given attempt.
///
/// Exponential from `retry_delay_secs`, capped so a machine left running
/// overnight keeps trying a few times rather than once every eight hours.
pub fn delay_secs(settings: &Settings, attempts: u32) -> i64 {
    let base = i64::from(settings.retry_delay_secs.max(30));
    let step = u32::from(attempts.max(1)).saturating_sub(1).min(3);
    let delay = base.saturating_mul(1 << step);
    delay.min(6 * 60 * 60)
}

/// Should this failure be retried, and after how long?
///
/// Returns the attempt number *after* this failure and the delay in seconds,
/// or `None` when the item should stay failed. Keeping the decision pure is
/// what lets the policy be tested without an `AppHandle`.
pub fn plan(settings: &Settings, transient: bool, attempts: u32) -> Option<(u32, i64)> {
    if !transient || !settings.auto_retry || attempts >= settings.auto_retry_max {
        return None;
    }
    let next = attempts.saturating_add(1);
    Some((next, delay_secs(settings, next)))
}

/// "5 minutes", "1 hour 20 minutes" — used inside the item's error text.
pub fn humanize(seconds: i64) -> String {
    let minutes = (seconds + 59) / 60;
    if minutes < 60 {
        format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" })
    } else {
        let hours = minutes / 60;
        let rest = minutes % 60;
        if rest == 0 {
            format!("{hours} hour{}", if hours == 1 { "" } else { "s" })
        } else {
            format!("{hours} h {rest} min")
        }
    }
}

/// Retry pending for this item?
fn is_due(inner: &ItemInner, now: i64) -> bool {
    inner.with_item(|item| {
        item.status == ItemStatus::Queued && item.retry_at.map(|at| at <= now).unwrap_or(false)
    })
}

/// Start every queued item whose retry time has arrived.
pub fn tick(app: &AppHandle, queue: &Arc<Queue>) {
    let now = chrono::Utc::now().timestamp_millis();
    for inner in queue.all() {
        if !is_due(&inner, now) {
            continue;
        }
        // Clear first so two ticks cannot start the same item twice.
        inner.with_item(|item| item.retry_at = None);
        crate::features::download::control::spawn_run(app, queue.clone(), inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_grows_and_is_capped() {
        let mut settings = Settings::default();
        settings.retry_delay_secs = 300;
        assert_eq!(delay_secs(&settings, 1), 300);
        assert_eq!(delay_secs(&settings, 2), 600);
        assert_eq!(delay_secs(&settings, 3), 1200);
        assert_eq!(delay_secs(&settings, 4), 2400);
        // Beyond four attempts the step stops growing.
        assert_eq!(delay_secs(&settings, 9), 2400);
    }

    #[test]
    fn transient_failures_are_retried() {
        let settings = Settings::default();
        assert_eq!(plan(&settings, true, 0), Some((1, 300)));
        assert_eq!(plan(&settings, true, 1), Some((2, 600)));
    }

    #[test]
    fn permanent_failures_are_not_retried() {
        let settings = Settings::default();
        assert_eq!(plan(&settings, false, 0), None);
    }

    #[test]
    fn the_budget_is_respected() {
        let mut settings = Settings::default();
        settings.auto_retry_max = 2;
        assert_eq!(plan(&settings, true, 1), Some((2, 600)));
        assert_eq!(plan(&settings, true, 2), None);
    }

    #[test]
    fn auto_retry_can_be_switched_off() {
        let mut settings = Settings::default();
        settings.auto_retry = false;
        assert_eq!(plan(&settings, true, 0), None);
    }

    #[test]
    fn humanized_delays() {
        assert_eq!(humanize(60), "1 minute");
        assert_eq!(humanize(300), "5 minutes");
        assert_eq!(humanize(3600), "1 hour");
        assert_eq!(humanize(4800), "1 h 20 min");
    }
}
