import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type {
  BinariesProgress,
  BinariesStatus,
  HistoryEntry,
  InfoResult,
  Item,
  Settings,
} from "./types";

// --- binaries ---------------------------------------------------------

export const binariesStatus = () => invoke<BinariesStatus>("binaries_status");
export const ensureBinaries = () => invoke<BinariesStatus>("ensure_binaries");
export const updateYtdlp = () => invoke<string>("update_ytdlp");

// --- info -------------------------------------------------------------

export const fetchInfo = (url: string, kind: string) =>
  invoke<InfoResult>("fetch_info", { url, kind });
export const cancelFetch = () => invoke("cancel_fetch");

// --- queue ------------------------------------------------------------

export interface EnqueuePayload {
  items: { videoId: string; title: string; url: string; index: number | null }[];
  dir: string;
  batchName: string;
  quality: string;
  audioOnly: boolean;
  numbering: boolean;
  scheduleStart?: string;
  scheduleEnd?: string;
}

export const enqueueBatch = (payload: EnqueuePayload) =>
  invoke<Item[]>("enqueue_batch", { payload });
export const getQueue = () => invoke<Item[]>("get_queue");
export const pauseItem = (id: string) => invoke("pause_item", { id });
export const resumeItem = (id: string) => invoke("resume_item", { id });
export const cancelItem = (id: string) => invoke("cancel_item", { id });
export const retryItem = (id: string) => invoke("retry_item", { id });
export const removeItem = (id: string) => invoke("remove_item", { id });
export const pauseAll = () => invoke("pause_all");
export const resumeAll = () => invoke("resume_all");
export const clearFinished = () => invoke("clear_finished");
export const openDownloadFolder = (id: string) =>
  invoke("open_download_folder", { id });

// --- settings & history -----------------------------------------------

export const getSettings = () => invoke<Settings>("get_settings");
export const setSettings = (settings: Settings) =>
  invoke("set_settings", { settings });
export const getHistory = () => invoke<HistoryEntry[]>("get_history");
export const clearHistory = () => invoke("clear_history");
export const revealPath = (path: string) => invoke("reveal_path", { path });

// --- import/export ------------------------------------------------------

export const exportData = (path: string) => invoke("export_data", { path });
export const importData = (path: string) => invoke("import_data", { path });

// --- scheduler ----------------------------------------------------------

export const updateItemSchedule = (
  id: string,
  scheduleStart: string | null,
  scheduleEnd: string | null,
) => invoke("update_item_schedule", { id, schedule_start: scheduleStart, schedule_end: scheduleEnd });

// --- events -----------------------------------------------------------

export function onItemUpdated(cb: (item: Item) => void): Promise<UnlistenFn> {
  return listen<Item>("item-updated", (e) => cb(e.payload));
}
export function onQueueChanged(cb: () => void): Promise<UnlistenFn> {
  return listen("queue-changed", () => cb());
}
export function onBinariesProgress(
  cb: (p: BinariesProgress) => void,
): Promise<UnlistenFn> {
  return listen<BinariesProgress>("binaries-progress", (e) => cb(e.payload));
}
export function onInfoProgress(
  cb: (p: { message: string }) => void,
): Promise<UnlistenFn> {
  return listen<{ message: string }>("info-progress", (e) => cb(e.payload));
}
export function onClipboardYoutube(
  cb: (url: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("clipboard-youtube", (e) => cb(e.payload));
}

// --- formatting helpers -----------------------------------------------

export function formatDuration(secs: number | null | undefined): string {
  if (secs == null || secs < 0 || Number.isNaN(secs)) return "—";
  const s = Math.round(secs);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const p = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${p(m)}:${p(sec)}` : `${m}:${p(sec)}`;
}

export function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}

export function formatSpeed(bps: number): string {
  if (!bps || bps <= 0) return "";
  return `${formatBytes(bps)}/s`;
}

export function formatEta(secs: number): string {
  if (secs == null || secs < 0 || Number.isNaN(secs)) return "";
  return formatDuration(secs);
}

export function formatDate(millis: number): string {
  const d = new Date(millis);
  return d.toLocaleString();
}
