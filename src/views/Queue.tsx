import { useContext, useMemo, useState } from "react";
import {
  cancelItem,
  formatBytes,
  formatEta,
  formatSpeed,
  openDownloadFolder,
  pauseAll,
  pauseItem,
  removeItem,
  resumeAll,
  resumeItem,
  retryItem,
  clearFinished,
  updateItemSchedule,
} from "../api";
import { I18nContext, type TKey } from "../i18n";
import type { Item, ItemStatus } from "../types";

export type QueueFilter = "all" | "downloading" | "completed" | "paused" | "failed";

const STATUS_KEY: Record<ItemStatus, TKey> = {
  queued: "statusQueued",
  active: "statusActive",
  paused: "statusPaused",
  completed: "statusCompleted",
  error: "statusError",
  cancelled: "statusCancelled",
  scheduled: "statusScheduled",
};

const STAGE_KEY: Record<string, TKey> = {
  download: "stageDownload",
  merge: "stageMerge",
  audio: "stageAudio",
  done: "stageDone",
};

interface Group {
  key: string;
  title: string | null;
  items: Item[];
  done: number;
  bytes: number;
  avgPercent: number;
  hasActive: boolean;
}

const FILTER_FN: Record<QueueFilter, (i: Item) => boolean> = {
  all: () => true,
  downloading: (i) => i.status === "active" || i.status === "queued" || i.status === "scheduled",
  completed: (i) => i.status === "completed",
  paused: (i) => i.status === "paused",
  failed: (i) => i.status === "error",
};

export default function Queue({
  items,
  filter,
}: {
  items: Item[];
  filter: QueueFilter;
}) {
  const { t } = useContext(I18nContext);
  const [search, setSearch] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  const groups: Group[] = useMemo(() => {
    const q = search.trim().toLowerCase();
    const filtered = items.filter(FILTER_FN[filter]).filter((i) =>
      q ? i.title.toLowerCase().includes(q) || i.batchName.toLowerCase().includes(q) : true,
    );
    const map = new Map<string, Group>();
    for (const it of filtered) {
      const key = it.batchName && it.batchName !== "untitled" ? it.batchName : "";
      if (!map.has(key)) {
        map.set(key, { key, title: key || null, items: [], done: 0, bytes: 0, avgPercent: 0, hasActive: false });
      }
      const g = map.get(key)!;
      g.items.push(it);
      if (it.status === "completed") {
        g.done += 1;
        g.bytes += it.totalBytes;
      }
      if (it.status === "active" || it.status === "queued") g.hasActive = true;
    }
    for (const g of map.values()) {
      g.avgPercent = g.items.reduce((s, i) => s + (i.status === "completed" ? 100 : i.percent), 0) / g.items.length;
    }
    const list = [...map.values()];
    list.sort((a, b) => {
      if (!a.title && b.title) return 1;
      if (a.title && !b.title) return -1;
      const at = a.items[0]?.createdAt ?? 0;
      const bt = b.items[0]?.createdAt ?? 0;
      return at - bt;
    });
    // groups with active downloads first
    list.sort((a, b) => Number(b.hasActive) - Number(a.hasActive));
    return list;
  }, [items, filter, search]);

  const toggle = (key: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const hasPaused = items.some((i) => i.status === "paused");
  const hasRunning = items.some((i) => i.status === "active" || i.status === "queued");
  const hasFinished = items.some((i) => i.status === "completed" || i.status === "cancelled");
  const isEmpty = groups.length === 0;

  return (
    <div className="view">
      <div className="toolbar">
        <input
          className="search-input"
          placeholder={t("search")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <div className="head-actions">
          <button className="ghost-btn" onClick={() => pauseAll()} disabled={!hasRunning}>
            ❚❚ {t("pauseAll")}
          </button>
          <button className="ghost-btn" onClick={() => resumeAll()} disabled={!hasPaused}>
            ▶ {t("resumeAll")}
          </button>
          <button className="ghost-btn" onClick={() => clearFinished()} disabled={!hasFinished}>
            {t("clearFinished")}
          </button>
        </div>
      </div>

      {isEmpty ? (
        <div className="empty">
          <div className="empty-icon">⇣</div>
          <p>{t("queueEmpty")}</p>
          <p className="muted small">{t("queueHint")}</p>
        </div>
      ) : (
        groups.map((g) => {
          const isOpen = !collapsed.has(g.key || "__singles__");
          return (
            <div className={`accordion ${isOpen ? "open" : ""}`} key={g.key || "__singles__"}>
              <button className="accordion-head" onClick={() => toggle(g.key || "__singles__")}>
                <span className="chev">{isOpen ? "▾" : "▸"}</span>
                <span className="group-icon">{g.title ? "📁" : "🎬"}</span>
                <span className="group-name">{g.title ?? t("singleVideosGroup")}</span>
                <span className="group-progress">
                  <span className="progress-track slim">
                    <span
                      className={`progress-fill ${g.hasActive ? "active" : "completed"}`}
                      style={{ width: `${g.avgPercent}%` }}
                    />
                  </span>
                </span>
                <span className="group-count muted small">
                  {g.done}/{g.items.length}
                </span>
              </button>
              {isOpen && (
                <div className="accordion-body">
                  {g.items.map((item) => (
                    <QueueRow key={item.id} item={item} t={t} />
                  ))}
                </div>
              )}
            </div>
          );
        })
      )}
    </div>
  );
}

function QueueRow({
  item,
  t,
}: {
  item: Item;
  t: (k: TKey, vars?: Record<string, string | number>) => string;
}) {
  const running = item.status === "active";
  const percent = Math.min(100, Math.max(0, item.percent));
  const [showSchedule, setShowSchedule] = useState(false);
  const [schedStart, setSchedStart] = useState(item.scheduleStart || "");
  const [schedEnd, setSchedEnd] = useState(item.scheduleEnd || "");

  const handleSaveSchedule = async () => {
    await updateItemSchedule(item.id, schedStart || null, schedEnd || null);
    setShowSchedule(false);
  };

  const handleClearSchedule = async () => {
    await updateItemSchedule(item.id, null, null);
    setShowSchedule(false);
  };

  const canSchedule = item.status === "queued" || item.status === "scheduled" || item.status === "paused";

  return (
    <div className={`dl-card ${item.status}`}>
      <div className="dl-card-main">
        <div className="dl-title" title={item.title}>
          {item.index != null && (
            <span className="row-index">{String(item.index).padStart(3, "0")}</span>
          )}
          <span className="dl-title-text">{item.title}</span>
          <span className="dl-percent">{item.status === "completed" ? "100%" : `${Math.floor(percent)}%`}</span>
        </div>
        <div className="dl-sub">
          <span className={`status-chip ${item.status}`}>
            {t(STATUS_KEY[item.status])}
            {running ? ` · ${t(STAGE_KEY[item.stage] ?? "stageDownload")}` : ""}
          </span>
          {item.audioOnly && <span className="badge-chip">MP3</span>}
          {!item.audioOnly && item.quality !== "best" && (
            <span className="badge-chip">{item.quality}p</span>
          )}
          {item.totalBytes > 0 && (
            <span className="muted small">
              {formatBytes(item.downloadedBytes)} / {formatBytes(item.totalBytes)}
            </span>
          )}
          {running && item.speedBps > 0 && <span className="dl-speed">{formatSpeed(item.speedBps)}</span>}
          {running && item.etaSecs > 0 && (
            <span className="dl-eta">{t("eta", { v: formatEta(item.etaSecs) })}</span>
          )}
          {item.status === "error" && item.error && (
            <span className="error-inline" title={item.error}>
              {item.error}
            </span>
          )}
        </div>
        <div className="progress-track card">
          <div
            className={`progress-fill ${item.status}`}
            style={{ width: `${item.status === "completed" ? 100 : percent}%` }}
          />
        </div>
      </div>
      <div className="dl-actions">
        {(item.status === "active" || item.status === "queued") && (
          <button className="icon-btn" title={t("pause")} onClick={() => pauseItem(item.id)}>
            ❚❚
          </button>
        )}
        {item.status === "paused" && (
          <button className="icon-btn" title={t("resume")} onClick={() => resumeItem(item.id)}>
            ▶
          </button>
        )}
        {(item.status === "active" || item.status === "queued" || item.status === "paused") && (
          <button className="icon-btn danger" title={t("cancel")} onClick={() => cancelItem(item.id)}>
            ✕
          </button>
        )}
        {item.status === "error" && (
          <button className="icon-btn" title={t("retry")} onClick={() => retryItem(item.id)}>
            ↻
          </button>
        )}
        {(item.status === "completed" || item.status === "cancelled") && (
          <button
            className="icon-btn"
            title={t("openFolder")}
            onClick={() => openDownloadFolder(item.id)}
          >
            📂
          </button>
        )}
        {canSchedule && !showSchedule && (
          <button className="icon-btn" title={item.status === "scheduled" ? t("perItemSchedule") : t("perItemSchedule")} onClick={() => setShowSchedule(true)}>
            🕐
          </button>
        )}
        {canSchedule && showSchedule && (
          <div className="schedule-inline">
            <input
              type="time"
              value={schedStart}
              onChange={(e) => setSchedStart(e.target.value)}
              style={{ width: 100 }}
            />
            <input
              type="time"
              value={schedEnd}
              onChange={(e) => setSchedEnd(e.target.value)}
              style={{ width: 100 }}
            />
            <button className="icon-btn" onClick={handleSaveSchedule} title={t("save")}>✓</button>
            <button className="icon-btn" onClick={handleClearSchedule} title={t("clearSchedule")}>✕</button>
            <button className="icon-btn" onClick={() => setShowSchedule(false)} title={t("dismiss")}>×</button>
          </div>
        )}
        <button className="icon-btn" title={t("remove")} onClick={() => removeItem(item.id)}>
          🗑
        </button>
      </div>
    </div>
  );
}
