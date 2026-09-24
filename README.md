<div align="center">

# ▶️ Vidaro

### A modern, cross-platform YouTube downloader

**Tauri 2 · Rust · React · TypeScript**

<p>
  <a href="#english">🇬🇧 English</a>
  &nbsp;•&nbsp;
  <a href="#فارسی">🇮🇷 فارسی</a>
</p>

<p>
  <a href="https://github.com/DashAkoul/vidaro/releases/latest">⬇️ Download</a>
  &nbsp;•&nbsp;
  <a href="https://github.com/DashAkoul/vidaro/releases">Releases</a>
  &nbsp;•&nbsp;
  <a href="https://github.com/DashAkoul/vidaro/issues">Issues</a>
</p>

</div>

---

<!--
README MAINTENANCE
==================
Before publishing a new major/minor release, update:
1. Current version badge/title if you add one.
2. The "Latest release" section.
3. Download links / filenames if they changed.
4. Screenshot files under docs/screenshots/.
5. The "What's new" section.
6. The "Roadmap / Next version" section.
7. Architecture details if the source structure changes.

Suggested screenshot files:
- docs/screenshots/overview.png
- docs/screenshots/new-download.png
- docs/screenshots/queue.png
- docs/screenshots/history.png
- docs/screenshots/settings.png
- docs/screenshots/dark-mode.png
- docs/screenshots/rtl.png
- docs/screenshots/browser-extension.png   <!-- V1.0 -->
-->

# English

<a id="english"></a>

## 🎬 YouTube Downloader

**Vidaro** is a modern desktop YouTube downloader built with **Tauri 2, Rust, React and TypeScript**.

It supports downloading individual videos, playlists and channel video lists, with queue management, pause/resume/cancel/retry controls, scheduling, proxy support, subtitles, multiple formats and persistent download state.

Vidaro is designed to provide a clean, minimal desktop experience while keeping the full source code open on GitHub.

## ✨ Features

- **Single videos, playlists and channels** — inspect available videos before starting downloads.
- **Automatic numbering** — playlist items follow playlist order; channel downloads can be ordered from oldest to newest.
- **Download manager** — queue management, configurable concurrent downloads, pause/resume/cancel/retry, progress, speed, size and remaining-time information.
- **Multiple formats** — MP4 video from 360p up to 4K when available, plus MP3 audio extraction with cover art and metadata.
- **Organized folders** — configurable download locations with separate folders for grouped downloads and single videos.
- **Grouped queues** — videos belonging to the same playlist/channel are displayed as a group with progress information.
- **Subtitles** — optional subtitle downloading.
- **Speed limiting** — configure download bandwidth limits.
- **Scheduling** — global and per-item scheduling.
- **Import / Export** — import and export queue and settings.
- **Proxy support** — HTTP, HTTPS and SOCKS5.
- **Clipboard watcher** — when a supported YouTube link is copied, Vidaro can detect it and offer a quick path to start the download without manually pasting the URL.
- **Bilingual UI** — English and Persian with RTL support.
- **Themes** — Light, Dark and System themes.
- **Persistent state** — queue and settings survive application restarts, allowing unfinished downloads to be resumed.
- **Automatic tools** — yt-dlp and FFmpeg are downloaded and managed by the application when required.

## 🖼️ Screenshots

> Replace the images below with your latest screenshots.  
> Recommended location: `docs/screenshots/`

### All Downloads

![Vidaro All Downloads](docs/screenshots/overview.png)

### New Download

![Vidaro New Download](docs/screenshots/new-download.png)

### Download Queue

![Vidaro Download Queue](docs/screenshots/queue.png)

### History

![Vidaro History](docs/screenshots/history.png)

### Settings

![Vidaro Settings](docs/screenshots/settings.png)

### Dark Mode

![Vidaro Dark Mode](docs/screenshots/dark-mode.png)

### Persian / RTL

![Vidaro Persian RTL](docs/screenshots/rtl.png)

---

## ⬇️ Download

The easiest way to install Vidaro is through the latest GitHub Release.

**Latest release:** [Vidaro v0.3](https://github.com/DashAkoul/vidaro/releases/tag/v0.3)

### Windows

- **NSIS Installer:** `Vidaro_v0.3_x64-setup.exe`
- **MSI Installer:** `Vidaro_v0.3_x64_en-US.msi`

[Download Vidaro for Windows](https://github.com/DashAkoul/vidaro/releases/latest)

### macOS

Universal builds for **Intel and Apple Silicon**:

- `Vidaro_v0.3_universal.dmg`
- `Vidaro_v0.3_universal.app.zip`

[Download Vidaro for macOS](https://github.com/DashAkoul/vidaro/releases/latest)

### Linux

- `Vidaro_v0.3_amd64.AppImage`
- `Vidaro_v0.3_amd64.deb`

[Download Vidaro for Linux](https://github.com/DashAkoul/vidaro/releases/latest)

> GitHub Actions is used to build release artifacts for the supported platforms. The complete source code and build configuration are available in this repository.

---

## 🆕 What's New in v0.3

Vidaro v0.3 expands the application into a more complete cross-platform download manager.

### New and improved

- YouTube video, playlist and channel downloading.
- Windows, macOS and Linux release builds.
- Queue management with pause, resume, cancel and retry.
- Global and per-item scheduling.
- Queue and settings import/export.
- HTTP, HTTPS and SOCKS5 proxy support.
- Clipboard watcher with a quick download notification.
- Light, Dark and System themes.
- Improved RTL support.
- Bug fixes and stability improvements.

### Clipboard quick-download workflow

When the user copies a supported YouTube link, Vidaro can detect the link through the clipboard watcher and show a lightweight notification/pop-up.

The user can then move directly toward preparing the download instead of manually opening Vidaro and pasting the URL.

---

## 🚀 What's planned for v1.0

> **This section describes the planned v1.0 functionality. It should be updated or moved to the release notes once v1.0 is officially released.**

### Browser Extension

Vidaro v1.0 introduces a browser extension designed to make downloading from YouTube significantly more convenient.

While the user is browsing YouTube, the extension can identify the context of the current page, such as:

- A single YouTube video.
- A YouTube playlist.
- Videos belonging to a YouTube channel.

The extension can then present a minimal download interface with relevant options such as:

- Download target.
- Video quality.
- Audio format.
- Subtitle options.
- Other download preferences supported by Vidaro.

The goal is to reduce the number of steps between **watching content in the browser** and **sending it to Vidaro for download**.

<!-- V1.0:
     After the browser extension is released:
     1. Add docs/screenshots/browser-extension.png
     2. Replace this roadmap section with a "Browser Extension" feature section.
     3. Add supported browsers.
     4. Add installation instructions for the extension.
     5. Add extension-to-desktop communication requirements if applicable.
-->

---

## 🛠️ Development

### Prerequisites

You will need:

- Node.js
- Rust toolchain (`rustup`)
- Tauri 2 development dependencies
- On Windows: Visual Studio C++ Build Tools
- On macOS: Xcode Command Line Tools
- On Linux: the required Tauri/WebKit system dependencies

### Install dependencies

```bash
npm install
```

### Run in development mode

```bash
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

Build artifacts are generated under:

```text
src-tauri/target/release/bundle/
```

The exact installer filenames depend on the application version and target platform.

---

## 🏗️ Architecture

```text
src-tauri/src/
  binaries.rs       yt-dlp / FFmpeg download and management
  info.rs           fetch_info and video-list discovery
  download.rs       download queue, yt-dlp execution and progress parsing
  settings.rs       persistent application settings
  history.rs        download history and file-location actions
  util.rs           URL detection, normalization and filename utilities

src/
  App.tsx                 application shell, i18n, themes and tool setup UI
  views/NewDownload.tsx   URL input, video selection, quality/format/folder
  views/Queue.tsx         grouped download queue and item controls
  views/History.tsx       download history
  views/SettingsView.tsx  application and tool settings
  i18n.ts                 English / Persian translations
```

---

## 🌐 Proxy

On some networks, yt-dlp may require a proxy.

Vidaro supports proxy configuration, including HTTP, HTTPS and SOCKS5. Depending on the configuration, yt-dlp can also use environment variables such as:

```text
HTTPS_PROXY
```

---

## ⚖️ Disclaimer

Vidaro is a downloader application. Users are responsible for complying with applicable laws, copyright rules, platform terms and the permissions associated with the content they download.

---

## 🤝 Contributing

Contributions, bug reports, feature requests and pull requests are welcome.

Before opening a new issue, please check whether an existing issue already describes the same problem or request.

[Open an Issue](https://github.com/DashAkoul/vidaro/issues)

---

## 📄 License

See the repository license file for the terms applicable to Vidaro.

---

# فارسی

<a id="فارسی"></a>

<div dir="rtl">

## 🎬 دانلودر یوتیوب Vidaro

**Vidaro** یک برنامه دسکتاپ مدرن برای دانلود محتوای YouTube است که با **Tauri 2، Rust، React و TypeScript** ساخته شده است.

این برنامه امکان دانلود **ویدیوهای تکی، پلی‌لیست‌ها و ویدیوهای کانال‌ها** را فراهم می‌کند و امکاناتی مانند مدیریت صف دانلود، توقف و ادامه دانلود، لغو، تلاش مجدد، زمان‌بندی، پروکسی، زیرنویس، فرمت‌های مختلف و ذخیره وضعیت دانلودها را ارائه می‌دهد.

هدف Vidaro ارائه یک تجربه ساده، سریع و مینیمال برای مدیریت دانلودهای YouTube در کنار انتشار کامل کد منبع پروژه است.

</div>

## ✨ امکانات

<div dir="rtl">

- **دانلود ویدیوی تکی، پلی‌لیست و کانال** — ابتدا اطلاعات ویدیوهای قابل دانلود دریافت و نمایش داده می‌شود و کاربر می‌تواند موارد موردنظر خود را انتخاب کند.
- **شماره‌گذاری خودکار** — آیتم‌های پلی‌لیست بر اساس ترتیب پلی‌لیست و دانلودهای کانال‌ها در صورت انتخاب، از قدیمی‌ترین به جدیدترین مرتب می‌شوند.
- **دانلود منیجر کامل** — مدیریت صف، دانلود هم‌زمان قابل تنظیم، توقف، ادامه، لغو و تلاش مجدد.
- **نمایش وضعیت دانلود** — درصد پیشرفت، سرعت، حجم و زمان باقی‌مانده.
- **فرمت‌های مختلف** — دانلود ویدیو با فرمت MP4 و کیفیت‌های مختلف، در صورت موجود بودن، و استخراج صوت MP3 همراه با کاور و متادیتا.
- **ساختار پوشه‌ها** — امکان تعیین محل ذخیره و سازمان‌دهی دانلودهای مربوط به کانال‌ها و پلی‌لیست‌ها.
- **گروه‌بندی صف دانلود** — ویدیوهای مربوط به یک پلی‌لیست یا کانال در یک گروه قرار می‌گیرند.
- **زیرنویس** — امکان دریافت زیرنویس در صورت پشتیبانی محتوای موردنظر.
- **محدودیت سرعت دانلود** — امکان تعیین محدودیت پهنای باند.
- **زمان‌بندی دانلود** — زمان‌بندی کلی و زمان‌بندی برای آیتم‌های مشخص.
- **Import / Export** — امکان ورود و خروجی گرفتن از صف و تنظیمات.
- **پشتیبانی از Proxy** — پشتیبانی از HTTP، HTTPS و SOCKS5.
- **تشخیص لینک از Clipboard** — با کپی کردن لینک مرتبط با YouTube، برنامه می‌تواند لینک را تشخیص داده و یک اعلان سریع برای شروع فرآیند دانلود نمایش دهد؛ بنابراین کاربر لازم نیست لینک را به‌صورت دستی در برنامه Paste کند.
- **رابط کاربری دوزبانه** — پشتیبانی از English و فارسی با حالت راست‌به‌چپ (RTL).
- **تم‌های مختلف** — Light، Dark و System.
- **ذخیره وضعیت برنامه** — صف دانلود و تنظیمات بین اجراهای مختلف برنامه حفظ می‌شوند و دانلودهای ناتمام قابلیت ادامه دارند.
- **مدیریت خودکار ابزارها** — برنامه در صورت نیاز ابزارهای `yt-dlp` و `FFmpeg` را دریافت و مدیریت می‌کند.

</div>

---

## 🖼️ تصاویر محیط برنامه

<div dir="rtl">

تصاویر زیر نمونه‌هایی از محیط Vidaro هستند. فایل‌های تصویر را می‌توانید در مسیر زیر قرار دهید:

</div>

```text
docs/screenshots/
```

<div dir="rtl">

### صفحه دانلودها

</div>

![محیط Vidaro](docs/screenshots/overview.png)

<div dir="rtl">

### دانلود جدید

</div>

![دانلود جدید](docs/screenshots/new-download.png)

<div dir="rtl">

### صف دانلود

</div>

![صف دانلود](docs/screenshots/queue.png)

<div dir="rtl">

### تاریخچه

</div>

![تاریخچه](docs/screenshots/history.png)

<div dir="rtl">

### تنظیمات

</div>

![تنظیمات](docs/screenshots/settings.png)

<div dir="rtl">

### حالت تاریک

</div>

![حالت تاریک](docs/screenshots/dark-mode.png)

<div dir="rtl">

### رابط فارسی و RTL

</div>

![رابط فارسی Vidaro](docs/screenshots/rtl.png)

---

## ⬇️ دانلود Vidaro

<div dir="rtl">

برای نصب Vidaro، ساده‌ترین روش استفاده از آخرین نسخه منتشرشده در بخش Releases گیت‌هاب است.

**آخرین نسخه فعلی:** Vidaro v0.3

</div>

[دانلود آخرین نسخه Vidaro](https://github.com/DashAkoul/vidaro/releases/latest)

### 🪟 Windows

<div dir="rtl">

برای ویندوز دو نوع Installer ارائه می‌شود:

- `Vidaro_v0.3_x64-setup.exe` — نصب‌کننده NSIS
- `Vidaro_v0.3_x64_en-US.msi` — نصب‌کننده MSI

</div>

[دانلود Vidaro برای Windows](https://github.com/DashAkoul/vidaro/releases/latest)

### 🍎 macOS

<div dir="rtl">

نسخه macOS به‌صورت Universal برای پردازنده‌های Intel و Apple Silicon ارائه می‌شود:

</div>

```text
Vidaro_v0.3_universal.dmg
Vidaro_v0.3_universal.app.zip
```

[دانلود Vidaro برای macOS](https://github.com/DashAkoul/vidaro/releases/latest)

### 🐧 Linux

<div dir="rtl">

نسخه لینوکس در قالب‌های زیر ارائه می‌شود:

</div>

```text
Vidaro_v0.3_amd64.AppImage
Vidaro_v0.3_amd64.deb
```

[دانلود Vidaro برای Linux](https://github.com/DashAkoul/vidaro/releases/latest)

<div dir="rtl">

فایل‌های Release با استفاده از **GitHub Actions** برای پلتفرم‌های پشتیبانی‌شده ساخته می‌شوند و کد منبع و تنظیمات Build نیز در همین مخزن به‌صورت عمومی در دسترس هستند.

</div>

---

## 🆕 تغییرات نسخه Vidaro v0.3

<div dir="rtl">

در نسخه **Vidaro v0.3** چندین مشکل و باگ برطرف شده و قابلیت‌های جدیدی برای تبدیل برنامه به یک Download Manager کامل‌تر اضافه شده است.

### قابلیت‌ها و بهبودهای v0.3

- دانلود ویدیو، پلی‌لیست و کانال YouTube.
- انتشار نسخه‌های Windows، macOS و Linux.
- مدیریت کامل صف دانلود.
- Pause / Resume / Cancel / Retry.
- زمان‌بندی کلی و زمان‌بندی برای هر آیتم.
- Import / Export صف و تنظیمات.
- پشتیبانی از Proxyهای HTTP، HTTPS و SOCKS5.
- تشخیص لینک‌های YouTube از Clipboard.
- نمایش اعلان سریع هنگام شناسایی لینک.
- پشتیبانی از حالت‌های Light، Dark و System.
- بهبود پشتیبانی از رابط راست‌به‌چپ.
- رفع باگ‌ها و بهبود پایداری برنامه.

### تشخیص خودکار لینک کپی‌شده

یکی از قابلیت‌های جدید v0.3، **Clipboard Watcher** است.

هنگامی که کاربر یک لینک مرتبط با YouTube را Copy می‌کند، Vidaro می‌تواند لینک را تشخیص داده و یک اعلان یا Popup کوچک نمایش دهد تا کاربر بتواند سریع وارد فرآیند آماده‌سازی دانلود شود.

به این ترتیب کاربر برای شروع دانلود مجبور نیست ابتدا برنامه را باز کرده و لینک را به‌صورت دستی Paste کند.

</div>

---

## 🚀 مسیر توسعه Vidaro v1.0

> ⚠️ **این قسمت مخصوص نسخه در حال توسعه است. پس از انتشار رسمی v1.0، این بخش را به بخش Features تبدیل کنید و متن مربوط به «در حال توسعه» را حذف کنید.**

<div dir="rtl">

یکی از مهم‌ترین قابلیت‌های نسخه **Vidaro v1.0**، اضافه شدن **Browser Extension** است.

هدف این قابلیت این است که Vidaro از یک برنامه دانلود دسکتاپ به یک تجربه دانلود بسیار یکپارچه‌تر با مرورگر تبدیل شود.

### 🌐 افزونه مرورگر Vidaro

هنگامی که کاربر در مرورگر وارد YouTube می‌شود، افزونه Vidaro می‌تواند وضعیت صفحه را تشخیص دهد.

برای مثال:

- کاربر در صفحه یک **ویدیوی تکی** قرار دارد.
- کاربر در صفحه یک **Playlist** قرار دارد.
- کاربر در صفحه مربوط به **ویدیوهای یک Channel** قرار دارد.

بر اساس نوع صفحه، افزونه می‌تواند یک رابط کوچک و مینیمال برای دانلود به کاربر پیشنهاد دهد.

این رابط می‌تواند گزینه‌هایی مانند موارد زیر را در اختیار کاربر قرار دهد:

- کیفیت ویدیو
- فرمت دانلود
- استخراج صوت
- زیرنویس
- محل یا روش ارسال دانلود
- سایر تنظیمات مرتبط با دانلود

هدف اصلی این قابلیت کاهش تعداد مراحل بین **مشاهده محتوا در مرورگر** و **ارسال آن برای دانلود در Vidaro** است.

</div>

<!-- V1.0 UPDATE:
     بعد از انتشار رسمی افزونه:
     - این بخش را از Roadmap به Features منتقل کنید.
     - اسکرین‌شات افزونه را اضافه کنید:
       docs/screenshots/browser-extension.png
     - مرورگرهای پشتیبانی‌شده را دقیقاً ذکر کنید.
     - روش نصب Extension را اضافه کنید.
     - نحوه ارتباط Extension با Vidaro Desktop را توضیح دهید.
-->

---

## 🛠️ توسعه پروژه

<div dir="rtl">

### پیش‌نیازها

برای اجرای پروژه در محیط توسعه به موارد زیر نیاز دارید:

- Node.js
- Rust Toolchain از طریق `rustup`
- پیش‌نیازهای توسعه Tauri 2
- در Windows: ابزارهای C++ مربوط به Visual Studio
- در macOS: Xcode Command Line Tools
- در Linux: وابستگی‌های سیستمی موردنیاز Tauri و WebKit

</div>

```bash
npm install
```

<div dir="rtl">

### اجرای برنامه در حالت توسعه

</div>

```bash
npm run tauri dev
```

<div dir="rtl">

### ساخت نسخه نهایی

</div>

```bash
npm run tauri build
```

<div dir="rtl">

فایل‌های خروجی معمولاً در مسیر زیر ایجاد می‌شوند:

</div>

```text
src-tauri/target/release/bundle/
```

<div dir="rtl">

نام دقیق فایل‌های Installer بر اساس نسخه برنامه و سیستم‌عامل هدف تعیین می‌شود.

</div>

---

## 🏗️ معماری پروژه

```text
src-tauri/src/
  binaries.rs       مدیریت yt-dlp و FFmpeg
  info.rs           دریافت اطلاعات و فهرست ویدیوها
  download.rs       صف دانلود، اجرای yt-dlp و پردازش پیشرفت
  settings.rs       تنظیمات دائمی برنامه
  history.rs        تاریخچه دانلودها و دسترسی به فایل‌ها
  util.rs           تشخیص و نرمال‌سازی URL و نام فایل‌ها

src/
  App.tsx                 ساختار اصلی برنامه، i18n و Theme
  views/NewDownload.tsx   ورود URL، انتخاب ویدیو و تنظیمات دانلود
  views/Queue.tsx         صف دانلود گروه‌بندی‌شده
  views/History.tsx       تاریخچه دانلود
  views/SettingsView.tsx  تنظیمات برنامه و ابزارها
  i18n.ts                 ترجمه فارسی و انگلیسی
```

---

## 🌐 Proxy

<div dir="rtl">

در بعضی شبکه‌ها ممکن است `yt-dlp` برای دسترسی به YouTube به Proxy نیاز داشته باشد.

Vidaro از Proxyهای HTTP، HTTPS و SOCKS5 پشتیبانی می‌کند.

همچنین در شرایط مناسب می‌توان از متغیرهای محیطی مانند موارد زیر استفاده کرد:

</div>

```text
HTTPS_PROXY
```

---

## ⚖️ نکته حقوقی

<div dir="rtl">

Vidaro یک ابزار دانلود است. مسئولیت استفاده از برنامه، رعایت قوانین قابل اعمال، حقوق مؤلفان و صاحبان محتوا و همچنین شرایط استفاده سرویس‌های مربوطه بر عهده کاربر است.

</div>

---

## 🤝 مشارکت در پروژه

<div dir="rtl">

گزارش Bug، پیشنهاد قابلیت جدید و Pull Request برای توسعه Vidaro مورد استقبال است.

پیش از ایجاد یک Issue جدید، لطفاً بررسی کنید که آیا موضوع مشابهی قبلاً ثبت شده است یا خیر.

</div>

[ثبت Issue در GitHub](https://github.com/DashAkoul/vidaro/issues)

---

## 📄 License

<div dir="rtl">

برای مشاهده شرایط مجوز استفاده از Vidaro، فایل License موجود در مخزن پروژه را مشاهده کنید.

</div>

---

<div align="center">

### Vidaro

**A modern YouTube downloader for Windows, macOS and Linux.**

[⬇️ Download](https://github.com/DashAkoul/vidaro/releases/latest)
&nbsp;•&nbsp;
[📦 Releases](https://github.com/DashAkoul/vidaro/releases)
&nbsp;•&nbsp;
[💻 Source Code](https://github.com/DashAkoul/vidaro)

</div>
