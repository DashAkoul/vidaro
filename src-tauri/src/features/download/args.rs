//! Building the yt-dlp command line for one queue item.

use std::path::Path;

use crate::core::config::Settings;
use crate::features::download::model::Item;
use crate::features::tools::paths::ToolPaths;

/// Maximum filename length yt-dlp may produce. Windows still caps the full
/// path at 260 characters on older builds, so leave headroom.
const MAX_FILENAME_LEN: u32 = 150;

pub fn build(item: &Item, settings: &Settings, tools: &ToolPaths) -> Vec<String> {
    let mut args: Vec<String> = [
        "--newline",
        "--no-playlist",
        "--continue",
        "--no-overwrites",
        "--no-colors",
        "--retries",
        "5",
        "--fragment-retries",
        "5",
        "--socket-timeout",
        "30",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    args.push("--trim-filenames".into());
    args.push(MAX_FILENAME_LEN.to_string());

    if cfg!(windows) {
        // Strips characters Windows rejects and avoids reserved device names.
        args.push("--windows-filenames".into());
    }

    args.push("--ffmpeg-location".into());
    args.push(tools.ffmpeg_dir().to_string_lossy().to_string());

    if let Some(proxy) = settings.proxy_url() {
        args.push("--proxy".into());
        args.push(proxy);
    }

    if item.audio_only {
        // Ask for the best audio stream directly instead of downloading video
        // and throwing the video track away afterwards.
        args.push("-f".into());
        args.push("ba/b".into());
        args.push("-x".into());
        args.push("--audio-format".into());
        args.push("mp3".into());
        args.push("--audio-quality".into());
        args.push("0".into());
        if settings.embed_thumbnail {
            args.push("--embed-thumbnail".into());
        }
        if settings.embed_metadata {
            args.push("--add-metadata".into());
        }
    } else {
        let selector = match item.quality.parse::<u32>() {
            Ok(height) if item.quality != "best" => {
                // Final `/b` fallback keeps the download working when a video
                // has no separate streams at that resolution.
                format!("bv*[height<={height}]+ba/b[height<={height}]/b")
            }
            _ => "bv*+ba/b".to_string(),
        };
        args.push("-f".into());
        args.push(selector);
        args.push("--merge-output-format".into());
        args.push("mp4".into());

        if settings.write_subs {
            args.push("--write-subs".into());
            args.push("--write-auto-subs".into());
            args.push("--sub-langs".into());
            args.push(settings.sub_langs.clone());
            args.push("--convert-subs".into());
            args.push("srt".into());
        }
    }

    let rate_limit = settings.rate_limit.trim();
    if !rate_limit.is_empty() {
        args.push("--limit-rate".into());
        args.push(rate_limit.to_string());
    }

    // `<dir>/<prefix>%(title)s.%(ext)s` — the prefix already carries "003 - ".
    let template = Path::new(&item.dir).join(format!("{}%(title)s.%(ext)s", item.prefix));
    args.push("-o".into());
    args.push(template.to_string_lossy().to_string());

    args.push(item.url.clone());
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::download::model::{Item, ItemStatus, Stage};

    fn sample(audio_only: bool, quality: &str) -> Item {
        Item {
            id: "1".into(),
            url: "https://youtu.be/x".into(),
            video_id: "x".into(),
            title: "T".into(),
            index: Some(3),
            dir: "/tmp/out".into(),
            prefix: "003 - ".into(),
            quality: quality.into(),
            audio_only,
            batch_name: "b".into(),
            status: ItemStatus::Queued,
            percent: 0.0,
            speed_bps: 0.0,
            eta_secs: 0.0,
            total_bytes: 0,
            downloaded_bytes: 0,
            stage: Stage::Download,
            error: None,
            output_path: None,
            created_at: 0,
            schedule_start: None,
            schedule_end: None,
            paused_by: None,
        }
    }

    fn tools() -> ToolPaths {
        ToolPaths {
            ytdlp: "/tmp/yt-dlp".into(),
            ffmpeg: "/tmp/ffmpeg".into(),
            ffprobe: "/tmp/ffprobe".into(),
        }
    }

    #[test]
    fn video_quality_has_fallback() {
        let args = build(&sample(false, "1080"), &Settings::default(), &tools());
        let selector = args.iter().position(|a| a == "-f").map(|i| args[i + 1].clone());
        assert_eq!(
            selector.as_deref(),
            Some("bv*[height<=1080]+ba/b[height<=1080]/b")
        );
    }

    #[test]
    fn audio_prefers_best_audio() {
        let args = build(&sample(true, "best"), &Settings::default(), &tools());
        assert!(args.iter().any(|a| a == "-x"));
        assert!(args.windows(2).any(|w| w[0] == "--audio-format" && w[1] == "mp3"));
    }

    #[test]
    fn output_template_keeps_prefix() {
        let args = build(&sample(false, "best"), &Settings::default(), &tools());
        let template = args.iter().position(|a| a == "-o").map(|i| args[i + 1].clone());
        assert!(template.unwrap().contains("003 - %(title)s.%(ext)s"));
    }
}
