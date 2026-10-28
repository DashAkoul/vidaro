export type ItemStatus =
  | "queued"
  | "active"
  | "paused"
  | "completed"
  | "error"
  | "cancelled"
  | "scheduled";

export type ItemStage = "download" | "merge" | "audio" | "done";

export type PausedBy = "user" | "scheduler";

export interface Item {
  id: string;
  url: string;
  videoId: string;
  title: string;
  index: number | null;
  dir: string;
  prefix: string;
  quality: string;
  audioOnly: boolean;
  batchName: string;
  status: ItemStatus;
  percent: number;
  speedBps: number;
  etaSecs: number;
  totalBytes: number;
  downloadedBytes: number;
  stage: ItemStage;
  error: string | null;
  outputPath: string | null;
  createdAt: number;
  scheduleStart: string | null;
  scheduleEnd: string | null;
  pausedBy: PausedBy | null;
}

export interface VideoEntry {
  index: number;
  id: string;
  title: string;
  duration: number | null;
  thumbnail: string | null;
  url: string;
}

export type InfoResult =
  | {
      kind: "video";
      id: string;
      title: string;
      duration: number | null;
      thumbnail: string | null;
      uploader: string | null;
      heights: number[];
    }
  | {
      kind: "collection";
      collType: "playlist" | "channel";
      id: string;
      title: string;
      uploader: string | null;
      thumbnail: string | null;
      entries: VideoEntry[];
      truncated: boolean;
    };

export interface Settings {
  language: "fa" | "en";
  theme: "system" | "light" | "dark";
  defaultDir: string;
  maxConcurrent: number;
  rateLimit: string;
  writeSubs: boolean;
  subLangs: string;
  embedMetadata: boolean;
  embedThumbnail: boolean;
  proxyMode: "off" | "system" | "manual";
  proxyHost: string;
  proxyPort: string;
  schedulerEnabled: boolean;
  schedulerStart: string;
  schedulerEnd: string;
  autostartEnabled: boolean;
  closeToTray: boolean;
  clipboardWatch: boolean;
  notifyOnClipboard: boolean;
}

export interface ToolStatus {
  ready: boolean;
  ytdlpFound: boolean;
  ytdlpVersion: string | null;
  ffmpegFound: boolean;
  ytdlpPath: string;
  ffmpegPath: string;
  platform: string;
}

export interface ToolsProgress {
  tool: string;
  stage: "download" | "extract" | "done";
  received: number;
  total: number;
}

export interface HistoryEntry {
  id: string;
  title: string;
  path: string;
  sizeBytes: number;
  finishedAt: number;
  kind: "video" | "audio";
  quality: string;
  batchName: string;
}

export interface AppInfo {
  name: string;
  version: string;
  platform: string;
  arch: string;
  startedHidden: boolean;
}

export interface ClipboardLink {
  url: string;
}

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
