# Vidaro

A desktop YouTube downloader for Windows, macOS and Linux — built with
Tauri 2 (Rust) + React + TypeScript.

![version](https://img.shields.io/badge/version-2.1.0-00a8ff)
![platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-00a8ff)

---

## Features

- **Single video, playlist or whole channel** — preview the list first, then pick what to download
- **Automatic numbering** — playlist order, or oldest → newest for channels: `001 - Video Name.mp4`
- **Full download manager** — queue, configurable concurrency, per-item pause/resume/cancel/retry, speed/size/ETA, history
- **Resumable downloads** — a download interrupted today continues from the same byte next week
- **Right-click menu** — open file, show in folder, copy link/title/path, pause, cancel, retry, remove
- **Formats** — MP4 from 360p to 4K, or MP3 with cover art and metadata
- **Folder layout** — `Downloads/YouTube Downloader/<playlist or channel>/` for batches, `.../Single Videos/` for one-offs
- **Two-level scheduler** — a global time window plus per-item schedules
- **Proxy** — off / system / manual (HTTP, HTTPS, SOCKS5, SOCKS5h)
- **Clipboard detection** — copying a YouTube link offers a one-click download
- **System tray** — closing the window keeps the app running
- **Hidden autostart** — Vidaro starts in the tray at boot without opening a window
- **Bilingual UI** — Persian (RTL) and English, dark/light/system themes
- **Self-provisioning tools** — yt-dlp and ffmpeg are fetched on first run, keeping the installer small
- **Browser extension** — send the current page to Vidaro from Chrome, Edge or Firefox

---

## Installation

Download the installer for your platform from the
[releases page](https://github.com/DashAkoul/vidaro/releases):

| Platform | File |
| --- | --- |
| Windows | `Vidaro_<version>_x64-setup.exe` (NSIS) or `Vidaro_<version>_x64_en-US.msi` |
| macOS | `Vidaro_<version>_universal.dmg` |
| Linux | `Vidaro_<version>_amd64.AppImage` or `.deb` |

yt-dlp and ffmpeg are **not** bundled. They are downloaded into the app data
folder on first run, or from **Settings → Download tools**.

---

## Browser extension

The extension adds a **Download with Vidaro** button to YouTube, a context-menu
entry, and a toolbar popup. It talks to the desktop app over native messaging,
so the app must be running.

### How the connection works

```
extension  ──chrome.runtime.connectNative──▶  vidaro.exe --native-messaging
                                                     │  (headless relay)
                                                     ▼  127.0.0.1 + token
                                              Vidaro GUI process
```

The browser launches the executable as its native host. Because Vidaro is a
single-instance GUI app, that process must not open a window or take the
instance slot — it runs as a headless relay and forwards each message to the
running app over a loopback socket. Registration is automatic: on every start
the app writes the host manifest and points Chrome, Chromium, Edge, Brave,
Vivaldi, Opera and Firefox at it.

### Install (Chrome / Edge / Brave)

1. Run `npm run ext:build` — this writes `extensions/dist/chrome`.
2. Open `chrome://extensions`, enable **Developer mode**.
3. **Load unpacked** → select `extensions/dist/chrome`.

The manifest carries a fixed `key`, so the extension id stays
`noaejgifefjnfjjedhghmejpnfmmmkfb` regardless of where the folder lives. That id
is what the native host allow-list checks, so keep the folder anywhere — the
link keeps working.

### Install (Firefox)

1. Run `npm run ext:build`.
2. Open `about:debugging#/runtime/this-firefox`.
3. **Load Temporary Add-on** → pick `extensions/dist/firefox/manifest.json`.

Firefox identifies the extension as `vidaro@dashakoul.dev` (declared in
`browser_specific_settings`), which is the id the native host allow-list uses.

### What it can do

| Action | Where |
| --- | --- |
| Download the current page | Popup → **Download**, or the button next to the video title |
| Download a linked video | Right-click a link → **Download with Vidaro** |
| Pick quality / audio-only | Popup → Format and Quality |
| Watch progress | The injected button shows live percentage and final state |
| Open the app | Popup → **Open in Vidaro App** |

---

## Development

```bash
npm install
npm run tauri dev
```

Requires Node.js 20+, a Rust toolchain (rustup), and Visual Studio C++ Build
Tools on Windows.

### Build

```bash
npm run tauri build     # desktop installers
npm run ext:build       # browser extension packages
```

Desktop artifacts land in `src-tauri/target/release/bundle/`, extension
packages in `extensions/dist/`.

---

## Architecture

```
src-tauri/src/
├── lib.rs                 composition root: state, plugins, commands, window
├── main.rs                entry point; detects native-host invocations
├── core/                  shared infrastructure
│   ├── config.rs          settings + ConfigStore (cache + atomic writes)
│   ├── paths.rs           application data layout
│   ├── process.rs         spawn helpers (no console window on Windows)
│   ├── filename.rs        name sanitising and path-traversal guards
│   ├── logging.rs         startup log + panic capture
│   ├── state.rs           typed access to Tauri managed state
│   ├── events.rs          shared event names
│   ├── error.rs           unified error type
│   └── plugin.rs          plugin trait + registry
└── features/              self-contained plugins
    ├── tools/             yt-dlp and ffmpeg (paths, install, update)
    ├── metadata/          list videos/playlists/channels without downloading
    ├── download/          queue, gate, runner, progress parser, scheduler
    ├── history/           completed downloads
    ├── settings/          settings commands + import/export
    ├── system/            tray, clipboard, autostart, window helpers
    └── ipc/               browser bridge: messages, relay, registration
```

Each feature is a self-contained plugin: it owns its state, registers its own
Tauri commands, and runs its own startup hook. `lib.rs` only wires them
together, so adding a capability never means editing unrelated modules.

```
src/
├── App.tsx                application shell
├── main.tsx               React entry point
├── lib/                   api, types, i18n, format, app-context, clipboard
├── components/            Sidebar, Topbar, StatusBar, ContextMenu, Toast
├── views/                 NewDownload, Queue, History, Settings
└── styles/app.css
```

```
extensions/
├── shared/                scripts and markup used by every browser
├── chrome/manifest.json   MV3 service worker + fixed key
├── firefox/manifest.json  MV3 event page + gecko id
├── build.mjs              assembles dist/chrome and dist/firefox
└── dist/                  generated packages (load these)
```

---

## Notes

- On networks where YouTube is filtered, configure a proxy in Settings.
- Release builds use `lto = false` because thin-LTO breaks linking of the
  `ring` crate on MSVC.
- The app writes `startup.log` next to its settings; it records each start and
  any panic, which is the fastest way to diagnose "it does not start".

---

## Code signing

Unsigned builds make Windows show "Windows protected your PC" and macOS refuse
to open the app without a manual override. The pipeline supports both signers
and stays unsigned when no credentials are configured.

**Windows** — add two repository secrets:

| Secret | Value |
| --- | --- |
| `WINDOWS_CERTIFICATE` | base64 of your `.pfx`, on a single line (`openssl base64 -A -in cert.pfx`) |
| `WINDOWS_CERTIFICATE_PASSWORD` | the `.pfx` password |

`src-tauri/sign-windows.ps1` is wired through `bundle.windows.signCommand` and
exits without signing when the secrets are absent, so local builds are
unaffected.

**macOS** — add `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
`APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID`. The
workflow exports them only when `APPLE_CERTIFICATE` exists (an empty value would
make the bundler attempt an invalid signature); Tauri then signs and notarizes
the `.app` and `.dmg`.

---

## Changelog

Every release and the exact changes in it are recorded in
[CHANGELOG.md](CHANGELOG.md).
