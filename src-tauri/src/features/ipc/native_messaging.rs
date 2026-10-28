//! Native-messaging bridge for the bundled browser extension.
//!
//! Implements Chrome's stdio protocol: a 4-byte little-endian length prefix
//! followed by a UTF-8 JSON payload, in both directions.

use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager as _};

use crate::core::config::ConfigStore;
use crate::features::download::model::Item;
use crate::features::download::queue::Manager;
use crate::features::history::{self, HistoryEntry};

/// Maximum accepted message size (Chrome's own limit is 1 MiB).
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

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

fn handle(app: &AppHandle, message: ExtensionMessage) -> NativeResponse {
    match message {
        ExtensionMessage::Ping => NativeResponse::Pong,

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

        ExtensionMessage::GetSettings => match app.try_state::<ConfigStore>() {
            Some(config) => NativeResponse::SettingsUpdate {
                settings: serde_json::to_value(config.get()).unwrap_or_default(),
            },
            None => error("settings not available"),
        },

        ExtensionMessage::UpdateSettings { settings } => {
            let Some(config) = app.try_state::<ConfigStore>() else {
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

fn write_response(response: &NativeResponse) {
    let Ok(json) = serde_json::to_string(response) else {
        return;
    };
    let bytes = json.as_bytes();
    if bytes.len() > MAX_MESSAGE_BYTES {
        return;
    }

    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(&(bytes.len() as u32).to_le_bytes());
    let _ = stdout.write_all(bytes);
    let _ = stdout.flush();
}

/// Start the bridge. Harmless when the app was not launched by a browser: the
/// very first read hits EOF and the thread exits.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut stdin = stdin.lock();

        loop {
            let mut length_buffer = [0u8; 4];
            if stdin.read_exact(&mut length_buffer).is_err() {
                break;
            }

            let length = u32::from_le_bytes(length_buffer) as usize;
            if length == 0 || length > MAX_MESSAGE_BYTES {
                break;
            }

            let mut payload = vec![0u8; length];
            if stdin.read_exact(&mut payload).is_err() {
                break;
            }

            let response = match String::from_utf8(payload) {
                Ok(text) => match serde_json::from_str::<ExtensionMessage>(&text) {
                    Ok(message) => handle(&app, message),
                    Err(e) => error(format!("invalid message: {e}")),
                },
                Err(_) => error("message was not valid UTF-8"),
            };

            write_response(&response);
        }
    });
}

pub struct IpcPlugin;

impl crate::core::plugin::Plugin for IpcPlugin {
    fn name(&self) -> &'static str {
        "ipc"
    }

    fn startup(&self, ctx: &crate::core::plugin::AppContext) -> crate::core::error::AppResult<()> {
        start(ctx.app.clone());
        Ok(())
    }
}
