import { useMemo, useState } from "react";

import { useApp, type QueueFilter } from "../lib/app-context";
import {
  cancelItem,
  clearFinished,
  describeError,
  openDownloadFolder,
  pauseAll,
  pauseItem,
  removeItem,
  resumeAll,
  resumeItem,
  retryItem,
  updateItemSchedule,
} from "../lib/api";
import { clampPercent, formatBytes, formatEta, formatSpeed } from "../lib/format";
import { useI18n, type TKey } from "../lib/i18n";
import type { Item, ItemStatus } from "../lib/types";

const STATUS_LABEL: Record<ItemStatus, TKey> = {
  queued: "statusQueued",
  active: "statusActive",
  paused: "statusPaused",
  completed: "statusCompleted",
  error: "statusError",
  cancelled: "statusCancelled",
  scheduled: "statusScheduled",
};

const STAGE_LABEL: Record<string, TKey> = {
  download: "stageDownload",
  merge: "stageMerge",
  audio: "stageAudio",
  done: "stageDone",
};

const FILTERS: Record<QueueFilter, (item: Item) => boolean> = {
  all: () => true,
  downloading: (item) =>
    item.status === "active" || item.status === "queued" || item.status === "scheduled",
  completed: (item) => item.status === "completed",
  paused: (item) => item.status === "paused",
  failed: (item) => item.status === "error",
};

interface Group {
  key: string;
  title: string | null;
  items: Item[];
  completed: number;
  averagePercent: number;
  hasActive: boolean;
}

export default function Queue() {
  const { t } = useI18n();
  const { items, queueFilter } = useApp();
  const [search, setSearch] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  const groups = useMemo<Group[]>(() => {
    const query = search.trim().toLowerCase();
    const visible = items
      .filter(FILTERS[queueFilter])
      .filter((item) =>
        query
          ? item.title.toLowerCase().includes(query) ||
            item.batchName.toLowerCase().includes(query)
          : true,
      );

    const map = new Map<string, Group>();
    for (const item of visible) {
      const key = item.batchName && item.batchName !== "untitled" ? item.batchName : "";
      if (!map.has(key)) {
        map.set(key, {
          key,
          title: key || null,
          items: [],
          completed: 0,
          averagePercent: 0,
          hasActive: false,
        });
      }
      const group = map.get(key)!;
      group.items.push(item);
      if (item.status === "completed") group.completed += 1;
      if (item.status === "active" || item.status === "queued") group.hasActive = true;
    }

    const list = [...map.values()];
    for (const group of list) {
      group.averagePercent =
        group.items.reduce(
          (sum, item) => sum + (item.status === "completed" ? 100 : clampPercent(item.percent)),
          0,
        ) / group.items.length;
    }

    list.sort((a, b) => {
      if (!a.title && b.title) return 1;
      if (a.title && !b.title) return -1;
      return (a.items[0]?.createdAt ?? 0) - (b.items[0]?.createdAt ?? 0);
    });
    // Groups with live downloads float to the top.
    list.sort((a, b) => Number(b.hasActive) - Number(a.hasActive));
    return list;
  }, [items, queueFilter, search]);

  const hasPaused = items.some((item) => item.status === "paused");
  const hasRunning = items.some(
    (item) => item.status === "active" || item.status === "queued",
  );
  const hasFinished = items.some(
    (item) => item.status === "completed" || item.status === "cancelled",
  );

  function toggleGroup(key: string) {
    setCollapsed((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  async function guard(action: () => Promise<unknown>) {
    try {
      await action();
    } catch (error) {
      console.error(describeError(error));
    }
  }

  return (
    <div className="view">
      <div className="toolbar">
        <input
          className="search-input"
          placeholder={t("search")}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <div className="head-actions">
          <button
            className="ghost-btn"
            onClick={() => void guard(pauseAll)}
            disabled={!hasRunning}
          >
            ❚❚ {t("pauseAll")}
          </button>
          <button
            className="ghost-btn"
            onClick={() => void guard(resumeAll)}
            disabled={!hasPaused}
          >
            ▶ {t("resumeAll")}
          </button>
          <button
            className="ghost-btn"
            onClick={() => void guard(clearFinished)}
            disabled={!hasFinished}
          >
            {t("clearFinished")}
          </button>
        </div>
      </div>

      {groups.length === 0 ? (
        <div className="empty">
          <div className="empty-icon">⇣</div>
          <p>{t("queueEmpty")}</p>
          <p className="muted small">{t("queueHint")}</p>
        </div>
      ) : (
        groups.map((group) => {
          const key = group.key || "__singles__";
          const open = !collapsed.has(key);
          return (
            <div className={`accordion ${open ? "open" : ""}`} key={key}>
              <button className="accordion-head" onClick={() => toggleGroup(key)}>
                <span className="chev">{open ? "▾" : "▸"}</span>
                <span className="group-icon">{group.title ? "📁" : "🎬"}</span>
                <span className="group-name">{group.title ?? t("singleVideosGroup")}</span>
                <span className="group-progress">
                  <span className="progress-track slim">
                    <span
                      className={`progress-fill ${group.hasActive ? "active" : "completed"}`}
                      style={{ width: `${group.averagePercent}%` }}
                    />
                  </span>
                </span>
                <span className="group-count muted small">
                  {group.completed}/{group.items.length}
                </span>
              </button>
              {open && (
                <div className="accordion-body">
                  {group.items.map((item) => (
                    <QueueRow key={item.id} item={item} onAction={guard} />
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
  onAction,
}: {
  item: Item;
  onAction: (action: () => Promise<unknown>) => Promise<void>;
}) {
  const { t } = useI18n();
  const [editingSchedule, setEditingSchedule] = useState(false);
  const [start, setStart] = useState(item.scheduleStart ?? "");
  const [end, setEnd] = useState(item.scheduleEnd ?? "");

  const percent = clampPercent(item.percent);
  const running = item.status === "active";
  const canSchedule =
    item.status === "queued" || item.status === "scheduled" || item.status === "paused";

  return (
    <div className={`dl-card ${item.status}`}>
      <div className="dl-card-main">
        <div className="dl-title" title={item.title}>
          {item.index != null && (
            <span className="row-index">{String(item.index).padStart(3, "0")}</span>
          )}
          <span className="dl-title-text">{item.title}</span>
          <span className="dl-percent">
            {item.status === "completed" ? "100%" : `${Math.floor(percent)}%`}
          </span>
        </div>

        <div className="dl-sub">
          <span className={`status-chip ${item.status}`}>
            {t(STATUS_LABEL[item.status])}
            {running ? ` · ${t(STAGE_LABEL[item.stage] ?? "stageDownload")}` : ""}
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
          {running && item.speedBps > 0 && (
            <span className="dl-speed">{formatSpeed(item.speedBps)}</span>
          )}
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
          <button
            className="icon-btn"
            title={t("pause")}
            onClick={() => void onAction(() => pauseItem(item.id))}
          >
            ❚❚
          </button>
        )}
        {item.status === "paused" && (
          <button
            className="icon-btn"
            title={t("resume")}
            onClick={() => void onAction(() => resumeItem(item.id))}
          >
            ▶
          </button>
        )}
        {(item.status === "active" ||
          item.status === "queued" ||
          item.status === "paused" ||
          item.status === "scheduled") && (
          <button
            className="icon-btn danger"
            title={t("cancel")}
            onClick={() => void onAction(() => cancelItem(item.id))}
          >
            ✕
          </button>
        )}
        {item.status === "error" && (
          <button
            className="icon-btn"
            title={t("retry")}
            onClick={() => void onAction(() => retryItem(item.id))}
          >
            ↻
          </button>
        )}
        {(item.status === "completed" || item.status === "cancelled") && (
          <button
            className="icon-btn"
            title={t("openFolder")}
            onClick={() => void onAction(() => openDownloadFolder(item.id))}
          >
            📂
          </button>
        )}
        {canSchedule && !editingSchedule && (
          <button
            className="icon-btn"
            title={t("perItemSchedule")}
            onClick={() => setEditingSchedule(true)}
          >
            🕐
          </button>
        )}
        {canSchedule && editingSchedule && (
          <div className="schedule-inline">
            <input
              type="time"
              value={start}
              onChange={(event) => setStart(event.target.value)}
              style={{ width: 100 }}
            />
            <input
              type="time"
              value={end}
              onChange={(event) => setEnd(event.target.value)}
              style={{ width: 100 }}
            />
            <button
              className="icon-btn"
              title={t("save")}
              onClick={() =>
                void onAction(async () => {
                  await updateItemSchedule(item.id, start || null, end || null);
                  setEditingSchedule(false);
                })
              }
            >
              ✓
            </button>
            <button
              className="icon-btn"
              title={t("clearSchedule")}
              onClick={() =>
                void onAction(async () => {
                  await updateItemSchedule(item.id, null, null);
                  setStart("");
                  setEnd("");
                  setEditingSchedule(false);
                })
              }
            >
              ✕
            </button>
            <button
              className="icon-btn"
              title={t("dismiss")}
              onClick={() => setEditingSchedule(false)}
            >
              ×
            </button>
          </div>
        )}
        <button
          className="icon-btn"
          title={t("remove")}
          onClick={() => void onAction(() => removeItem(item.id))}
        >
          🗑
        </button>
      </div>
    </div>
  );
}
