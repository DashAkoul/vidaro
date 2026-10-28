import { useEffect, useState } from "react";

import { useApp } from "../lib/app-context";
import { describeError, ensureBinaries, onToolsProgress } from "../lib/api";
import { useI18n } from "../lib/i18n";
import type { ToolsProgress } from "../lib/types";

/**
 * First-run gate: offers to install yt-dlp and ffmpeg.
 *
 * The app stays usable if dismissed — Settings exposes the same action.
 */
export default function ToolsOverlay() {
  const { t } = useI18n();
  const { tools, setTools } = useApp();

  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ToolsProgress | null>(null);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    const subscription = onToolsProgress(setProgress);
    return () => {
      subscription.then((off) => off()).catch(() => undefined);
    };
  }, []);

  if (dismissed || !tools || tools.ready) return null;

  async function install() {
    setBusy(true);
    setError(null);
    try {
      const status = await ensureBinaries();
      setTools(status);
    } catch (e) {
      setError(describeError(e));
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }

  const percent =
    progress && progress.total > 0
      ? Math.floor((progress.received / progress.total) * 100)
      : progress
        ? 8
        : 0;

  return (
    <div className="overlay">
      <div className="overlay-card">
        <h2>{t("toolsMissingTitle")}</h2>
        <p className="muted">{t("toolsMissingDesc")}</p>

        <div className="tool-row">
          <span className="tool-name">yt-dlp</span>
          <div className="tool-bar">
            <div
              className="tool-bar-fill"
              style={{ width: `${tools.ytdlpFound ? 100 : percent}%` }}
            />
          </div>
        </div>
        <div className="tool-row">
          <span className="tool-name">ffmpeg</span>
          <div className="tool-bar">
            <div
              className="tool-bar-fill"
              style={{ width: `${tools.ffmpegFound ? 100 : progress ? percent : 0}%` }}
            />
          </div>
        </div>

        {busy && progress && (
          <div className="muted small">
            {t("installing", { tool: progress.tool })}
            {progress.total > 0 ? ` — ${percent}%` : ""}
          </div>
        )}

        {error && (
          <div className="error-box">
            <b>{t("error")}</b>
            <code>{error}</code>
          </div>
        )}

        <div className="toast-actions">
          <button className="btn primary" onClick={install} disabled={busy}>
            {busy ? t("installing", { tool: "…" }) : error ? t("retryInstall") : t("installTools")}
          </button>
          <button className="ghost-btn" onClick={() => setDismissed(true)} disabled={busy}>
            {t("dismiss")}
          </button>
        </div>
      </div>
    </div>
  );
}
