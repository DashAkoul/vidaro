//! Event names shared between backend and frontend.
//!
//! Keeping them in one place makes it impossible for the two sides to drift.

pub const ITEM_UPDATED: &str = "item-updated";
pub const QUEUE_CHANGED: &str = "queue-changed";
pub const HISTORY_CHANGED: &str = "history-changed";
pub const INFO_PROGRESS: &str = "info-progress";
pub const TOOLS_PROGRESS: &str = "tools-progress";
pub const CLIPBOARD_LINK: &str = "clipboard-link";
pub const SETTINGS_CHANGED: &str = "settings-changed";

pub fn emit<T: serde::Serialize + Clone>(app: &tauri::AppHandle, event: &str, payload: T) {
    use tauri::Emitter;
    let _ = app.emit(event, payload);
}

pub fn emit_empty(app: &tauri::AppHandle, event: &str) {
    use tauri::Emitter;
    let _ = app.emit(event, ());
}
