import { useContext, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { binariesStatus, updateYtdlp, exportData, importData, getSettings } from "../api";
import { I18nContext } from "../i18n";
import { AppContext } from "../App";
import type { BinariesStatus } from "../types";

export default function SettingsView({
  binaries,
  onBinaries,
}: {
  binaries: BinariesStatus | null;
  onBinaries: (b: BinariesStatus) => void;
}) {
  const { t } = useContext(I18nContext);
  const app = AppContext.current;
  const [saved, setSaved] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [updateOut, setUpdateOut] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);

  if (!app) return null;
  const { settings, updateSettings } = app;

  useEffect(() => {
    if (!saved) return;
    const id = setTimeout(() => setSaved(false), 1500);
    return () => clearTimeout(id);
  }, [saved]);

  async function chooseDefaultDir() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string" && picked) {
      updateSettings({ defaultDir: picked });
      setSaved(true);
    }
  }

  async function doUpdateYtdlp() {
    setUpdating(true);
    setUpdateOut(null);
    try {
      const out = await updateYtdlp();
      setUpdateOut(out);
      // refresh version
      onBinaries(await binariesStatus());
    } catch (e) {
      setUpdateOut(String(e));
    } finally {
      setUpdating(false);
    }
  }

  async function doExport() {
    setExporting(true);
    try {
      const picked = await open({ directory: false, multiple: false, title: t("exportData") });
      if (typeof picked === "string" && picked) {
        await exportData(picked);
        setSaved(true);
      }
    } catch (e) {
      setImportError(String(e));
    } finally {
      setExporting(false);
    }
  }

  async function doImport() {
    setImporting(true);
    setImportError(null);
    try {
      const picked = await open({ directory: false, multiple: false, title: t("importData"), filters: [{ name: "JSON", extensions: ["json"] }] });
      if (typeof picked === "string" && picked) {
        await importData(picked);
        setSaved(true);
        // Reload settings
        const newSettings = await getSettings();
        updateSettings(newSettings);
      }
    } catch (e) {
      setImportError(String(e));
    } finally {
      setImporting(false);
    }
  }

  return (
    <div className="view settings-view">
      <h1>{t("tabSettings")}</h1>

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
            <button className="btn secondary" onClick={chooseDefaultDir}>
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
            onChange={(e) =>
              updateSettings({ maxConcurrent: Math.min(8, Math.max(1, Number(e.target.value) || 1)) })
            }
            style={{ width: 90 }}
          />
        </div>

        <div className="settings-row">
          <span>{t("rateLimit")}</span>
          <input
            type="text"
            value={settings.rateLimit}
            onChange={(e) => updateSettings({ rateLimit: e.target.value })}
            placeholder="5M"
            dir="ltr"
            style={{ width: 140 }}
          />
        </div>

        <div className="settings-row">
          <span>{t("proxy")}</span>
          <input
            type="text"
            value={settings.proxy}
            onChange={(e) => updateSettings({ proxy: e.target.value })}
            placeholder={t("proxyPlaceholder")}
            dir="ltr"
            style={{ width: 300 }}
          />
        </div>
      </section>

      <section className="settings-section">
        <h3>{t("scheduler")}</h3>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.schedulerEnabled}
            onChange={(e) => updateSettings({ schedulerEnabled: e.target.checked })}
          />
          <span>{t("schedulerEnabled")}</span>
        </label>

        <div className="settings-row">
          <span>{t("schedulerStart")}</span>
          <input
            type="time"
            value={settings.schedulerStart}
            onChange={(e) => updateSettings({ schedulerStart: e.target.value })}
            disabled={!settings.schedulerEnabled}
            style={{ width: 140 }}
          />
        </div>

        <div className="settings-row">
          <span>{t("schedulerEnd")}</span>
          <input
            type="time"
            value={settings.schedulerEnd}
            onChange={(e) => updateSettings({ schedulerEnd: e.target.value })}
            disabled={!settings.schedulerEnabled}
            style={{ width: 140 }}
          />
        </div>

        <div className="muted small" style={{ marginTop: 8, marginLeft: 24 }}>
          {t("schedulerNote")}
        </div>
      </section>

      <section className="settings-section">
        <h3>{t("format")}</h3>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.writeSubs}
            onChange={(e) => updateSettings({ writeSubs: e.target.checked })}
          />
          <span>{t("subs")}</span>
        </label>

        <div className="settings-row">
          <span>{t("subLangs")}</span>
          <input
            type="text"
            value={settings.subLangs}
            onChange={(e) => updateSettings({ subLangs: e.target.value })}
            dir="ltr"
            style={{ width: 140 }}
          />
        </div>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.embedMetadata}
            onChange={(e) => updateSettings({ embedMetadata: e.target.checked })}
          />
          <span>{t("embedMetadata")}</span>
        </label>

        <label className="settings-row check">
          <input
            type="checkbox"
            checked={settings.embedThumbnail}
            onChange={(e) => updateSettings({ embedThumbnail: e.target.checked })}
          />
          <span>{t("embedThumbnail")}</span>
        </label>
      </section>

      <section className="settings-section">
        <h3>{t("tools")}</h3>
        {binaries && (
          <>
            <div className="settings-row">
              <span>yt-dlp</span>
              <span className="muted">
                {binaries.ytdlpFound
                  ? `${t("version")} ${binaries.ytdlpVersion ?? "?"}`
                  : t("missing")}
              </span>
              <button className="btn secondary" onClick={doUpdateYtdlp} disabled={updating || !binaries.ytdlpFound}>
                {updating ? t("updating") : t("updateYtdlp")}
              </button>
            </div>
            <div className="settings-row">
              <span>ffmpeg</span>
              <span className="muted">{binaries.ffmpegFound ? t("installed") : t("missing")}</span>
            </div>
            {updateOut && (
              <div className="notice-box">
                <b>{t("updateOutput")}:</b>
                <pre>{updateOut}</pre>
              </div>
            )}
          </>
        )}
      </section>

      <section className="settings-section">
        <h3>{t("importExport")}</h3>

        <div className="settings-row">
          <button className="btn secondary" onClick={doExport} disabled={exporting}>
            {exporting ? "..." : t("exportData")}
          </button>
          <button className="btn secondary" onClick={doImport} disabled={importing}>
            {importing ? "..." : t("importData")}
          </button>
        </div>

        {importError && <div className="error-box">{t("importError", { error: importError })}</div>}
      </section>

      {saved && <div className="saved-toast">{t("saved")}</div>}
    </div>
  );
}
