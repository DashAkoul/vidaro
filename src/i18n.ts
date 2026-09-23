import { createContext, useContext } from "react";

export type Lang = "fa" | "en";
export type ThemeMode = "system" | "light" | "dark";

export const dict = {
  fa: {
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
    detected: "نوع تشخیص‌شده",
    typeVideo: "تک ویدیو",
    typePlaylist: "پلی‌لیست",
    typeChannel: "کانال",

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
      "ویدیوها به ترتیب پلی‌لیست (یا از قدیمی‌ترین ویدیوی کانال به جدیدترین) با شماره ذخیره می‌شوند: «001 - نام ویدیو».",
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
    saved: "ذخیره شد ✓",

    tools: "ابزارهای دانلود",
    version: "نسخه",
    missing: "نصب نیست",
    installed: "آماده",
    updateYtdlp: "به‌روزرسانی yt-dlp",
    updating: "در حال به‌روزرسانی…",
    updateOutput: "خروجی به‌روزرسانی",

    // Proxy
    proxy: "پروکسی",
    proxyPlaceholder: "مثال: http://host:port یا socks5://host:port",

    // Scheduler
    scheduler: "زمان‌بندی",
    schedulerEnabled: "فعال‌سازی زمان‌بندی سراسری",
    schedulerStart: "شروع (مثال: 02:00)",
    schedulerEnd: "پایان (مثال: 06:00)",
    schedulerNote: "تنها در این بازه دانلودها انجام می‌شوند. آیتم‌های دارای زمان‌بندی جداگانه مستثنی هستند.",
    perItemSchedule: "زمان‌بندی این دانلود",
    scheduleStart: "شروع",
    scheduleEnd: "پایان",
    clearSchedule: "حذف زمان‌بندی",
    statusScheduled: "زمان‌بندی شده",

    // Import/Export
    importExport: "ورود/صادرکردن داده‌ها",
    exportData: "صادرکردن (Export)",
    importData: "ورود (Import)",
    exportSuccess: "داده‌ها با موفقیت صادر شدند",
    importSuccess: "داده‌ها با موفقیت وارد شدند",
    importError: "خطا در ورود داده‌ها: {error}",

    binariesTitle: "آماده‌سازی ابزارهای دانلود",
    binariesDesc:
      "در اولین اجرا، yt-dlp و ffmpeg دانلود و در کنار برنامه ذخیره می‌شوند. این کار فقط یک‌بار انجام می‌شود.",
    binariesReady: "همه ابزارها آماده است!",
    downloading: "در حال دانلود",
    extracting: "در حال استخراج",
    done: "انجام شد",
    errorTitle: "خطا",

    fetchFailed: "دریافت اطلاعات ناموفق بود:",
    noSelection: "ابتدا ویدیو(ها) را انتخاب کنید.",
    eta: "باقی‌مانده {v}",
    error: "خطا",
    of: "از",

    totalSpeed: "سرعت کل",
    activeDownloads: "دانلود فعال",
    clipboardFound: "لینک یوتیوب در کلیپ‌بورد!",
    downloadNow: "دانلود",
    openApp: "باز کردن برنامه",
    dismiss: "نادیده",
  },
  en: {
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
      "Videos are saved numbered in playlist order (or from the oldest to the newest channel video): “001 - Video Name”.",
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
    saved: "Saved ✓",

    tools: "Download tools",
    version: "Version",
    missing: "Missing",
    installed: "Ready",
    updateYtdlp: "Update yt-dlp",
    updating: "Updating…",
    updateOutput: "Update output",

    // Proxy
    proxy: "Proxy",
    proxyPlaceholder: "e.g. http://host:port or socks5://host:port",

    // Scheduler
    scheduler: "Scheduler",
    schedulerEnabled: "Enable global schedule",
    schedulerStart: "Start (e.g. 02:00)",
    schedulerEnd: "End (e.g. 06:00)",
    schedulerNote: "Downloads only run in this window. Items with per-item schedule are excluded.",
    perItemSchedule: "Schedule this download",
    scheduleStart: "Start",
    scheduleEnd: "End",
    clearSchedule: "Clear schedule",
    statusScheduled: "Scheduled",

    // Import/Export
    importExport: "Import/Export Data",
    exportData: "Export",
    importData: "Import",
    exportSuccess: "Data exported successfully",
    importSuccess: "Data imported successfully",
    importError: "Import failed: {error}",

    binariesTitle: "Preparing download tools",
    binariesDesc:
      "On first launch, yt-dlp and ffmpeg are downloaded and stored next to the app. This only happens once.",
    binariesReady: "All tools are ready!",
    downloading: "Downloading",
    extracting: "Extracting",
    done: "Done",
    errorTitle: "Error",

    fetchFailed: "Failed to fetch info:",
    noSelection: "Select video(s) first.",
    eta: "ETA {v}",
    error: "Error",
    of: "of",

    totalSpeed: "Total speed",
    activeDownloads: "Active",
    clipboardFound: "YouTube link detected!",
    downloadNow: "Download",
    openApp: "Open App",
    dismiss: "Dismiss",
  },
} as const;

export type TKey = keyof typeof dict.fa;

export function translate(lang: Lang, key: TKey, vars?: Record<string, string | number>): string {
  let s: string = dict[lang][key] ?? dict.en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replace(`{${k}}`, String(v));
    }
  }
  return s;
}

export interface I18n {
  lang: Lang;
  dir: "rtl" | "ltr";
  t: (key: TKey, vars?: Record<string, string | number>) => string;
}

export const I18nContext = createContext<I18n>({
  lang: "en",
  dir: "ltr",
  t: (k) => dict.en[k],
});

export function useI18n(): I18n {
  return useContext(I18nContext);
}
