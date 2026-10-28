//! Keeping the OS autostart entry in sync with the user's preference.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// The autostart entry is registered with `--hidden`, so booting the machine
/// starts Vidaro silently in the tray instead of popping a window open.
pub fn reconcile(app: &AppHandle, enabled: bool) {
    let manager = app.autolaunch();
    let currently_enabled = manager.is_enabled().unwrap_or(false);
    if enabled == currently_enabled {
        return;
    }

    let outcome = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };

    if let Err(e) = outcome {
        eprintln!("[autostart] could not update the login item: {e}");
    }
}
