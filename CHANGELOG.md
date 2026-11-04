# Changelog

All notable changes to Vidaro are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [Semantic Versioning](https://semver.org/).

## Version index

| Version | Date | Headline |
| --- | --- | --- |
| [2.1.0](#210--2026-11-04) | 2026-11-04 | Browser extension for Chrome and Firefox |
| [2.0.7](#207--2026-11-04) | 2026-11-04 | Resumable downloads, right-click menu |
| [2.0.6](#206--2026-11-04) | 2026-11-04 | Cancel stops the whole process tree |
| [2.0.5](#205--2026-11-04) | 2026-11-04 | Release builds embed the frontend |
| [2.0.4](#204--2026-11-03) | 2026-11-03 | Downloads can be queued again |
| [2.0.3](#203--2026-11-01) | 2026-11-01 | Real reason for a failed fetch |
| [2.0.2](#202--2026-10-29) | 2026-10-29 | Error boundary, typed parsing, signing |
| [2.0.1](#201--2026-10-29) | 2026-10-29 | URL validation before yt-dlp |
| [2.0.0](#200--2026-10-29) | 2026-10-29 | Rewrite on Tauri 2 |
| [1.1.0](#110--2025-09-27) | 2025-09-27 | Pre-rewrite release |
| [0.2.0](#020--2025-09-20) | 2025-09-20 | Early prototype |
| [0.1.0](#010--2025-09-13) | 2025-09-13 | First release |

---

## [2.1.0] — 2026-11-04

Ships the browser extension for Chrome and Firefox, and reworks the bridge it
needs so it coexists with the single-instance desktop app.

### Added

- **Chrome / Edge / Brave / Vivaldi / Opera extension** (Manifest V3). A
  "Download with Vidaro" button appears next to the video title and on
  playlist/channel rows, showing live progress and the final state; a
  right-click entry downloads any linked YouTube video; the toolbar popup
  offers quality and audio-only selection.
- **Firefox extension** (Manifest V3 event page) built from the same sources.
  Firefox identifies it as `vidaro@dashakoul.dev`; Chromium sees the fixed id
  `noaejgifefjnfjjedhghmejpnfmmmkfb`, pinned by the manifest `key`.
- **Automatic native-host registration.** On every start the app writes the
  host manifest and points Chrome, Chromium, Edge, Brave, Vivaldi, Opera and
  Firefox at it — a registry key on Windows, the well-known directories on
  macOS and Linux. Only browsers that are actually installed are touched.
- **Headless relay mode.** Browsers launch the executable as their native host.
  Because Vidaro is a single-instance GUI app, that process now detects the
  browser origin (`chrome-extension://…`, `moz-extension://…`) and runs as a
  windowless relay that forwards each message to the running app over a
  loopback socket guarded by a random token. Previously such a launch would
  either open a second window or be terminated by the single-instance guard,
  which made native messaging impossible.
- **`show` message** so the extension can bring the app window to the front.
- **`npm run ext:build`** assembles `extensions/dist/chrome` and
  `extensions/dist/firefox`, verifying that every file the manifest references
  exists.

### Changed

- **README rewritten in English**, with a full extension guide and the current
  architecture map.
- This changelog gained a version index for quick tracing.
- Version bumped to 2.1.0 across the app, the manifests and the installers.

### Fixed

- **The popup never worked**: `popup.js` declared `const typeEl` twice in the
  same function, which is a `SyntaxError`, so the whole script failed to parse.
- **The popup opened its own native port**, which dies as soon as the popup
  closes. A single port is now owned by the background worker, with every call
  matched to its reply by `requestId`.
- **The background never answered content-script requests**, so every action
  waited for a response that never came.
- **Duplicate `case` labels** in the background and content scripts, and use of
  the `Notification` API inside a service worker, where it does not exist
  (notifications now go through `chrome.notifications`).

---

## [2.0.7] — 2026-11-04

Resuming an interrupted download is now reliable — even days later — and the
queue has a proper right-click menu.

### Added

- **Context menu on every queue row.** Right-clicking a download opens a
  desktop-style menu: pause/resume, cancel, retry or download again, copy link,
  copy title, copy file path, open the finished file, show it in the folder,
  and remove. Entries follow the item's state (a finished download offers
  *Open file*, a cancelled one offers *Download again*). The menu clamps itself
  to the window, closes on Escape, click-outside, scroll or resize, and
  confirms copies with a short toast.
- **`open_file` command** so a finished download can be opened with the system
  player instead of only being revealed in its folder.

### Improved

- **Progress survives an interruption.** Items that were downloading when the
  app closed come back *paused at the percentage they reached*; previously the
  percentage was reset to 0, so a half-finished file looked untouched.
  Resuming no longer zeroes the bar either — it continues from where it was.
- **The queue is flushed every 20 s while downloading**, so a crash or a hard
  shutdown loses at most a few seconds of position instead of everything.
- **More retries** (10 instead of 5, for both `--retries` and
  `--fragment-retries`) so a long download crossing a flaky connection does not
  fail the whole item.
- **Partial files are cleaned up.** Cancelling or removing an unfinished
  download now deletes the `.part` files yt-dlp left behind; pausing keeps them
  so the download can continue later.

---

## [2.0.6] — 2026-11-04

Cancelling a running download now actually stops it, and the browser context
menu is gone.

### Fixed

- **Cancel did not stop a running download.** `yt-dlp.exe` is a PyInstaller
  bundle: the process we spawn is a bootloader that starts the real Python
  process (and ffmpeg is a third). `Child::kill()` terminated only the
  bootloader, so the download kept running in the child while the UI showed
  "cancelled". Cancel and pause now kill the whole process tree — `taskkill /T`
  on Windows, the process group on Unix — verified against a live download
  where both `yt-dlp.exe` processes died and the file stopped growing.
- **A cancel could arrive before the process was registered.** The gap between
  `spawn()` and `set_child()` meant the kill found nothing to terminate. The
  runner now re-checks the cancel flag right after registering the handle.
- **`wait_child` held the child mutex across `await`.** A cancel arriving
  while the process was being awaited blocked on that mutex until the process
  exited on its own. The handle is now taken and the lock released before
  waiting, and the pid is tracked separately so the tree can still be killed.

### Changed

- **Right-click is disabled.** The webview no longer shows the browser context
  menu (Back / Reload / Save as / Inspect). Ctrl+V still pastes.

---

## [2.0.5] — 2026-11-04

Fixes the window showing "localhost refused to connect" instead of the app.

### Fixed

- **The app opened to a browser error page, not the interface.** `Cargo.toml`
  never declared Tauri's `custom-protocol` feature, so the `tauri` crate
  compiled in development mode and the executable loaded `devUrl`
  (`http://localhost:1420`) instead of the bundled frontend. Unless the Vite
  dev server happened to be running, every launch showed
  `ERR_CONNECTION_REFUSED — localhost refused to connect`. The feature is now
  declared, so production builds embed the assets and run standalone.
- **A minimized window looked like a crash.** `toggle_main_window` treated a
  minimized window as visible (Windows reports it that way) and hid it, so the
  tray icon could never bring it back. Minimized now counts as hidden, and the
  reveal path un-minimizes before showing and focusing.

### Added

- **Startup log.** `<app data>/startup.log` records the pid, executable, launch
  arguments, every setup step, window state and any panic. A
  `windows_subsystem` app has no console, so a silent startup failure was
  previously undiagnosable.

---

## [2.0.4] — 2026-11-03

Fixes "state not managed for field `config`", which stopped every download from
being added to the queue.

### Fixed

- **Downloads could not be added to the queue.** The settings store was
  registered with Tauri as `Arc<ConfigStore>`, but the `enqueue_batch` command
  asked for `ConfigStore`. Tauri keys managed state by its exact type, so the
  lookup failed and the command returned
  `state not managed for field 'config'`. The URL was fetched and displayed
  correctly, but adding the video to the queue always failed. Every managed
  type now matches every consumer, and the lookup goes through a single
  `core::state` helper.
- **A settings mismatch could hide silently.** `try_state` falls back to
  default settings when the type does not match, so the same mistake could
  have reappeared as "settings mysteriously reset" instead of an error.
  Settings are now read through one helper that fails loudly, and a unit test
  ties the registered type to the lookup type so a future drift fails the
  build instead of the download.

---

## [2.0.3] — 2026-11-01

Fixes the "could not parse metadata: invalid type: null" regression introduced
in 2.0.2, and makes YouTube's intermittent failures survivable.

### Fixed

- **A failed fetch reported a serde type error instead of the real reason.**
  2.0.2 started deserialising into `RawInfo`; yt-dlp prints a bare `null` when it
  cannot extract anything, and a struct rejects `null`. So an unavailable video
  surfaced as `invalid type: null, expected struct RawInfo at line 1 column 4`
  and hid the actual cause. The document is now parsed as `Option<RawInfo>`, and
  when it is `null` the message from yt-dlp is reported instead.

### Added

- **Automatic retry.** YouTube answers with a bot check or a degraded player
  response intermittently — the same request often succeeds seconds later. A
  fetch is now retried up to three times, but only for failures that are worth
  retrying (`Sign in to confirm`, `Requested format is not available`,
  `Unable to extract`, …). A permanently unavailable video fails immediately.
- **Readable error messages.** yt-dlp's `ERROR: [youtube] <id>: …` prefix is
  stripped, and the two failures users actually hit are translated into
  actionable text: a bot check is explained as a network-level block with a
  suggestion to switch VPN/proxy server, and an unavailable video says so.

---

## [2.0.2] — 2026-10-29

Follow-up to the 2.0.1 review: the three items that were listed as known gaps.

### Added

- **Error boundary.** A render crash used to leave a blank window. The app now
  shows a recovery screen with the error details, a reload button and a "try
  again" that re-mounts the tree without losing the running downloads. The
  screen is deliberately context-free (it reads `<html lang>`), so it still
  renders when the thing that crashed was a provider.
- **Code-signing pipeline.** `bundle.windows.signCommand` runs
  `src-tauri/sign-windows.ps1`, which signs with `signtool` when
  `WINDOWS_CERTIFICATE` is set and exits cleanly otherwise — verified to no-op
  without a certificate, so local builds are unaffected. The macOS job exports
  the Apple credentials only when `APPLE_CERTIFICATE` exists, because an empty
  value makes the bundler attempt an invalid signature. README documents the
  required secrets.

### Changed

- **Metadata parsing is typed.** `fetch_info` used to build a
  `serde_json::Value` tree — one map per entry, holding every key yt-dlp emits —
  before reducing it to a handful of fields. It now deserialises into
  `RawInfo`/`RawEntry`, keeping only what is used, and reads stdout as raw bytes
  instead of validating the whole document into a `String` first. Parsing runs
  on a blocking thread. `null` entries (removed videos) are tolerated.

---

## [2.0.1] — 2026-10-29

A review pass over 2.0.0. One security fix and several concurrency fixes.

### Security

- **Command-line injection through the download URL.** yt-dlp parses its final
  argument as a URL *or* as an option, and the URL can arrive from the browser
  extension over native messaging. A crafted value such as `--exec=calc` was
  passed straight through and executed as a yt-dlp flag. Every entry point now
  runs the URL through `normalize_url`, which rejects anything starting with `-`,
  containing whitespace, or not resolving to an absolute http(s) URL. Bare hosts
  (`youtube.com/watch?v=…`) are still accepted and upgraded to `https://`.

### Fixed

- **Two processes could run for one item.** Nothing stopped a second resume —
  a double click, or the scheduler racing the user — from spawning another
  yt-dlp process writing to the same output file. `ItemInner::begin_run` now
  hands out an exclusive claim, and a retry that overlaps a cancelling run waits
  briefly instead of being dropped (which left the item stuck in "Queued").
- **Removed items came back.** A control command or a finishing download could
  emit `item-updated` after the item was removed, and the UI re-added it. The
  backend skips events for items no longer in the queue, and the frontend no
  longer appends unknown ids (new items always arrive via `queue-changed`).
- **`default_download_root` could return an error message as a path.** When the
  platform failed to resolve a downloads folder, the fallback stringified an
  `AppError` and used it as the destination directory. It now returns `None`
  and callers ask the user instead.
- **Concurrent tool installs corrupted each other.** Two installs wrote the same
  temp file; the manager now takes an install claim and returns a clear error.

### Changed

- `binaries_status` is async and probes `yt-dlp --version` on a blocking thread,
  so the first call cannot stall the UI.
- Settings updates are serialised, so two concurrent read-modify-write cycles
  cannot overwrite each other.
- The destination folder in New Download follows changes made in Settings.
- Queue rows are memoised and the sidebar badge counts are computed in one pass,
  which cuts re-renders during progress updates.

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
