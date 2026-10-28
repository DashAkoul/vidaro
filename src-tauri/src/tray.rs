use std::sync::OnceLock;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

/// Build the system-tray icon and hide-to-tray on window close.
pub fn setup(app: &AppHandle) {
    let show = MenuItem::with_id(app, "show", "Show Vidaro", true, None::<&str>).unwrap();
    let pause = MenuItem::with_id(app, "pause_all", "Pause all", true, None::<&str>).unwrap();
    let resume = MenuItem::with_id(app, "resume_all", "Resume all", true, None::<&str>).unwrap();
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).unwrap();
    let menu = Menu::with_items(app, &[&show, &pause, &resume, &quit]).unwrap();

    let app2 = app.clone();
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Vidaro")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            "pause_all" => {
                let a = app.clone();
                let inner = app.state::<crate::download::Manager>().inner.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::download::pause_all_inner(a, inner).await;
                });
            }
            "resume_all" => {
                let a = app.clone();
                let inner = app.state::<crate::download::Manager>().inner.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::download::resume_all_inner(a, inner).await;
                });
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, button_state: tauri::tray::MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.unminimize();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(&app2)
        .expect("failed to build tray icon");

    // hide to tray instead of exiting
    let main_window = app.get_webview_window("main");
    if let Some(w) = &main_window {
        let w2 = w.clone();
        w.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = w2.hide();
                api.prevent_close();
            }
        });
    }
}

/// Ignore the current clipboard contents (called when the user dismisses
/// the suggestion so it does not reappear).
#[allow(dead_code)]
#[tauri::command]
pub fn dismiss_clipboard_suggestion() {
    // nothing to do: LAST_URL already recorded, so it won't re-fire
}

/// Start clipboard watcher for YouTube URLs
pub fn start_clipboard_watcher(app: &AppHandle) {
    static LAST_URL: OnceLock<std::sync::Mutex<String>> = OnceLock::new();
    let mutex = LAST_URL.get_or_init(|| std::sync::Mutex::new(String::new()));

    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(1000));
            
            // Get clipboard content
            let clipboard_text = {
                let mut clipboard = match arboard::Clipboard::new() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                clipboard.get_text().unwrap_or_default()
            };
            
            if clipboard_text.is_empty() {
                continue;
            }
            
            // Check if it's a YouTube URL
            let url = clipboard_text.trim();
            if !crate::util::is_youtube_url(url) {
                continue;
            }
            
            // Check if we already processed this URL
            let mut last = mutex.lock().unwrap();
            if *last == url {
                continue;
            }
            *last = url.to_string();
            drop(last);
            
            // Emit event to frontend
            let _ = app.emit("clipboard-youtube", url);
        }
    });
}