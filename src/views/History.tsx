import { useCallback, useEffect, useMemo, useState } from "react";

import { clearHistory, describeError, getHistory, onHistoryChanged, revealPath } from "../lib/api";
import { formatBytes, formatDate } from "../lib/format";
import { useI18n } from "../lib/i18n";
import type { HistoryEntry } from "../lib/types";

interface Group {
  key: string;
  title: string | null;
  entries: HistoryEntry[];
  bytes: number;
}

export default function History() {
  const { t } = useI18n();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  const refresh = useCallback(async () => {
    try {
      setEntries(await getHistory());
    } catch (error) {
      console.error(describeError(error));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const subscription = onHistoryChanged(() => void refresh());
    return () => {
      subscription.then((off) => off()).catch(() => undefined);
    };
  }, [refresh]);

  const groups = useMemo<Group[]>(() => {
    const query = search.trim().toLowerCase();
    const visible = entries.filter((entry) =>
      query
        ? entry.title.toLowerCase().includes(query) ||
          entry.batchName.toLowerCase().includes(query)
        : true,
    );

    const map = new Map<string, Group>();
    for (const entry of visible) {
      const key = entry.batchName && entry.batchName !== "untitled" ? entry.batchName : "";
      if (!map.has(key)) {
        map.set(key, { key, title: key || null, entries: [], bytes: 0 });
      }
      const group = map.get(key)!;
      group.entries.push(entry);
      group.bytes += entry.sizeBytes;
    }

    return [...map.values()].sort(
      (a, b) => (b.entries[0]?.finishedAt ?? 0) - (a.entries[0]?.finishedAt ?? 0),
    );
  }, [entries, search]);

  function toggleGroup(key: string) {
    setCollapsed((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
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
            disabled={entries.length === 0}
            onClick={() =>
              void clearHistory()
                .then(refresh)
                .catch((error) => console.error(describeError(error)))
            }
          >
            {t("clearHistory")}
          </button>
        </div>
      </div>

      {!loading && groups.length === 0 ? (
        <div className="empty">
          <div className="empty-icon">☰</div>
          <p>{t("historyEmpty")}</p>
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
                {group.bytes > 0 && (
                  <span className="muted small">{formatBytes(group.bytes)}</span>
                )}
                <span className="group-count muted small">{group.entries.length}</span>
              </button>

              {open && (
                <div className="accordion-body">
                  {group.entries.map((entry) => (
                    <div className="history-row" key={`${entry.id}-${entry.finishedAt}`}>
                      <span className={`badge-chip ${entry.kind === "audio" ? "audio" : ""}`}>
                        {entry.kind === "audio" ? "MP3" : "MP4"}
                      </span>
                      <span className="history-title" title={entry.path}>
                        {entry.title}
                      </span>
                      <span className="badge-chip">
                        {entry.quality === "best" ? t("best") : entry.quality}
                      </span>
                      <span className="muted small history-size">
                        {entry.sizeBytes > 0 ? formatBytes(entry.sizeBytes) : "—"}
                      </span>
                      <span className="muted small">{formatDate(entry.finishedAt)}</span>
                      <button
                        className="icon-btn"
                        title={t("openFolder")}
                        onClick={() =>
                          void revealPath(entry.path).catch((error) =>
                            console.error(describeError(error)),
                          )
                        }
                      >
                        📂
                      </button>
                    </div>
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
