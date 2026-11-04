//! Message handling for the browser extension.
//!
//! The stdio protocol itself lives in `host.rs` (relay) and the transport in
//! `bridge.rs`; this module only decides what each message does.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager as _};

use crate::features::download::model::Item;
use crate::features::download::queue::Manager;
use crate::features::history::{self, HistoryEntry};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ExtensionMessage {
    Download {
        url: String,
        quality: Option<String>,
        audio_only: Option<bool>,
        title: Option<String>,
    },
    GetQueue,
    GetSettings,
    UpdateSettings { settings: serde_json::Value },
    GetHistory,
    ControlDownload { id: String, action: String },
    GetItemStatus { id: String },
    /// Bring the running window to the front.
    Show,
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum NativeResponse {
    Success { data: serde_json::Value },
    Error { message: String },
    QueueUpdate { items: Vec<Item> },
    SettingsUpdate { settings: serde_json::Value },
    HistoryUpdate { items: Vec<HistoryEntry> },
    Pong,
    ItemStatus { item: Option<Item> },
}

fn error(message: impl Into<String>) -> NativeResponse {
    NativeResponse::Error {
        message: message.into(),
    }
}

pub fn handle(app: &AppHandle, message: ExtensionMessage) -> NativeResponse {
    match message {
        ExtensionMessage::Ping => NativeResponse::Pong,

        ExtensionMessage::Show => {
            crate::features::system::window::show_main_window(app);
            NativeResponse::Success {
                data: serde_json::json!({ "ok": true }),
            }
        }

        ExtensionMessage::Download {
            url,
            quality,
            audio_only,
            title,
        } => {
            let audio_only = audio_only.unwrap_or(false);
            let quality = quality.unwrap_or_else(|| "best".to_string());
            match crate::features::download::enqueue_single(
                app,
                &url,
                title.as_deref().unwrap_or(""),
                &quality,
                audio_only,
            ) {
                Ok(item) => NativeResponse::Success {
                    data: serde_json::json!({
                        "status": "queued",
                        "id": item.id,
                        "title": item.title,
                    }),
                },
                Err(e) => error(e.to_string()),
            }
        }

        ExtensionMessage::GetQueue => match app.try_state::<Manager>() {
            Some(manager) => NativeResponse::QueueUpdate {
                items: manager.queue.snapshot(),
            },
            None => error("queue not available"),
        },

        ExtensionMessage::GetSettings => match crate::core::state::config_opt(app) {
            Some(config) => NativeResponse::SettingsUpdate {
                settings: serde_json::to_value(config.get()).unwrap_or_default(),
            },
            None => error("settings not available"),
        },

        ExtensionMessage::UpdateSettings { settings } => {
            let Some(config) = crate::core::state::config_opt(app) else {
                return error("settings not available");
            };
            match serde_json::from_value::<crate::core::config::Settings>(settings) {
                Ok(next) => match config.replace(next) {
                    Ok(saved) => NativeResponse::SettingsUpdate {
                        settings: serde_json::to_value(saved).unwrap_or_default(),
                    },
                    Err(e) => error(e.to_string()),
                },
                Err(e) => error(format!("invalid settings: {e}")),
            }
        }

        ExtensionMessage::GetHistory => NativeResponse::HistoryUpdate {
            items: history::load(app),
        },

        ExtensionMessage::ControlDownload { id, action } => {
            let Some(manager) = app.try_state::<Manager>() else {
                return error("queue not available");
            };
            let queue = manager.queue.clone();
            let Some(inner) = queue.get(&id) else {
                return error("item not found");
            };

            tauri::async_runtime::block_on(async {
                match action.as_str() {
                    "pause" => {
                        crate::features::download::control::pause(
                            app,
                            &queue,
                            &inner,
                            crate::features::download::model::PausedBy::User,
                        )
                        .await;
                    }
                    "resume" => {
                        crate::features::download::control::resume(app, queue.clone(), inner);
                    }
                    "cancel" => {
                        crate::features::download::control::cancel(app, &queue, &inner).await;
                    }
                    "retry" => {
                        crate::features::download::control::retry(app, queue.clone(), inner).await;
                    }
                    "remove" => {
                        crate::features::download::control::remove(app, &queue, &id).await;
                    }
                    other => return error(format!("unknown action: {other}")),
                }
                NativeResponse::Success {
                    data: serde_json::json!({ "ok": true }),
                }
            })
        }

        ExtensionMessage::GetItemStatus { id } => match app.try_state::<Manager>() {
            Some(manager) => NativeResponse::ItemStatus {
                item: manager.queue.get(&id).map(|inner| inner.snapshot()),
            },
            None => error("queue not available"),
        },
    }
}

pub struct IpcPlugin;

impl crate::core::plugin::Plugin for IpcPlugin {
    fn name(&self) -> &'static str {
        "ipc"
    }

    fn startup(&self, ctx: &crate::core::plugin::AppContext) -> crate::core::error::AppResult<()> {
        // Publish the loopback bridge the browser relay talks to, then make
        // sure every installed browser knows where this executable is.
        super::bridge::start(ctx.app.clone(), &ctx.paths);
        super::registration::ensure(&ctx.paths);
        Ok(())
    }
}
