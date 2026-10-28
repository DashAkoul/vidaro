use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use crate::settings::get_cached_settings;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum ExtensionMessage {
    Download {
        url: String,
        quality: Option<String>,
        audio_only: Option<bool>,
        format: Option<String>,
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

#[allow(dead_code)]
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum NativeResponse {
    Success { data: serde_json::Value },
    Error { message: String },
    QueueUpdate { items: Vec<QueueItemInfo> },
    SettingsUpdate { settings: serde_json::Value },
    HistoryUpdate { items: Vec<HistoryItemInfo> },
    Pong,
    ItemStatus { item: QueueItemInfo },
}

#[derive(Debug, Serialize, Clone)]
pub struct QueueItemInfo {
    pub id: String,
    pub url: String,
    pub video_id: String,
    pub title: String,
    pub status: String,
    pub percent: f64,
    pub speed_bps: f64,
    pub eta_secs: f64,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub stage: String,
    pub audio_only: bool,
    pub quality: String,
    pub batch_name: String,
    pub index: Option<u32>,
    pub created_at: i64,
    pub error: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct HistoryItemInfo {
    pub id: String,
    pub title: String,
    pub path: String,
    pub size_bytes: u64,
    pub finished_at: i64,
    pub kind: String,
    pub quality: String,
    pub batch_name: String,
}

struct NativeMessagingServer {
    app_handle: tauri::AppHandle,
    running: Arc<Mutex<bool>>,
}

impl NativeMessagingServer {
    fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            running: Arc::new(Mutex::new(false)),
        }
    }

    fn start(&self) {
        let mut running = self.running.lock().unwrap();
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let app_handle = self.app_handle.clone();

        thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut stdin = stdin.lock();
            let running = true;

            while running {
                let mut len_buf = [0u8; 4];
                if stdin.read_exact(&mut len_buf).is_err() {
                    break;
                }
                let length = u32::from_le_bytes(len_buf) as usize;

                if length == 0 || length > 1024 * 1024 {
                    continue;
                }

                let mut msg_buf = vec![0u8; length];
                if stdin.read_exact(&mut msg_buf).is_err() {
                    continue;
                }

                let message_str = match String::from_utf8(msg_buf) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let msg: ExtensionMessage = match serde_json::from_str(&message_str) {
                    Ok(m) => m,
                    Err(_) => {
                        send_response(NativeResponse::Error {
                            message: "Invalid message format".to_string()
                        });
                        continue;
                    }
                };

                let response = handle_extension_message_internal(&app_handle, msg);
                send_response(response);
            }
        });
    }

    #[allow(dead_code)]
fn stop(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }
}

fn send_response(response: NativeResponse) {
    let json = serde_json::to_string(&response).unwrap_or_default();
    let bytes = json.as_bytes();
    let len_bytes = (bytes.len() as u32).to_le_bytes();

    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(&len_bytes);
    let _ = stdout.write_all(&bytes);
    let _ = stdout.flush();
}

fn handle_extension_message_internal(_app_handle: &tauri::AppHandle, msg: ExtensionMessage) -> NativeResponse {
    match msg {
        ExtensionMessage::Download { url: _, quality, audio_only, format, title: _ } => {
            let _quality = quality.unwrap_or_else(|| "best".to_string());
            let _audio_only = audio_only.unwrap_or(false);
            let _format = format.unwrap_or_else(|| if _audio_only { "mp3".to_string() } else { "mp4".to_string() });

            NativeResponse::Success { data: serde_json::json!({ "status": "queued" }) }
        }
        ExtensionMessage::GetQueue => {
            NativeResponse::Success { data: serde_json::json!({ "items": [] }) }
        }
        ExtensionMessage::GetSettings => {
            NativeResponse::Success { data: serde_json::to_value(get_cached_settings()).unwrap() }
        }
        ExtensionMessage::UpdateSettings { settings: _ } => {
            NativeResponse::Success { data: serde_json::to_value(true).unwrap() }
        }
        ExtensionMessage::GetHistory => {
            NativeResponse::Success { data: serde_json::json!({ "items": [] }) }
        }
        ExtensionMessage::ControlDownload { id: _, action: _ } => {
            NativeResponse::Success { data: serde_json::to_value(true).unwrap() }
        }
        ExtensionMessage::GetItemStatus { id: _ } => {
            NativeResponse::ItemStatus { item: QueueItemInfo {
                id: "".to_string(),
                url: "".to_string(),
                video_id: "".to_string(),
                title: "".to_string(),
                status: "".to_string(),
                percent: 0.0,
                speed_bps: 0.0,
                eta_secs: 0.0,
                total_bytes: 0,
                downloaded_bytes: 0,
                stage: "".to_string(),
                audio_only: false,
                quality: "".to_string(),
                batch_name: "".to_string(),
                index: None,
                created_at: 0,
                error: None,
                output_path: None,
            }}
        }
        ExtensionMessage::Ping => {
            NativeResponse::Pong
        }
    }
}

pub fn start_server(app_handle: tauri::AppHandle) {
    let server = NativeMessagingServer::new(app_handle);
    server.start();
}

#[tauri::command]
pub fn handle_extension_message(msg: ExtensionMessage) -> NativeResponse {
    // For frontend usage, we can't access app_handle directly here
    // This is a simplified version - real implementation would need app state
    match msg {
        ExtensionMessage::Ping => NativeResponse::Pong,
        ExtensionMessage::GetSettings => {
            NativeResponse::Success { data: serde_json::to_value(get_cached_settings()).unwrap() }
        }
        _ => NativeResponse::Error { message: "Not fully implemented via Tauri command".to_string() }
    }
}