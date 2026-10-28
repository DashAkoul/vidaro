use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

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
    pub proxy_mode: String,       // "off" | "system" | "manual"
    pub proxy_host: String,       // proxy host
    pub proxy_port: String,       // proxy port
    // Scheduler (global)
    pub scheduler_enabled: bool,  // enable global schedule
    pub scheduler_start: String,  // "HH:MM" 24h format
    pub scheduler_end: String,    // "HH:MM" 24h format
    // Auto-start
    pub autostart_enabled: bool,  // start app on system boot
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
            proxy_mode: "off".to_string(),
            proxy_host: String::new(),
            proxy_port: String::new(),
            scheduler_enabled: false,
            scheduler_start: "02:00".to_string(),
            scheduler_end: "06:00".to_string(),
            autostart_enabled: true,
        }
    }
}

// In-memory settings cache - loaded once at startup
static SETTINGS_CACHE: OnceLock<std::sync::RwLock<Option<Settings>>> = OnceLock::new();

/// Initialize settings cache at app startup
pub fn init_settings_cache(app: &AppHandle) {
    let cache = SETTINGS_CACHE.get_or_init(|| std::sync::RwLock::new(None));
    let settings = load(app);
    *cache.write().unwrap() = Some(settings);
}

/// Get cached settings (loads from disk only on first call)
pub fn get_cached_settings() -> Settings {
    let cache = SETTINGS_CACHE.get_or_init(|| std::sync::RwLock::new(None));
    let mut guard = cache.write().unwrap();
    if guard.is_none() {
        // This should only happen if init_settings_cache wasn't called
        eprintln!("[settings] Warning: cache not initialized, loading from disk");
        *guard = Some(Settings::default());
    }
    guard.as_ref().unwrap().clone()
}

/// Update cached settings and persist to disk
pub fn update_cached_settings(app: &AppHandle, f: impl FnOnce(&mut Settings)) -> Result<(), String> {
    let mut settings = get_cached_settings();
    f(&mut settings);
    save(app, &settings)?;
    let cache = SETTINGS_CACHE.get_or_init(|| std::sync::RwLock::new(None));
    *cache.write().unwrap() = Some(settings);
    Ok(())
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
    let mut s = get_cached_settings();
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
    update_cached_settings(&app, |s| *s = settings)
}

/// Update autostart setting
#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    update_cached_settings(&app, |s| s.autostart_enabled = enabled)?;
    
    // Update system autostart
    let autostart_manager = app.autolaunch();
    if enabled {
        let _ = autostart_manager.enable();
    } else {
        let _ = autostart_manager.disable();
    }
    
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
