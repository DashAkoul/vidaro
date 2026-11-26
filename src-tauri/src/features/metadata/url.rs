//! YouTube URL classification.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlKind {
    Video,
    Playlist,
    Channel,
}

impl UrlKind {
    pub fn as_str(self) -> &'static str {
        match self {
            UrlKind::Video => "video",
            UrlKind::Playlist => "playlist",
            UrlKind::Channel => "channel",
        }
    }
}

/// Strict check used by the clipboard watcher — only real YouTube hosts.
pub fn is_youtube_url(input: &str) -> bool {
    let url = input.trim().to_lowercase();
    const HOSTS: [&str; 8] = [
        "https://www.youtube.com/",
        "http://www.youtube.com/",
        "https://youtube.com/",
        "http://youtube.com/",
        "https://m.youtube.com/",
        "https://youtu.be/",
        "http://youtu.be/",
        "https://music.youtube.com/",
    ];
    HOSTS.iter().any(|prefix| url.starts_with(prefix))
}

/// What the UI can show about a link before anything is fetched.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    /// "video" | "playlist" | "channel"
    pub kind: String,
    pub video_id: Option<String>,
}

/// Recognise a link locally.
///
/// This is what makes pasting a link feel instant: no process is spawned and
/// no request is made, so the UI can say "single video / playlist / channel"
/// while yt-dlp is still starting up.
pub fn detect(url: &str) -> Option<Detection> {
    let normalized = normalize_url(url)?;
    let kind = classify_url(&normalized);
    Some(Detection {
        kind: kind.as_str().to_string(),
        video_id: if kind == UrlKind::Video {
            video_id(&normalized)
        } else {
            None
        },
    })
}

/// Pull the 11-character video id out of any YouTube URL shape.
///
/// The id is case-sensitive, so the URL is searched in lower case but the id
/// itself is sliced from the original text.
pub fn video_id(url: &str) -> Option<String> {
    let lower = url.to_ascii_lowercase();
    if !(lower.contains("youtube.com")
        || lower.contains("youtu.be")
        || lower.contains("youtube-nocookie.com"))
    {
        return None;
    }

    if let Some(index) = lower.find("v=") {
        let preceded_by_param = lower[..index]
            .chars()
            .next_back()
            .map(|c| c == '?' || c == '&')
            .unwrap_or(false);
        if preceded_by_param {
            if let Some(id) = take_id(url, index + 2) {
                return Some(id);
            }
        }
    }

    for marker in ["youtu.be/", "/shorts/", "/embed/", "/live/", "/v/"] {
        if let Some(index) = lower.find(marker) {
            if let Some(id) = take_id(url, index + marker.len()) {
                return Some(id);
            }
        }
    }
    None
}

/// Slice an id starting at `start` and stop at the first delimiter.
fn take_id(url: &str, start: usize) -> Option<String> {
    let rest = url.get(start..)?;
    let end = rest
        .find(|c: char| c == '?' || c == '&' || c == '#' || c == '/')
        .unwrap_or(rest.len());
    let id = &rest[..end];
    if is_video_id(id) {
        Some(id.to_string())
    } else {
        None
    }
}

fn is_video_id(value: &str) -> bool {
    value.len() == 11 && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn classify_url(url: &str) -> UrlKind {
    let lowered = url.to_lowercase();
    let is_youtube = lowered.contains("youtube.com")
        || lowered.contains("youtu.be")
        || lowered.contains("youtube-nocookie.com");

    if !is_youtube {
        // yt-dlp supports many other sites; treat them as single videos.
        return UrlKind::Video;
    }

    if lowered.contains("youtube.com/@")
        || lowered.contains("youtube.com/channel/")
        || lowered.contains("youtube.com/c/")
        || lowered.contains("youtube.com/user/")
    {
        return UrlKind::Channel;
    }

    if lowered.contains("list=") {
        return UrlKind::Playlist;
    }

    UrlKind::Video
}

/// Point a channel URL at its "Videos" tab so we never pull Shorts or streams.
pub fn normalize_channel_url(url: &str) -> String {
    let base = url
        .split('?')
        .next()
        .unwrap_or(url)
        .trim_end_matches('/');

    const TABS: [&str; 6] = [
        "/videos",
        "/shorts",
        "/streams",
        "/featured",
        "/playlists",
        "/community",
    ];

    if let Some((head, _)) = base.rsplit_once('/') {
        if TABS.iter().any(|tab| base.ends_with(tab)) {
            return format!("{head}/videos");
        }
    }
    format!("{base}/videos")
}

/// Only absolute http(s) URLs may be handed to yt-dlp.
///
/// This is a security boundary, not a convenience check. yt-dlp parses its
/// trailing argument as a URL *or* as an option, so a value starting with `-`
/// (for example `--exec=calc`) would be executed as a command-line flag. The
/// URL can arrive from the browser extension over native messaging, so it is
/// attacker-influenced input.
pub fn is_http_url(input: &str) -> bool {
    let value = input.trim();

    if value.is_empty() || value.len() > 4096 || value.starts_with('-') {
        return false;
    }
    // A raw space means the value is not a single URL.
    if value.chars().any(char::is_whitespace) {
        return false;
    }

    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Normalise user input into an absolute http(s) URL.
///
/// Accepts a bare host (`youtube.com/watch?v=x`) because people paste those,
/// but never returns anything yt-dlp could read as an option.
pub fn normalize_url(input: &str) -> Option<String> {
    let value = input.trim();

    if value.is_empty() || value.len() > 4096 || value.starts_with('-') {
        return None;
    }
    if value.chars().any(char::is_whitespace) {
        return None;
    }

    let lower = value.to_ascii_lowercase();
    let candidate = if lower.starts_with("http://") || lower.starts_with("https://") {
        value.to_string()
    } else if value.contains('.') {
        // Looks like a host name; assume https.
        format!("https://{value}")
    } else {
        return None;
    };

    if is_http_url(&candidate) {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_urls() {
        assert_eq!(classify_url("https://www.youtube.com/@MrBeast"), UrlKind::Channel);
        assert_eq!(
            classify_url("https://www.youtube.com/playlist?list=PL123"),
            UrlKind::Playlist
        );
        assert_eq!(
            classify_url("https://www.youtube.com/watch?v=abc&list=PL123"),
            UrlKind::Playlist
        );
        assert_eq!(classify_url("https://www.youtube.com/watch?v=abc"), UrlKind::Video);
        assert_eq!(classify_url("https://youtu.be/abc"), UrlKind::Video);
    }

    #[test]
    fn normalizes_channel_tabs() {
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/@foo/"),
            "https://www.youtube.com/@foo/videos"
        );
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/@foo/shorts"),
            "https://www.youtube.com/@foo/videos"
        );
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/@foo/videos"),
            "https://www.youtube.com/@foo/videos"
        );
    }

    #[test]
    fn clipboard_check_is_strict() {
        assert!(is_youtube_url("https://youtu.be/abc"));
        assert!(!is_youtube_url("https://evil.com/youtube.com/watch"));
        assert!(!is_youtube_url("not a url"));
    }

    #[test]
    fn detects_video_ids_in_every_url_shape() {
        for url in [
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            "https://www.youtube.com/watch?list=PL123&v=jNQXAC9IVRw&t=10s",
            "https://youtu.be/jNQXAC9IVRw?t=5",
            "https://www.youtube.com/shorts/jNQXAC9IVRw",
            "https://m.youtube.com/watch?v=jNQXAC9IVRw",
            "https://music.youtube.com/watch?v=jNQXAC9IVRw",
            "https://www.youtube.com/embed/jNQXAC9IVRw",
            "https://www.youtube.com/live/jNQXAC9IVRw",
        ] {
            assert_eq!(video_id(url).as_deref(), Some("jNQXAC9IVRw"), "{url}");
        }
        assert_eq!(video_id("https://example.com/watch?v=jNQXAC9IVRw"), None);
        assert_eq!(video_id("https://www.youtube.com/watch?v=short"), None);
    }

    #[test]
    fn detection_is_local_and_typed() {
        let video = detect("https://youtu.be/jNQXAC9IVRw").unwrap();
        assert_eq!(video.kind, "video");
        assert_eq!(video.video_id.as_deref(), Some("jNQXAC9IVRw"));

        let playlist = detect("https://www.youtube.com/playlist?list=PL123").unwrap();
        assert_eq!(playlist.kind, "playlist");
        assert_eq!(playlist.video_id, None);

        let channel = detect("https://www.youtube.com/@someone").unwrap();
        assert_eq!(channel.kind, "channel");

        assert!(detect("not a url").is_none());
    }

    #[test]
    fn rejects_option_injection() {
        assert!(is_http_url("https://www.youtube.com/watch?v=abc"));
        assert!(is_http_url("  http://example.com/x  "));

        // Anything yt-dlp could read as a flag must be rejected.
        assert!(!is_http_url("--exec=calc"));
        assert!(!is_http_url("-o/tmp/x"));
        assert!(!is_http_url("file:///etc/passwd"));
        assert!(!is_http_url("https://example.com/a b"));
        assert!(!is_http_url(""));
    }

    #[test]
    fn normalizes_bare_hosts() {
        assert_eq!(
            normalize_url("youtube.com/watch?v=abc").as_deref(),
            Some("https://youtube.com/watch?v=abc")
        );
        assert_eq!(
            normalize_url("  https://youtu.be/abc  ").as_deref(),
            Some("https://youtu.be/abc")
        );

        assert!(normalize_url("--exec=calc").is_none());
        assert!(normalize_url("-rf /").is_none());
        assert!(normalize_url("notaurl").is_none());
        assert!(normalize_url("").is_none());
    }
}
