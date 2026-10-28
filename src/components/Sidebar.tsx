import { useApp, type QueueFilter } from "../lib/app-context";
import { useI18n } from "../lib/i18n";

interface Entry {
  id: string;
  icon: string;
  label: string;
  badge: number | null;
  active: boolean;
  onClick: () => void;
}

export default function Sidebar() {
  const { t } = useI18n();
  const { items, tab, queueFilter, goTab } = useApp();

  const counts = {
    all: items.length,
    downloading: items.filter(
      (i) => i.status === "active" || i.status === "queued" || i.status === "scheduled",
    ).length,
    completed: items.filter((i) => i.status === "completed").length,
    paused: items.filter((i) => i.status === "paused").length,
    failed: items.filter((i) => i.status === "error").length,
  };

  const filterEntries: { filter: QueueFilter; icon: string; label: string }[] = [
    { filter: "all", icon: "▣", label: t("tabQueue") },
    { filter: "downloading", icon: "↓", label: t("tabDownloading") },
    { filter: "completed", icon: "✓", label: t("tabCompleted") },
    { filter: "paused", icon: "❚❚", label: t("tabPaused") },
    { filter: "failed", icon: "×", label: t("tabFailed") },
  ];

  const renderEntry = ({ id, icon, label, badge, active, onClick }: Entry) => (
    <button key={id} className={`side-item ${active ? "active" : ""}`} onClick={onClick}>
      <span className="side-icon">{icon}</span>
      <span className="side-label">{label}</span>
      {badge ? <span className="side-badge">{badge}</span> : null}
    </button>
  );

  return (
    <aside className="sidebar">
      <div className="brand">
        <img src="/vidaro-logo.png" alt="" className="brand-logo" draggable={false} />
        <span className="brand-name">Vidaro</span>
      </div>

      <div className="side-section">
        {renderEntry({
          id: "new",
          icon: "＋",
          label: t("tabNew"),
          badge: null,
          active: tab === "new",
          onClick: () => goTab("new"),
        })}
      </div>

      <div className="side-title">Downloads</div>
      <div className="side-section">
        {filterEntries.map(({ filter, icon, label }) =>
          renderEntry({
            id: `q-${filter}`,
            icon,
            label,
            badge: counts[filter] || null,
            active: tab === "queue" && queueFilter === filter,
            onClick: () => goTab("queue", filter),
          }),
        )}
      </div>

      <div className="side-section bottom">
        {renderEntry({
          id: "history",
          icon: "☰",
          label: t("tabHistory"),
          badge: null,
          active: tab === "history",
          onClick: () => goTab("history"),
        })}
        {renderEntry({
          id: "settings",
          icon: "⚙",
          label: t("tabSettings"),
          badge: null,
          active: tab === "settings",
          onClick: () => goTab("settings"),
        })}
      </div>
    </aside>
  );
}
