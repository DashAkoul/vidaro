//! Registers the native-messaging host with every browser that supports it.
//!
//! Browsers find a host through a JSON manifest whose location is fixed per
//! browser: a registry key on Windows, a well-known directory elsewhere. The
//! manifest points at this executable; Chrome launches it with the extension
//! origin as its first argument, which is how `main` knows to run in relay mode.
//!
//! Registration is idempotent and runs on every start, so moving the app or
//! updating the extension id never leaves a stale entry behind.

use std::path::Path;

#[cfg(not(windows))]
use std::path::PathBuf;

use crate::core::paths::AppPaths;

/// Must match `EXTENSION_HOST` in the extension's background script.
pub const HOST_NAME: &str = "com.vidaro.downloader";

/// Fixed by the `key` field in the Chrome manifest, so the id is stable no
/// matter where the unpacked extension lives.
pub const CHROME_EXTENSION_ID: &str = "noaejgifefjnfjjedhghmejpnfmmmkfb";

/// Firefox identifies extensions by the id in `browser_specific_settings`.
pub const FIREFOX_EXTENSION_ID: &str = "vidaro@dashakoul.dev";

/// Browsers that read the Chromium registry layout / directories.
const CHROMIUM_VENDORS: &[&str] = &[
    r"Google\Chrome",
    "Chromium",
    r"Microsoft\Edge",
    r"BraveSoftware\Brave-Browser",
    "Vivaldi",
    r"Opera Software\Opera Stable",
];

/// Write the manifest and point every installed browser at it.
pub fn ensure(paths: &AppPaths) {
    let Some(exe) = std::env::current_exe().ok() else {
        return;
    };

    let manifest_dir = paths.data_dir.join("native-host");
    if std::fs::create_dir_all(&manifest_dir).is_err() {
        return;
    }
    let manifest_path = manifest_dir.join(format!("{HOST_NAME}.json"));

    let manifest = serde_json::json!({
        "name": HOST_NAME,
        "description": "Vidaro native messaging host",
        "path": exe.to_string_lossy(),
        "type": "stdio",
        // Chromium reads allowed_origins, Firefox reads allowed_extensions;
        // each ignores the key it does not use.
        "allowed_origins": [format!("chrome-extension://{CHROME_EXTENSION_ID}/")],
        "allowed_extensions": [FIREFOX_EXTENSION_ID],
    });

    let Ok(json) = serde_json::to_string_pretty(&manifest) else {
        return;
    };
    if std::fs::write(&manifest_path, json).is_err() {
        crate::core::logging::log_startup("native-host", "could not write the host manifest");
        return;
    }

    let registered = register(&manifest_path);
    crate::core::logging::log_startup(
        "native-host",
        &format!("registered for {registered} browser(s) at {}", manifest_path.display()),
    );
}

#[cfg(windows)]
fn register(manifest: &Path) -> usize {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let value = manifest.to_string_lossy().to_string();
    let mut count = 0;

    for vendor in CHROMIUM_VENDORS {
        let key_path = format!(r"Software\{vendor}\NativeMessagingHosts\{HOST_NAME}");
        if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(&key_path) {
            if key.0.set_value("", &value).is_ok() {
                count += 1;
            }
        }
    }

    // Firefox uses its own root, not a Chromium vendor path.
    let firefox = format!(r"Software\Mozilla\NativeMessagingHosts\{HOST_NAME}");
    if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(&firefox) {
        if key.0.set_value("", &value).is_ok() {
            count += 1;
        }
    }

    count
}

#[cfg(target_os = "macos")]
fn register(manifest: &Path) -> usize {
    let Some(home) = std::env::var_os("HOME") else {
        return 0;
    };
    let base = PathBuf::from(home).join("Library/Application Support");
    let dirs = [
        base.join("Google/Chrome/NativeMessagingHosts"),
        base.join("Chromium/NativeMessagingHosts"),
        base.join("Microsoft Edge/NativeMessagingHosts"),
        base.join("BraveSoftware/Brave-Browser/NativeMessagingHosts"),
        base.join("Mozilla/NativeMessagingHosts"),
    ];
    link_into(&dirs, manifest)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn register(manifest: &Path) -> usize {
    let Some(home) = std::env::var_os("HOME") else {
        return 0;
    };
    let home = PathBuf::from(home);
    let dirs = [
        home.join(".config/google-chrome/NativeMessagingHosts"),
        home.join(".config/chromium/NativeMessagingHosts"),
        home.join(".config/microsoft-edge/NativeMessagingHosts"),
        home.join(".config/BraveSoftware/Brave-Browser/NativeMessagingHosts"),
        home.join(".mozilla/native-messaging-hosts"),
    ];
    link_into(&dirs, manifest)
}

/// Copy the manifest into each browser directory that already exists.
#[cfg(not(windows))]
fn link_into(dirs: &[PathBuf], manifest: &Path) -> usize {
    let Ok(contents) = std::fs::read(manifest) else {
        return 0;
    };
    let file_name = manifest
        .file_name()
        .map(|n| n.to_owned())
        .unwrap_or_else(|| std::ffi::OsString::from(format!("{HOST_NAME}.json")));

    let mut count = 0;
    for dir in dirs {
        // Only register for browsers the user actually has.
        let Some(parent) = dir.parent() else { continue };
        if !parent.exists() {
            continue;
        }
        if std::fs::create_dir_all(dir).is_err() {
            continue;
        }
        if std::fs::write(dir.join(&file_name), &contents).is_ok() {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_carries_both_permission_models() {
        let manifest = serde_json::json!({
            "name": HOST_NAME,
            "path": "/tmp/vidaro",
            "type": "stdio",
            "allowed_origins": [format!("chrome-extension://{CHROME_EXTENSION_ID}/")],
            "allowed_extensions": [FIREFOX_EXTENSION_ID],
        });

        // Chromium rejects a host whose origin list does not contain the
        // calling extension.
        let origins = manifest["allowed_origins"].as_array().unwrap();
        assert_eq!(origins.len(), 1);
        assert!(origins[0].as_str().unwrap().ends_with(&format!("{CHROME_EXTENSION_ID}/")));

        // Firefox rejects a host without the extension id.
        let extensions = manifest["allowed_extensions"].as_array().unwrap();
        assert_eq!(extensions[0].as_str(), Some(FIREFOX_EXTENSION_ID));
    }

    #[test]
    fn chrome_extension_id_is_a_valid_id() {
        // Chrome ids are 32 characters from a-p.
        assert_eq!(CHROME_EXTENSION_ID.len(), 32);
        assert!(CHROME_EXTENSION_ID.chars().all(|c| ('a'..='p').contains(&c)));
    }
}
