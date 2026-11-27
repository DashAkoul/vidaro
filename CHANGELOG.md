# Changelog

All notable changes to Vidaro are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [Semantic Versioning](https://semver.org/).

## Version index

| Version | Date | Headline |
| --- | --- | --- |
| [2.2.2](#222--2026-11-27) | 2026-11-27 | Minimal interface, English only, native packages for every Linux family |
| [2.2.1](#221--2026-11-27) | 2026-11-27 | Settings organised into tabs; honest browser-cookie diagnosis |
| [2.2.0](#220--2026-11-26) | 2026-11-26 | Survive YouTube bot checks; keep non-Latin file names |
| [2.1.1](#211--2026-11-07) | 2026-11-07 | Show the real fetch error; make proxy modes work |
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

## [2.2.2] — 2026-11-27

### Changed

- **The interface is minimal and modern.** A neutral graphite palette with one
  calm accent replaces the neon glow and gradients; flatter buttons and cards,
  quieter sidebar and status bar, consistent radii, lighter typography in both
  themes.
- **English only.** The Persian UI language, its dictionary, RTL layout and
  every translated tray/notification string were removed; the language picker
  is gone from the top bar and settings.
- **Every Linux package family is covered.** The release now ships, besides
  AppImage and `.deb`: `.rpm` (Fedora/RHEL/openSUSE), `.pkg.tar.zst` for
  Arch-based distros (built by `scripts/package-arch.sh`), a Gentoo `.ebuild`
  recipe generated from `Cargo.lock` (`scripts/gen-ebuild.sh`), and an Alpine
  `.apk` compiled against musl in an Alpine 3.21 container
  (`scripts/build-alpine.sh`).

### Fixed

- README gained real screenshots of the interface.

---

## [2.2.1] — 2026-11-27

Small quality-of-life release built from a real session where every import
path was tried.

### Fixed

- **Settings is organised into tabs.** The page had grown to nine stacked
  sections over three screens of scrolling. A sticky tab bar now splits it
  into *General*, *YouTube account*, *Network*, *Tools & formats*,
  *Downloads* and *Data*, and the error box sits directly under the tabs so
  it is visible on every tab.

### Changed

- **Honest diagnosis when browser cookies cannot be read.** Chrome and Edge
  (version 127 and later) encrypt their cookie store with a Windows
  app-bound key that only the browser itself can unwrap — yt-dlp fails with
  `Failed to decrypt with DPAPI` and so does Vidaro. The import error now
  names this cause for those browsers instead of the generic "keep the
  browser open" advice, Settings warns before the attempt, and a new
  *Sign in to YouTube* button opens youtube.com in the selected browser so
  the sign-in can happen where the cookies are readable.

---

## [2.2.0] — 2026-11-26

Makes the app survive the one failure that produced almost every complaint:
"YouTube asked to confirm you're not a bot", which arrives in bursts and made
Vidaro look like it worked only sometimes.

### The evidence

A real queue from a machine that had been using Vidaro held **36 items: 22
failed, 14 completed**, and every failure was the bot check. The block is
answered by YouTube's servers, not by the extractor: querying InnerTube
directly from the blocked network returned `LOGIN_REQUIRED` for `TVHTML5`,
`WEB`, `MWEB` and `ANDROID_VR` alike. Replacing yt-dlp with another extractor
would hit the same wall while losing its format and signature handling, so the
work went into mitigation instead.

Passing cookies then exposed a **second, independent** failure: yt-dlp answered
`No video formats found!`. Its log named the cause:

```
[jsc] JS Challenge Providers: bun (unavailable), deno (unavailable), node (unavailable)
WARNING: n challenge solving failed: Some formats may be missing
```

YouTube's player challenge (`n`/`sig`) is JavaScript, and yt-dlp needs a
runtime to evaluate it. The build installed on that machine (2026.10.26) also
crashed inside its bundled solver (`a.addOnDisposeCallback is not a function`);
2026.11.23 solved the same challenge correctly. With the new build, a runtime
and cookies, a real download completed inside the app.

### Added

- **Cookies** — Settings → YouTube account can pass a signed-in session to
  yt-dlp, either read from a browser (`--cookies-from-browser`) or from an
  exported `cookies.txt`. This is the only dependable cure for the bot check;
  reading Chrome's database can fail with DPAPI/lock errors, and the UI now
  says so and suggests the file instead.
- **Automatic retries** for transient failures (bot check, degraded player
  response, network). The item goes back to the queue with a persisted
  `retry_at`, the delay grows exponentially from a configurable base (5 min,
  four attempts by default) and the state survives a restart.
- **Player-client rotation** on retries (`web_safari`, `tv`, `android_vr`,
  `mweb`) for downloads and metadata, instead of repeating the exact request
  that just failed.
- **Test connection** in Settings: probes one known video with the current
  proxy and cookie settings and reports whether the problem is the tools, the
  network or YouTube's bot check, including the age of the installed yt-dlp.
- **Retry failed** in the queue toolbar re-queues every errored item at once.
- **JavaScript runtime management.** Vidaro looks for a managed Deno, then for
  `deno`, `node`, `bun` or `qjs` on the machine, and passes the result to
  yt-dlp. When nothing is found it installs Deno into the managed tools
  folder, because without a runtime YouTube returns no formats at all.
- **Automatic yt-dlp refresh.** A build older than 30 days is replaced in the
  background at startup. The stale build was the root cause of the
  `n`-challenge failure, and no user should have to notice that.
- **Resumable helper downloads.** Installing or updating yt-dlp/ffmpeg/Deno
  now retries and continues from the bytes already on disk. A dropped
  connection used to leave a `.download` file behind and a failed install,
  which is another way the app could "just not work" on a slow link.
- **Instant link recognition.** Pasting a URL no longer waits for a spawned
  process: `detect_url` classifies the link locally, the UI shows
  "video / playlist / channel" plus the id straight away, and oEmbed fills in
  the title, channel and thumbnail in about a second while yt-dlp is still
  starting. Measured on the same machine: 13–17 s for the full yt-dlp fetch
  versus 1.3–1.7 s for the preview.
- **Download now.** A single video can be queued immediately with the best
  quality, without waiting for the format list. The previous flow put a
  10-second metadata fetch between the user and the download.
- **Metadata cache.** A fetch result is reused for ten minutes, so re-pasting
  the same link does not start yt-dlp again.
- **Saved session ("import once").** The browser's cookie jar can be copied
  into `session/cookies.txt` and used from then on, so closing, updating or
  uninstalling the browser no longer breaks downloads. Settings shows how many
  signed-in cookies were captured.
- **Parallel fragments.** `--concurrent-fragments 4` downloads fragmented
  formats in parallel, which is the difference between minutes and tens of
  minutes on a slow link.
- **ffmpeg is optional.** Without it Vidaro asks for a single progressive
  stream instead of an unmuxable video+audio pair, and keeps audio in its
  native container instead of failing the MP3 conversion.
- **The download options no longer depend on the metadata fetch.** When
  YouTube blocks the fetch, the app still shows the preview title plus the
  standard quality list and lets the video be queued; the queue retries on its
  own. Before this, a bot check meant no quality list and no way to enqueue.
- **Paste a `cookies.txt`.** Settings → YouTube account accepts the contents of
  an exported cookie file, which works with any browser, any extension and any
  machine — the browser-independent alternative to reading a live profile.
- **System tools are used before downloads.** If `yt-dlp`, `ffmpeg` or a JS
  runtime is already installed on the machine (scoop, choco, brew, apt,
  nodejs), Vidaro uses it instead of fetching another copy from GitHub.
- **Tool downloads honour the proxy.** Installing or updating yt-dlp/ffmpeg/
  Deno used to connect directly even when a proxy was configured, which failed
  on networks where GitHub is filtered.
- [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) documents the diagnosis
  and the order in which the workarounds actually help.

### Fixed

- **Non-Latin titles destroyed the recorded file path.** A Persian title was
  stored as `  ADHD  .mp4` because yt-dlp printed its log line in the hidden
  console code page and the path was scraped from that text. Helper processes
  now run with UTF-8 stdio (`PYTHONUTF8`, `PYTHONIOENCODING`), and the final
  path comes from `--print-to-file after_move:filepath`, which is written as
  UTF-8 and reflects a merge rename. "Open file" and "Show in folder" work for
  Persian, Arabic and emoji titles again.
- Queue failures now show an actionable sentence instead of raw yt-dlp text,
  and items waiting for a retry are marked in the list.

### Changed

- Error classification (transient vs permanent, bot check vs network vs tools)
  lives in one module shared by the metadata fetch and the download runner, so
  the retry policy and the UI can no longer disagree.

---

## [2.1.1] — 2026-11-07

Diagnosing "yt-dlp could not read this URL" and making the proxy setting do
what it says.

### What was actually happening

YouTube was returning **"Sign in to confirm you're not a bot"** for every link —
not a URL-recognition failure. Verified against both a Shorts URL and an
ordinary `watch?v=` URL: all failed 3/3 times, and none of the alternative
`player_client` values helped. The block cleared on its own a few minutes
later, then returned; it is temporary IP/network-level throttling, not a bug
in how Vidaro parses links. Browser cookies were also tested: they bypass the
bot check but only expose storyboards, so they are not a usable workaround.

### Fixed

- **The real error was hidden.** The stderr reader discarded `WARNING:` lines
  and rate-limited which lines it kept — a burst of progress lines could push
  the final `ERROR:` out of `last_message`, leaving the UI with the generic
  "yt-dlp could not read this URL". The reader now keeps a bounded tail of the
  stream and explicitly prefers the last error / bot-check line. Because the
  generic text did not match the transient markers, the retry loop also gave
  up immediately.
- **`proxyMode` "system" and "off" did nothing.** `proxy_url()` only handled
  "manual", so yt-dlp silently inherited whatever the registry/environment
  provided — switching VPN or proxy in Windows had no effect, and "off" did
  not actually bypass a proxy. A new `ProxySetting` enum now resolves to
  `Direct` (`--proxy ""`), `Url(...)`, or `Inherit`, and "system" reads the
  Windows registry (or the usual environment variables on Unix). Applied
  consistently to metadata fetches, downloads and the environment passed to
  ffmpeg.
- **Bot-check guidance** now mentions Settings → Proxy and that the block is
  temporary; the retry delay grows (2 s, then 4 s) instead of retrying into
  the same block after 1.2 s.
- Failure messages include yt-dlp's exit code when it prints no error at all.

### Added

- Six regression tests: stderr tail selection (including a warning-only bot
  check), the friendly error staying transient, and all proxy modes.

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
