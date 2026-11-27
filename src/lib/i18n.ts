import { createContext, useContext } from "react";

export type Lang = "en";
export type ThemeMode = "system" | "light" | "dark";

const en = {
  appName: "Vidaro",
  tabNew: "New Download",
  tabQueue: "All Downloads",
  tabDownloading: "Downloading",
  tabCompleted: "Completed",
  tabPaused: "Paused",
  tabFailed: "Failed",
  tabHistory: "History",
  tabSettings: "Settings",

  urlPlaceholder: "Paste a YouTube video, playlist or channel URL…",
  fetchBtn: "Fetch",
  quickDownload: "Download now",
  fetching: "Fetching…",
  detected: "Detected",
  typeVideo: "Single video",
  typePlaylist: "Playlist",
  typeChannel: "Channel",
  kindAuto: "Auto",

  selectAll: "Select all",
  selectNone: "None",
  selected: "{n} videos selected",
  videosCount: "{n} videos",
  colIndex: "#",
  colTitle: "Title",
  colDuration: "Duration",
  truncated: "Only the first 5000 videos are shown.",
  by: "by",

  format: "Format",
  videoFormat: "Video (MP4)",
  audioFormat: "Audio only (MP3)",
  quality: "Quality",
  best: "Best quality",
  destination: "Destination folder",
  chooseFolder: "Choose folder",
  addToQueue: "Add to queue ({n})",
  numberingNote:
    "Videos are numbered in playlist order (or oldest to newest for channels): “001 - Video Name”.",
  chooseFolderFirst: "Choose a destination folder first.",
  noVideos: "No videos found.",
  search: "Search…",

  queueEmpty: "No downloads here.",
  queueHint: "Start with “+ New Download” — or just copy a YouTube link.",
  singleVideosGroup: "Single videos",
  pauseAll: "Pause all",
  resumeAll: "Resume all",
  clearFinished: "Clear finished",
  remove: "Remove",
  retry: "Retry",
  openFolder: "Show in folder",
  pause: "Pause",
  resume: "Resume",
  cancel: "Cancel",
  copyLink: "Copy link",
  copyTitle: "Copy title",
  copyPath: "Copy file path",
  openFile: "Open file",
  downloadAgain: "Download again",
  copied: "Copied to clipboard",

  statusQueued: "Queued",
  statusActive: "Downloading",
  statusPaused: "Paused",
  statusCompleted: "Completed",
  statusError: "Error",
  statusCancelled: "Cancelled",
  statusScheduled: "Scheduled",
  stageDownload: "Downloading",
  stageMerge: "Merging video & audio",
  stageAudio: "Converting to MP3",
  stageDone: "Done",

  historyEmpty: "History is empty.",
  clearHistory: "Clear history",
  size: "Size",
  date: "Date",

  settingsGeneral: "General",
  settingsTabGeneral: "General",
  settingsTabYouTube: "YouTube account",
  settingsTabNetwork: "Network",
  settingsTabTools: "Tools & formats",
  settingsTabDownloads: "Downloads",
  settingsTabData: "Data",
  theme: "Theme",
  themeSystem: "System",
  dark: "Dark",
  light: "Light",
  defaultFolder: "Default download folder",
  maxConcurrent: "Parallel downloads",
  rateLimit: "Speed limit (e.g. 5M — empty = unlimited)",
  subs: "Download subtitles when available (video only)",
  subLangs: "Subtitle languages (comma separated)",
  embedMetadata: "Embed metadata into audio files",
  embedThumbnail: "Embed thumbnail into audio files",
  save: "Save",
  saved: "Saved",

  behavior: "Behaviour",
  closeToTray: "Closing the window keeps Vidaro running in the tray",
  clipboardWatch: "Detect YouTube links copied to the clipboard",
  notifyOnClipboard: "Show a system notification when the window is hidden",
  autostartEnabled: "Start with the system (runs hidden in the tray)",
  autostartHint:
    "Vidaro starts silently in the system tray when the computer boots. Click the tray icon to open it.",

  tools: "Download tools",
  version: "Version",
  missing: "Missing",
  installed: "Ready",
  updateYtdlp: "Update yt-dlp",
  jsRuntime: "JavaScript runtime",
  jsRuntimeMissing: "not found",
  updating: "Updating…",
  updateOutput: "Update output",
  openToolsFolder: "Open tools folder",

  proxy: "Proxy",
  proxyMode: "Proxy mode",
  proxyOff: "Off",
  proxySystem: "System",
  proxyManual: "Manual",
  proxyHost: "Host",
  proxyPort: "Port",
  proxyNote: "Supports HTTP, HTTPS, SOCKS5 and SOCKS5h.",

  account: "YouTube account",
  cookiesMode: "Send cookies",
  cookiesOff: "Off",
  cookiesBrowser: "From browser",
  cookiesFile: "cookies.txt",
  cookiesBrowserPick: "Import from browser",
  cookiesFilePick: "cookies.txt file",
  chooseCookies: "Choose file",
  savedSession: "Saved session",
  importSession: "Import session from browser",
  signInBrowser: "Sign in to YouTube",
  appBoundWarn:
    "Windows encrypts this browser's cookies with a key only the browser can use, so Vidaro cannot read them (yt-dlp cannot either). Sign in with Firefox instead, or paste a cookies.txt export.",
  pasteCookies: "Paste cookies.txt",
  pasteCookiesHint:
    "Works with any browser and any machine: export youtube.com cookies with a browser extension, open the file in a text editor and paste its contents here.",
  savePaste: "Save cookies",
  importingSession: "Importing…",
  sessionReady: "{n} signed-in cookies saved",
  sessionMissing: "not imported yet",
  sessionHint:
    "Import once and the browser is no longer needed: Vidaro keeps its own copy of the signed-in cookies. Re-import if downloads start failing again.",
  cookiesHint:
    "YouTube blocks downloaders with a “confirm you’re not a bot” check. Sending cookies from a browser where you are signed in is the most reliable fix. Nothing is uploaded: the cookies go straight to yt-dlp.",
  cookiesBrowserHint:
    "Firefox works most reliably. Chrome cookies are often encrypted by Windows and fail with a DPAPI error; in that case export a cookies.txt and pick it instead.",

  reliability: "Reliability",
  autoRetry: "Retry failed downloads automatically",
  autoRetryMax: "Maximum retries",
  retryDelay: "Base delay (seconds)",
  retryHint:
    "Bot checks and dropped connections clear up on their own. Transient failures are retried with a growing delay, including after a restart.",

  testConnection: "Test connection",
  testingConnection: "Testing…",
  testOk: "Connection OK",
  statusRetrying: "Retrying",
  retryFailed: "Retry failed",

  scheduler: "Scheduler",
  schedulerEnabled: "Enable global schedule",
  schedulerStart: "Start",
  schedulerEnd: "End",
  schedulerNote:
    "Downloads only run inside this window. Items with their own schedule are excluded.",
  perItemSchedule: "Schedule this download",
  scheduleStart: "Start",
  scheduleEnd: "End",
  clearSchedule: "Clear schedule",

  importExport: "Import / Export",
  exportData: "Export",
  importData: "Import",
  exportSuccess: "Backup exported",
  importSuccess: "Backup imported",
  importError: "Import failed: {error}",

  toolsMissingTitle: "Download tools are missing",
  toolsMissingDesc:
    "Vidaro needs yt-dlp and ffmpeg to download videos. They are fetched once and stored in your app data folder.",
  installTools: "Install tools",
  retryInstall: "Try again",
  installing: "Installing {tool}…",

  fetchFailed: "Failed to fetch info:",
  fetchFailedHint:
    "The video can still be queued: choose a quality below and press Add. Vidaro keeps retrying and finishes it as soon as YouTube lets the request through.",
  noSelection: "Select video(s) first.",
  eta: "ETA {v}",
  error: "Error",
  of: "of",

  totalSpeed: "Total speed",
  activeDownloads: "Active",

  clipboardToast: "YouTube link detected",
  clipboardToastHint: "Add it to the download queue?",
  downloadNow: "Download",
  openApp: "Open app",
  dismiss: "Dismiss",
} as const;

export const dict = { en };

export type TKey = keyof typeof en;

export function translate(
  lang: Lang,
  key: TKey,
  vars?: Record<string, string | number>,
): string {
  let text: string = dict[lang][key] ?? dict.en[key] ?? key;
  if (vars) {
    for (const [name, value] of Object.entries(vars)) {
      text = text.replace(`{${name}}`, String(value));
    }
  }
  return text;
}

export interface I18n {
  t: (key: TKey, vars?: Record<string, string | number>) => string;
}

export const I18nContext = createContext<I18n>({
  t: (key) => dict.en[key],
});

export function useI18n(): I18n {
  return useContext(I18nContext);
}
