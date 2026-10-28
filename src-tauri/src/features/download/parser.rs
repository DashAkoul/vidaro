//! Parsing yt-dlp's stdout/stderr.

use std::borrow::Cow;
use std::sync::OnceLock;

use regex::Regex;

use crate::features::download::model::Stage;

fn ansi_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Compiled once. The previous implementation recompiled this regex for
    // every single line of yt-dlp output.
    RE.get_or_init(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("valid ansi regex"))
}

pub fn strip_ansi(input: &str) -> Cow<'_, str> {
    if input.contains('\x1b') {
        ansi_regex().replace_all(input, "")
    } else {
        Cow::Borrowed(input)
    }
}

#[derive(Debug)]
pub struct Progress {
    pub percent: f64,
    pub speed_bps: Option<f64>,
    pub eta_secs: Option<f64>,
    pub total_bytes: Option<u64>,
}

/// A single meaningful event extracted from one output line.
#[derive(Debug)]
pub enum LineEvent {
    Error(String),
    /// A file path became known, optionally advancing the stage.
    OutputPath {
        path: String,
        stage: Option<Stage>,
    },
    Stage(Stage),
    Progress(Progress),
}

/// Interpret one line of yt-dlp output.
pub fn classify(raw: &str) -> Option<LineEvent> {
    let cleaned = strip_ansi(raw);
    let line = cleaned.trim();
    if line.is_empty() {
        return None;
    }

    if let Some(msg) = line.strip_prefix("ERROR:") {
        return Some(LineEvent::Error(msg.trim().to_string()));
    }

    if let Some(rest) = line.strip_prefix("[download] Destination:") {
        return Some(LineEvent::OutputPath {
            path: rest.trim().to_string(),
            stage: None,
        });
    }

    if line.contains("has already been downloaded") {
        if let Some(rest) = line.strip_prefix("[download] ") {
            return Some(LineEvent::OutputPath {
                path: rest
                    .replace(" has already been downloaded", "")
                    .trim()
                    .to_string(),
                stage: None,
            });
        }
    }

    if line.starts_with("[Merger]") {
        return Some(match quoted_path(line) {
            Some(path) => LineEvent::OutputPath {
                path,
                stage: Some(Stage::Merge),
            },
            None => LineEvent::Stage(Stage::Merge),
        });
    }

    if line.starts_with("[ExtractAudio]") {
        return Some(match line.strip_prefix("[ExtractAudio] Destination:") {
            Some(rest) => LineEvent::OutputPath {
                path: rest.trim().to_string(),
                stage: Some(Stage::Audio),
            },
            None => LineEvent::Stage(Stage::Audio),
        });
    }

    if line.starts_with("[download]") {
        if let Some(progress) = parse_progress(line) {
            return Some(LineEvent::Progress(progress));
        }
    }

    None
}

/// Extract the first quoted path in a line, e.g. `[Merger] Merging formats into "C:\a.mp4"`.
fn quoted_path(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line.rfind('"')?;
    if end > start {
        Some(line[start + 1..end].to_string())
    } else {
        None
    }
}

/// Parse lines such as:
/// `[download]  45.3% of    5.83MiB at    2.34MiB/s ETA 00:12`
/// `[download]   2.1% of ~  101.83MiB at  411.53KiB/s ETA 04:12`
pub fn parse_progress(line: &str) -> Option<Progress> {
    let pct_index = line.find('%')?;
    let percent: f64 = line[..pct_index]
        .rsplit(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?
        .parse()
        .ok()?;

    let mut progress = Progress {
        percent,
        speed_bps: None,
        eta_secs: None,
        total_bytes: None,
    };

    let Some(rest) = line[pct_index..].strip_prefix("% of") else {
        return Some(progress);
    };

    let rest = rest.trim_start().trim_start_matches('~').trim_start();
    let (size_token, after) = split_token(rest);
    if let Some(bytes) = parse_size(&size_token) {
        progress.total_bytes = Some(bytes);
    }

    let after = after.trim_start();
    if let Some(rest) = after.strip_prefix("at") {
        let rest = rest.trim_start();
        let (speed_token, after) = split_token(rest);
        progress.speed_bps = parse_speed(&speed_token);

        let after = after.trim_start();
        if let Some(rest) = after.strip_prefix("ETA") {
            let (eta_token, _) = split_token(rest.trim_start());
            progress.eta_secs = parse_eta(&eta_token);
        }
    }

    Some(progress)
}

fn split_token(s: &str) -> (String, &str) {
    match s.find(char::is_whitespace) {
        Some(i) => (s[..i].to_string(), &s[i..]),
        None => (s.to_string(), ""),
    }
}

fn unit_multiplier(unit: &str) -> Option<f64> {
    Some(match unit {
        "TiB" => 1024.0_f64.powi(4),
        "GiB" => 1024.0_f64.powi(3),
        "MiB" => 1024.0_f64.powi(2),
        "KiB" => 1024.0,
        "TB" => 1e12,
        "GB" => 1e9,
        "MB" => 1e6,
        "KB" => 1e3,
        _ => return None,
    })
}

fn parse_size(token: &str) -> Option<u64> {
    for unit in ["TiB", "GiB", "MiB", "KiB", "TB", "GB", "MB", "KB"] {
        if let Some(number) = token.strip_suffix(unit) {
            let value: f64 = number.trim().parse().ok()?;
            return Some((value * unit_multiplier(unit)?) as u64);
        }
    }
    None
}

fn parse_speed(token: &str) -> Option<f64> {
    let token = token.trim_end_matches("/s");
    for unit in ["TiB", "GiB", "MiB", "KiB", "GB", "MB", "KB"] {
        if let Some(number) = token.strip_suffix(unit) {
            let value: f64 = number.trim().parse().ok()?;
            return Some(value * unit_multiplier(unit)?);
        }
    }
    token.trim().parse::<f64>().ok()
}

/// "00:32" / "1:02:03" / "Unknown" -> seconds (`-1.0` when unknown).
pub fn parse_eta(token: &str) -> Option<f64> {
    let parts: Vec<f64> = token
        .split(':')
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect();
    if parts.is_empty() {
        return None;
    }
    Some(parts.iter().fold(0.0, |acc, p| acc * 60.0 + p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_progress() {
        let p = parse_progress("[download]  45.3% of    5.83MiB at    2.34MiB/s ETA 00:12").unwrap();
        assert!((p.percent - 45.3).abs() < 0.01);
        assert_eq!(p.total_bytes, Some((5.83 * 1024.0 * 1024.0) as u64));
        assert!(p.speed_bps.unwrap() > 2.0 * 1024.0 * 1024.0);
        assert_eq!(p.eta_secs, Some(12.0));
    }

    #[test]
    fn parses_approximate_size() {
        let p = parse_progress("[download]   2.1% of ~  101.83MiB at  411.53KiB/s ETA 04:12").unwrap();
        assert!((p.percent - 2.1).abs() < 0.01);
        assert_eq!(p.eta_secs, Some(252.0));
    }

    #[test]
    fn strips_ansi() {
        assert_eq!(strip_ansi("\x1b[0;32mOK\x1b[0m"), "OK");
    }

    #[test]
    fn detects_stage_and_errors() {
        assert!(matches!(
            classify("[Merger] Merging formats into \"C:\\out.mp4\""),
            Some(LineEvent::OutputPath { .. })
        ));
        assert!(matches!(
            classify("ERROR: Video unavailable"),
            Some(LineEvent::Error(_))
        ));
        assert!(classify("").is_none());
    }
}
