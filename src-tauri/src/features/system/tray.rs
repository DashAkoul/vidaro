//! System tray icon and its menu.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager as _};

use crate::core::config::ConfigStore;
use crate::features::download::control;
use crate::features::download::model::{ItemStatus, PausedBy};
use crate::features::download::queue::Manager;
use crate::features::system::window;

/// Menu labels per language.
struct Labels {
    show: &'static str,
    pause_all: &'static str,
    resume_all: &'static str,
    quit: &'static str,
}

fn labels(language: &str) -> Labels {
    if language == "fa" {
        Labels {
            show: "نمایش Vidaro",
            pause_all: "توقف همه",
            resume_all: "ادامه همه",
            quit: "خروج",
        }
    } else {
        Labels {
            show: "Show Vidaro",
            pause_all: "Pause all",
            resume_all: "Resume all",
            quit: "Quit",
        }
    }
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let language = app
        .try_state::<ConfigStore>()
        .map(|c| c.get().language)
        .unwrap_or_else(|| "en".to_string());
    let text = labels(&language);

    let show = MenuItem::with_id(app, "show", text.show, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause_all", text.pause_all, true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume_all", text.resume_all, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", text.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &pause, &resume, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Vidaro")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => window::show_main_window(app),
            "pause_all" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(manager) = app.try_state::<Manager>() {
                        let queue = manager.queue.clone();
                        for inner in queue.all() {
                            if matches!(
                                inner.status(),
                                ItemStatus::Active | ItemStatus::Queued | ItemStatus::Scheduled
                            ) {
                                control::pause(&app, &queue, &inner, PausedBy::User).await;
                            }
                        }
                    }
                });
            }
            "resume_all" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(manager) = app.try_state::<Manager>() {
                        let queue = manager.queue.clone();
                        for inner in queue.all() {
                            if inner.status() == ItemStatus::Paused {
                                control::resume(&app, queue.clone(), inner);
                            }
                        }
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
