# Changelog

All notable changes to Vidaro are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [Semantic Versioning](https://semver.org/).

---

## [2.0.0] — 2026-10-29

A structural rewrite: the flat backend became a plugin architecture, and every
known bug from 1.x was fixed.

### Fixed

**Console window flashing on launch**
`ytdlp_version()` spawned `yt-dlp.exe --version` without `CREATE_NO_WINDOW`.
Because yt-dlp is a console application, Windows opened (and immediately
closed) a terminal window on every start. All process spawning now goes through
`core::process`, which always sets the flag, and the version probe is cached for
the lifetime of the process instead of running on every `binaries_status` call.

**Autostart forced itself on, and showed the window**
`setup()` called `autostart_manager.enable()` unconditionally on Windows,
ignoring the user's setting. Worse, `tauri-plugin-autostart` never actually
wrote the registry value in a release build, so the argument-less entry left
behind by 1.x was never corrected and the app kept opening at login.

Login-item registration is now owned by Vidaro (`features/system/autostart.rs`)
and always writes the command line with `--hidden`:

| Platform | Mechanism |
| --- | --- |
| Windows | `HKCU\...\Run\Vidaro` = `"<exe>" --hidden` |
| macOS | `~/Library/LaunchAgents/com.akoul.yt-downloader.plist` |
| Linux | `~/.config/autostart/vidaro.desktop` |

Enabling always rewrites the entry, so a stale 1.x entry repairs itself on the
next launch. The window is created hidden in `tauri.conf.json` and only shown on
a normal launch, which also removes the white flash before React mounts.

**Downloads disappeared from `queue.json`**
`persist_queue` used `try_lock` and silently skipped any item that was mid-write,
so an active download could vanish from the persisted queue. Item state now uses
a short-lived blocking mutex, so every save captures the full queue.

**Concurrent saves could corrupt `queue.json` and lose history entries**
Several downloads finishing together each wrote the same temp file before
renaming it into place, so interleaved writes could publish a truncated
document. `history::add` had the mirror problem: two completions loaded the same
snapshot and one entry was dropped. Both read-modify-write paths are now
serialised with a mutex.

**Global scheduler paused items without stopping them**
It only rewrote the status string; the yt-dlp child kept running, and resuming
spawned a second process for the same item. Pausing now kills the child, and a
new `pausedBy` field ensures the scheduler only resumes what it paused itself.

**Completed downloads vanished from the queue**
`maybe_release_finished_batch` removed finished items as soon as a batch ended,
which made the "Completed" filter permanently empty. Items now stay until the
user clears them.

**Regex compiled per output line**
`strip_ansi` built a new `Regex` for every line of yt-dlp output. It is now
compiled once via `OnceLock`.

**Clipboard handle reopened every second**
A new `arboard::Clipboard` was created on each poll, a known source of transient
lock errors on Windows. One handle is now reused and only recreated on failure.

**`SettingsView` violated the rules of hooks**
It returned early on a null context before calling `useEffect`, which crashes
React when the context is briefly unavailable. The whole view was rewritten.

**Native-messaging handlers were stubs**
`Download`, `GetQueue`, `GetHistory`, `ControlDownload` and `GetItemStatus` all
returned empty placeholders. They are now fully implemented against the real
queue and settings.

**`update_ytdlp` was a stub**
It returned "Auto-update not implemented yet". It now downloads the latest
release and swaps it in atomically.

**`ensure_binaries` only checked, never installed**
The README promised a first-run download that did not exist. A real installer
with progress events was added for Windows, macOS and Linux.

### Changed

- **Plugin architecture.** The flat `src-tauri/src/*.rs` layout became
  `core/` (config, paths, process, events, errors, plugin registry) and
  `features/` (tools, metadata, download, history, settings, system, ipc).
  Each feature is a plugin with its own state, commands and startup hook.
- **Frontend restructured** into `lib/` (api, types, i18n, format, app-context),
  `components/` and `views/`. The mutable `AppContext.current` global was
  replaced with a real React context.
- **Queue concurrency** uses a `Semaphore` that can be resized at runtime instead
  of a hand-rolled counter with a lost-wakeup window.
- **Settings writes are atomic** (temp file + rename) and sanitised on load and
  on import.
- **Tool discovery** searches app data, bundle resources, the executable folder
  and (in debug builds) the crate directory, instead of assuming one location.
- **Installer no longer bundles yt-dlp/ffmpeg** (`resources` removed). This cuts
  roughly 450 MB from the build and makes yt-dlp updatable in place.
- Proxy settings now also apply to metadata fetching.

### Added

- `app_info` command exposing the real version, platform and whether the process
  started hidden.
- Settings: `closeToTray`, `clipboardWatch`, `notifyOnClipboard`.
- System notification when a YouTube link is copied while the window is hidden;
  an in-app toast when it is visible.
- `open_tools_folder` command.
- Rust unit tests for progress parsing, URL classification, filename sanitising
  and argument building.

### Security

- **Content-Security-Policy enabled.** `csp` was `null`, leaving the webview
  with no script or resource restrictions.
- Destination paths are validated as absolute and free of `..` components before
  use; batch folder names are sanitised and confined to the chosen root.
- Imported backups are version-checked and passed through the same sanitiser as
  live settings.
- `--windows-filenames` and a 150-character filename cap prevent reserved names
  and MAX_PATH overflows.
- Removed unused dependencies (`tauri-plugin-global-shortcut`, `serde_bytes`,
  `byteorder`).
- Replaced `tauri-plugin-autostart` with a small first-party implementation so
  the login command line is under our control (adds `winreg` on Windows only).

### Removed

- `vidaro-release/` — a stale, duplicated snapshot of the source tree and
  installers that only drifted out of date.
- ~1.2 GB of build leftovers: `ffmpeg.zip`, `ffmpeg.7z`, extracted ffmpeg
  directories, duplicate binaries and a second copy of the source tree.
- One-off icon-fix scripts (`fix_icon*.ps1`, `update_pngs.ps1`,
  `create_installer_bmps.ps1`).
- `CHANGES.md`, superseded by this file.

---

## [1.1.0] — 2025-09-27

### Added
- Clipboard watcher with a system notification and click-to-download.
- Autostart with the OS.
- Custom NSIS/MSI installer artwork.
- Rebuilt `icon.ico` with 12 sizes (16–256 px, PNG-compressed) to fix the
  blurry taskbar icon.
- Proxy support forwarded to ffmpeg via environment variables.

### Changed
- Release profile: `lto = false` to work around a `ring` linking failure.

---

## [0.2.0] — 2025-09-20

### Added
- Import/Export of settings, queue and history as a single JSON file.
- Global scheduler with overnight window support.
- Per-item scheduling from the queue.
- Proxy configuration UI (HTTP/HTTPS/SOCKS5).

---

## [0.1.0] — 2025-09-13

Initial release: single video / playlist / channel downloads, queue with
pause-resume-cancel, MP4 and MP3 output, Persian and English UI.
