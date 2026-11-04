//! Browser-extension support: message handling, the loopback bridge, the
//! headless relay and native-host registration.

pub mod bridge;
pub mod host;
pub mod native_messaging;
pub mod registration;

pub use native_messaging::IpcPlugin;
