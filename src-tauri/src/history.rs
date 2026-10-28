use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub title: String,
    pub path: String,      // final file path (file or folder)
    pub size_bytes: u64,   // 0 if unknown
    pub finished_at: i64,  // unix millis
    pub kind: String,      // "video" | "audio"
    pub quality: String,
    pub batch_name: String,
}

pub fn history_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("history.json")
}

pub fn load_all(app: &AppHandle) -> Vec<HistoryEntry> {
    fs::read_to_string(history_path(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_all(app: &AppHandle, entries: &[HistoryEntry]) -> Result<(), String> {
    let path = history_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn add_entry(app: &AppHandle, entry: HistoryEntry) {
    let mut all = load_all(app);
    all.insert(0, entry);
    // keep the history bounded
    all.truncate(500);
    let _ = save_all(app, &all);
}

#[tauri::command]
pub fn get_history(app: AppHandle) -> Vec<HistoryEntry> {
    load_all(&app)
}

#[tauri::command]
pub fn clear_history(app: AppHandle) -> Result<(), String> {
    save_all(&app, &[])
}

/// Reveal a file in Explorer/Finder, or open the folder itself.
#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    if p.is_file() {
        tauri_plugin_opener::reveal_item_in_dir(&p).map_err(|e| e.to_string())
    } else if p.is_dir() {
        tauri_plugin_opener::open_path(&p, None::<&str>).map_err(|e| e.to_string())
    } else {
        // file may not exist yet (e.g. renamed) — open its parent folder
        if let Some(parent) = p.parent() {
            tauri_plugin_opener::open_path(parent, None::<&str>).map_err(|e| e.to_string())
        } else {
            Err("path not found".to_string())
        }
    }
}
