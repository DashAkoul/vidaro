//! YouTube URL classification.

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
}
