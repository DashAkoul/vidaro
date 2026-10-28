//! In-memory queue plus its JSON persistence.
//!
//! Item state lives behind a **blocking** mutex with very short critical
//! sections. The previous implementation used a `try_lock` and silently skipped
//! items it could not lock, which made downloads disappear from `queue.json`
//! whenever they were mid-flight during a save.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::core::error::AppResult;
use crate::features::download::model::{Item, ItemStatus, PausedBy, Stage};

pub struct Control {
    pub paused: bool,
    pub cancelled: bool,
}

pub struct ItemInner {
    item: Mutex<Item>,
    child: tokio::sync::Mutex<Option<tokio::process::Child>>,
    pub control: Mutex<Control>,
}

impl ItemInner {
    pub fn new(item: Item) -> Arc<Self> {
        Arc::new(Self {
            item: Mutex::new(item),
            child: tokio::sync::Mutex::new(None),
            control: Mutex::new(Control {
                paused: false,
                cancelled: false,
            }),
        })
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
        *self.child.lock().await = child;
    }

    pub async fn kill_child(&self) {
        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut() {
            let _ = child.kill().await;
        }
        *guard = None;
    }

    /// Wait for the child to exit, consuming the handle. Returns true on success.
    pub async fn wait_child(&self) -> bool {
        let mut guard = self.child.lock().await;
        let Some(mut child) = guard.take() else {
            return false;
        };
        matches!(child.wait().await, Ok(status) if status.success())
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
            item.percent = 0.0;
            item.speed_bps = 0.0;
            item.eta_secs = 0.0;
            item.stage = Stage::Download;
        }
        queue.insert(ItemInner::new(item));
    }
}
