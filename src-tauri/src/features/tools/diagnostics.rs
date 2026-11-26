//! Self-test for the YouTube connection.
//!
//! "It works sometimes" is hard to act on from an error message that only
//! appears after a download fails. This probe asks YouTube for one known video
//! with exactly the configuration a real download would use, then reports
//! which layer is at fault: the helper binaries, the network/proxy, or
//! YouTube's bot check.

use serde::Serialize;
use tauri::AppHandle;

use crate::core::config::ProxySetting;
use crate::core::process::async_command;
use crate::features::tools::ToolManager;
use crate::features::ytdlp::{classify, cookie_args, Failure};

/// A short, public, long-lived video. Only its id is printed.
const PROBE_URL: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionReport {
    /// The probe reached YouTube and got video information back.
    pub ok: bool,
    /// "ok" | "botCheck" | "network" | "tools" | "unavailable" | "unknown".
    pub kind: String,
    /// Actionable sentence shown in Settings.
    pub message: String,
    pub ytdlp_version: Option<String>,
    /// Days since the installed yt-dlp was released; YouTube breaks older
    /// builds long before anyone notices.
    pub ytdlp_age_days: Option<i64>,
}

impl ConnectionReport {
    fn failure(kind: Failure, message: String) -> Self {
        Self {
            ok: false,
            kind: kind_name(kind).to_string(),
            message,
            ytdlp_version: None,
            ytdlp_age_days: None,
        }
    }
}

fn kind_name(failure: Failure) -> &'static str {
    match failure {
        Failure::BotCheck => "botCheck",
        Failure::Degraded => "degraded",
        Failure::Unavailable => "unavailable",
        Failure::Network => "network",
        Failure::Tool => "tools",
        Failure::Unknown => "unknown",
    }
}

/// Days between a yt-dlp `YYYY.MM.DD` version and today.
fn version_age_days(version: &str) -> Option<i64> {
    let date = chrono::NaiveDate::parse_from_str(version.trim(), "%Y.%m.%d").ok()?;
    Some((chrono::Utc::now().date_naive() - date).num_days())
}

pub async fn probe(app: &AppHandle, tools: &ToolManager) -> ConnectionReport {
    let paths = tools.paths();
    if !paths.ytdlp.is_file() {
        return ConnectionReport::failure(
            Failure::Tool,
            "yt-dlp is not installed. Use \"Install tools\" first.".to_string(),
        );
    }

    let settings = crate::core::state::config_opt(app)
        .map(|config| config.get())
        .unwrap_or_default();

    let mut command = async_command(&paths.ytdlp);
    command.args([
        "--no-warnings",
        "--no-colors",
        "--no-update",
        "--socket-timeout",
        "25",
        "--simulate",
        "--print",
        "%(id)s",
    ]);
    command.args(cookie_args(&settings));

    match settings.proxy_setting() {
        ProxySetting::Url(proxy) => {
            command.args(["--proxy", &proxy]);
        }
        ProxySetting::Direct => {
            command.args(["--proxy", ""]);
        }
        ProxySetting::Inherit => {}
    }
    command.arg(PROBE_URL);

    let output = command.output().await;
    let (success, stdout, stderr) = match output {
        Ok(output) => (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ),
        Err(error) => {
            return ConnectionReport::failure(
                Failure::Tool,
                format!("yt-dlp could not be started: {error}"),
            )
        }
    };

    let version = tools.ytdlp_version();
    let age = version.as_deref().and_then(version_age_days);

    if success && stdout.trim().lines().any(|line| !line.trim().is_empty()) {
        return ConnectionReport {
            ok: true,
            kind: "ok".to_string(),
            message: "YouTube answered and returned video information.".to_string(),
            ytdlp_version: version,
            ytdlp_age_days: age,
        };
    }

    let line = crate::features::ytdlp::best_error_line(
        &stderr.lines().map(|line| line.trim().to_string()).collect::<Vec<_>>(),
    );
    let failure = classify(&line);
    let mut message = if line.is_empty() {
        "yt-dlp failed without printing a reason.".to_string()
    } else {
        crate::features::ytdlp::friendly(&line)
    };

    if failure == Failure::BotCheck {
        message.push_str(" Cookies from a signed-in browser are the most reliable fix.");
    }
    if age.unwrap_or(0) > 90 {
        message.push_str(&format!(
            " The installed yt-dlp is {} days old; updating it often clears YouTube errors.",
            age.unwrap_or(0)
        ));
    }

    ConnectionReport {
        ok: false,
        kind: kind_name(failure).to_string(),
        message,
        ytdlp_version: version,
        ytdlp_age_days: age,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_version_age() {
        // A fixed far-future date cannot be compared to "today" reliably, but
        // the parser must at least accept the release format and reject noise.
        assert!(version_age_days("2026.01.01").is_some());
        assert!(version_age_days("nightly").is_none());
        assert!(version_age_days("").is_none());
    }

    #[test]
    fn reports_the_bot_check_as_its_own_kind() {
        assert_eq!(kind_name(Failure::BotCheck), "botCheck");
        assert_eq!(kind_name(Failure::Network), "network");
    }
}
