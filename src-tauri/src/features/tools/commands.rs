//! Tauri commands for the tools plugin.

use tauri::{AppHandle, State};

use crate::core::error::{AppError, AppResult};
use crate::features::tools::{ToolManager, ToolStatus};

#[tauri::command]
pub fn binaries_status(tools: State<'_, ToolManager>) -> ToolStatus {
    tools.status()
}

#[tauri::command]
pub async fn ensure_binaries(
    app: AppHandle,
    tools: State<'_, ToolManager>,
) -> AppResult<ToolStatus> {
    tools.ensure(&app).await
}

#[tauri::command]
pub async fn update_ytdlp(
    app: AppHandle,
    tools: State<'_, ToolManager>,
) -> AppResult<String> {
    tools.update_ytdlp(&app).await
}

/// Open the folder that holds yt-dlp / ffmpeg.
#[tauri::command]
pub fn open_tools_folder(tools: State<'_, ToolManager>) -> AppResult<()> {
    let dir = tools.bin_dir();
    if !dir.exists() {
        return Err(AppError::NotFound(dir.display().to_string()));
    }
    tauri_plugin_opener::open_path(&dir, None::<&str>).map_err(|e| AppError::Tool(e.to_string()))
}
