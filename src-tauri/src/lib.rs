//! Vidaro — application composition root.
//!
//! Everything here is wiring: build managed state, register plugins, map the
//! command surface, and decide whether the window starts visible. Feature logic
//! lives in `features/`, shared infrastructure in `core/`.

mod core;
mod features;

use std::sync::Arc;

use tauri::{Manager as _, WindowEvent};

use crate::core::config::ConfigStore;
use crate::core::paths::AppPaths;
use crate::core::plugin::{AppContext, Registry};
use crate::features::download::queue::{self, Manager};
use crate::features::tools::ToolManager;

/// True when the OS autostart entry launched us: stay in the tray.
fn launched_hidden() -> bool {
    std::env::args().any(|arg| arg == "--hidden" || arg == "--autostart")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let start_hidden = launched_hidden();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            // Booting the machine must not pop a window open.
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            crate::features::system::window::show_main_window(app);
        }))
        .setup(move |app| {
            let handle = app.handle().clone();

            // ---- paths & configuration -----------------------------------
            let paths = AppPaths::resolve(&handle);
            paths
                .ensure_dirs()
                .map_err(|e| format!("could not create the application data directory: {e}"))?;

            let config = Arc::new(ConfigStore::load(paths.config_file.clone()));
            let settings = config.get();

            app.manage(paths.clone());
            app.manage(config.clone());

            // ---- queue ---------------------------------------------------
            let manager = Manager::new(settings.max_concurrent as usize);
            queue::load_from(&paths.queue_file, &manager.queue);
            app.manage(manager);

            // ---- helper binaries -----------------------------------------
            app.manage(ToolManager::new(&handle, paths.clone()));

            // ---- plugins -------------------------------------------------
            let registry = Registry::new()
                .register(features::tools::ToolsPlugin)
                .register(features::download::DownloadPlugin)
                .register(features::download::scheduler::SchedulerPlugin)
                .register(features::system::SystemPlugin)
                .register(features::ipc::IpcPlugin);

            let context = AppContext {
                app: handle.clone(),
                paths,
                config,
            };
            registry.startup(&context);

            // ---- window --------------------------------------------------
            // The window is created hidden (see tauri.conf.json) so a normal
            // launch does not flash a blank white frame before React mounts.
            if !start_hidden {
                crate::features::system::window::show_main_window(&handle);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = window
                    .app_handle()
                    .try_state::<ConfigStore>()
                    .map(|config| config.get().close_to_tray)
                    .unwrap_or(true);

                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // tools
            features::tools::commands::binaries_status,
            features::tools::commands::ensure_binaries,
            features::tools::commands::update_ytdlp,
            features::tools::commands::open_tools_folder,
            // metadata
            features::metadata::commands::fetch_info,
            features::metadata::commands::cancel_fetch,
            // download queue
            features::download::commands::enqueue_batch,
            features::download::commands::get_queue,
            features::download::commands::pause_item,
            features::download::commands::resume_item,
            features::download::commands::cancel_item,
            features::download::commands::retry_item,
            features::download::commands::remove_item,
            features::download::commands::pause_all,
            features::download::commands::resume_all,
            features::download::commands::clear_finished,
            features::download::commands::open_download_folder,
            features::download::commands::update_item_schedule,
            // settings
            features::settings::app_info,
            features::settings::get_settings,
            features::settings::set_settings,
            features::settings::set_autostart,
            features::settings::export_data,
            features::settings::import_data,
            // history
            features::history::commands::get_history,
            features::history::commands::clear_history,
            features::history::commands::reveal_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Vidaro application");
}
