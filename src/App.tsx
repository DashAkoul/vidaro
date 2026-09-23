import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ensureBinaries,
  getQueue,
  getSettings,
  onBinariesProgress,
  onClipboardYoutube,
  onItemUpdated,
  onQueueChanged,
  setSettings as saveSettingsApi,
  binariesStatus,
} from "./api";
import type { BinariesProgress, BinariesStatus, Item, Settings } from "./types";
import { I18nContext, type Lang, type TKey, type ThemeMode, translate } from "./i18n";
import NewDownload from "./views/NewDownload";
import Queue, { type QueueFilter } from "./views/Queue";
import HistoryView from "./views/History";
import SettingsView from "./views/SettingsView";
import "./app.css";

export type Tab = "new" | "queue" | "history" | "settings";

export interface AppCtx {
  settings: Settings;
  updateSettings: (patch: Partial<Settings>) => void;
  goTab: (t: Tab) => void;
  openUrl: (url: string) => void;
}

export const AppContext = { current: null as AppCtx | null };

export default function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [binaries, setBinaries] = useState<BinariesStatus | null>(null);
  const [binProg, setBinProg] = useState<BinariesProgress | null>(null);
  const [binError, setBinError] = useState<string | null>(null);
  const [items, setItems] = useState<Item[]>([]);
  const [tab, setTab] = useState<Tab>("queue");
  const [filter, setFilter] = useState<QueueFilter>("all");
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? true,
  );
  const [clipboardUrl, setClipboardUrl] = useState<string | null>(null);
  const clipboardToastTimer = useRef<number | null>(null);

  // initial load
  useEffect(() => {
    (async () => {
      const s = await getSettings();
      setSettings(s);
      const b = await binariesStatus();
      setBinaries(b);
      if (!b.ready) {
        try {
          const done = await ensureBinaries();
          setBinaries(done);
        } catch (e) {
          setBinError(String(e));
        }
      }
      setItems(await getQueue());
    })();

    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const onMedia = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    media?.addEventListener("change", onMedia);

    const unlisteners = [
      onItemUpdated((it) => {
        setItems((prev) => {
          const exists = prev.some((p) => p.id === it.id);
          return exists
            ? prev.map((p) => (p.id === it.id ? it : p))
            : [...prev, it].sort((a, b) => a.createdAt - b.createdAt);
        });
      }),
      onQueueChanged(async () => setItems(await getQueue())),
      onBinariesProgress((p) => setBinProg(p)),
      onClipboardYoutube((url) => {
        setClipboardUrl(url);
        if (clipboardToastTimer.current) {
          window.clearTimeout(clipboardToastTimer.current);
        }
        clipboardToastTimer.current = window.setTimeout(() => setClipboardUrl(null), 12000);
      }),
    ];
    return () => {
      media?.removeEventListener("change", onMedia);
      unlisteners.forEach((p) => p.then((f) => f()));
    };
  }, []);

  const lang: Lang = settings?.language ?? "en";
  const dir: "rtl" | "ltr" = lang === "fa" ? "rtl" : "ltr";
  const themeMode: ThemeMode = settings?.theme ?? "system";
  const isDark = themeMode === "system" ? systemDark : themeMode === "dark";

  // apply language + theme to the document
  useEffect(() => {
    document.documentElement.dir = dir;
    document.documentElement.lang = lang;
    document.documentElement.dataset.theme = isDark ? "dark" : "light";
  }, [lang, dir, isDark]);

  const updateSettings = useCallback((patch: Partial<Settings>) => {
    setSettings((prev) => {
      if (!prev) return prev;
      const next = { ...prev, ...patch };
      void saveSettingsApi(next);
      return next;
    });
  }, []);

  const t = useMemo(
    () => (key: TKey, vars?: Record<string, string | number>) =>
      translate(lang, key, vars),
    [lang],
  );

  const openUrl = useCallback((url: string) => {
    setClipboardUrl(null);
    setTab("new");
    setPendingUrl(url);
  }, []);

  const goTab = useCallback((x: Tab) => {
    setTab(x);
    if (x === "queue") setFilter("all");
  }, []);

  // pending URL handed over to NewDownload view (from the clipboard toast)
  const [pendingUrl, setPendingUrl] = useState<string | null>(null);
  useEffect(() => {
    if (pendingUrl) {
      const id = window.setTimeout(() => setPendingUrl(null), 50);
      return () => window.clearTimeout(id);
    }
  }, [pendingUrl]);

  if (!settings) {
    return <div className="splash" />;
  }

  const i18n = { lang, dir, t };
  const activeItems = items.filter((i) => i.status === "active");
  const totalSpeed = activeItems.reduce((s, i) => s + (i.speedBps || 0), 0);
  const filterCounts = {
    all: items.length,
    downloading: items.filter((i) => i.status === "active" || i.status === "queued").length,
    completed: items.filter((i) => i.status === "completed").length,
    paused: items.filter((i) => i.status === "paused").length,
    failed: items.filter((i) => i.status === "error").length,
  };

  const appCtx: AppCtx = { settings, updateSettings, goTab, openUrl };
  AppContext.current = appCtx;

  const sidebarItem = (
    id: string,
    icon: string,
    label: string,
    badge: number | null,
    active: boolean,
    onClick: () => void,
  ) => (
    <button className={`side-item ${active ? "active" : ""}`} onClick={onClick} key={id}>
      <span className="side-icon">{icon}</span>
      <span className="side-label">{label}</span>
      {badge ? <span className="side-badge">{badge}</span> : null}
    </button>
  );

  const isQueueTab = tab === "queue";

  return (
    <I18nContext.Provider value={i18n}>
      <div className="app">
        <aside className="sidebar">
          <div className="brand">
            <img src="/vidaro-logo.png" alt="" className="brand-logo" draggable={false} />
            <span className="brand-name">Vidaro</span>
          </div>

          <div className="side-section">
            {sidebarItem("new", "＋", t("tabNew"), null, tab === "new", () => setTab("new"))}
          </div>

          <div className="side-title">Downloads</div>
          <div className="side-section">
            {sidebarItem("q-all", "▣", t("tabQueue"), filterCounts.all || null, isQueueTab && filter === "all", () => { setTab("queue"); setFilter("all"); })}
            {sidebarItem("q-down", "↓", t("tabDownloading"), filterCounts.downloading || null, isQueueTab && filter === "downloading", () => { setTab("queue"); setFilter("downloading"); })}
            {sidebarItem("q-done", "✓", t("tabCompleted"), filterCounts.completed || null, isQueueTab && filter === "completed", () => { setTab("queue"); setFilter("completed"); })}
            {sidebarItem("q-pause", "❚❚", t("tabPaused"), filterCounts.paused || null, isQueueTab && filter === "paused", () => { setTab("queue"); setFilter("paused"); })}
            {sidebarItem("q-fail", "×", t("tabFailed"), filterCounts.failed || null, isQueueTab && filter === "failed", () => { setTab("queue"); setFilter("failed"); })}
          </div>

          <div className="side-section bottom">
            {sidebarItem("hist", "☰", t("tabHistory"), null, tab === "history", () => setTab("history"))}
            {sidebarItem("set", "⚙", t("tabSettings"), null, tab === "settings", () => setTab("settings"))}
          </div>
        </aside>

        <div className="main-col">
          <header className="topbar">
            <div className="topbar-left">
              {tab === "new" && <h1>{t("tabNew")}</h1>}
              {tab === "history" && <h1>{t("tabHistory")}</h1>}
              {tab === "settings" && <h1>{t("tabSettings")}</h1>}
              {isQueueTab && (
                <h1>{t(filter === "all" ? "tabQueue" : filter === "downloading" ? "tabDownloading" : filter === "completed" ? "tabCompleted" : filter === "paused" ? "tabPaused" : "tabFailed")}</h1>
              )}
            </div>
            <div className="topbar-right">
              <Dropdown
                button={<span className="ctl-btn">🌐 {lang === "fa" ? "فا" : "EN"}</span>}
              >
                <div className="menu-item" onClick={() => updateSettings({ language: "en" })}>
                  {lang === "en" ? "✓" : ""} English
                </div>
                <div className="menu-item" onClick={() => updateSettings({ language: "fa" })}>
                  {lang === "fa" ? "✓" : ""} فارسی
                </div>
              </Dropdown>
              <Dropdown
                button={
                  <span className="ctl-btn">
                    {themeMode === "system" ? "◉" : themeMode === "dark" ? "◐" : "☼"}
                  </span>
                }
              >
                <div className="menu-item" onClick={() => updateSettings({ theme: "light" })}>
                  {themeMode === "light" ? "✓" : ""} ☼ {t("light")}
                </div>
                <div className="menu-item" onClick={() => updateSettings({ theme: "dark" })}>
                  {themeMode === "dark" ? "✓" : ""} ◐ {t("dark")}
                </div>
                <div className="menu-item" onClick={() => updateSettings({ theme: "system" })}>
                  {themeMode === "system" ? "✓" : ""} ◉ {t("themeSystem")}
                </div>
              </Dropdown>
              <button className="btn primary" onClick={() => setTab("new")}>
                ＋ {t("tabNew")}
              </button>
            </div>
          </header>

          <main className="content">
            {tab === "new" && <NewDownload pendingUrl={pendingUrl} />}
            {isQueueTab && <Queue items={items} filter={filter} />}
            {tab === "history" && <HistoryView />}
            {tab === "settings" && <SettingsView binaries={binaries} onBinaries={setBinaries} />}
          </main>

          <footer className="statusbar">
            <span className={totalSpeed > 0 ? "status-speed on" : "status-speed"}>
              ≋ {totalSpeed > 0 ? formatSpeedShort(totalSpeed) : "0 B/s"}
              <span className="muted"> {t("totalSpeed")}</span>
            </span>
            <span className="muted">
              {t("activeDownloads")}: {activeItems.length + items.filter((i) => i.status === "queued").length}
            </span>
            <span className="muted status-right">Vidaro 0.2.0</span>
          </footer>
        </div>

        {clipboardUrl && (
          <div className="toast clipboard-toast show">
            <div className="toast-body">
              <div className="toast-icon">📥</div>
              <div className="toast-content">
                <b>{t("clipboardFound")}</b>
                <code dir="ltr">{clipboardUrl.length > 60 ? clipboardUrl.slice(0, 60) + "…" : clipboardUrl}</code>
              </div>
            </div>
            <div className="toast-actions">
              <button className="btn primary small" onClick={() => openUrl(clipboardUrl)}>
                {t("downloadNow")}
              </button>
              <button className="btn secondary small" onClick={() => { openUrl(clipboardUrl); goTab("new"); }}>
                {t("openApp")}
              </button>
              <button className="ghost-btn" onClick={() => setClipboardUrl(null)}>
                {t("dismiss")}
              </button>
            </div>
          </div>
        )}

        {binaries && !binaries.ready && (
          <div className="overlay">
            <div className="overlay-card">
              <h2>{t("binariesTitle")}</h2>
              <p className="muted">{t("binariesDesc")}</p>
              {binError ? (
                <div className="error-box">
                  <b>{t("errorTitle")}</b>
                  <code>{binError}</code>
                </div>
              ) : (
                <>
                  <ToolRow
                    label="yt-dlp"
                    done={binProg?.tool === "ffmpeg" || binProg?.tool === "ffprobe" || binProg?.tool === "done"}
                    prog={binProg?.tool === "yt-dlp" ? binProg : null}
                  />
                  <ToolRow
                    label="ffmpeg"
                    done={binProg?.tool === "done"}
                    prog={
                      binProg?.tool === "ffmpeg" || binProg?.tool === "ffprobe" ? binProg : null
                    }
                  />
                  {binProg && (
                    <div className="muted small">
                      {binProg.stage === "download"
                        ? `${t("downloading")} ${binProg.tool} — ${pct(binProg)}`
                        : binProg.stage === "extract"
                          ? `${t("extracting")} ${binProg.tool}…`
                          : t("done")}
                    </div>
                  )}
                </>
              )}
            </div>
          </div>
        )}
      </div>
    </I18nContext.Provider>
  );
}

function formatSpeedShort(bps: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let v = bps;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${units[i]}/s`;
}

function pct(p: BinariesProgress): string {
  if (p.total > 0) return `${Math.floor((p.received / p.total) * 100)}%`;
  return `${(p.received / 1024 / 1024).toFixed(1)} MB`;
}

function ToolRow({
  label,
  done,
  prog,
}: {
  label: string;
  done: boolean;
  prog: BinariesProgress | null;
}) {
  const percent = done ? 100 : prog ? (prog.total > 0 ? (prog.received / prog.total) * 100 : 8) : 0;
  return (
    <div className="tool-row">
      <span className="tool-name">{label}</span>
      <div className="tool-bar">
        <div className="tool-bar-fill" style={{ width: `${Math.min(100, percent)}%` }} />
      </div>
    </div>
  );
}

function Dropdown({ button, children }: { button: React.ReactNode; children: React.ReactNode }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  return (
    <div className="dropdown" ref={ref}>
      <div onClick={() => setOpen((o) => !o)}>{button}</div>
      {open && <div className="dropdown-menu">{children}</div>}
    </div>
  );
}
