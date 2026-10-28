import { useApp } from "../lib/app-context";
import { useI18n } from "../lib/i18n";

/** In-app suggestion shown when a YouTube link is copied while the app is open. */
export default function ClipboardToast() {
  const { t } = useI18n();
  const { clipboardLink, dismissClipboard, openUrl } = useApp();

  if (!clipboardLink) return null;

  return (
    <div className="toast show">
      <span className="toast-icon">📥</span>
      <div className="toast-content">
        <b>{t("clipboardToast")}</b>
        <code dir="ltr">{clipboardLink}</code>
      </div>
      <div className="toast-actions">
        <button className="btn primary" onClick={() => openUrl(clipboardLink)}>
          {t("downloadNow")}
        </button>
        <button className="ghost-btn" onClick={dismissClipboard}>
          {t("dismiss")}
        </button>
      </div>
    </div>
  );
}
