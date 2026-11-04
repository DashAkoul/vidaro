//! In-memory queue plus its JSON persistence.
//!
//! Item state lives behind a **blocking** mutex with very short critical
//! sections. The previous implementation used a `try_lock` and silently skipped
//! items it could not lock, which made downloads disappear from `queue.json`
//! whenever they were mid-flight during a save.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::core::error::AppResult;
use crate::features::download::model::{Item, ItemStatus, PausedBy, Stage};

pub struct Control {
    pub paused: bool,
    pub cancelled: bool,
}

/// Held for as long as a task is driving an item, so a second resume cannot
/// start a duplicate yt-dlp process writing to the same file.
pub struct RunGuard {
    inner: Arc<ItemInner>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        self.inner.running.store(false, Ordering::SeqCst);
    }
}

pub struct ItemInner {
    item: Mutex<Item>,
    child: tokio::sync::Mutex<Option<tokio::process::Child>>,
    /// PID of the running yt-dlp process, kept outside the child mutex so a
    /// cancel can kill the process tree even while `wait_child` is awaiting.
    child_pid: AtomicU32,
    /// Files yt-dlp announced with `Destination:` — used to delete the
    /// `.part` leftovers when the user cancels.
    destinations: Mutex<Vec<String>>,
    pub control: Mutex<Control>,
    running: AtomicBool,
}

impl ItemInner {
    pub fn new(item: Item) -> Arc<Self> {
        Arc::new(Self {
            item: Mutex::new(item),
            child: tokio::sync::Mutex::new(None),
            child_pid: AtomicU32::new(0),
            destinations: Mutex::new(Vec::new()),
            control: Mutex::new(Control {
                paused: false,
                cancelled: false,
            }),
            running: AtomicBool::new(false),
        })
    }

    /// Remember a destination yt-dlp is writing to (called once per stream).
    pub fn note_destination(&self, path: &str) {
        if path.is_empty() {
            return;
        }
        let mut guard = self
            .destinations
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if !guard.iter().any(|p| p == path) {
            guard.push(path.to_string());
        }
    }

    pub fn clear_destinations(&self) {
        self.destinations
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
    }

    /// Delete yt-dlp's partial downloads for this item.
    ///
    /// Only paths yt-dlp itself reported are touched, so a shared folder with
    /// other items cannot lose their data.
    pub fn remove_partials(&self) -> usize {
        let paths: Vec<String> = self
            .destinations
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();

        let mut removed = 0;
        for dest in paths {
            for suffix in [".part", ".ytdl", ".temp"] {
                let candidate = format!("{dest}{suffix}");
                if std::fs::remove_file(&candidate).is_ok() {
                    removed += 1;
                }
            }
        }
        if removed > 0 {
            self.clear_destinations();
        }
        removed
    }

    /// Claim the item for this task. Returns `None` when it is already running.
    pub fn begin_run(self: &Arc<Self>) -> Option<RunGuard> {
        if self.running.swap(true, Ordering::SeqCst) {
            return None;
        }
        Some(RunGuard { inner: self.clone() })
    }

    /// Cheap clone of the current state, for events and persistence.
    pub fn snapshot(&self) -> Item {
        self.item
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn with_item<R>(&self, f: impl FnOnce(&mut Item) -> R) -> R {
        let mut guard = self.item.lock().unwrap_or_else(|p| p.into_inner());
        f(&mut guard)
    }

    pub fn status(&self) -> ItemStatus {
        self.with_item(|i| i.status)
    }

    pub fn is_cancelled(&self) -> bool {
        self.control
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .cancelled
    }

    pub fn is_paused(&self) -> bool {
        self.control
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .paused
    }

    pub async fn set_child(&self, child: Option<tokio::process::Child>) {
        let pid = child.as_ref().and_then(|c| c.id()).unwrap_or(0);
        self.child_pid.store(pid, Ordering::SeqCst);
        *self.child.lock().await = child;
    }

    /// Stop the download and everything it spawned.
    ///
    /// `yt-dlp.exe` is a PyInstaller bundle: the process we spawn is a
    /// bootloader that starts the real Python process, and ffmpeg is a third
    /// one. Killing only the direct child leaves the actual download running,
    /// so the whole tree has to go.
    pub async fn kill_child(&self) {
        let pid = self.child_pid.swap(0, Ordering::SeqCst);

        #[cfg(windows)]
        if pid != 0 {
            // `/T` includes children, `/F` forces termination. Absolute path so
            // a broken PATH cannot silently disable cancellation.
            let taskkill = std::env::var("SystemRoot")
                .map(|root| format!(r"{root}\System32\taskkill.exe"))
                .unwrap_or_else(|_| "taskkill".to_string());
            let _ = crate::core::process::async_command(taskkill)
                .args(["/T", "/F", "/PID", &pid.to_string()])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;
        }

        #[cfg(unix)]
        if pid != 0 {
            // The child was started in its own process group (see runner), so
            // a negative pid signals the whole group.
            let _ = crate::core::process::async_command("kill")
                .args(["-9", &format!("-{pid}")])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;
        }

        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut() {
            let _ = child.kill().await;
        }
        *guard = None;
    }

    /// Wait for the child to exit, consuming the handle. Returns true on success.
    ///
    /// The mutex is released before awaiting the process, so a cancel arriving
    /// mid-wait can still take the handle and kill it.
    pub async fn wait_child(&self) -> bool {
        let child = {
            let mut guard = self.child.lock().await;
            guard.take()
        };
        let Some(mut child) = child else {
            return false;
        };
        let success = matches!(child.wait().await, Ok(status) if status.success());
        self.child_pid.store(0, Ordering::SeqCst);
        success
    }
}

/// Concurrency limiter whose capacity can change at runtime.
pub struct Gate {
    semaphore: Arc<Semaphore>,
    max: AtomicUsize,
}

impl Gate {
    pub fn new(max: usize) -> Self {
        let max = max.max(1);
        Self {
            semaphore: Arc::new(Semaphore::new(max)),
            max: AtomicUsize::new(max),
        }
    }

    pub async fn acquire(&self) -> OwnedSemaphorePermit {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("gate semaphore is never closed")
    }

    pub fn set_max(&self, max: usize) {
        let max = max.clamp(1, 16);
        let previous = self.max.swap(max, Ordering::SeqCst);
        match max.cmp(&previous) {
            std::cmp::Ordering::Greater => self.semaphore.add_permits(max - previous),
            std::cmp::Ordering::Less => {
                let _ = self.semaphore.forget_permits(previous - max);
            }
            std::cmp::Ordering::Equal => {}
        }
    }
}

/// The queue itself. Cheap to clone via `Arc`.
pub struct Queue {
    pub items: Mutex<BTreeMap<String, Arc<ItemInner>>>,
    pub gate: Gate,
}

impl Queue {
    pub fn new(max_concurrent: usize) -> Arc<Self> {
        Arc::new(Self {
            items: Mutex::new(BTreeMap::new()),
            gate: Gate::new(max_concurrent),
        })
    }

    pub fn insert(&self, inner: Arc<ItemInner>) {
        let id = inner.with_item(|i| i.id.clone());
        self.items
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id, inner);
    }

    pub fn get(&self, id: &str) -> Option<Arc<ItemInner>> {
        self.items
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(id)
            .cloned()
    }

    pub fn remove(&self, id: &str) -> Option<Arc<ItemInner>> {
        self.items
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(id)
    }

    pub fn all(&self) -> Vec<Arc<ItemInner>> {
        self.items
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Snapshot sorted by creation time (oldest first).
    pub fn snapshot(&self) -> Vec<Item> {
        let mut out: Vec<Item> = self.all().iter().map(|i| i.snapshot()).collect();
        out.sort_by_key(|i| i.created_at);
        out
    }
}

/// Managed Tauri state wrapper.
pub struct Manager {
    pub queue: Arc<Queue>,
}

impl Manager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            queue: Queue::new(max_concurrent),
        }
    }
}

/// Serialises writes to `queue.json`.
///
/// Several downloads can finish in the same instant, and each one persists the
/// queue. Without this guard two writers could interleave inside the same temp
/// file and then rename a corrupted document into place.
static PERSIST_GUARD: Mutex<()> = Mutex::new(());

pub fn save_to(path: &Path, queue: &Queue) -> AppResult<()> {
    let _guard = PERSIST_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let items = queue.snapshot();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&items)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Persist the queue next to the app's data directory.
pub fn persist(app: &tauri::AppHandle, queue: &Queue) {
    use tauri::Manager as _;
    let Some(paths) = app.try_state::<crate::core::paths::AppPaths>() else {
        return;
    };
    if let Err(e) = save_to(&paths.queue_file, queue) {
        eprintln!("[queue] could not persist queue: {e}");
    }
}

pub fn load_from(path: &Path, queue: &Queue) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(items) = serde_json::from_str::<Vec<Item>>(&text) else {
        eprintln!("[queue] ignoring malformed queue file");
        return;
    };

    for mut item in items {
        // Anything that was running when the app closed comes back paused, so
        // the user can decide when to resume it.
        if matches!(
            item.status,
            ItemStatus::Active | ItemStatus::Queued | ItemStatus::Scheduled
        ) {
            item.status = ItemStatus::Paused;
            item.paused_by = Some(PausedBy::User);
            // Progress is kept on purpose: yt-dlp resumes from the `.part`
            // file, so a download interrupted today and resumed next week
            // should still show where it stopped instead of jumping to 0%.
            item.speed_bps = 0.0;
            item.eta_secs = 0.0;
            if item.stage == Stage::Merge {
                item.stage = Stage::Download;
            }
        }
        queue.insert(ItemInner::new(item));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_queue(path: &Path, json: &str) {
        std::fs::write(path, json).expect("write queue fixture");
    }

    /// A download interrupted by closing the app must come back paused *at the
    /// percentage it reached* — that is what makes resuming days later
    /// understandable instead of showing 0% next to a half-written file.
    #[test]
    fn interrupted_items_keep_their_progress() {
        let dir = std::env::temp_dir().join("vidaro-queue-test-progress");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("queue.json");
        write_queue(
            &path,
            r#"[{
                "id":"a","url":"https://youtu.be/x","videoId":"x","title":"T","index":null,
                "dir":"/tmp/out","prefix":"","quality":"720","audioOnly":false,"batchName":"",
                "status":"active","percent":42.5,"speedBps":1234.0,"etaSecs":30.0,
                "totalBytes":49000000,"downloadedBytes":20825000,"stage":"download",
                "error":null,"outputPath":null,"createdAt":1,
                "scheduleStart":null,"scheduleEnd":null,"pausedBy":null
            }]"#,
        );

        let queue = Queue::new(2);
        load_from(&path, &queue);

        let items = queue.snapshot();
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.status, ItemStatus::Paused);
        assert_eq!(item.paused_by, Some(PausedBy::User));
        assert!((item.percent - 42.5).abs() < 0.01, "percent was {}", item.percent);
        assert_eq!(item.downloaded_bytes, 20_825_000);
        assert_eq!(item.total_bytes, 49_000_000);
        // Volatile values are cleared.
        assert_eq!(item.speed_bps, 0.0);
        assert_eq!(item.eta_secs, 0.0);
    }

    /// A queue entry that was mid-merge when the app closed restarts at the
    /// download stage, because the merge has to be redone from the parts.
    #[test]
    fn interrupted_merge_returns_to_download_stage() {
        let dir = std::env::temp_dir().join("vidaro-queue-test-merge");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("queue.json");
        write_queue(
            &path,
            r#"[{
                "id":"b","url":"https://youtu.be/y","videoId":"y","title":"T2","index":null,
                "dir":"/tmp/out","prefix":"","quality":"best","audioOnly":false,"batchName":"",
                "status":"queued","percent":100.0,"speedBps":0.0,"etaSecs":0.0,
                "totalBytes":10,"downloadedBytes":10,"stage":"merge",
                "error":null,"outputPath":null,"createdAt":2,
                "scheduleStart":null,"scheduleEnd":null,"pausedBy":null
            }]"#,
        );

        let queue = Queue::new(2);
        load_from(&path, &queue);
        let items = queue.snapshot();
        assert_eq!(items[0].status, ItemStatus::Paused);
        assert_eq!(items[0].stage, Stage::Download);
    }
}
