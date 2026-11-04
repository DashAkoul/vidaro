//! Headless native-messaging relay.
//!
//! Browsers launch the native host themselves and speak the stdio protocol with
//! it. Vidaro is a single-instance GUI app, so a second process would either
//! open a second window or be terminated by the single-instance guard. This
//! module runs *before* Tauri is initialised: it creates no window, and simply
//! forwards each message to the running GUI process over the loopback bridge.
//!
//! When the GUI is not running the relay answers with a clear error, which the
//! extension surfaces as "open Vidaro".

use std::io::{Read, Write};
use std::net::TcpStream;

use crate::core::paths::AppPaths;
use crate::features::ipc::bridge;

/// Chrome's own limit is 1 MiB.
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

/// True when the process was started by a browser as its native host.
///
/// Chrome passes `chrome-extension://<id>/` as the first argument, Firefox
/// passes `moz-extension://<uuid>/`, and both keep stdin/stdout as pipes.
pub fn is_host_invocation(args: &[String]) -> bool {
    if args.iter().any(|a| a == "--native-messaging") {
        return true;
    }
    args.get(1)
        .map(|a| a.starts_with("chrome-extension://") || a.starts_with("moz-extension://"))
        .unwrap_or(false)
}

/// Read messages from the browser, relay them, write the answers back.
pub fn run() -> ! {
    let paths = AppPaths::standalone();
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

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

        let request = match String::from_utf8(payload) {
            Ok(text) => text,
            Err(_) => {
                write_message(&mut stdout, &error("message was not valid UTF-8"));
                continue;
            }
        };

        let response = forward(&paths, &request);
        write_message(&mut stdout, &response);
    }

    std::process::exit(0);
}

/// Send one request to the GUI and return its reply as raw JSON.
fn forward(paths: &AppPaths, request: &str) -> String {
    let Some(info) = bridge::read_info(paths) else {
        return error("Vidaro is not running. Open the Vidaro app and try again.");
    };

    let parsed: serde_json::Value = match serde_json::from_str(request) {
        Ok(value) => value,
        Err(e) => return error(&format!("invalid message: {e}")),
    };

    // The relay re-wraps the message so the GUI can validate the token, and
    // carries the extension's id along for the reply.
    let envelope = serde_json::json!({
        "token": info.token,
        "requestId": parsed.get("requestId").cloned(),
        "message": parsed,
    });

    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", info.port)) else {
        return error("Vidaro is not reachable. Restart the app and try again.");
    };
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(120)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(10)));

    let line = match serde_json::to_string(&envelope) {
        Ok(line) => line,
        Err(e) => return error(&format!("could not encode request: {e}")),
    };

    if writeln!(stream, "{line}").is_err() {
        return error("Vidaro closed the connection. Restart the app and try again.");
    }

    let mut reader = std::io::BufReader::new(stream);
    let mut reply = String::new();
    if std::io::BufRead::read_line(&mut reader, &mut reply).is_err() || reply.trim().is_empty() {
        return error("Vidaro did not answer the request.");
    }
    reply.trim().to_string()
}

fn error(message: &str) -> String {
    serde_json::json!({ "type": "error", "message": message }).to_string()
}

/// Chrome's framing: a 4-byte little-endian length followed by the JSON body.
fn write_message(stdout: &mut impl Write, json: &str) {
    let bytes = json.as_bytes();
    if bytes.len() > MAX_MESSAGE_BYTES {
        return;
    }
    let _ = stdout.write_all(&(bytes.len() as u32).to_le_bytes());
    let _ = stdout.write_all(bytes);
    let _ = stdout.flush();
}
