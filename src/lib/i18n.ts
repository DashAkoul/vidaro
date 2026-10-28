import { createContext, useContext } from "react";

export type Lang = "fa" | "en";
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
  language: "Language",
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

const fa: Record<keyof typeof en, string> = {
  appName: "Vidaro",
  tabNew: "دانلود جدید",
  tabQueue: "همه دانلودها",
  tabDownloading: "در حال دانلود",
  tabCompleted: "کامل‌شده",
  tabPaused: "متوقف",
  tabFailed: "ناموفق",
  tabHistory: "تاریخچه",
  tabSettings: "تنظیمات",

  urlPlaceholder: "لینک ویدیو، پلی‌لیست یا کانال یوتیوب را بچسبانید…",
  fetchBtn: "دریافت اطلاعات",
  fetching: "در حال دریافت…",
  detected: "نوع تشخیص‌داده‌شده",
  typeVideo: "تک ویدیو",
  typePlaylist: "پلی‌لیست",
  typeChannel: "کانال",
  kindAuto: "خودکار",

  selectAll: "انتخاب همه",
  selectNone: "هیچ‌کدام",
  selected: "{n} ویدیو انتخاب شده",
  videosCount: "{n} ویدیو",
  colIndex: "#",
  colTitle: "عنوان",
  colDuration: "مدت",
  truncated: "فقط ۵۰۰۰ ویدیوی اول نمایش داده می‌شود.",
  by: "سازنده",

  format: "فرمت",
  videoFormat: "ویدیو (MP4)",
  audioFormat: "فقط صوت (MP3)",
  quality: "کیفیت",
  best: "بهترین کیفیت",
  destination: "پوشه مقصد",
  chooseFolder: "انتخاب پوشه",
  addToQueue: "افزودن به صف ({n})",
  numberingNote:
    "ویدیوها به ترتیب پلی‌لیست (یا از قدیمی‌ترین ویدیوی کانال به جدیدترین) شماره‌گذاری می‌شوند: «001 - نام ویدیو».",
  chooseFolderFirst: "ابتدا پوشه مقصد را انتخاب کنید.",
  noVideos: "ویدیویی یافت نشد.",
  search: "جستجو…",

  queueEmpty: "هیچ دانلودی نیست.",
  queueHint: "با دکمه «+ دانلود جدید» شروع کنید یا فقط لینک یوتیوب را کپی کنید.",
  singleVideosGroup: "تک‌ویدیوها",
  pauseAll: "توقف همه",
  resumeAll: "ادامه همه",
  clearFinished: "پاک‌سازی تمام‌شده‌ها",
  remove: "حذف",
  retry: "تلاش مجدد",
  openFolder: "نمایش در پوشه",
  pause: "توقف",
  resume: "ادامه",
  cancel: "لغو",

  statusQueued: "در صف",
  statusActive: "در حال دانلود",
  statusPaused: "متوقف",
  statusCompleted: "کامل شد",
  statusError: "خطا",
  statusCancelled: "لغو شد",
  statusScheduled: "زمان‌بندی شده",
  stageDownload: "دانلود",
  stageMerge: "ادغام ویدیو و صدا",
  stageAudio: "تبدیل به MP3",
  stageDone: "تمام",

  historyEmpty: "تاریخچه خالی است.",
  clearHistory: "پاک‌کردن تاریخچه",
  size: "حجم",
  date: "تاریخ",

  settingsGeneral: "عمومی",
  language: "زبان",
  theme: "تم",
  themeSystem: "سیستمی",
  dark: "تاریک",
  light: "روشن",
  defaultFolder: "پوشه پیش‌فرض دانلود",
  maxConcurrent: "تعداد دانلود هم‌زمان",
  rateLimit: "محدودیت سرعت (مثلاً 5M — خالی = بدون محدودیت)",
  subs: "دانلود زیرنویس در صورت وجود (فقط ویدیو)",
  subLangs: "زبان‌های زیرنویس (با کاما جدا کنید)",
  embedMetadata: "افزودن اطلاعات به فایل صوتی",
  embedThumbnail: "افزودن تصویر کاور به فایل صوتی",
  save: "ذخیره",
  saved: "ذخیره شد",

  behavior: "رفتار برنامه",
  closeToTray: "با بستن پنجره، برنامه در سینی سیستم باز بماند",
  clipboardWatch: "تشخیص لینک یوتیوب کپی‌شده در کلیپ‌بورد",
  notifyOnClipboard: "نمایش اعلان سیستم وقتی پنجره بسته است",
  autostartEnabled: "اجرا با روشن شدن سیستم (در پس‌زمینه و مخفی)",
  autostartHint:
    "با روشن شدن کامپیوتر، Vidaro بی‌صدا در سینی سیستم اجرا می‌شود. برای باز کردن، روی آیکون سینی کلیک کنید.",

  tools: "ابزارهای دانلود",
  version: "نسخه",
  missing: "نصب نیست",
  installed: "آماده",
  updateYtdlp: "به‌روزرسانی yt-dlp",
  updating: "در حال به‌روزرسانی…",
  updateOutput: "خروجی به‌روزرسانی",
  openToolsFolder: "پوشه ابزارها",

  proxy: "پروکسی",
  proxyMode: "حالت پروکسی",
  proxyOff: "خاموش",
  proxySystem: "سیستم",
  proxyManual: "دستی",
  proxyHost: "آدرس",
  proxyPort: "پورت",
  proxyNote: "پشتیبانی از HTTP، HTTPS، SOCKS5 و SOCKS5h.",

  scheduler: "زمان‌بندی",
  schedulerEnabled: "فعال‌سازی زمان‌بندی سراسری",
  schedulerStart: "شروع",
  schedulerEnd: "پایان",
  schedulerNote:
    "دانلودها فقط در این بازه انجام می‌شوند. آیتم‌های دارای زمان‌بندی جداگانه مستثنی هستند.",
  perItemSchedule: "زمان‌بندی این دانلود",
  scheduleStart: "شروع",
  scheduleEnd: "پایان",
  clearSchedule: "حذف زمان‌بندی",

  importExport: "ورود / صدور داده‌ها",
  exportData: "صادرکردن",
  importData: "ورود",
  exportSuccess: "پشتیبان با موفقیت ذخیره شد",
  importSuccess: "پشتیبان با موفقیت وارد شد",
  importError: "خطا در ورود داده‌ها: {error}",

  toolsMissingTitle: "ابزارهای دانلود نصب نیستند",
  toolsMissingDesc:
    "برای دانلود ویدیو، Vidaro به yt-dlp و ffmpeg نیاز دارد. این ابزارها یک‌بار دانلود و در پوشه داده برنامه ذخیره می‌شوند.",
  installTools: "نصب ابزارها",
  retryInstall: "تلاش مجدد",
  installing: "در حال نصب {tool}…",

  fetchFailed: "دریافت اطلاعات ناموفق بود:",
  noSelection: "ابتدا ویدیو(ها) را انتخاب کنید.",
  eta: "باقی‌مانده {v}",
  error: "خطا",
  of: "از",

  totalSpeed: "سرعت کل",
  activeDownloads: "دانلود فعال",

  clipboardToast: "لینک یوتیوب پیدا شد",
  clipboardToastHint: "به صف دانلود اضافه شود؟",
  downloadNow: "دانلود",
  openApp: "باز کردن برنامه",
  dismiss: "نادیده",
};

export const dict = { en, fa };
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
  lang: Lang;
  dir: "rtl" | "ltr";
  t: (key: TKey, vars?: Record<string, string | number>) => string;
}

export const I18nContext = createContext<I18n>({
  lang: "en",
  dir: "ltr",
  t: (key) => dict.en[key],
});

export function useI18n(): I18n {
  return useContext(I18nContext);
}
