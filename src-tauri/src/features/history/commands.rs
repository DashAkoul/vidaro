//! Tauri commands for the history plugin.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::core::error::{AppError, AppResult};
use crate::features::history::{self, HistoryEntry};

#[tauri::command]
pub fn get_history(app: AppHandle) -> Vec<HistoryEntry> {
    history::load(&app)
}

#[tauri::command]
pub fn clear_history(app: AppHandle) -> AppResult<()> {
    history::save(&app, &[]).map_err(|e| AppError::Io(e))
}

/// Reveal a file in Explorer / Finder, or open its folder.
#[tauri::command]
pub fn reveal_path(path: String) -> AppResult<()> {
    let target = PathBuf::from(&path);
    if target.is_file() {
        tauri_plugin_opener::reveal_item_in_dir(&target)
            .map_err(|e| AppError::Tool(e.to_string()))
    } else if target.is_dir() {
        tauri_plugin_opener::open_path(&target, None::<&str>)
            .map_err(|e| AppError::Tool(e.to_string()))
    } else {
        // The file may have been moved or renamed; fall back to its parent.
        match target.parent() {
            Some(parent) if parent.exists() => {
                tauri_plugin_opener::open_path(parent, None::<&str>)
                    .map_err(|e| AppError::Tool(e.to_string()))
            }
            _ => Err(AppError::NotFound(path)),
        }
    }
}
