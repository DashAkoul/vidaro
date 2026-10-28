//! Clipboard watcher: notices when a YouTube link is copied.
//!
//! One `arboard::Clipboard` handle is created per thread and reused. The
//! previous implementation opened a new handle every second, which is both
//! wasteful and a known source of transient clipboard lock errors on Windows.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager as _};

use crate::core::config::ConfigStore;
use crate::core::events;
use crate::features::metadata::url::is_youtube_url;
use crate::features::system::window;

const POLL_INTERVAL: Duration = Duration::from_millis(900);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardLink {
    pub url: String,
}

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let mut clipboard: Option<arboard::Clipboard> = arboard::Clipboard::new().ok();
        let mut last_seen = String::new();

        loop {
            std::thread::sleep(POLL_INTERVAL);

            let (watching, notify) = match app.try_state::<ConfigStore>() {
                Some(config) => {
                    let settings = config.get();
                    (settings.clipboard_watch, settings.notify_on_clipboard)
                }
                None => (true, true),
            };
            if !watching {
                continue;
            }

            // Re-create the handle lazily: some clipboard managers reset it.
            if clipboard.is_none() {
                clipboard = arboard::Clipboard::new().ok();
            }
            let Some(handle) = clipboard.as_mut() else {
                continue;
            };

            let text = match handle.get_text() {
                Ok(text) => text,
                Err(_) => {
                    clipboard = None;
                    continue;
                }
            };

            let candidate = text.trim();
            if candidate.is_empty() || candidate == last_seen || !is_youtube_url(candidate) {
                continue;
            }
            last_seen = candidate.to_string();

            events::emit(
                &app,
                events::CLIPBOARD_LINK,
                ClipboardLink {
                    url: candidate.to_string(),
                },
            );

            // Only interrupt with a system notification when the user cannot
            // see the app; otherwise the in-app toast is enough.
            if notify && !window::is_window_visible(&app) {
                show_notification(&app, candidate);
            }
        }
    });
}

fn show_notification(app: &AppHandle, url: &str) {
    use tauri_plugin_notification::NotificationExt;

    let language = app
        .try_state::<ConfigStore>()
        .map(|c| c.get().language)
        .unwrap_or_else(|| "en".to_string());

    let (title, body) = if language == "fa" {
        (
            "Vidaro",
            format!("لینک یوتیوب کپی شد — برای دانلود باز کنید\n{url}"),
        )
    } else {
        (
            "Vidaro",
            format!("YouTube link detected — open Vidaro to download\n{url}"),
        )
    };

    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
}
