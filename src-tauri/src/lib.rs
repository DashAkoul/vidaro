mod binaries;
mod download;
mod history;
mod info;
mod settings;
mod tray;
mod util;

use tauri::{Manager, image::Image};
use tauri_plugin_autostart::ManagerExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(download::Manager::new(2))
        .setup(|app| {
            // Set window icon AND ensure window is visible
            if let Some(window) = app.get_webview_window("main") {
                let icon_bytes = include_bytes!("../icons/128x128.png");
                let icon = Image::new_owned(icon_bytes.to_vec(), 128, 128);
                window.set_icon(icon).ok();
                // Ensure window is visible
                window.show().ok();
                window.set_focus().ok();
            }
            
            // Setup autostart
            let autostart_manager = app.autolaunch();
            #[cfg(target_os = "windows")]
            {
                let _ = autostart_manager.enable();
            }
            
            download::restore_queue(app.handle());
            download::start_global_scheduler(app.handle().clone());
            tray::setup(app.handle());
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
            // tray / clipboard
            tray::dismiss_clipboard_suggestion,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}