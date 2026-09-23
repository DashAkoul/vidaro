use arboard::Clipboard;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::util::is_youtube_url;

static CLIPBOARD_LOCK: AtomicBool = AtomicBool::new(false);
static LAST_URL: Mutex<Option<String>> = Mutex::new(None);

/// Build the system-tray icon, hide-to-tray on window close, and start
/// watching the clipboard for YouTube links.
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

    // clipboard watcher for YouTube links
    let app4 = app.clone();
    std::thread::spawn(move || {
        eprintln!("[clip] watcher thread started");
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[clip] clipboard init failed: {e}");
                return;
            }
        };
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            if CLIPBOARD_LOCK.load(Ordering::SeqCst) {
                continue;
            }
            let text = clipboard.get_text().unwrap_or_default();
            let text = text.trim();
            if text.len() < 15 || text.len() > 2048 || text.contains(char::is_whitespace) {
                continue;
            }
            eprintln!("[clip] candidate: {text}");
            if !is_youtube_url(text) {
                continue;
            }
            let mut last = LAST_URL.lock().unwrap();
            if last.as_deref() == Some(text) {
                continue; // already suggested
            }
            *last = Some(text.to_string());
            eprintln!("[clip] emitting suggestion: {text}");
            let _ = app4.emit("clipboard-youtube", text.to_string());
        }
    });
}

/// Ignore the current clipboard contents (called when the user dismisses
/// the suggestion so it does not reappear).
#[tauri::command]
pub fn dismiss_clipboard_suggestion() {
    // nothing to do: LAST_URL already recorded, so it won't re-fire
}
