# Vidaro

**دانلودر یوتیوب برای دسکتاپ** — ساخته‌شده با Tauri 2 (Rust) + React + TypeScript.

**A desktop YouTube downloader** built with Tauri 2 (Rust) + React + TypeScript.

![version](https://img.shields.io/badge/version-2.0.0-00a8ff)

---

## فارسی

### امکانات

- **تک ویدیو، پلی‌لیست یا کل کانال** — ابتدا لیست ویدیوها بدون دانلود نمایش داده می‌شود، سپس انتخاب و دانلود
- **شماره‌گذاری خودکار** — پلی‌لیست‌ها به ترتیب پلی‌لیست و کانال‌ها از قدیمی‌ترین به جدیدترین: `001 - نام ویدیو.mp4`
- **مدیریت صف کامل** — صف، دانلود هم‌زمان قابل تنظیم، Pause/Resume/Cancel/Retry برای هر آیتم، درصد/سرعت/حجم/زمان باقی‌مانده، تاریخچه
- **فرمت‌ها** — MP4 در کیفیت‌های 360p تا 4K، یا MP3 با کاور و متادیتا
- **ساختار پوشه‌ها** — `Downloads/YouTube Downloader/<نام کانال یا پلی‌لیست>/` برای دسته‌ها و `.../Single Videos/` برای تک‌ویدیوها
- **زمان‌بندی دو سطحی** — بازه سراسری در تنظیمات + زمان‌بندی اختصاصی برای هر آیتم
- **پروکسی** — خاموش / سیستم / دستی (HTTP، HTTPS، SOCKS5، SOCKS5h)
- **کلیپ‌بورد** — با کپی لینک یوتیوب، پیشنهاد دانلود نمایش داده می‌شود
- **سینی سیستم** — بستن پنجره برنامه را نمی‌بندد؛ در سینی می‌ماند
- **اجرای خودکار در پس‌زمینه** — با روشن شدن ویندوز، برنامه **مخفی** در سینی اجرا می‌شود و پنجره باز نمی‌شود
- **دوزبانه** — فارسی (RTL) و انگلیسی، تم تاریک/روشن/سیستمی
- **ابزارهای خودکار** — yt-dlp و ffmpeg در اولین اجرا دانلود می‌شوند (نصب‌کننده سبک می‌ماند)

### اجرای توسعه

```bash
npm install
npm run tauri dev
```

پیش‌نیازها: Node.js 20+، Rust toolchain (rustup)، و در ویندوز Visual Studio C++ Build Tools.

### بیلد

```bash
npm run tauri build
```

خروجی در `src-tauri/target/release/bundle/`:
- `msi/Vidaro_2.0.0_x64_en-US.msi`
- `nsis/Vidaro_2.0.0_x64-setup.exe`

### معماری

```
src-tauri/src/
├── lib.rs                 ترکیب‌بندی: state، پلاگین‌ها، فرمان‌ها، پنجره
├── core/                  زیرساخت مشترک
│   ├── config.rs          تنظیمات + ConfigStore (cache + ذخیره اتمیک)
│   ├── paths.rs           مسیرهای داده برنامه
│   ├── process.rs         اجرای فرآیندها بدون پنجره کنسول
│   ├── filename.rs        پاک‌سازی نام و جلوگیری از path traversal
│   ├── events.rs          نام رویدادهای مشترک
│   ├── error.rs           نوع خطای یکپارچه
│   └── plugin.rs          trait پلاگین + Registry
└── features/              پلاگین‌های مستقل
    ├── tools/             yt-dlp و ffmpeg (مسیر، نصب، به‌روزرسانی)
    ├── metadata/          لیست ویدیو/پلی‌لیست/کانال بدون دانلود
    ├── download/          صف، Gate، اجرا، پارس پیشرفت، زمان‌بندی
    ├── history/           تاریخچه
    ├── settings/          فرمان‌های تنظیمات + Import/Export
    ├── system/            سینی، کلیپ‌بورد، autostart
    └── ipc/               پل Native Messaging برای افزونه مرورگر
```

هر پلاگین سه چیز را خودش مدیریت می‌کند: state، فرمان‌های Tauri، و هوک راه‌اندازی.
برای افزودن قابلیت جدید فقط یک ماژول در `features/` بسازید و در `lib.rs` ثبت کنید.

```
src/
├── App.tsx                پوسته برنامه
├── main.tsx               ورودی React
├── lib/                   api، types، i18n، format، app-context
├── components/            Sidebar، Topbar، StatusBar، Toast، Overlay
├── views/                 NewDownload، Queue، History، Settings
└── styles/app.css
```

---

## English

### Features

- **Single video, playlist or whole channel** — preview the list first, then pick what to download
- **Automatic numbering** — playlist order, or oldest → newest for channels: `001 - Video Name.mp4`
- **Full download manager** — queue, configurable concurrency, per-item pause/resume/cancel/retry, speed/size/ETA, history
- **Formats** — MP4 from 360p to 4K, or MP3 with cover art and metadata
- **Folder layout** — `Downloads/YouTube Downloader/<playlist or channel>/` for batches, `.../Single Videos/` for one-offs
- **Two-level scheduler** — a global time window plus per-item schedules
- **Proxy** — off / system / manual (HTTP, HTTPS, SOCKS5, SOCKS5h)
- **Clipboard detection** — copying a YouTube link offers a one-click download
- **System tray** — closing the window keeps the app running
- **Hidden autostart** — Vidaro starts in the tray at boot without opening a window
- **Bilingual UI** — Persian (RTL) and English, dark/light/system themes
- **Self-provisioning tools** — yt-dlp and ffmpeg are fetched on first run, keeping the installer small

### Development

```bash
npm install
npm run tauri dev
```

Requires Node.js 20+, a Rust toolchain (rustup), and Visual Studio C++ Build Tools on Windows.

### Build

```bash
npm run tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`.

### Architecture

Each feature is a self-contained plugin: it owns its state, registers its own
Tauri commands, and runs its own startup hook. `lib.rs` only wires them
together, so adding a capability never means editing unrelated modules.

### Notes

- yt-dlp and ffmpeg are **not** bundled in the installer. They are downloaded
  into the app data folder on first run (or from Settings → Download tools).
- On networks where YouTube is filtered, configure a proxy in Settings.
- Release builds use `lto = false` because thin-LTO breaks linking of the
  `ring` crate on MSVC.
