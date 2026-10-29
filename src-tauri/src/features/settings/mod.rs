//! Settings commands, data import/export, and app metadata.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::core::config::{default_download_root, ConfigStore, Settings};
use crate::core::error::{AppError, AppResult};
use crate::core::events;
use crate::features::download::model::ItemStatus;
use crate::features::download::queue::{self, ItemInner, Manager};
use crate::features::history::{self, HistoryEntry};
use crate::features::system::autostart;

const EXPORT_VERSION: u32 = 2;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub arch: String,
    /// True when the process was launched by the OS autostart entry.
    pub started_hidden: bool,
}

#[tauri::command]
pub fn app_info(app: AppHandle) -> AppInfo {
    let started_hidden = std::env::args().any(|a| a == "--hidden" || a == "--autostart");
    AppInfo {
        name: "Vidaro".to_string(),
        version: app.package_info().version.to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        started_hidden,
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle, config: State<'_, ConfigStore>) -> Settings {
    let mut settings = config.get();
    if settings.default_dir.trim().is_empty() {
        if let Some(dir) = default_download_root(&app) {
            settings.default_dir = dir;
        }
    }
    settings
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    config: State<'_, ConfigStore>,
    settings: Settings,
) -> AppResult<Settings> {
    let previous = config.get();
    let saved = config.replace(settings)?;

    if previous.autostart_enabled != saved.autostart_enabled {
        autostart::reconcile(&app, saved.autostart_enabled);
    }

    events::emit(&app, events::SETTINGS_CHANGED, &saved);
    Ok(saved)
}

#[tauri::command]
pub fn set_autostart(
    app: AppHandle,
    config: State<'_, ConfigStore>,
    enabled: bool,
) -> AppResult<Settings> {
    autostart::set_enabled(enabled)?;
    let saved = config.update(|settings| settings.autostart_enabled = enabled)?;
    events::emit(&app, events::SETTINGS_CHANGED, &saved);
    Ok(saved)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportBundle {
    version: u32,
    exported_at: String,
    settings: Settings,
    queue: Vec<crate::features::download::model::Item>,
    history: Vec<HistoryEntry>,
}

#[tauri::command]
pub fn export_data(
    app: AppHandle,
    config: State<'_, ConfigStore>,
    mgr: State<'_, Manager>,
    path: String,
) -> AppResult<()> {
    let bundle = ExportBundle {
        version: EXPORT_VERSION,
        exported_at: chrono::Utc::now().to_rfc3339(),
        settings: config.get(),
        queue: mgr.queue.snapshot(),
        history: history::load(&app),
    };

    let json = serde_json::to_string_pretty(&bundle)?;
    let target = std::path::PathBuf::from(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, json)?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportBundle {
    version: u32,
    settings: Settings,
    #[serde(default)]
    queue: Vec<crate::features::download::model::Item>,
    #[serde(default)]
    history: Vec<HistoryEntry>,
}

#[tauri::command]
pub async fn import_data(
    app: AppHandle,
    config: State<'_, ConfigStore>,
    mgr: State<'_, Manager>,
    path: String,
) -> AppResult<()> {
    let content = std::fs::read_to_string(&path)?;
    let bundle: ImportBundle = serde_json::from_str(&content)
        .map_err(|e| AppError::Invalid(format!("invalid backup file: {e}")))?;

    if bundle.version > EXPORT_VERSION {
        return Err(AppError::Invalid(
            "this backup was created by a newer version of Vidaro".into(),
        ));
    }

    // Settings first — sanitised inside `replace`.
    let previous = config.get();
    let saved = config.replace(bundle.settings)?;
    if previous.autostart_enabled != saved.autostart_enabled {
        autostart::reconcile(&app, saved.autostart_enabled);
    }
    events::emit(&app, events::SETTINGS_CHANGED, &saved);

    // Queue: reset volatile state and skip duplicates.
    let queue = mgr.queue.clone();
    for mut item in bundle.queue {
        item.status = ItemStatus::Queued;
        item.reset_progress();
        item.output_path = None;
        item.paused_by = None;
        item.created_at = chrono::Utc::now().timestamp_millis();

        let duplicate = queue.all().iter().any(|existing| {
            existing.with_item(|current| {
                current.video_id == item.video_id
                    && current.dir == item.dir
                    && !matches!(current.status, ItemStatus::Error | ItemStatus::Cancelled)
            })
        });
        if duplicate {
            continue;
        }

        let inner = ItemInner::new(item);
        queue.insert(inner.clone());
        crate::features::download::control::spawn_run(&app, queue.clone(), inner);
    }
    queue::persist(&app, &queue);
    events::emit_empty(&app, events::QUEUE_CHANGED);

    history::merge(&app, bundle.history);
    events::emit_empty(&app, events::HISTORY_CHANGED);

    Ok(())
}
