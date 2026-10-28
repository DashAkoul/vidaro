//! Minimal plugin system.
//!
//! Each feature (downloads, metadata, tools, history, system integration, IPC)
//! is a self-contained plugin. A plugin owns its state, registers its own
//! Tauri commands, and runs its own startup hook. `lib.rs` only wires them
//! together, so adding a feature never means editing unrelated modules.

use std::sync::Arc;

use tauri::AppHandle;

use crate::core::config::ConfigStore;
use crate::core::error::AppResult;
use crate::core::paths::AppPaths;

/// Everything a plugin needs at startup.
#[derive(Clone)]
pub struct AppContext {
    pub app: AppHandle,
    pub paths: AppPaths,
    pub config: Arc<ConfigStore>,
}

impl AppContext {
    pub fn settings(&self) -> crate::core::config::Settings {
        self.config.get()
    }
}

/// A feature module.
pub trait Plugin: Send + Sync + 'static {
    /// Stable identifier, used for logging.
    fn name(&self) -> &'static str;

    /// Called once during `setup`, after the managed state exists.
    fn startup(&self, _ctx: &AppContext) -> AppResult<()> {
        Ok(())
    }
}

/// Ordered collection of plugins.
#[derive(Default)]
pub struct Registry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(mut self, plugin: impl Plugin) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Run every startup hook. Failures are logged, never fatal — a broken
    /// optional feature must not stop the app from opening.
    pub fn startup(&self, ctx: &AppContext) {
        for plugin in &self.plugins {
            if let Err(e) = plugin.startup(ctx) {
                eprintln!("[plugin:{}] startup failed: {e}", plugin.name());
            }
        }
    }
}
