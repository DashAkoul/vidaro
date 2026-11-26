//! A durable YouTube session that does not need a browser at runtime.
//!
//! Reading cookies straight from Firefox or Chrome works only while that
//! browser still holds the session: closing Firefox can drop session cookies,
//! Chrome's database is often DPAPI-encrypted, and an update may log the user
//! out. "It worked yesterday and not today" was almost always this.
//!
//! Importing copies the browser's cookie jar into the app's own
//! `session/cookies.txt` once. From then on yt-dlp reads that file, so the
//! browser can be closed, uninstalled or logged out without affecting Vidaro.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager as _};

use crate::core::config::ConfigStore;
use crate::core::error::{AppError, AppResult};
use crate::core::paths::AppPaths;
use crate::core::process::async_command;

/// Cookies that only exist after a real sign-in.
const AUTH_COOKIES: [&str; 8] = [
    "SID",
    "HSID",
    "SSID",
    "APISID",
    "SAPISID",
    "__Secure-1PSID",
    "__Secure-3PSID",
    "__Secure-1PAPISID",
];

/// Browsers that encrypt their cookie database with Windows' app-bound key.
/// Chrome and Edge do this from version 127, which leaves the cookies readable
/// by the browser's own extensions and by nothing else — no external tool,
/// yt-dlp included, can decrypt them.
fn is_app_bound(browser: &str) -> bool {
    matches!(
        browser,
        "chrome" | "chromium" | "edge" | "brave" | "vivaldi" | "opera"
    )
}

fn browser_exe(browser: &str) -> Option<PathBuf> {
    let candidates: &[&str] = match browser {
        "firefox" => &[
            r"C:\Program Files\Mozilla Firefox\firefox.exe",
            r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe",
        ],
        "edge" => &[
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ],
        "chrome" | "chromium" => &[
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ],
        "brave" => &[
            r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe",
        ],
        "vivaldi" => &[r"C:\Program Files\Vivaldi\Application\vivaldi.exe"],
        _ => &[],
    };

    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .or_else(|| crate::features::tools::paths::which(&format!("{browser}.exe")))
}

/// Open youtube.com in the chosen browser so the user can sign in there.
///
/// The sign-in has to happen somewhere the cookies can be read back: Firefox
/// keeps them in a plain SQLite file, while every Chromium-based browser
/// encrypts them with a key only the browser itself can unwrap. Opening the
/// browser at the right page turns a confusing dead end into one click.
#[tauri::command]
pub fn open_browser_login(browser: String) -> AppResult<()> {
    let exe = browser_exe(&browser).ok_or_else(|| {
        AppError::Tool(format!(
            "{browser} was not found on this computer. Install it, or paste a cookies.txt file instead."
        ))
    })?;

    std::process::Command::new(exe)
        .arg("https://www.youtube.com")
        .spawn()
        .map_err(|e| AppError::Tool(format!("could not start {browser}: {e}")))?;

    Ok(())
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub has_file: bool,
    pub auth_cookies: usize,
    /// Last time the jar was imported (epoch ms).
    pub updated_at: Option<i64>,
    pub path: String,
}

pub fn session_file(app: &AppHandle) -> Option<PathBuf> {
    let paths = app.try_state::<AppPaths>()?;
    Some(paths.data_dir.join("session").join("cookies.txt"))
}

/// Inspect the managed jar without touching the network.
pub fn status(app: &AppHandle) -> SessionStatus {
    let path = session_file(app);
    let Some(path) = path else {
        return SessionStatus {
            has_file: false,
            auth_cookies: 0,
            updated_at: None,
            path: String::new(),
        };
    };

    SessionStatus {
        has_file: path.is_file(),
        auth_cookies: count_auth_cookies(&path),
        updated_at: std::fs::metadata(&path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .map(|time| {
                chrono::DateTime::<chrono::Utc>::from(time)
                    .timestamp_millis()
            }),
        path: path.to_string_lossy().to_string(),
    }
}

/// Count the signed-in cookies in a Netscape cookie file.
///
/// Zero means the jar is useless: the browser was not logged in, or it had
/// already discarded the session cookies when it closed.
fn count_auth_cookies(path: &Path) -> usize {
    let Ok(text) = std::fs::read_to_string(path) else {
        return 0;
    };
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let _domain = fields.next()?;
            let _include_subdomains = fields.next()?;
            let _path = fields.next()?;
            let _secure = fields.next()?;
            let _expires = fields.next()?;
            let name = fields.next()?;
            Some(name)
        })
        .filter(|name| AUTH_COOKIES.contains(name))
        .count()
}

/// Copy the browser's cookies into the managed jar and switch the app to it.
///
/// yt-dlp is the only component that can read every browser's cookie store,
/// so it performs the import: `--cookies-from-browser` loads the session and
/// `--cookies` writes the merged jar back out.
#[tauri::command]
pub async fn import_browser_cookies(
    app: AppHandle,
    config: tauri::State<'_, std::sync::Arc<ConfigStore>>,
    browser: String,
) -> AppResult<SessionStatus> {
    let browser = browser.trim().to_lowercase();
    const ALLOWED: [&str; 8] = [
        "chrome", "chromium", "edge", "brave", "vivaldi", "opera", "firefox", "safari",
    ];
    if !ALLOWED.contains(&browser.as_str()) {
        return Err(AppError::Invalid(format!("unsupported browser: {browser}")));
    }

    let path = session_file(&app)
        .ok_or_else(|| AppError::Tool("application paths not available".into()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tools = app
        .try_state::<crate::features::tools::ToolManager>()
        .ok_or_else(|| AppError::Tool("tools not initialised".into()))?;
    let ytdlp = tools.paths().ytdlp;
    if !ytdlp.is_file() {
        return Err(AppError::Tool("yt-dlp is not installed".into()));
    }

    let settings = config.get();
    let mut command = async_command(&ytdlp);
    command.args([
        "--no-warnings",
        "--no-colors",
        "--no-update",
        "--socket-timeout",
        "20",
        "--cookies-from-browser",
        &browser,
        "--cookies",
        &path.to_string_lossy(),
        "--simulate",
        "--print",
        "%(id)s",
        // Any public video works; the point is to make yt-dlp load and flush
        // the cookie jar.
        "https://www.youtube.com/watch?v=jNQXAC9IVRw",
    ]);
    if let Some(runtime) = tools.paths().javascript() {
        command.args([
            "--js-runtimes",
            &format!("{}:{}", runtime.name, runtime.path.to_string_lossy()),
        ]);
    }
    if let crate::core::config::ProxySetting::Url(proxy) = settings.proxy_setting() {
        command.args(["--proxy", &proxy]);
    }

    // The import must finish even if extraction fails; the jar is flushed on
    // the way out either way, so the status code is deliberately ignored.
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        command.output(),
    )
    .await;

    let imported = count_auth_cookies(&path);
    if imported == 0 {
        return Err(AppError::Invalid(format!(
            "no signed-in YouTube cookies were found in {browser}. {}",
            if is_app_bound(&browser) {
                "This browser encrypts its cookies with a Windows key (app-bound encryption) that only the browser itself can unwrap, so nothing outside it can read them — yt-dlp included. Sign in with Firefox and import from there, or export a cookies.txt with a browser extension and paste it below."
            } else {
                "Sign in to youtube.com in this browser, then import again."
            }
        )));
    }

    // From now on the app uses the managed file, not the browser.
    config.update(|current| {
        current.cookies_mode = "file".into();
        current.cookies_file = path.to_string_lossy().to_string();
    })?;

    Ok(status(&app))
}

/// Store a cookie file the user pasted or copied in by hand.
///
/// This is the browser-independent path: a Netscape `cookies.txt` exported with
/// any extension, on any machine, for any browser. The app validates that the
/// text really contains a signed-in session before switching to it, so a wrong
/// paste fails loudly instead of silently breaking downloads.
#[tauri::command]
pub fn save_cookies_text(
    app: AppHandle,
    config: tauri::State<'_, std::sync::Arc<ConfigStore>>,
    text: String,
) -> AppResult<SessionStatus> {
    let path = session_file(&app)
        .ok_or_else(|| AppError::Tool("application paths not available".into()))?;

    if !text.contains('\t') || !text.to_ascii_lowercase().contains("youtube.com") {
        return Err(AppError::Invalid(
            "this does not look like a Netscape cookies.txt file (tab-separated, with youtube.com entries)".into(),
        ));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Keep only real cookie lines: comments are allowed, blank lines dropped.
    let cleaned: String = text
        .lines()
        .filter(|line| line.starts_with('#') || line.split('\t').count() >= 7)
        .collect::<Vec<_>>()
        .join("
");
    std::fs::write(&path, format!("{cleaned}
"))?;

    let imported = count_auth_cookies(&path);
    if imported == 0 {
        let _ = std::fs::remove_file(&path);
        return Err(AppError::Invalid(
            "the file has no signed-in YouTube cookies (SID/SAPISID). Export it while logged in to youtube.com.".into(),
        ));
    }

    config.update(|current| {
        current.cookies_mode = "file".into();
        current.cookies_file = path.to_string_lossy().to_string();
    })?;

    Ok(status(&app))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(text: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vidaro-cookies-{}.txt",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&path, text).expect("write fixture");
        path
    }

    #[test]
    fn counts_only_signed_in_cookies() {
        let path = write(
            "# Netscape HTTP Cookie File\n\
             .youtube.com\tTRUE\t/\tTRUE\t0\tPREF\tabc\n\
             .youtube.com\tTRUE\t/\tTRUE\t0\tSID\tsid\n\
             .youtube.com\tTRUE\t/\tTRUE\t0\tSAPISID\tx\n\
             .google.com\tTRUE\t/\tTRUE\t0\tNID\tn\n",
        );
        assert_eq!(count_auth_cookies(&path), 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_jar_without_a_session_counts_zero() {
        let path = write(".youtube.com\tTRUE\t/\tTRUE\t0\tPREF\tabc\n");
        assert_eq!(count_auth_cookies(&path), 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_missing_file_counts_zero() {
        assert_eq!(count_auth_cookies(Path::new("/nonexistent/cookies.txt")), 0);
    }
}
