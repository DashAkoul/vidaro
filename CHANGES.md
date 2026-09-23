# Vidaro - تغییرات و برنامه‌ریزی

## تاریخ: 2026-09-20

---

## ✅ تسک‌های انجام شده

### 1. رفع مشکل آیکون تار و ناقص (Blurry/Half Icon)
- **مشکل**: آیکون در Taskbar و Desktop Shortcut نصفه/ناقص نمایش داده می‌شد
- **علت**: فایل `icon.ico` فرمت BMP قدیمی داشت که شفافیت را به درستی پشتیبانی نمی‌کرد و سایزهای کامل (16 تا 256) را نداشت
- **راه حل**: 
  1. بازسازی `icon.ico` با ۱۲ سایز استاندارد (16, 20, 24, 32, 40, 48, 64, 72, 80, 96, 128, 256) در فرمت **PNG** داخل ICO (پشتیبانی کامل از شفافیت)
  2. به‌روزرسانی تمام فایل‌های PNG (16x16, 32x32, 48x48, 128x128, 256x256) از منبع اصلی
  3. استفاده از `Image::new_owned` در Rust برای تنظیم آیکون پنجره در dev mode
- **تأثیر**: آیکون در Taskbar، System Tray، Desktop Shortcut، نصب‌کننده (MSI/NSIS) و Alt+Tab کامل و شفاف نمایش داده می‌شود

### 2. حذف فایل‌های اضافی
- حذف `node_modules/`, `dist/`, `src-tauri/target/` (پوشه‌های بیلد)
- حذف فایل‌های موقت و لاگ (`src-tauri/target/debug/*.pdb`, `*.log`, cache)
- پاکسازی کدهای استفاده نشده (importهای اضافی، متغیرهای مرده)
- حذف فایل‌های Generated در `src-tauri/gen/`

### 3. Import/Export صف و تنظیمات
- **فرمت**: یک فایل JSON واحد حاوی `queue` + `settings` + `history`
- **Backend** (`settings.rs`):
  - `export_data(app, path)` - صادر کردن تمام داده‌ها به JSON
  - `import_data(app, path)` - وارد کردن داده‌ها با deduplication
- **Frontend** (`SettingsView.tsx`):
  - دکمه‌های Export/Import با File Dialog
  - اعتبارسنجی نسخه و سازگاری هنگام Import

### 4. Scheduler (زمان‌بندی دانلود) - دو سطح
- **سطح سراسری** (Global):
  - فعال/غیرفعال کردن در Settings
  - بازه زمانی (مثال: 02:00 تا 06:00)
  - پشتیبانی از بازه‌های شبانه‌روز (مثال: 22:00 تا 04:00)
  - Task پس‌زمینه هر ۳۰ ثانیه چک می‌کند و آیتم‌های بدون برنامه جداگانه را pause/resume می‌کند
- **سطح آیتم** (Per-item):
  - در NewDownload: فیلدهای Start/End time برای کل batch
  - در Queue: دکمه 🕐 برای تنظیم/حذف زمان‌بندی هر آیتم
  - حالت `scheduled` جدید برای آیتم‌های در انتظار زمان
- **Backend** (`download.rs`):
  - `wait_for_schedule()` - صبر تا بازه زمانی فعال
  - `update_item_schedule` command
  - `start_global_scheduler` background task

### 5. Proxy در UI
- فیلد Proxy در SettingsView (HTTP/HTTPS/SOCKS5)
- اعمال مستقیم به آرگومان‌های yt-dlp با `--proxy`
- پشتیبانی از فرمت‌های: `http://host:port`, `https://host:port`, `socks5://host:port`

### 6. بازسازی خروجی تمیز
- **پوشه `source_code/`**: کد تمیز، مستند، بدون فایل‌های بیلد
- **پوشه `install/`**: باینری Release بهینه شده + نصب‌کننده MSI/NSIS

### 8. اعلان پاپ‌آپ کلیپ‌بورد (Clipboard Popup Notification)
- **ویژگی**: وقتی لینک یوتیوب در کلیپ‌بورد کپی می‌شود، یک توست (Toast) زیبا و برجسته در گوشه پایین‌چپ صفحه ظاهر می‌شود
- **Backend** (`tray.rs`): Clipboard watcher هر ۱ ثانیه چک می‌کند و رویداد `clipboard-youtube` emit می‌کند
- **Frontend** (`App.tsx` + `app.css`):
  - توست با انیمیشن slide-up و آیکون 📥
  - دکمه «دانلود» (مستقیم به تب New Download)
  - دکمه «باز کردن برنامه» (نمایش و فوکوس روی پنجره + رفتن به تب New Download)
  - دکمه «نادیده» برای بستن
  - استایل جدید با `.show` class برای انیمیشن ورود/خروج
- **ترجمه**: کلید `openApp` به فارسی («باز کردن برنامه») و انگلیسی («Open App») اضافه شد

---

## 📋 وضعیت تسک‌ها (همه کامل ✅)

### فاز 1: اصلاحات پایه و پاکسازی
- [x] بررسی و رفع آیکون
- [x] حذف فایل‌های اضافی
- [x] ایجاد اسکریپت تمیزسازی خودکار

### فاز 2: ویژگی‌های جدید (Back-end)
- [x] Import/Export در `settings.rs` و `download.rs`
- [x] Scheduler در `download.rs` (Gate + scheduled queue)
- [x] Proxy support در `settings.rs` و `download.rs` (args yt-dlp)
- [x] History در Import/Export

### فاز 3: ویژگی‌های جدید (Front-end)
- [x] UI Import/Export در `SettingsView.tsx`
- [x] UI Scheduler در `SettingsView.tsx` (Global) + `Queue.tsx` (Per-item)
- [x] UI Proxy در `SettingsView.tsx`
- [x] آیکون/استایل برای وضعیت `scheduled`
- [x] **Toast اعلان کلیپ‌بورد با دکمه‌های «دانلود»، «باز کردن برنامه»، «نادیده»**

### فاز 4: بیلد و بسته‌بندی
- [x] بیلد Debug با `cargo build` (تست شد)
- [x] بیلد Frontend با `npm run build` (تست شد)
- [x] بیلد Release با `cargo build --release` (LTO=false، opt-level=3، strip=true) ✅
- [x] بیلد نصب‌کننده با `npm run tauri build` ✅
  - MSI: `Vidaro_0.2.0_x64_en-US.msi` (4.5 MB)
  - NSIS: `Vidaro_0.2.0_x64-setup.exe` (2.7 MB)
- [x] کپی باینری و installerها به `install/`
- [x] کپی کد تمیز به `source_code/`

### فاز 5: مستندسازی
- [x] به‌روزرسانی `CHANGES.md`
- [ ] به‌روزرسانی `README.md` (اختیاری - پایه وجود دارد)

---

## 📝 نکات فنی مهم

### Scheduler Implementation Details
```rust
// در download.rs
// parse_time: "HH:MM" -> minutes since midnight
// now_minutes: local time -> minutes
// wait_for_schedule: sleeps until window opens, handles overnight windows

// Global scheduler: background task checks every 30s
// Per-item: checked before gate.acquire() in run_item()
```

### Import/Export Format
```json
{
  "version": 1,
  "exportedAt": "2026-09-20T10:00:00Z",
  "settings": { ... },
  "queue": [ ... ],
  "history": [ ... ]
}
```

### Proxy Format
```
http://user:pass@host:port
https://host:port
socks5://host:port
```

---

## 🎯 خروجی نهایی

```
vidaro-release/
├── source_code/
│   ├── src/
│   ├── src-tauri/
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── README.md
│   └── CHANGES.md
└── install/
    ├── vidaro.exe                 # Release binary (8.3 MB)
    ├── Vidaro_0.2.0_x64-setup.exe # NSIS installer (2.7 MB)
    ├── Vidaro_0.2.0_x64_en-US.msi # MSI installer (4.5 MB)
    ├── checksums.txt              # SHA256 hashes
    └── README.txt                 # Installation guide
```

---

## ✅ تست‌ها و نتایج

| تست | وضعیت | جزئیات |
|------|-------|---------|
| بیلد TypeScript (`npm run build`) | ✅ Pass | 26 modules transformed, 262KB JS gzip: 80KB |
| بیلد Rust Debug (`cargo build`) | ✅ Pass | Finished dev profile in ~27s, 0 warnings |
| بیلد Rust Release (`cargo build --release`) | ✅ Pass | Finished in ~4m, 8.3 MB binary |
| بیلد Tauri Installer (`npm run tauri build`) | ✅ Pass | MSI + NSIS created |
| اجرای باینری Release (`vidaro.exe`) | ✅ Pass | Clipboard watcher started, UI loaded |
| بیلد Tauri Dev (`npm run tauri dev`) | ✅ Pass | Hot reload working, compiled successfully |
| آیکون در Taskbar/Tray/Shortcut | ✅ Pass | Sharp icon with all 12 sizes (16-256), PNG in ICO |
| Import/Export UI | ✅ Verified | Buttons present in SettingsView |
| Scheduler UI (Global/Per-item) | ✅ Verified | Time inputs in Settings, 🕐 button in Queue |
| Proxy UI | ✅ Verified | Input field in Settings |
| **Clipboard Toast Notification** | ✅ **Pass** | **Animated toast with Download/Open App/Dismiss buttons** |

---

## ⚠️ نکات باقی‌مانده
1. **Scheduler**: با Pause/Resume موجود تداخل ندارد (چک شده)
2. **Import**: deduplication بر اساس video_id + dir انجام می‌شود
3. **Proxy**: yt-dlp از آرگومان `--proxy` استفاده می‌کند، نیاز به restart ندارد
4. **Icon**: بازسازی شده با ۱۲ سایز (16, 20, 24, 32, 40, 48, 64, 72, 80, 96, 128, 256)
5. **LTO غیرفعال**: در Release LTO=false برای جلوگیری از مشکل کامپایل `ring` crate
6. **Bundled Binaries (externalBin)**: در حال حاضر به دلیل محدودیت Tauri v2 در مدیریت target triple، باینری‌های yt-dlp/ffmpeg در installer بسته‌بندی نشده‌اند. در اولین اجرا، برنامه به صورت خودکار آن‌ها را دانلود می‌کند (مکانیزم `ensure_binaries` فعال است). برای رفع کامل، نیاز به بررسی دقیق‌تر مستندات Tauri v2 یا استفاده از build script سفارشی است.