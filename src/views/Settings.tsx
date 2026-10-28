import { useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";

import { useApp } from "../lib/app-context";
import {
  binariesStatus,
  describeError,
  ensureBinaries,
  exportData,
  importData,
  onToolsProgress,
  openToolsFolder,
  updateYtdlp,
} from "../lib/api";
import { useI18n } from "../lib/i18n";
import type { ToolsProgress } from "../lib/types";

export default function Settings() {
  const { t } = useI18n();
  const { settings, updateSettings, tools, setTools } = useApp();

  const [toast, setToast] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ToolsProgress | null>(null);
  const toastTimer = useRef<number | null>(null);

  useEffect(() => {
    const subscription = onToolsProgress(setProgress);
    return () => {
      subscription.then((off) => off()).catch(() => undefined);
    };
  }, []);

  useEffect(
    () => () => {
      if (toastTimer.current) window.clearTimeout(toastTimer.current);
    },
    [],
  );

  function flash(message: string) {
    setToast(message);
    if (toastTimer.current) window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 1800);
  }

  async function chooseFolder() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string" && picked) {
      updateSettings({ defaultDir: picked });
      flash(t("saved"));
    }
  }

  async function refreshTools() {
    try {
      setTools(await binariesStatus());
    } catch (e) {
      setError(describeError(e));
    }
  }

  async function installTools() {
    setBusy("install");
    setError(null);
    try {
      setTools(await ensureBinaries());
      flash(t("saved"));
    } catch (e) {
      setError(describeError(e));
    } finally {
      setBusy(null);
      setProgress(null);
    }
  }

  async function updateTools() {
    setBusy("update");
    setError(null);
    try {
      await updateYtdlp();
      await refreshTools();
      flash(t("saved"));
    } catch (e) {
      setError(describeError(e));
    } finally {
      setBusy(null);
      setProgress(null);
    }
  }

  async function doExport() {
    setError(null);
    try {
      const target = await save({
        title: t("exportData"),
        defaultPath: "vidaro-backup.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (typeof target === "string" && target) {
        await exportData(target);
        flash(t("exportSuccess"));
      }
    } catch (e) {
      setError(describeError(e));
    }
  }

  async function doImport() {
    setError(null);
    try {
      const source = await open({
        title: t("importData"),
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (typeof source === "string" && source) {
        await importData(source);
        flash(t("importSuccess"));
      }
    } catch (e) {
      setError(t("importError", { error: describeError(e) }));
    }
  }

  const installPercent =
    progress && progress.total > 0
      ? Math.floor((progress.received / progress.total) * 100)
      : 0;

  return (
    <div className="view settings-view">
      <section className="settings-section">
        <h3>{t("settingsGeneral")}</h3>

        <div className="settings-row">
          <span>{t("language")}</span>
          <div className="seg">
            <button
              className={`seg-btn ${settings.language === "fa" ? "active" : ""}`}
              onClick={() => updateSettings({ language: "fa" })}
            >
              فارسی
            </button>
            <button
              className={`seg-btn ${settings.language === "en" ? "active" : ""}`}
              onClick={() => updateSettings({ language: "en" })}
            >
              English
            </button>
          </div>
        </div>

        <div className="settings-row">
          <span>{t("theme")}</span>
          <div className="seg">
            <button
              className={`seg-btn ${settings.theme === "light" ? "active" : ""}`}
              onClick={() => updateSettings({ theme: "light" })}
            >
              ☼ {t("light")}
            </button>
            <button
              className={`seg-btn ${settings.theme === "dark" ? "active" : ""}`}
              onClick={() => updateSettings({ theme: "dark" })}
            >
              ◐ {t("dark")}
            </button>
            <button
              className={`seg-btn ${settings.theme === "system" ? "active" : ""}`}
              onClick={() => updateSettings({ theme: "system" })}
            >
              ◉ {t("themeSystem")}
            </button>
          </div>
        </div>

        <div className="settings-row">
          <span>{t("defaultFolder")}</span>
          <div className="dir-row">
            <input className="dir-input" value={settings.defaultDir} readOnly dir="ltr" />
            <button className="btn secondary" onClick={() => void chooseFolder()}>
              {t("chooseFolder")}
            </button>
          </div>
        </div>

        <div className="settings-row">
          <span>{t("maxConcurrent")}</span>
          <input
            type="number"
            min={1}
            max={8}
            value={settings.maxConcurrent}
            onChange={(event) =>
              updateSettings({
                maxConcurrent: Math.min(8, Math.max(1, Number(event.target.value) || 1)),
              })
            }
            style={{ width: 90 }}
          />
        </div>

        <div className="settings-row">
          <span>{t("rateLimit")}</span>
          <input
            type="text"
            value={settings.rateLimit}
            onChange={(event) => updateSettings({ rateLimit: event.target.value })}
            placeholder="5M"
            dir="ltr"
            style={{ width: 140 }}
          />
        </div>
      </section>

      <section className="settings-section">
        <h3>{t("behavior")}</h3>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.closeToTray}
            onChange={(event) => updateSettings({ closeToTray: event.target.checked })}
          />
          <span>{t("closeToTray")}</span>
        </label>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.clipboardWatch}
            onChange={(event) => updateSettings({ clipboardWatch: event.target.checked })}
          />
          <span>{t("clipboardWatch")}</span>
        </label>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.notifyOnClipboard}
            onChange={(event) => updateSettings({ notifyOnClipboard: event.target.checked })}
            disabled={!settings.clipboardWatch}
          />
          <span>{t("notifyOnClipboard")}</span>
        </label>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.autostartEnabled}
            onChange={(event) => updateSettings({ autostartEnabled: event.target.checked })}
          />
          <span>{t("autostartEnabled")}</span>
        </label>
        <div className="muted small" style={{ marginInlineStart: 24 }}>
          {t("autostartHint")}
        </div>
      </section>

      <section className="settings-section">
        <h3>{t("tools")}</h3>

        <div className="settings-row">
          <span>yt-dlp</span>
          <span className="muted">
            {tools?.ytdlpFound
              ? `${t("version")} ${tools.ytdlpVersion ?? "?"}`
              : t("missing")}
          </span>
          <button
            className="btn secondary"
            onClick={() => void updateTools()}
            disabled={busy !== null || !tools?.ytdlpFound}
          >
            {busy === "update" ? t("updating") : t("updateYtdlp")}
          </button>
        </div>

        <div className="settings-row">
          <span>ffmpeg</span>
          <span className="muted">{tools?.ffmpegFound ? t("installed") : t("missing")}</span>
          {!tools?.ready && (
            <button
              className="btn secondary"
              onClick={() => void installTools()}
              disabled={busy !== null}
            >
              {busy === "install" ? t("installing", { tool: "…" }) : t("installTools")}
            </button>
          )}
          <button className="ghost-btn" onClick={() => void openToolsFolder()}>
            {t("openToolsFolder")}
          </button>
        </div>

        {busy === "install" && progress && (
          <div className="muted small">
            {t("installing", { tool: progress.tool })}
            {progress.total > 0 ? ` — ${installPercent}%` : ""}
          </div>
        )}
      </section>

      <section className="settings-section">
        <h3>{t("format")}</h3>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.writeSubs}
            onChange={(event) => updateSettings({ writeSubs: event.target.checked })}
          />
          <span>{t("subs")}</span>
        </label>

        <div className="settings-row">
          <span>{t("subLangs")}</span>
          <input
            type="text"
            value={settings.subLangs}
            onChange={(event) => updateSettings({ subLangs: event.target.value })}
            dir="ltr"
            style={{ width: 140 }}
          />
        </div>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.embedMetadata}
            onChange={(event) => updateSettings({ embedMetadata: event.target.checked })}
          />
          <span>{t("embedMetadata")}</span>
        </label>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.embedThumbnail}
            onChange={(event) => updateSettings({ embedThumbnail: event.target.checked })}
          />
          <span>{t("embedThumbnail")}</span>
        </label>
      </section>

      <section className="settings-section">
        <h3>{t("proxy")}</h3>

        <div className="settings-row">
          <span>{t("proxyMode")}</span>
          <div className="seg" style={{ flex: 1, minWidth: 0 }}>
            {(["off", "system", "manual"] as const).map((mode) => (
              <button
                key={mode}
                className={`seg-btn ${settings.proxyMode === mode ? "active" : ""}`}
                onClick={() => updateSettings({ proxyMode: mode })}
              >
                {mode === "off"
                  ? t("proxyOff")
                  : mode === "system"
                    ? t("proxySystem")
                    : t("proxyManual")}
              </button>
            ))}
          </div>
        </div>

        {settings.proxyMode === "manual" && (
          <div className="settings-row" style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <label style={{ minWidth: 60 }}>{t("proxyHost")}</label>
            <input
              type="text"
              value={settings.proxyHost}
              onChange={(event) => updateSettings({ proxyHost: event.target.value })}
              placeholder="127.0.0.1"
              dir="ltr"
              style={{ flex: 1, minWidth: 120 }}
            />
            <label style={{ minWidth: 50 }}>{t("proxyPort")}</label>
            <input
              type="text"
              inputMode="numeric"
              value={settings.proxyPort}
              onChange={(event) =>
                updateSettings({ proxyPort: event.target.value.replace(/\D/g, "") })
              }
              placeholder="8080"
              dir="ltr"
              style={{ width: 80 }}
            />
          </div>
        )}

        <div className="muted small" style={{ marginTop: 8, marginInlineStart: 24 }}>
          {t("proxyNote")}
        </div>
      </section>

      <section className="settings-section">
        <h3>{t("scheduler")}</h3>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.schedulerEnabled}
            onChange={(event) => updateSettings({ schedulerEnabled: event.target.checked })}
          />
          <span>{t("schedulerEnabled")}</span>
        </label>

        <div className="settings-row">
          <span>{t("schedulerStart")}</span>
          <input
            type="time"
            value={settings.schedulerStart}
            onChange={(event) => updateSettings({ schedulerStart: event.target.value })}
            disabled={!settings.schedulerEnabled}
            style={{ width: 140 }}
          />
        </div>

        <div className="settings-row">
          <span>{t("schedulerEnd")}</span>
          <input
            type="time"
            value={settings.schedulerEnd}
            onChange={(event) => updateSettings({ schedulerEnd: event.target.value })}
            disabled={!settings.schedulerEnabled}
            style={{ width: 140 }}
          />
        </div>

        <div className="muted small" style={{ marginTop: 8, marginInlineStart: 24 }}>
          {t("schedulerNote")}
        </div>
      </section>

      <section className="settings-section">
        <h3>{t("importExport")}</h3>
        <div className="settings-row">
          <button className="btn secondary" onClick={() => void doExport()}>
            {t("exportData")}
          </button>
          <button className="btn secondary" onClick={() => void doImport()}>
            {t("importData")}
          </button>
        </div>
      </section>

      {error && (
        <div className="error-box">
          <b>{t("error")}</b>
          <code>{error}</code>
        </div>
      )}

      {toast && <div className="saved-toast">{toast}</div>}
    </div>
  );
}
