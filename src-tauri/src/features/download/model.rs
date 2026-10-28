//! Data model for the download queue.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ItemStatus {
    Queued,
    Active,
    Paused,
    Completed,
    Error,
    Cancelled,
    Scheduled,
}

impl ItemStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, ItemStatus::Completed | ItemStatus::Cancelled)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Download,
    Merge,
    Audio,
    Done,
}

/// Why an item is paused — lets the scheduler resume only what it paused.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PausedBy {
    User,
    Scheduler,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub url: String,
    pub video_id: String,
    pub title: String,
    /// Position inside a playlist/channel; `None` for standalone videos.
    pub index: Option<u32>,
    pub dir: String,
    /// e.g. `"003 - "` (empty for standalone videos).
    pub prefix: String,
    pub quality: String, // "best" | "2160" | "1080" ...
    pub audio_only: bool,
    pub batch_name: String,
    pub status: ItemStatus,
    pub percent: f64,
    pub speed_bps: f64,
    pub eta_secs: f64,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub stage: Stage,
    pub error: Option<String>,
    pub output_path: Option<String>,
    pub created_at: i64,
    pub schedule_start: Option<String>, // "HH:MM"
    pub schedule_end: Option<String>,
    pub paused_by: Option<PausedBy>,
}

impl Item {
    /// Reset the volatile fields before (re)starting a download.
    pub fn reset_progress(&mut self) {
        self.percent = 0.0;
        self.speed_bps = 0.0;
        self.eta_secs = 0.0;
        self.downloaded_bytes = 0;
        self.stage = Stage::Download;
        self.error = None;
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewItem {
    pub video_id: String,
    pub title: String,
    pub url: String,
    pub index: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueuePayload {
    pub items: Vec<NewItem>,
    pub dir: String,
    pub batch_name: String,
    pub quality: String,
    pub audio_only: bool,
    pub numbering: bool,
    pub schedule_start: Option<String>,
    pub schedule_end: Option<String>,
}
