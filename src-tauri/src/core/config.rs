//! Persistent user configuration.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

use serde::{Deserialize, Serialize};

use crate::core::error::AppResult;

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

/// How yt-dlp should reach the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxySetting {
    /// Explicitly bypass every proxy — yt-dlp needs `--proxy ""` for this,
    /// otherwise it inherits the system/registry proxy on its own.
    Direct,
    /// Use this proxy URL.
    Url(String),
    /// Leave the environment alone.
    Inherit,
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

    /// Resolve the proxy setting into something yt-dlp understands.
    ///
    /// "off" must actively pass an empty proxy: yt-dlp (through Python's
    /// `urllib`) otherwise picks up the Windows registry proxy or the
    /// `HTTPS_PROXY` environment variable, so "off" used to do nothing.
    /// "system" reads that same configuration and makes it explicit, which
    /// keeps the download and the metadata fetch on the same connection.
    pub fn proxy_setting(&self) -> ProxySetting {
        match self.proxy_mode.as_str() {
            "manual" => {
                let host = self.proxy_host.trim();
                if host.is_empty() {
                    return ProxySetting::Direct;
                }
                let port = self.proxy_port.trim();
                let url = if port.is_empty() {
                    format!("http://{host}")
                } else {
                    format!("http://{host}:{port}")
                };
                ProxySetting::Url(url)
            }
            "system" => match detect_system_proxy() {
                Some(url) => ProxySetting::Url(url),
                // No explicit proxy found: let yt-dlp follow the environment
                // (PAC files and per-app settings are invisible to us).
                None => ProxySetting::Inherit,
            },
            _ => ProxySetting::Direct,
        }
    }
}

/// Turn a proxy server value into a URL yt-dlp accepts.
///
/// Windows stores either `host:port` or a per-protocol list such as
/// `http=host:port;https=host:port`. Keep the https entry when present.
fn normalise_proxy_server(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let chosen = if trimmed.contains('=') {
        let https = trimmed
            .split(';')
            .find_map(|part| part.strip_prefix("https="));
        https
            .or_else(|| trimmed.split(';').next().and_then(|p| p.split_once('=').map(|(_, v)| v)))
            .unwrap_or(trimmed)
    } else {
        trimmed
    };

    if chosen.is_empty() {
        return None;
    }
    if chosen.starts_with("http://")
        || chosen.starts_with("https://")
        || chosen.starts_with("socks")
    {
        Some(chosen.to_string())
    } else {
        Some(format!("http://{chosen}"))
    }
}

#[cfg(windows)]
fn detect_system_proxy() -> Option<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")
        .ok()?;
    let enabled: u32 = key.get_value("ProxyEnable").unwrap_or(0);
    if enabled == 0 {
        return None;
    }
    let server: String = key.get_value("ProxyServer").ok()?;
    normalise_proxy_server(&server)
}

#[cfg(not(windows))]
fn detect_system_proxy() -> Option<String> {
    for name in ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy", "HTTP_PROXY", "http_proxy"] {
        if let Ok(value) = std::env::var(name) {
            if let Some(url) = normalise_proxy_server(&value) {
                return Some(url);
            }
        }
    }
    None
}

#[cfg(test)]
mod proxy_tests {
    use super::*;

    #[test]
    fn off_means_direct_connection() {
        let settings = Settings::default();
        assert_eq!(settings.proxy_setting(), ProxySetting::Direct);
    }

    #[test]
    fn manual_proxy_is_a_url() {
        let mut settings = Settings::default();
        settings.proxy_mode = "manual".into();
        settings.proxy_host = "127.0.0.1".into();
        settings.proxy_port = "10808".into();
        assert_eq!(
            settings.proxy_setting(),
            ProxySetting::Url("http://127.0.0.1:10808".to_string())
        );
    }

    #[test]
    fn manual_without_host_is_direct() {
        let mut settings = Settings::default();
        settings.proxy_mode = "manual".into();
        assert_eq!(settings.proxy_setting(), ProxySetting::Direct);
    }

    #[test]
    fn system_proxy_shapes() {
        assert_eq!(
            normalise_proxy_server("127.0.0.1:10808"),
            Some("http://127.0.0.1:10808".to_string())
        );
        assert_eq!(
            normalise_proxy_server("http=127.0.0.1:8080;https=127.0.0.1:8443"),
            Some("http://127.0.0.1:8443".to_string())
        );
        assert_eq!(
            normalise_proxy_server("socks5://127.0.0.1:1080"),
            Some("socks5://127.0.0.1:1080".to_string())
        );
        assert_eq!(normalise_proxy_server("  "), None);
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
    /// Serialises read-modify-write cycles so two concurrent `update` calls
    /// cannot overwrite each other.
    write_guard: Mutex<()>,
}

impl ConfigStore {
    pub fn load(path: PathBuf) -> Self {
        let settings = read_settings(&path);
        Self {
            inner: RwLock::new(settings),
            path,
            write_guard: Mutex::new(()),
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
        let _guard = self
            .write_guard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

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
///
/// Returns `None` when the platform cannot resolve a downloads folder, so the
/// caller can ask the user instead of inventing a path.
pub fn default_download_root(app: &tauri::AppHandle) -> Option<String> {
    use tauri::Manager;
    app.path()
        .download_dir()
        .ok()
        .map(|dir| dir.join(DEFAULT_APP_FOLDER).to_string_lossy().to_string())
}
