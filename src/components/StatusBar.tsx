import { useApp } from "../lib/app-context";
import { formatSpeed } from "../lib/format";
import { useI18n } from "../lib/i18n";

export default function StatusBar() {
  const { t } = useI18n();
  const { items, info } = useApp();

  const active = items.filter((item) => item.status === "active");
  const pending = items.filter((item) => item.status === "queued").length;
  const totalSpeed = active.reduce((sum, item) => sum + (item.speedBps || 0), 0);

  return (
    <footer className="statusbar">
      <span className={totalSpeed > 0 ? "status-speed on" : "status-speed"}>
        ≋ {totalSpeed > 0 ? formatSpeed(totalSpeed) : "0 B/s"}
        <span className="muted"> {t("totalSpeed")}</span>
      </span>
      <span className="muted">
        {t("activeDownloads")}: {active.length + pending}
      </span>
      <span className="muted status-right">
        Vidaro {info?.version ?? "2.0.0"}
      </span>
    </footer>
  );
}
