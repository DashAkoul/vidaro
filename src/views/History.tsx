import { useContext, useEffect, useMemo, useState } from "react";
import { clearHistory, formatBytes, formatDate, getHistory, revealPath } from "../api";
import { I18nContext } from "../i18n";
import type { HistoryEntry } from "../types";

interface HGroup {
  key: string;
  title: string | null;
  entries: HistoryEntry[];
  bytes: number;
}

export default function HistoryView() {
  const { t } = useContext(I18nContext);
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  async function refresh() {
    setLoading(true);
    setEntries(await getHistory());
    setLoading(false);
  }

  useEffect(() => {
    refresh();
  }, []);

  const groups: HGroup[] = useMemo(() => {
    const q = search.trim().toLowerCase();
    const filtered = entries.filter((e) =>
      q ? e.title.toLowerCase().includes(q) || e.batchName.toLowerCase().includes(q) : true,
    );
    const map = new Map<string, HGroup>();
    for (const e of filtered) {
      const key = e.batchName && e.batchName !== "untitled" ? e.batchName : "";
      if (!map.has(key)) {
        map.set(key, { key, title: key || null, entries: [], bytes: 0 });
      }
      const g = map.get(key)!;
      g.entries.push(e);
      g.bytes += e.sizeBytes;
    }
    const list = [...map.values()];
    list.sort((a, b) => (b.entries[0]?.finishedAt ?? 0) - (a.entries[0]?.finishedAt ?? 0));
    return list;
  }, [entries, search]);

  const toggle = (key: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

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
          <button
            className="ghost-btn"
            onClick={async () => {
              await clearHistory();
              await refresh();
            }}
            disabled={entries.length === 0}
          >
            {t("clearHistory")}
          </button>
        </div>
      </div>

      {loading ? null : groups.length === 0 ? (
        <div className="empty">
          <div className="empty-icon">☰</div>
          <p>{t("historyEmpty")}</p>
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
                {g.bytes > 0 && <span className="muted small">{formatBytes(g.bytes)}</span>}
                <span className="group-count muted small">{g.entries.length}</span>
              </button>
              {isOpen && (
                <div className="accordion-body">
                  {g.entries.map((e) => (
                    <div className="history-row" key={e.id + e.finishedAt}>
                      <span className={`badge-chip ${e.kind === "audio" ? "audio" : ""}`}>
                        {e.kind === "audio" ? "MP3" : "MP4"}
                      </span>
                      <span className="history-title" title={e.path}>
                        {e.title}
                      </span>
                      <span className="badge-chip">{e.quality === "best" ? t("best") : e.quality}</span>
                      <span className="muted small history-size">
                        {e.sizeBytes > 0 ? formatBytes(e.sizeBytes) : "—"}
                      </span>
                      <span className="muted small">{formatDate(e.finishedAt)}</span>
                      <button
                        className="icon-btn"
                        title={t("openFolder")}
                        onClick={() => revealPath(e.path)}
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
