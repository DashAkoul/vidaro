import { useApp, type QueueFilter } from "../lib/app-context";
import { useI18n, type TKey } from "../lib/i18n";
import Dropdown from "./Dropdown";

const TITLES: Record<QueueFilter, TKey> = {
  all: "tabQueue",
  downloading: "tabDownloading",
  completed: "tabCompleted",
  paused: "tabPaused",
  failed: "tabFailed",
};

export default function Topbar() {
  const { t } = useI18n();
  const { settings, updateSettings, tab, queueFilter, goTab } = useApp();

  const title =
    tab === "new"
      ? t("tabNew")
      : tab === "history"
        ? t("tabHistory")
        : tab === "settings"
          ? t("tabSettings")
          : t(TITLES[queueFilter]);

  const themeIcon =
    settings.theme === "system" ? "◉" : settings.theme === "dark" ? "◐" : "☼";

  return (
    <header className="topbar">
      <div className="topbar-left">
        <h1>{title}</h1>
      </div>

      <div className="topbar-right">
        <Dropdown button={<span className="ctl-btn">🌐 {settings.language === "fa" ? "فا" : "EN"}</span>}>
          <div className="menu-item" onClick={() => updateSettings({ language: "en" })}>
            {settings.language === "en" ? "✓ " : ""}English
          </div>
          <div className="menu-item" onClick={() => updateSettings({ language: "fa" })}>
            {settings.language === "fa" ? "✓ " : ""}فارسی
          </div>
        </Dropdown>

        <Dropdown button={<span className="ctl-btn">{themeIcon}</span>}>
          <div className="menu-item" onClick={() => updateSettings({ theme: "light" })}>
            {settings.theme === "light" ? "✓ " : ""}☼ {t("light")}
          </div>
          <div className="menu-item" onClick={() => updateSettings({ theme: "dark" })}>
            {settings.theme === "dark" ? "✓ " : ""}◐ {t("dark")}
          </div>
          <div className="menu-item" onClick={() => updateSettings({ theme: "system" })}>
            {settings.theme === "system" ? "✓ " : ""}◉ {t("themeSystem")}
          </div>
        </Dropdown>

        <button className="btn primary" onClick={() => goTab("new")}>
          ＋ {t("tabNew")}
        </button>
      </div>
    </header>
  );
}
