/**
 * Application state: settings, queue, navigation and clipboard suggestions.
 *
 * This replaces the old `AppContext.current` mutable global with a real React
 * context, so every consumer re-renders when settings or the queue change.
 */
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

import * as api from "./api";
import type { AppInfo, Item, Settings, ToolStatus } from "./types";

export type Tab = "new" | "queue" | "history" | "settings";
export type QueueFilter = "all" | "downloading" | "completed" | "paused" | "failed";

interface AppContextValue {
  settings: Settings;
  updateSettings: (patch: Partial<Settings>) => void;

  info: AppInfo | null;
  tools: ToolStatus | null;
  setTools: (status: ToolStatus) => void;

  items: Item[];
  tab: Tab;
  queueFilter: QueueFilter;
  goTab: (tab: Tab, filter?: QueueFilter) => void;

  pendingUrl: string | null;
  openUrl: (url: string) => void;
  consumePendingUrl: () => void;

  clipboardLink: string | null;
  dismissClipboard: () => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export function useApp(): AppContextValue {
  const value = useContext(AppContext);
  if (!value) {
    throw new Error("useApp must be used inside <AppProvider>");
  }
  return value;
}

const SAVE_DEBOUNCE_MS = 350;
const CLIPBOARD_TOAST_MS = 12_000;

export function AppProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [tools, setTools] = useState<ToolStatus | null>(null);
  const [items, setItems] = useState<Item[]>([]);
  const [tab, setTab] = useState<Tab>("queue");
  const [queueFilter, setQueueFilter] = useState<QueueFilter>("all");
  const [pendingUrl, setPendingUrl] = useState<string | null>(null);
  const [clipboardLink, setClipboardLink] = useState<string | null>(null);

  const saveTimer = useRef<number | null>(null);
  const clipboardTimer = useRef<number | null>(null);

  /* ------------------------------------------------------------ bootstrap */

  useEffect(() => {
    let cancelled = false;

    (async () => {
      const [loadedSettings, loadedInfo] = await Promise.allSettled([
        api.getSettings(),
        api.appInfo(),
      ]);

      if (cancelled) return;

      if (loadedSettings.status === "fulfilled") {
        setSettings(loadedSettings.value);
      } else {
        console.error("could not load settings", loadedSettings.reason);
        setSettings(FALLBACK_SETTINGS);
      }

      if (loadedInfo.status === "fulfilled") setInfo(loadedInfo.value);

      try {
        setItems(await api.getQueue());
      } catch (error) {
        console.error("could not load queue", error);
      }

      try {
        const status = await api.binariesStatus();
        if (!cancelled) setTools(status);
      } catch (error) {
        console.error("could not read tool status", error);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  /* --------------------------------------------------------------- events */

  useEffect(() => {
    const unlisten: Promise<() => void>[] = [];

    unlisten.push(
      api.onItemUpdated((updated) => {
        setItems((current) => {
          const index = current.findIndex((item) => item.id === updated.id);
          if (index === -1) {
            return [...current, updated].sort((a, b) => a.createdAt - b.createdAt);
          }
          const next = current.slice();
          next[index] = updated;
          return next;
        });
      }),
    );

    unlisten.push(
      api.onQueueChanged(() => {
        void api.getQueue().then(setItems).catch(() => undefined);
      }),
    );

    unlisten.push(
      api.onSettingsChanged((next) => {
        // Ignore the echo of a save that is still being written: a local edit
        // that happened after the request would otherwise be reverted.
        if (saveTimer.current !== null) return;
        setSettings(next);
      }),
    );

    unlisten.push(
      api.onClipboardLink(({ url }) => {
        setClipboardLink(url);
        if (clipboardTimer.current) window.clearTimeout(clipboardTimer.current);
        clipboardTimer.current = window.setTimeout(
          () => setClipboardLink(null),
          CLIPBOARD_TOAST_MS,
        );
      }),
    );

    return () => {
      unlisten.forEach((promise) =>
        promise.then((off) => off()).catch(() => undefined),
      );
      if (clipboardTimer.current) window.clearTimeout(clipboardTimer.current);
    };
  }, []);

  /* ---------------------------------------------------- theme + direction */

  const lang = settings?.language ?? "en";
  const dir = lang === "fa" ? "rtl" : "ltr";
  const themeMode = settings?.theme ?? "system";

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = themeMode === "system" ? media.matches : themeMode === "dark";
      document.documentElement.dataset.theme = dark ? "dark" : "light";
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [themeMode]);

  useEffect(() => {
    document.documentElement.dir = dir;
    document.documentElement.lang = lang;
  }, [dir, lang]);

  /* ------------------------------------------------------------ settings */

  const updateSettings = useCallback((patch: Partial<Settings>) => {
    setSettings((current) => {
      if (!current) return current;
      const next = { ...current, ...patch };

      if (saveTimer.current) window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(() => {
        saveTimer.current = null;
        void api.setSettings(next).catch((error) => {
          console.error("could not save settings", error);
        });
      }, SAVE_DEBOUNCE_MS);

      return next;
    });
  }, []);

  /* ---------------------------------------------------------- navigation */

  const goTab = useCallback((next: Tab, filter?: QueueFilter) => {
    setTab(next);
    if (next === "queue") setQueueFilter(filter ?? "all");
  }, []);

  const openUrl = useCallback((url: string) => {
    setPendingUrl(url);
    setClipboardLink(null);
    setTab("new");
  }, []);

  const consumePendingUrl = useCallback(() => setPendingUrl(null), []);
  const dismissClipboard = useCallback(() => setClipboardLink(null), []);

  /* ------------------------------------------------------------- context */

  const value = useMemo<AppContextValue | null>(() => {
    if (!settings) return null;
    return {
      settings,
      updateSettings,
      info,
      tools,
      setTools,
      items,
      tab,
      queueFilter,
      goTab,
      pendingUrl,
      openUrl,
      consumePendingUrl,
      clipboardLink,
      dismissClipboard,
    };
  }, [
    settings,
    updateSettings,
    info,
    tools,
    items,
    tab,
    queueFilter,
    goTab,
    pendingUrl,
    openUrl,
    consumePendingUrl,
    clipboardLink,
    dismissClipboard,
  ]);

  if (!value) {
    return <div className="splash" />;
  }

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

const FALLBACK_SETTINGS: Settings = {
  language: "en",
  theme: "system",
  defaultDir: "",
  maxConcurrent: 2,
  rateLimit: "",
  writeSubs: false,
  subLangs: "en",
  embedMetadata: true,
  embedThumbnail: true,
  proxyMode: "off",
  proxyHost: "",
  proxyPort: "",
  schedulerEnabled: false,
  schedulerStart: "02:00",
  schedulerEnd: "06:00",
  autostartEnabled: true,
  closeToTray: true,
  clipboardWatch: true,
  notifyOnClipboard: true,
};
