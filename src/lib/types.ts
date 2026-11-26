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
  /** Automatic retries already spent on this item. */
  attempts?: number;
  /** When the next automatic retry is due (epoch ms). */
  retryAt?: number | null;
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

  autoRetry: boolean;
  autoRetryMax: number;
  retryDelaySecs: number;

  cookiesMode: "off" | "browser" | "file";
  cookiesBrowser: string;
  cookiesFile: string;
}

export interface ToolStatus {
  ready: boolean;
  ytdlpFound: boolean;
  ytdlpVersion: string | null;
  ffmpegFound: boolean;
  ytdlpPath: string;
  ffmpegPath: string;
  /** JavaScript runtime used for YouTube's player challenge, if any. */
  jsRuntime: string | null;
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

export interface ConnectionReport {
  ok: boolean;
  kind: "ok" | "botCheck" | "degraded" | "network" | "tools" | "unavailable" | "unknown";
  message: string;
  ytdlpVersion: string | null;
  ytdlpAgeDays: number | null;
  jsRuntime: string | null;
}

/** Recognition of a pasted link, available before any fetch. */
export interface Detection {
  kind: "video" | "playlist" | "channel";
  videoId: string | null;
}

/** Fast oEmbed preview shown while the full metadata is being fetched. */
export interface InfoPreview {
  videoId: string;
  title: string;
  thumbnail: string | null;
  uploader: string | null;
}

/** State of the browser-independent cookie jar. */
export interface SessionStatus {
  hasFile: boolean;
  authCookies: number;
  updatedAt: number | null;
  path: string;
}
