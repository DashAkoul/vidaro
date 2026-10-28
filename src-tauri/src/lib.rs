mod binaries;
mod download;
mod history;
mod info;
mod settings;
mod tray;
mod util;
mod native_messaging;

use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // When a second instance is launched, show and focus the main window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .manage(download::Manager::new(2))
        .setup(|app| {
            // Initialize caches at startup
            crate::settings::init_settings_cache(app.handle());
            crate::binaries::init_binaries_paths(app.handle());
            
            // Setup autostart (run in background on Windows startup)
            let autostart_manager = app.autolaunch();
            #[cfg(target_os = "windows")]
            {
                let _ = autostart_manager.enable();
            }
            
            // Start background services
            download::restore_queue(app.handle());
            download::start_global_scheduler(app.handle().clone());
            tray::setup(app.handle());
            
            // Start clipboard watcher for YouTube URLs
            tray::start_clipboard_watcher(app.handle());
            
            // Initialize native messaging for browser extensions
            native_messaging::start_server(app.handle().clone());
            
            // Ensure main window is visible and focused
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            // binaries
            binaries::binaries_status,
            binaries::ensure_binaries,
            binaries::update_ytdlp,
            // info
            info::fetch_info,
            info::cancel_fetch,
            // download manager
            download::enqueue_batch,
            download::get_queue,
            download::pause_item,
            download::resume_item,
            download::cancel_item,
            download::retry_item,
            download::remove_item,
            download::pause_all,
            download::resume_all,
            download::clear_finished,
            download::open_download_folder,
            download::update_item_schedule,
            // settings
            settings::get_settings,
            settings::set_settings,
            settings::set_autostart,
            settings::export_data,
            settings::import_data,
            // history
            history::get_history,
            history::clear_history,
            history::reveal_path,
            // native messaging for browser extensions
            native_messaging::handle_extension_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}