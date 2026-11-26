# Troubleshooting

## "Sometimes it works, sometimes nothing downloads"

This is almost always YouTube's **"Sign in to confirm you're not a bot"**
check, not a bug in how Vidaro reads links.

### How to confirm it

1. Open **Settings → Reliability → Test connection**.
2. The report names the failing layer:
   - `botCheck` — YouTube is challenging this IP.
   - `network` — DNS, TLS or proxy problem.
   - `tools` — yt-dlp/ffmpeg missing or too old.

The same reason is written next to any failed item in the queue.

### Two separate walls

The bot check is only the first. Once a session is accepted, YouTube returns
formats only if its player challenge can be evaluated — and that challenge is
JavaScript:

```
[jsc] JS Challenge Providers: bun (unavailable), deno (unavailable), node (unavailable)
WARNING: n challenge solving failed: Some formats may be missing
ERROR: [youtube] ...: No video formats found!
```

Vidaro handles this on its own: it uses a managed Deno, or any `deno`/`node`/
`bun`/`qjs` already installed, and installs Deno when none is present. The
runtime used is shown in **Settings → Download tools**.

The other half of this failure was the age of the helper itself. Build
`2026.10.26` crashed inside its own challenge solver
(`a.addOnDisposeCallback is not a function`) while `2026.11.23` succeeded. A
yt-dlp older than 30 days is therefore refreshed automatically at startup;
**Update yt-dlp** forces it.

### Why it is not a yt-dlp problem

The challenge is answered by YouTube's servers before any extractor sees the
data. Asking the InnerTube player endpoint directly from a blocked IP returns
the same thing, for every client:

```
TVHTML5     status=LOGIN_REQUIRED "Sign in to confirm you're not a bot"
WEB         status=LOGIN_REQUIRED "Sign in to confirm you're not a bot"
MWEB        status=LOGIN_REQUIRED "Sign in to confirm you're not a bot"
ANDROID_VR  status=LOGIN_REQUIRED "Sign in to confirm you're not a bot"
```

Swapping yt-dlp for a hand-written extractor therefore does not help: it makes
the same request to the same servers and gets the same refusal, while losing
the format handling, signature work and site support yt-dlp provides. What
does help is changing *who* is asking or *how*.

### What actually helps, in order

1. **Send cookies** — Settings → YouTube account → *From browser*, pick the
   browser you are signed into YouTube with. If reading the browser fails
   (Chrome is often locked or DPAPI-encrypted), export a `cookies.txt` with a
   browser extension and choose *cookies.txt* instead. This is the only fix
   that works without changing the network.
2. **Switch proxy/VPN server** — a new egress IP is usually enough. Configure
   it in Settings → Proxy; Vidaro passes it to both metadata fetches and
   downloads.
3. **Wait** — the block is temporary and lifts on its own. Vidaro now retries
   transient failures automatically with a growing delay and a different
   player client, so a block of a few minutes no longer means a failed item.

### Automated retries

When a failure is transient (bot check, degraded player response, network
error) the item returns to the queue instead of the error tab. The retry delay
starts at the configured base (5 minutes by default) and doubles up to a cap,
for at most four attempts. The schedule is stored in `queue.json`, so a pending
retry still happens after the app is restarted.

Retries are not used for permanent failures: a deleted, private or
region-blocked video is reported immediately.

**Retry failed** in the queue toolbar re-queues every errored item at once.

## Downloads finish but the file is "missing"

Before 2.2.0 the final path was scraped out of yt-dlp's log line. On Windows
that line was printed in the console code page, so Persian, Arabic, emoji and
other non-Latin titles were destroyed and the queue pointed at a file that did
not exist. Vidaro now forces UTF-8 for the helper processes and asks yt-dlp for
the path with `--print-to-file after_move:filepath`, which is written as UTF-8
and reflects the real file after a merge rename.

## Tools will not install

Helper downloads are resumable: an interrupted install continues from the
bytes already on disk, and the next launch retries it.


`yt-dlp` and `ffmpeg` are downloaded on first run from their official release
pages. On a filtered network that download can fail. Use **Settings → Download
tools → Install tools** once the network is working, or place the three
binaries (`yt-dlp`, `ffmpeg`, `ffprobe`) in the app data folder under `bin/`.

An old yt-dlp is itself a source of errors. The connection report shows its
age; anything older than 90 days should be updated with **Update yt-dlp**.

## Where the app keeps its state

| Platform | Folder |
| --- | --- |
| Windows | `%APPDATA%\com.akoul.yt-downloader` |
| macOS | `~/Library/Application Support/com.akoul.yt-downloader` |
| Linux | `~/.local/share/com.akoul.yt-downloader` |

`startup.log` records each start and any panic, `queue.json` and
`history.json` hold the queue and finished downloads, and `bin/` holds the
helper binaries.
