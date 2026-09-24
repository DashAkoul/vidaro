<div align="center">

### Vidaro

[🇬🇧 English](README.md) · [🇮🇷 فارسی](README_FA.md) · [⬇️ Download](DOWNLOAD.md) · **🛠️ Technical**

</div>

---
# 🛠️ Vidaro — Technical Information

This page is for developers, contributors and users who want the technical details.

## Technology stack

- **Tauri 2**
- **Rust**
- **React**
- **TypeScript**
- **yt-dlp**
- **FFmpeg**
- **GitHub Actions**

## Project structure

```text
src-tauri/src/
  binaries.rs       yt-dlp / FFmpeg management
  info.rs           video and playlist information
  download.rs       download queue and yt-dlp execution
  settings.rs       persistent settings
  history.rs        download history
  util.rs           URL and filename utilities

src/
  App.tsx
  views/NewDownload.tsx
  views/Queue.tsx
  views/History.tsx
  views/SettingsView.tsx
  i18n.ts
```

## Development

### Requirements

- Node.js
- Rust toolchain via `rustup`
- Tauri 2 prerequisites
- Windows: Visual Studio C++ Build Tools
- macOS: Xcode Command Line Tools
- Linux: Tauri/WebKit system dependencies

### Install

```bash
npm install
```

### Run

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

## Release pipeline

GitHub Actions is used to build platform-specific release artifacts, which are then published through GitHub Releases.

## yt-dlp and FFmpeg

Vidaro uses yt-dlp for supported media extraction/download workflows and FFmpeg for required media processing.

## Proxy

Vidaro supports HTTP, HTTPS and SOCKS5 proxy configuration. Depending on the environment, yt-dlp can also use:

```text
HTTPS_PROXY
```

## Clipboard workflow

The v0.3 clipboard watcher detects supported YouTube links copied by the user and offers a quick route into the download workflow.

## v1.0 browser extension

The v1.0 extension is intended to act as a browser-side companion to the desktop application.

It is designed to recognize the current YouTube context:

```text
Single video
Playlist
Channel videos
```

and expose a minimal set of download controls before handing the request to Vidaro.

The exact browser-to-desktop communication mechanism and supported browsers should be documented here after the v1.0 implementation is finalized.

# Licensing — important

## Your requested licensing model

You described the desired permissions as:

- users can use the source;
- users can modify it;
- users can redistribute modified versions;
- the application is free to use;
- commercial selling or monetization requires your permission.

There is an important legal distinction here.

### This is not an OSI Open Source license

The Open Source Definition requires licenses not to restrict selling or commercial use. Therefore, a license that requires your permission before commercial use cannot honestly be called an OSI-compliant Open Source license.

GNU GPL also does **not** provide the restriction you described: GNU explicitly says GPL permits selling copies and commercial distribution, subject to GPL conditions.

So **GPL is not the license you are remembering** if the key requirement is “ask me before commercial monetization.”

## Closest existing model: PolyForm Noncommercial

A license worth evaluating is **PolyForm Noncommercial License 1.0.0**.

It permits noncommercial use and allows changes and new works for permitted purposes, while restricting commercial use.

Official license:

https://polyformproject.org/licenses/noncommercial/1.0.0/

If you choose this model, a more accurate description is:

> **Free and source-available for noncommercial use**

rather than:

> **Open Source**

### Commercial use

You can then offer separate commercial permission/licenses from the copyright holder.

A simple README explanation could be:

> Vidaro is free for personal and other permitted noncommercial use. Commercial use, commercial distribution, paid redistribution, or monetization requires a separate commercial license or written permission from the copyright holder.

The exact definition of “commercial use” should be finalized in the actual license terms, preferably with legal advice.

## Two possible directions before v1.0

### A — Genuine Open Source

Choose an OSI-compliant license such as GPLv3.

Users can commercially use and sell the software under the license terms. You cannot require them to ask you first merely because they want to sell it.

### B — Commercial permission required

Use a noncommercial/source-available license such as PolyForm Noncommercial and grant commercial licenses separately.

This is much closer to the model you described, but it should not be marketed as formal Open Source.

> **Do not publish the final LICENSE file until you choose A or B.**

## Contributions

If external contributors will be accepted, add a clear contribution policy defining how contributed code is licensed and who owns copyright in contributions.

---

<div align="center">

[🇬🇧 English](README.md) · [🇮🇷 فارسی](README_FA.md) · [⬇️ Download](DOWNLOAD.md)

</div>
