//! Login-item registration.
//!
//! Owned by Vidaro rather than delegated to `tauri-plugin-autostart`, because
//! the entry must always carry `--hidden`: booting the machine has to start the
//! app silently in the tray, never with a window. Owning the code also means a
//! stale entry written by an older version gets rewritten instead of silently
//! kept.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::core::error::{AppError, AppResult};

/// Argument that tells the app to stay in the tray.
pub const HIDDEN_FLAG: &str = "--hidden";

/// Reverse-DNS label used for the macOS LaunchAgent and the Linux desktop entry.
#[cfg(not(windows))]
const LABEL: &str = "com.akoul.yt-downloader";

fn current_exe() -> AppResult<PathBuf> {
    std::env::current_exe()
        .map_err(|e| AppError::Io(e))
}

/* ------------------------------------------------------------- windows -- */

#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "Vidaro";

    fn run_key() -> AppResult<RegKey> {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE)
            .map_err(|e| AppError::Tool(format!("cannot open the Run key: {e}")))
    }

    pub fn is_enabled() -> bool {
        run_key()
            .and_then(|key| {
                key.get_value::<String, _>(VALUE_NAME)
                    .map_err(|e| AppError::Tool(e.to_string()))
            })
            .is_ok()
    }

    pub fn set_enabled(enabled: bool) -> AppResult<()> {
        let key = run_key()?;

        if enabled {
            let exe = current_exe()?;
            // Quoted so paths containing spaces survive; the flag keeps the
            // window hidden at login.
            let command = format!("\"{}\" {}", exe.display(), super::HIDDEN_FLAG);
            key.set_value(VALUE_NAME, &command)
                .map_err(|e| AppError::Tool(format!("cannot write the Run key: {e}")))?;
        } else {
            // Missing value is not an error.
            let _ = key.delete_value(VALUE_NAME);
        }
        Ok(())
    }
}

/* --------------------------------------------------------------- macos -- */

#[cfg(target_os = "macos")]
mod imp {
    use super::*;

    fn plist_path() -> AppResult<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| AppError::NotFound("HOME is not set".into()))?;
        Ok(PathBuf::from(home)
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist")))
    }

    pub fn is_enabled() -> bool {
        plist_path().map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn set_enabled(enabled: bool) -> AppResult<()> {
        let path = plist_path()?;

        if !enabled {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        }

        let exe = current_exe()?;
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>{HIDDEN_FLAG}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"#,
            exe = exe.display(),
            HIDDEN_FLAG = super::HIDDEN_FLAG
        );

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, plist)?;
        Ok(())
    }
}

/* --------------------------------------------------------------- linux -- */

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::*;

    fn desktop_path() -> AppResult<PathBuf> {
        let config = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map_err(|_| AppError::NotFound("no config directory".into()))?;
        Ok(config.join("autostart").join("vidaro.desktop"))
    }

    pub fn is_enabled() -> bool {
        desktop_path().map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn set_enabled(enabled: bool) -> AppResult<()> {
        let path = desktop_path()?;

        if !enabled {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        }

        let exe = current_exe()?;
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName=Vidaro\nComment=YouTube downloader\nExec=\"{}\" {}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            exe.display(),
            super::HIDDEN_FLAG
        );

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, entry)?;
        Ok(())
    }
}

/* ----------------------------------------------------------- interface -- */

pub fn is_enabled() -> bool {
    imp::is_enabled()
}

pub fn set_enabled(enabled: bool) -> AppResult<()> {
    imp::set_enabled(enabled)
}

/// Make the login item match the stored preference.
///
/// Enabling always rewrites the entry so that a command line left behind by an
/// older version (without `--hidden`) is corrected on the next launch.
pub fn reconcile(_app: &AppHandle, enabled: bool) {
    if enabled {
        if let Err(e) = set_enabled(true) {
            eprintln!("[autostart] could not register the login item: {e}");
        }
        return;
    }

    if is_enabled() {
        if let Err(e) = set_enabled(false) {
            eprintln!("[autostart] could not remove the login item: {e}");
        }
    }
}
