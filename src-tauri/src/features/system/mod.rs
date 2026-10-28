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

        tray::build(&ctx.app)
            .map_err(|e| crate::core::error::AppError::Tool(e.to_string()))?;

        if settings.clipboard_watch {
            clipboard::spawn(ctx.app.clone());
        }

        autostart::reconcile(&ctx.app, settings.autostart_enabled);

        Ok(())
    }
}
