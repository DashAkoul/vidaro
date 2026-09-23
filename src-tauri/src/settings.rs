use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub language: String,         // "fa" | "en"
    pub theme: String,            // "system" | "light" | "dark"
    pub default_dir: String,      // default download folder ("" = not chosen yet)
    pub max_concurrent: u32,      // parallel downloads
    pub rate_limit: String,       // "" = off, e.g. "5M"
    pub write_subs: bool,         // download subtitles when available
    pub sub_langs: String,        // e.g. "en"
    pub embed_metadata: bool,     // embed metadata into audio files
    pub embed_thumbnail: bool,    // embed thumbnail into audio files
    // Proxy
    pub proxy: String,            // "" = off, e.g. "http://host:port" or "socks5://host:port"
    // Scheduler (global)
    pub scheduler_enabled: bool,  // enable global schedule
    pub scheduler_start: String,  // "HH:MM" 24h format
    pub scheduler_end: String,    // "HH:MM" 24h format
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme: "system".to_string(),
            default_dir: String::new(),
            max_concurrent: 2,
            rate_limit: String::new(),
            write_subs: false,
            sub_langs: "en".to_string(),
            embed_metadata: true,
            embed_thumbnail: true,
            proxy: String::new(),
            scheduler_enabled: false,
            scheduler_start: "02:00".to_string(),
            scheduler_end: "06:00".to_string(),
        }
    }
}

pub fn settings_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("settings.json")
}

pub fn load(app: &AppHandle) -> Settings {
    let path = settings_path(app);
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// Folder name used for single (non-batch) downloads inside the root dir.
pub const SINGLE_VIDEOS_DIR: &str = "Single Videos";

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    let mut s = load(&app);
    if s.default_dir.trim().is_empty() {
        // <Downloads>/<AppFolder>
        if let Ok(dl) = app.path().download_dir() {
            s.default_dir = dl.join("YouTube Downloader").to_string_lossy().to_string();
        }
    }
    s
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    save(&app, &settings)?;
    Ok(())
}

/// Export all data (settings, queue, history) to a JSON file
#[tauri::command]
pub fn export_data(app: AppHandle, path: String) -> Result<(), String> {
    use crate::download::Manager;
    use crate::history;
    
    let settings = load(&app);
    let mgr = app.state::<Manager>();
    let queue = {
        let map = mgr.inner.items.lock().unwrap();
        let mut v = Vec::new();
        for k in map.values() {
            if let Ok(item) = k.item.try_lock() {
                v.push(item.clone());
            }
        }
        v
    };
    let history = history::load_all(&app);
    
    #[derive(Serialize)]
    struct ExportData {
        version: u32,
        exported_at: String,
        settings: Settings,
        queue: Vec<crate::download::Item>,
        history: Vec<history::HistoryEntry>,
    }
    
    let data = ExportData {
        version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        settings,
        queue,
        history,
    };
    
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// Import data from a JSON file
#[tauri::command]
pub fn import_data(app: AppHandle, path: String) -> Result<(), String> {
    use crate::download::Manager;
    use crate::history;
    
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    
    #[derive(Deserialize)]
    struct ImportData {
        version: u32,
        settings: Settings,
        queue: Vec<crate::download::Item>,
        history: Vec<history::HistoryEntry>,
    }
    
    let data: ImportData = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    
    if data.version > 1 {
        return Err("Unsupported export version".to_string());
    }
    
    // Import settings
    save(&app, &data.settings)?;
    
    // Import queue - merge with existing, avoiding duplicates
    let mgr = app.state::<Manager>();
    for mut item in data.queue {
        // Reset runtime fields
        item.status = "queued".to_string();
        item.percent = 0.0;
        item.speed_bps = 0.0;
        item.eta_secs = 0.0;
        item.downloaded_bytes = 0;
        item.stage = "download".to_string();
        item.error = None;
        item.output_path = None;
        item.created_at = chrono::Utc::now().timestamp_millis();
        
        // Check for duplicates
        let dest_str = item.dir.clone();
        let is_dup = {
            let map = mgr.inner.items.lock().unwrap();
            map.values().any(|inner| {
                if let Ok(existing) = inner.item.try_lock() {
                    existing.video_id == item.video_id && existing.dir == dest_str
                        && !matches!(existing.status.as_str(), "error" | "cancelled")
                } else { false }
            })
        };
        if !is_dup {
            let inner = crate::download::ItemInner::new(item.clone());
            mgr.inner.items.lock().unwrap().insert(item.id.clone(), inner.clone());
            let app2 = app.clone();
            let gate = crate::download::GateProxy(mgr.inner.clone());
            tauri::async_runtime::spawn(crate::download::run_item(app2, inner, gate));
        }
    }
    crate::download::persist_queue(&app, &mgr.inner);
    crate::download::emit_queue_changed(&app);
    
    // Import history - merge, avoiding duplicates by id
    let mut existing_history = history::load_all(&app);
    for entry in data.history {
        if !existing_history.iter().any(|e| e.id == entry.id) {
            existing_history.insert(0, entry);
        }
    }
    existing_history.truncate(500);
    history::save_all(&app, &existing_history)?;
    
    Ok(())
}
