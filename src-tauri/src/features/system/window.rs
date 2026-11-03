//! Window helpers shared by the tray, the single-instance handler and
//! notifications.

use tauri::{AppHandle, Manager as _};

pub fn main_window(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    app.get_webview_window("main")
}

/// Bring the window to the foreground, creating focus if the app was hidden.
///
/// On Windows a window created with `visible: false` can end up shown but
/// *minimized* (the shell reports it at the classic off-screen restore
/// position), which looks exactly like "the app did not start". Showing,
/// un-minimising and focusing must therefore happen in this order, every
/// time, on every path that reveals the window.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        crate::core::logging::log_startup(
            "window",
            &format!(
                "show_main_window: visible={} minimized={}",
                window.is_visible().unwrap_or(false),
                window.is_minimized().unwrap_or(false),
            ),
        );
    }
}

pub fn is_window_visible(app: &AppHandle) -> bool {
    main_window(app).map(|w| window_effectively_visible(&w)).unwrap_or(false)
}

/// A minimized window still reports `is_visible() == true`, but the user
/// cannot see it. Treat minimized as hidden for every toggle decision.
fn window_effectively_visible(window: &tauri::WebviewWindow) -> bool {
    window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
}

/// Toggle visibility — used by the tray icon's left click.
pub fn toggle_main_window(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        return;
    };
    if window_effectively_visible(&window) {
        let _ = window.hide();
    } else {
        show_main_window(app);
    }
}
