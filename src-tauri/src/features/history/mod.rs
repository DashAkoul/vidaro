//! Download history.

pub mod commands;

use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager as _};

use crate::core::paths::AppPaths;

const MAX_ENTRIES: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub title: String,
    /// Final file path, or the containing folder when unknown.
    pub path: String,
    pub size_bytes: u64,
    pub finished_at: i64,
    pub kind: String, // "video" | "audio"
    pub quality: String,
    pub batch_name: String,
}

pub fn load_from(path: &Path) -> Vec<HistoryEntry> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_to(path: &Path, entries: &[HistoryEntry]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(entries)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

pub fn load(app: &AppHandle) -> Vec<HistoryEntry> {
    match app.try_state::<AppPaths>() {
        Some(paths) => load_from(&paths.history_file),
        None => Vec::new(),
    }
}

pub fn save(app: &AppHandle, entries: &[HistoryEntry]) -> std::io::Result<()> {
    match app.try_state::<AppPaths>() {
        Some(paths) => save_to(&paths.history_file, entries),
        None => Ok(()),
    }
}

/// Prepend an entry, keeping the log bounded.
pub fn add(app: &AppHandle, entry: HistoryEntry) {
    let mut all = load(app);
    all.retain(|existing| existing.id != entry.id);
    all.insert(0, entry);
    all.truncate(MAX_ENTRIES);
    if let Err(e) = save(app, &all) {
        eprintln!("[history] could not save: {e}");
    }
}

/// Merge imported entries, skipping duplicates.
pub fn merge(app: &AppHandle, incoming: Vec<HistoryEntry>) {
    let mut all = load(app);
    for entry in incoming {
        if !all.iter().any(|e| e.id == entry.id) {
            all.insert(0, entry);
        }
    }
    all.truncate(MAX_ENTRIES);
    let _ = save(app, &all);
}
