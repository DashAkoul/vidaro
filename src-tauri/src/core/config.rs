//! Persistent user configuration.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};

pub const SINGLE_VIDEOS_DIR: &str = "Single Videos";
pub const DEFAULT_APP_FOLDER: &str = "YouTube Downloader";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub language: String,    // "fa" | "en"
    pub theme: String,       // "system" | "light" | "dark"
    pub default_dir: String, // "" = not chosen yet
    pub max_concurrent: u32,

    // Download tuning
    pub rate_limit: String, // "" = unlimited, e.g. "5M"
    pub write_subs: bool,
    pub sub_langs: String,
    pub embed_metadata: bool,
    pub embed_thumbnail: bool,

    // Network
    pub proxy_mode: String, // "off" | "system" | "manual"
    pub proxy_host: String,
    pub proxy_port: String,

    // Scheduler
    pub scheduler_enabled: bool,
    pub scheduler_start: String, // "HH:MM"
    pub scheduler_end: String,   // "HH:MM"

    // Shell integration
    pub autostart_enabled: bool,
    pub close_to_tray: bool,
    pub clipboard_watch: bool,
    pub notify_on_clipboard: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            theme: "system".into(),
            default_dir: String::new(),
            max_concurrent: 2,
            rate_limit: String::new(),
            write_subs: false,
            sub_langs: "en".into(),
            embed_metadata: true,
            embed_thumbnail: true,
            proxy_mode: "off".into(),
            proxy_host: String::new(),
            proxy_port: String::new(),
            scheduler_enabled: false,
            scheduler_start: "02:00".into(),
            scheduler_end: "06:00".into(),
            autostart_enabled: true,
            close_to_tray: true,
            clipboard_watch: true,
            notify_on_clipboard: true,
        }
    }
}

impl Settings {
    /// Clamp values that come from untrusted sources (imported files, the UI).
    pub fn sanitize(&mut self) {
        if self.language != "fa" && self.language != "en" {
            self.language = "en".into();
        }
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            self.theme = "system".into();
        }
        if !matches!(self.proxy_mode.as_str(), "off" | "system" | "manual") {
            self.proxy_mode = "off".into();
        }
        self.max_concurrent = self.max_concurrent.clamp(1, 8);
        self.scheduler_start = normalize_time(&self.scheduler_start, "02:00");
        self.scheduler_end = normalize_time(&self.scheduler_end, "06:00");
        if self.sub_langs.trim().is_empty() {
            self.sub_langs = "en".into();
        }
    }

    /// Resolved proxy URL, if a manual proxy is configured.
    pub fn proxy_url(&self) -> Option<String> {
        if self.proxy_mode != "manual" {
            return None;
        }
        let host = self.proxy_host.trim();
        if host.is_empty() {
            return None;
        }
        let port = self.proxy_port.trim();
        if port.is_empty() {
            Some(format!("http://{host}"))
        } else {
            Some(format!("http://{host}:{port}"))
        }
    }
}

fn normalize_time(value: &str, fallback: &str) -> String {
    match parse_hhmm(value) {
        Some(_) => value.trim().to_string(),
        None => fallback.to_string(),
    }
}

/// Parse "HH:MM" into minutes since midnight.
pub fn parse_hhmm(value: &str) -> Option<u32> {
    let (h, m) = value.trim().split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

/// Thread-safe settings holder backed by a JSON file.
pub struct ConfigStore {
    inner: RwLock<Settings>,
    path: PathBuf,
}

impl ConfigStore {
    pub fn load(path: PathBuf) -> Self {
        let settings = read_settings(&path);
        Self {
            inner: RwLock::new(settings),
            path,
        }
    }

    pub fn get(&self) -> Settings {
        self.inner.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn replace(&self, mut settings: Settings) -> AppResult<Settings> {
        settings.sanitize();
        self.persist(&settings)?;
        if let Ok(mut guard) = self.inner.write() {
            *guard = settings.clone();
        }
        Ok(settings)
    }

    pub fn update<F: FnOnce(&mut Settings)>(&self, mutate: F) -> AppResult<Settings> {
        let mut next = self.get();
        mutate(&mut next);
        self.replace(next)
    }

    fn persist(&self, settings: &Settings) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(settings)?;
        // Write to a temp file first so a crash mid-write cannot corrupt config.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

fn read_settings(path: &Path) -> Settings {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    match serde_json::from_str::<Settings>(&text) {
        Ok(mut s) => {
            s.sanitize();
            s
        }
        Err(e) => {
            eprintln!("[config] ignoring malformed settings file: {e}");
            Settings::default()
        }
    }
}

/// Default download root: `<Downloads>/YouTube Downloader`.
pub fn default_download_root(app: &tauri::AppHandle) -> String {
    use tauri::Manager;
    app.path()
        .download_dir()
        .map(|d| d.join(DEFAULT_APP_FOLDER).to_string_lossy().to_string())
        .unwrap_or_else(|_| {
            AppError::NotFound("downloads directory".into())
                .to_string()
        })
}
