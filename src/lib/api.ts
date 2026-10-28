/**
 * Thin, typed wrapper around the Tauri command surface.
 *
 * Every backend command is mirrored here exactly once, so a rename in Rust
 * surfaces as a single edit on this side.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AppInfo,
  ClipboardLink,
  EnqueuePayload,
  HistoryEntry,
  InfoResult,
  Item,
  Settings,
  ToolsProgress,
  ToolStatus,
} from "./types";

/* ---------------------------------------------------------------- tools -- */

export const binariesStatus = () => invoke<ToolStatus>("binaries_status");
export const ensureBinaries = () => invoke<ToolStatus>("ensure_binaries");
export const updateYtdlp = () => invoke<string>("update_ytdlp");
export const openToolsFolder = () => invoke<void>("open_tools_folder");

/* ------------------------------------------------------------- metadata -- */

export const fetchInfo = (url: string, kind: string) =>
  invoke<InfoResult>("fetch_info", { url, kind });

export const cancelFetch = () => invoke<void>("cancel_fetch");

/* --------------------------------------------------------------- queue -- */

export const enqueueBatch = (payload: EnqueuePayload) =>
  invoke<Item[]>("enqueue_batch", { payload });

export const getQueue = () => invoke<Item[]>("get_queue");
export const pauseItem = (id: string) => invoke<void>("pause_item", { id });
export const resumeItem = (id: string) => invoke<void>("resume_item", { id });
export const cancelItem = (id: string) => invoke<void>("cancel_item", { id });
export const retryItem = (id: string) => invoke<void>("retry_item", { id });
export const removeItem = (id: string) => invoke<void>("remove_item", { id });
export const pauseAll = () => invoke<void>("pause_all");
export const resumeAll = () => invoke<void>("resume_all");
export const clearFinished = () => invoke<void>("clear_finished");
export const openDownloadFolder = (id: string) =>
  invoke<void>("open_download_folder", { id });

export const updateItemSchedule = (
  id: string,
  start: string | null,
  end: string | null,
) => invoke<void>("update_item_schedule", { payload: { id, start, end } });

/* ------------------------------------------------------ settings / info -- */

export const appInfo = () => invoke<AppInfo>("app_info");
export const getSettings = () => invoke<Settings>("get_settings");
export const setSettings = (settings: Settings) =>
  invoke<Settings>("set_settings", { settings });
export const setAutostart = (enabled: boolean) =>
  invoke<Settings>("set_autostart", { enabled });

export const exportData = (path: string) => invoke<void>("export_data", { path });
export const importData = (path: string) => invoke<void>("import_data", { path });

/* -------------------------------------------------------------- history -- */

export const getHistory = () => invoke<HistoryEntry[]>("get_history");
export const clearHistory = () => invoke<void>("clear_history");
export const revealPath = (path: string) => invoke<void>("reveal_path", { path });

/* --------------------------------------------------------------- events -- */

const subscribe = <T,>(event: string, handler: (payload: T) => void) =>
  listen<T>(event, (e) => handler(e.payload));

export const onItemUpdated = (cb: (item: Item) => void) =>
  subscribe<Item>("item-updated", cb);

export const onQueueChanged = (cb: () => void): Promise<UnlistenFn> =>
  listen("queue-changed", () => cb());

export const onHistoryChanged = (cb: () => void): Promise<UnlistenFn> =>
  listen("history-changed", () => cb());

export const onInfoProgress = (cb: (p: { message: string }) => void) =>
  subscribe<{ message: string }>("info-progress", cb);

export const onToolsProgress = (cb: (p: ToolsProgress) => void) =>
  subscribe<ToolsProgress>("tools-progress", cb);

export const onClipboardLink = (cb: (p: ClipboardLink) => void) =>
  subscribe<ClipboardLink>("clipboard-link", cb);

export const onSettingsChanged = (cb: (settings: Settings) => void) =>
  subscribe<Settings>("settings-changed", cb);

/** Tauri rejects with a plain string; normalise to an Error for callers. */
export function describeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
