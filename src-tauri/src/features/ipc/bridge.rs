//! Local bridge between the GUI process and the native-messaging host.
//!
//! A browser cannot talk to an already-running application directly: it spawns
//! the native host as a child and speaks the stdio protocol with it. Vidaro is
//! a single-instance GUI app, so that spawned process would either open a
//! second window or be killed by the single-instance guard.
//!
//! Instead the spawned process runs in a headless relay mode (see `host.rs`)
//! and forwards each message here over a loopback socket, where the real queue
//! and settings live. The socket is bound to 127.0.0.1 on an ephemeral port and
//! guarded by a random token written next to it, so another local user cannot
//! drive the queue.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::core::paths::AppPaths;
use crate::features::ipc::native_messaging::{handle, ExtensionMessage};

/// File the relay reads to find the port and token.
pub const BRIDGE_FILE: &str = "bridge.json";

#[derive(Serialize, Deserialize)]
pub struct BridgeInfo {
    pub port: u16,
    pub token: String,
    pub pid: u32,
}

#[derive(Deserialize)]
struct BridgeRequest {
    token: String,
    #[serde(default, rename = "requestId")]
    request_id: Option<serde_json::Value>,
    message: serde_json::Value,
}

/// Bind the loopback socket, publish its coordinates, and serve forever.
pub fn start(app: AppHandle, paths: &AppPaths) {
    let listener = match TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => listener,
        Err(e) => {
            crate::core::logging::log_startup("bridge", &format!("bind failed: {e}"));
            return;
        }
    };

    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(_) => return,
    };

    let token = uuid::Uuid::new_v4().to_string();
    let info = BridgeInfo {
        port,
        token: token.clone(),
        pid: std::process::id(),
    };

    let path = paths.data_dir.join(BRIDGE_FILE);
    if let Err(e) = write_bridge_file(&path, &info) {
        crate::core::logging::log_startup("bridge", &format!("could not write bridge file: {e}"));
        return;
    }
    crate::core::logging::log_startup("bridge", &format!("listening on 127.0.0.1:{port}"));

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let app = app.clone();
            let token = token.clone();
            std::thread::spawn(move || {
                let _ = serve(stream, &app, &token);
            });
        }
    });
}

fn write_bridge_file(path: &std::path::Path, info: &BridgeInfo) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(info)?;
    std::fs::write(path, json)?;
    restrict_permissions(path);
    Ok(())
}

/// The file carries a credential, so keep it readable by this user only.
#[cfg(unix)]
fn restrict_permissions(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &std::path::Path) {
    // Windows inherits the ACL of %APPDATA%, which is already user-private.
}

/// One request per line, one response per line.
fn serve(stream: TcpStream, app: &AppHandle, expected_token: &str) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);

    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(());
    }

    let response = match serde_json::from_str::<BridgeRequest>(&line) {
        Ok(request) if request.token == expected_token => {
            let reply = match serde_json::from_value::<ExtensionMessage>(request.message) {
                Ok(message) => handle(app, message),
                Err(e) => crate::features::ipc::native_messaging::NativeResponse::Error {
                    message: format!("invalid message: {e}"),
                },
            };

            let mut value = serde_json::to_value(reply)
                .unwrap_or_else(|_| serde_json::json!({ "type": "error", "message": "serialisation failed" }));
            // Echo the id back so the extension can match the reply to its call.
            if let (Some(id), Some(object)) = (request.request_id, value.as_object_mut()) {
                object.insert("requestId".into(), id);
            }
            value
        }
        Ok(_) => serde_json::json!({ "type": "error", "message": "invalid bridge token" }),
        Err(e) => serde_json::json!({ "type": "error", "message": format!("invalid request: {e}") }),
    };

    let _ = writeln!(writer, "{response}");
    let _ = writer.flush();
    Ok(())
}

/// Read the published coordinates. Used by the relay, which has no `AppHandle`.
pub fn read_info(paths: &AppPaths) -> Option<BridgeInfo> {
    let text = std::fs::read_to_string(paths.data_dir.join(BRIDGE_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}
