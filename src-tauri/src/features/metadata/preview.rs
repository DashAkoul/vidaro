//! Fast, unauthenticated preview of a single video.
//!
//! yt-dlp needs 10–20 seconds to return a video's metadata on a slow link
//! (process start, cookie read, player challenge). oEmbed — the endpoint
//! YouTube uses for embedding — answers in about a second with the title,
//! channel and thumbnail, and needs no session.
//!
//! The preview is emitted immediately so the user sees the link was
//! recognised while the full format list is still being fetched.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::core::config::{ProxySetting, Settings};
use crate::core::events;

#[derive(Deserialize)]
struct OEmbed {
    title: Option<String>,
    author_name: Option<String>,
    thumbnail_url: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub video_id: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
}

/// Ask oEmbed for the video and emit it, ignoring every failure.
///
/// A preview is a nicety: if the request times out or the video has no oEmbed
/// data, the full fetch still arrives a few seconds later.
pub async fn emit(app: &AppHandle, settings: &Settings, video_id: &str) {
    if let Some(preview) = fetch(settings, video_id).await {
        events::emit(app, events::INFO_PREVIEW, preview);
    }
}

async fn fetch(settings: &Settings, video_id: &str) -> Option<Preview> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("Vidaro/2.2 (+https://github.com/DashAkoul/vidaro)");

    match settings.proxy_setting() {
        ProxySetting::Url(url) => {
            builder = builder.proxy(reqwest::Proxy::all(&url).ok()?);
        }
        ProxySetting::Direct => {
            // oEmbed must use the same path as the download; otherwise a proxy
            // user would get a preview from one IP and a download from another.
            builder = builder.no_proxy();
        }
        ProxySetting::Inherit => {}
    }

    let client = builder.build().ok()?;
    let url = format!(
        "https://www.youtube.com/oembed?url={}&format=json",
        urlencoding(&format!("https://www.youtube.com/watch?v={video_id}"))
    );

    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    // `reqwest` is built without its `json` feature; decode the body here.
    let body = response.bytes().await.ok()?;
    let data: OEmbed = serde_json::from_slice(&body).ok()?;

    Some(Preview {
        video_id: video_id.to_string(),
        title: data.title.unwrap_or_else(|| "Untitled".to_string()),
        thumbnail: data.thumbnail_url,
        uploader: data.author_name,
    })
}

/// Minimal percent-encoding for a URL used as a query value.
fn urlencoding(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_the_query_value() {
        assert_eq!(
            urlencoding("https://www.youtube.com/watch?v=abc"),
            "https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3Dabc"
        );
    }
}
