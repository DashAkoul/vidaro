//! Desktop integration: tray icon, clipboard watcher, autostart.

pub mod autostart;
pub mod clipboard;
pub mod tray;
pub mod window;

use crate::core::plugin::{AppContext, Plugin};

pub struct SystemPlugin;

impl Plugin for SystemPlugin {
    fn name(&self) -> &'static str {
        "system"
    }

    fn startup(&self, ctx: &AppContext) -> crate::core::error::AppResult<()> {
        let settings = ctx.settings();

        // If the tray cannot be created while starting hidden, the window would
        // be unreachable — fall back to showing it.
        if let Err(e) = tray::build(&ctx.app) {
            eprintln!("[system] tray unavailable: {e}");
            window::show_main_window(&ctx.app);
        }

        if settings.clipboard_watch {
            clipboard::spawn(ctx.app.clone());
        }

        autostart::reconcile(&ctx.app, settings.autostart_enabled);

        Ok(())
    }
}
