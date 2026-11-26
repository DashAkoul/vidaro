//! Shared knowledge about the yt-dlp helper.
//!
//! yt-dlp reports failures as free-form text on stderr, and the same sentence
//! is read by two very different callers: the metadata fetch (which shows it
//! immediately) and the download queue (which decides whether the failure is
//! worth retrying). Keeping the interpretation in one place means the UI and
//! the retry policy can never disagree about what an error means.

/// What kind of failure yt-dlp reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// YouTube wants a signed-in session for this IP. Temporary, and the only
    /// reliable cure is cookies or a different egress IP.
    BotCheck,
    /// The video exists but YouTube answered with a degraded player response.
    Degraded,
    /// Deleted, private, region-blocked or otherwise gone.
    Unavailable,
    /// Transport problem: DNS, TLS, proxy, timeout, reset.
    Network,
    /// yt-dlp itself is missing, too old, or crashed.
    Tool,
    /// Anything else.
    Unknown,
}

/// Classify a yt-dlp error line.
///
/// Markers are matched case-insensitively against the raw text, which is what
/// yt-dlp actually prints. Order matters: a bot check often arrives wrapped in
/// a generic `[youtube] ...: Unable to extract ...` line.
pub fn classify(raw: &str) -> Failure {
    let text = raw.to_lowercase();

    const BOT: [&str; 6] = [
        "sign in to confirm",
        "not a bot",
        "confirm you’re not a bot",
        "confirm you're not a bot",
        "use --cookies",
        "login_required",
    ];
    if BOT.iter().any(|marker| text.contains(marker)) {
        return Failure::BotCheck;
    }

    const DEGRADED: [&str; 4] = [
        "page needs to be reloaded",
        "no video formats found",
        "unable to extract",
        "requested format is not available",
    ];
    if DEGRADED.iter().any(|marker| text.contains(marker)) {
        return Failure::Degraded;
    }

    const UNAVAILABLE: [&str; 7] = [
        "video unavailable",
        "this video is unavailable",
        "has been removed",
        "private video",
        "members-only",
        "not available in your country",
        "blocked it in your country",
    ];
    if UNAVAILABLE.iter().any(|marker| text.contains(marker)) {
        return Failure::Unavailable;
    }

    const NETWORK: [&str; 12] = [
        "failed to resolve",
        "getaddrinfo",
        "connection reset",
        "connection refused",
        "actively refused",
        "connection could not be made",
        "timed out",
        "timeout",
        "ssl",
        "certificate",
        "proxy",
        "unreachable",
    ];
    if NETWORK.iter().any(|marker| text.contains(marker)) {
        return Failure::Network;
    }

    const TOOL: [&str; 5] = [
        "no such file",
        "is not installed",
        "could not start yt-dlp",
        "unsupported url",
        "python",
    ];
    if TOOL.iter().any(|marker| text.contains(marker)) {
        return Failure::Tool;
    }

    Failure::Unknown
}

/// True when waiting and trying again can succeed.
///
/// Bot checks, degraded player responses and transport errors are all
/// transient: the same request usually works minutes later, which is exactly
/// why the app used to look like it "works sometimes".
pub fn is_transient(raw: &str) -> bool {
    matches!(
        classify(raw),
        Failure::BotCheck | Failure::Degraded | Failure::Network
    )
}

/// Strip yt-dlp's decorations so the UI shows a readable sentence.
pub fn clean(raw: &str) -> String {
    let mut text = raw.trim();
    if let Some(rest) = text.strip_prefix("ERROR: ") {
        text = rest;
    }
    if text.starts_with('[') {
        if let Some(bracket) = text.find("] ") {
            let rest = &text[bracket + 2..];
            text = match rest.find(": ") {
                Some(colon) if !rest[..colon].contains(' ') => &rest[colon + 2..],
                _ => rest,
            };
        }
    }
    text.trim().to_string()
}

/// Pick the most useful diagnostic out of yt-dlp's stderr tail.
///
/// An `ERROR:` always wins; a `WARNING:` that mentions a bot challenge wins
/// over unrelated progress chatter; otherwise the last meaningful line is
/// better than a generic "could not read this URL".
pub fn best_error_line(lines: &[String]) -> String {
    if let Some(line) = lines
        .iter()
        .rev()
        .find(|line| line.starts_with("ERROR:") || is_transient(line))
    {
        return line.clone();
    }
    if let Some(line) = lines.iter().rev().find(|line| line.starts_with("WARNING:")) {
        return line.clone();
    }
    lines
        .iter()
        .rev()
        .find(|line| !line.starts_with("[download]"))
        .cloned()
        .unwrap_or_default()
}

/// Turn a raw yt-dlp failure into a sentence that says what to do next.
///
/// Every branch deliberately keeps the words that make the text recognisable
/// to [`is_transient`] ("not a bot", "not available"), so a friendly message
/// can still be fed back into the retry policy.
pub fn friendly(raw: &str) -> String {
    let message = clean(raw);

    match classify(&message) {
        Failure::BotCheck => "YouTube asked to confirm you are not a bot. This is a block on your network/IP, not on the video. Add cookies in Settings → YouTube account, switch to another VPN or proxy server, or wait a few minutes — Vidaro will retry automatically."
            .to_string(),
        Failure::Unavailable => "This video is unavailable — it may be deleted, private, or blocked in your region."
            .to_string(),
        Failure::Tool => {
            "yt-dlp could not run. Open Settings → Download tools and reinstall it."
                .to_string()
        }
        Failure::Network => format!(
            "{} — the connection failed. Check your proxy/VPN or try again.",
            shorten(&message, 200)
        ),
        Failure::Degraded => format!(
            "{} — YouTube answered with an incomplete player response. This is usually temporary.",
            shorten(&message, 200)
        ),
        Failure::Unknown => shorten(&message, 300),
    }
}

/// Trim a technical sentence to something a queue row can display.
///
/// yt-dlp nests transport errors (`TransportError(...)` inside `HTTPConnection(...)`),
/// which produced 400-character rows that told the user nothing new.
fn shorten(message: &str, limit: usize) -> String {
    if message.chars().count() <= limit {
        return message.to_string();
    }
    let cut: String = message.chars().take(limit).collect();
    // Prefer a word boundary so the text does not end mid-word.
    let trimmed = match cut.rfind(' ') {
        Some(index) if index > limit / 2 => &cut[..index],
        _ => &cut[..],
    };
    format!("{}…", trimmed.trim_end_matches([' ', ',', '.']))
}

/// Arguments that give yt-dlp a signed-in YouTube session.
///
/// YouTube's "confirm you're not a bot" wall is tied to the IP and to the
/// absence of a session. Cookies are the only workaround that works without
/// changing the network, so both the metadata fetch and the download pass the
/// same flags. A missing cookie file is ignored rather than failing the whole
/// command: the user may have deleted the export.
pub fn cookie_args(settings: &crate::core::config::Settings) -> Vec<String> {
    match settings.cookies_mode.as_str() {
        "browser" if !settings.cookies_browser.trim().is_empty() => vec![
            "--cookies-from-browser".to_string(),
            settings.cookies_browser.trim().to_string(),
        ],
        "file" => {
            let path = settings.cookies_file.trim();
            if path.is_empty() || !std::path::Path::new(path).is_file() {
                return Vec::new();
            }
            vec!["--cookies".to_string(), path.to_string()]
        }
        _ => Vec::new(),
    }
}

/// Player clients tried in order after a transient failure.
///
/// The default client is what yt-dlp picks; the rest are alternates that
/// sometimes succeed when YouTube is throttling one of them. Rotating on retry
/// costs nothing and turns some hard failures into a completed download.
pub const ALTERNATE_CLIENTS: [&str; 4] = ["web_safari", "tv", "android_vr", "mweb"];

/// Which alternate client a retry attempt should use, if any.
///
/// `attempt` is 1 for the first retry. Returns `None` when the rotation is
/// exhausted, so the caller falls back to yt-dlp's own default.
pub fn alternate_client(attempt: u32) -> Option<&'static str> {
    let index = (attempt as usize).checked_sub(1)?;
    ALTERNATE_CLIENTS.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_bot_check() {
        assert_eq!(
            classify("ERROR: [youtube] x: Sign in to confirm you’re not a bot. Use --cookies"),
            Failure::BotCheck
        );
        assert!(is_transient("Sign in to confirm you are not a bot"));
        assert!(is_transient(&friendly("Sign in to confirm you are not a bot")));
    }

    #[test]
    fn unavailable_is_not_retried() {
        assert_eq!(classify("This video is unavailable"), Failure::Unavailable);
        assert!(!is_transient("This video is unavailable"));
    }

    #[test]
    fn network_errors_are_retried() {
        assert_eq!(classify("Connection reset by peer"), Failure::Network);
        assert!(is_transient("Proxy CONNECT failed: timed out"));
    }

    #[test]
    fn degraded_player_is_retried() {
        assert_eq!(classify("The page needs to be reloaded."), Failure::Degraded);
        assert!(is_transient("No video formats found"));
    }

    #[test]
    fn cleans_ytdlp_prefixes() {
        assert_eq!(
            clean("ERROR: [youtube] cXeLjuzQglg: This video is unavailable"),
            "This video is unavailable"
        );
        assert_eq!(clean("ERROR: something broke"), "something broke");
        assert_eq!(clean("plain message"), "plain message");
    }

    #[test]
    fn bot_check_wins_over_generic_wrapper() {
        let lines: Vec<String> = vec![
            "[youtube] x: Downloading player API JSON".into(),
            "WARNING: [youtube] Unable to extract nsig: Sign in to confirm you’re not a bot".into(),
        ];
        let picked = best_error_line(&lines);
        assert!(classify(&picked) == Failure::BotCheck, "{picked}");
    }

    #[test]
    fn empty_stderr_returns_empty() {
        assert_eq!(best_error_line(&[]), "");
    }

    #[test]
    fn long_transport_errors_are_shortened() {
        let raw = format!(
            "{} (caused by TransportError(\"{}\"))",
            "Unable to download webpage: HTTPConnection reported a failure".repeat(3),
            "No connection could be made because the target machine actively refused it".repeat(2)
        );
        let friendly = friendly(&raw);
        assert!(friendly.chars().count() < 330, "{} chars", friendly.chars().count());
        assert!(friendly.contains('…'));
        // Shortening must not destroy the classification.
        assert!(!friendly.is_empty());
    }

    #[test]
    fn alternate_clients_rotate_then_stop() {
        assert_eq!(alternate_client(0), None);
        assert_eq!(alternate_client(1), Some("web_safari"));
        assert_eq!(alternate_client(4), Some("mweb"));
        assert_eq!(alternate_client(5), None);
    }
}
