//! Metadata models returned to the UI.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VideoEntry {
    /// 1-based position. Playlist order, or oldest→newest for channels.
    pub index: u32,
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub url: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "kind")]
pub enum InfoResult {
    Video {
        id: String,
        title: String,
        duration: Option<f64>,
        thumbnail: Option<String>,
        uploader: Option<String>,
        /// Available video heights, descending.
        heights: Vec<u32>,
    },
    Collection {
        coll_type: String, // "playlist" | "channel"
        id: String,
        title: String,
        uploader: Option<String>,
        thumbnail: Option<String>,
        entries: Vec<VideoEntry>,
        truncated: bool,
    },
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InfoProgress {
    pub message: String,
}

/* ---------------------------------------------------------------------------
 * Raw shape of `yt-dlp --dump-single-json`
 *
 * Only the fields we consume are declared. Parsing into these structs instead
 * of `serde_json::Value` keeps memory proportional to the data we keep: a
 * 5000-entry channel used to build a full `Value` tree (one map per entry with
 * every yt-dlp key) before being reduced to the handful of fields below.
 * ------------------------------------------------------------------------- */

#[derive(Deserialize, Debug, Default)]
pub(crate) struct RawInfo {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub uploader: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub formats: Option<Vec<RawFormat>>,
    #[serde(default)]
    pub thumbnails: Option<Vec<RawThumbnail>>,
    /// `None` entries appear when a video has been removed.
    #[serde(default)]
    pub entries: Option<Vec<Option<RawEntry>>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct RawFormat {
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub vcodec: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct RawThumbnail {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct RawEntry {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub thumbnails: Option<Vec<RawThumbnail>>,
}
