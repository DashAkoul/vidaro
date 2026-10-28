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
#[serde(rename_all = "camelCase", tag = "kind")]
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
