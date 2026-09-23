use regex::Regex;

/// Remove characters that are illegal in file/folder names on Windows/macOS.
pub fn sanitize_name(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if (c as u32) < 32 => ' ',
            c => c,
        })
        .collect();
    // Windows does not allow names ending with dots or spaces
    while s.ends_with('.') || s.ends_with(' ') {
        s.pop();
    }
    let s = s.trim();
    if s.is_empty() {
        "untitled".to_string()
    } else {
        s.to_string()
    }
}

/// Parse a human size like "123.45MiB" into bytes.
pub fn parse_size(value: f64, unit: &str) -> f64 {
    match unit {
        "KiB" => value * 1024.0,
        "MiB" => value * 1024.0 * 1024.0,
        "GiB" => value * 1024.0 * 1024.0 * 1024.0,
        "TiB" => value * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "KB" => value * 1000.0,
        "MB" => value * 1000.0 * 1000.0,
        "GB" => value * 1000.0 * 1000.0 * 1000.0,
        _ => value,
    }
}

/// Parse "00:32" / "1:02:03" / "Unknown" into seconds.
pub fn parse_eta(s: &str) -> f64 {
    let parts: Vec<f64> = s
        .split(':')
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect();
    if parts.is_empty() {
        return -1.0;
    }
    parts.iter().fold(0.0, |acc, p| acc * 60.0 + p)
}

/// Strip ANSI escape sequences (yt-dlp may emit color codes).
pub fn strip_ansi(line: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(line, "").to_string()
}

/// Classify a YouTube URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlKind {
    Video,
    Playlist,
    Channel,
}

/// Quick check used by the clipboard watcher (exact-match domains only).
pub fn is_youtube_url(url: &str) -> bool {
    let u = url.to_lowercase();
    u.starts_with("https://www.youtube.com/")
        || u.starts_with("http://www.youtube.com/")
        || u.starts_with("https://youtube.com/")
        || u.starts_with("http://youtube.com/")
        || u.starts_with("https://m.youtube.com/")
        || u.starts_with("https://youtu.be/")
        || u.starts_with("http://youtu.be/")
        || u.starts_with("https://music.youtube.com/")
}

pub fn classify_url(url: &str) -> UrlKind {
    let u = url.to_lowercase();
    let is_youtube =
        u.contains("youtube.com") || u.contains("youtu.be") || u.contains("youtube-nocookie.com");
    if !is_youtube {
        // let yt-dlp decide (other supported sites behave like single videos)
        return UrlKind::Video;
    }
    if u.contains("youtube.com/@")
        || u.contains("youtube.com/channel/")
        || u.contains("youtube.com/c/")
        || u.contains("youtube.com/user/")
        || u.contains("/@")
    {
        return UrlKind::Channel;
    }
    if u.contains("list=") {
        return UrlKind::Playlist;
    }
    UrlKind::Video
}

/// Normalize a channel URL to point to its "Videos" tab only.
pub fn normalize_channel_url(url: &str) -> String {
    let base = url.split('?').next().unwrap_or(url).trim_end_matches('/');
    if base.ends_with("/videos")
        || base.ends_with("/shorts")
        || base.ends_with("/streams")
        || base.ends_with("/featured")
        || base.ends_with("/playlists")
        || base.ends_with("/community")
    {
        format!("{}/videos", base.rsplit_once('/').map(|(b, _)| b).unwrap_or(base))
    } else {
        format!("{}/videos", base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize() {
        assert_eq!(sanitize_name("a/b:c*?\"<>|"), "a b c");
        assert_eq!(sanitize_name("name.."), "name");
    }

    #[test]
    fn classify() {
        assert_eq!(classify_url("https://www.youtube.com/@MrBeast"), UrlKind::Channel);
        assert_eq!(
            classify_url("https://www.youtube.com/playlist?list=PL123"),
            UrlKind::Playlist
        );
        assert_eq!(
            classify_url("https://www.youtube.com/watch?v=abc&list=PL123"),
            UrlKind::Playlist
        );
        assert_eq!(
            classify_url("https://www.youtube.com/watch?v=abc"),
            UrlKind::Video
        );
        assert_eq!(classify_url("https://youtu.be/abc"), UrlKind::Video);
    }

    #[test]
    fn normalize() {
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/@foo/"),
            "https://www.youtube.com/@foo/videos"
        );
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/@foo/videos"),
            "https://www.youtube.com/@foo/videos"
        );
        assert_eq!(
            normalize_channel_url("https://www.youtube.com/@foo/shorts"),
            "https://www.youtube.com/@foo/videos"
        );
    }
}
