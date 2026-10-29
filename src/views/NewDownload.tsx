import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { useApp } from "../lib/app-context";
import {
  cancelFetch,
  describeError,
  enqueueBatch,
  fetchInfo,
  onInfoProgress,
} from "../lib/api";
import { formatDuration } from "../lib/format";
import { useI18n } from "../lib/i18n";
import type { InfoResult, VideoEntry } from "../lib/types";

const STANDARD_HEIGHTS = [2160, 1440, 1080, 720, 480, 360];

function heightLabel(height: number): string {
  if (height >= 2160) return `${height}p (4K)`;
  if (height >= 1440) return `${height}p (2K)`;
  return `${height}p`;
}

type KindHint = "auto" | "video" | "playlist" | "channel";

export default function NewDownload() {
  const { t } = useI18n();
  const { settings, updateSettings, pendingUrl, consumePendingUrl, goTab } = useApp();

  const [url, setUrl] = useState("");
  const [kindHint, setKindHint] = useState<KindHint>("auto");
  const [fetching, setFetching] = useState(false);
  const [progressMessage, setProgressMessage] = useState("");
  const [info, setInfo] = useState<InfoResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [audioOnly, setAudioOnly] = useState(false);
  const [quality, setQuality] = useState("best");
  const [dir, setDir] = useState(settings.defaultDir);
  const [notice, setNotice] = useState<string | null>(null);
  const [scheduleStart, setScheduleStart] = useState("");
  const [scheduleEnd, setScheduleEnd] = useState("");

  // Follow the default folder if it is changed in Settings.
  useEffect(() => {
    setDir(settings.defaultDir);
  }, [settings.defaultDir]);

  useEffect(() => {
    const subscription = onInfoProgress(({ message }) => {
      setProgressMessage(message === "done" ? "" : message);
    });
    return () => {
      subscription.then((off) => off()).catch(() => undefined);
    };
  }, []);

  const runFetch = useCallback(
    async (target: string) => {
      const trimmed = target.trim();
      if (!trimmed) return;

      setFetching(true);
      setError(null);
      setInfo(null);
      setNotice(null);
      setProgressMessage("");
      try {
        setInfo(await fetchInfo(trimmed, kindHint));
      } catch (e) {
        setError(describeError(e));
      } finally {
        setFetching(false);
      }
    },
    [kindHint],
  );

  // A link handed over by the clipboard toast.
  useEffect(() => {
    if (!pendingUrl) return;
    setUrl(pendingUrl);
    consumePendingUrl();
    void runFetch(pendingUrl);
  }, [pendingUrl, consumePendingUrl, runFetch]);

  // Pick sensible defaults once the metadata arrives.
  useEffect(() => {
    if (!info) return;
    if (info.kind === "collection") {
      setSelected(new Set(info.entries.map((entry) => entry.index)));
    } else if (info.heights.length > 0) {
      setQuality(String(info.heights.find((h) => h <= 1080) ?? info.heights[0]));
    }
  }, [info]);

  const heights = useMemo(
    () => (info?.kind === "video" ? info.heights : STANDARD_HEIGHTS),
    [info],
  );

  async function chooseDir(): Promise<string> {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string" && picked) {
      setDir(picked);
      updateSettings({ defaultDir: picked });
      return picked;
    }
    return dir;
  }

  function toggle(index: number) {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  }

  async function addToQueue() {
    if (!info) return;

    let destination = dir;
    if (!destination) {
      destination = await chooseDir();
      if (!destination) {
        setNotice(t("chooseFolderFirst"));
        return;
      }
    }
    setNotice(null);

    const entries: { videoId: string; title: string; url: string; index: number | null }[] =
      info.kind === "video"
        ? [{ videoId: info.id, title: info.title, url: url.trim(), index: null }]
        : info.entries
            .filter((entry) => selected.has(entry.index))
            .map((entry: VideoEntry) => ({
              videoId: entry.id,
              title: entry.title,
              url: entry.url,
              index: entry.index,
            }));

    if (entries.length === 0) {
      setNotice(t("noSelection"));
      return;
    }

    try {
      await enqueueBatch({
        items: entries,
        dir: destination,
        batchName: info.kind === "collection" ? info.title : "",
        quality,
        audioOnly,
        numbering: info.kind === "collection",
        scheduleStart: scheduleStart || undefined,
        scheduleEnd: scheduleEnd || undefined,
      });

      setInfo(null);
      setUrl("");
      setSelected(new Set());
      setScheduleStart("");
      setScheduleEnd("");
      goTab("queue");
    } catch (e) {
      setError(describeError(e));
    }
  }

  const isCollection = info?.kind === "collection";
  const selectedCount = isCollection ? selected.size : info ? 1 : 0;

  return (
    <div className="view">
      <div className="url-row">
        <input
          className="url-input"
          placeholder={t("urlPlaceholder")}
          value={url}
          onChange={(event) => setUrl(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") void runFetch(url);
          }}
          dir="ltr"
          spellCheck={false}
        />
        {fetching ? (
          <button className="btn secondary" onClick={() => void cancelFetch()}>
            {t("fetching")}
          </button>
        ) : (
          <button
            className="btn primary"
            onClick={() => void runFetch(url)}
            disabled={!url.trim()}
          >
            {t("fetchBtn")}
          </button>
        )}
      </div>

      <div className="kind-row">
        {(["auto", "video", "playlist", "channel"] as const).map((kind) => (
          <button
            key={kind}
            className={`chip ${kindHint === kind ? "active" : ""}`}
            onClick={() => setKindHint(kind)}
          >
            {kind === "auto"
              ? t("kindAuto")
              : kind === "video"
                ? t("typeVideo")
                : kind === "playlist"
                  ? t("typePlaylist")
                  : t("typeChannel")}
          </button>
        ))}
        {progressMessage && <span className="muted small">{progressMessage}</span>}
      </div>

      {error && (
        <div className="error-box">
          {t("fetchFailed")} <code>{error}</code>
        </div>
      )}
      {notice && <div className="notice-box">{notice}</div>}

      {info?.kind === "video" && (
        <div className="video-card">
          {info.thumbnail && <img className="thumb" src={info.thumbnail} alt="" />}
          <div className="video-meta">
            <div className="video-title">{info.title}</div>
            <div className="muted small">
              {info.uploader && <span>{info.uploader} · </span>}
              {formatDuration(info.duration)}
            </div>
          </div>
        </div>
      )}

      {info?.kind === "collection" && (
        <div className="collection">
          <div className="collection-head">
            {info.thumbnail && <img className="thumb small" src={info.thumbnail} alt="" />}
            <div>
              <div className="video-title">{info.title}</div>
              <div className="muted small">
                <span className="badge-chip">
                  {info.collType === "channel" ? t("typeChannel") : t("typePlaylist")}
                </span>{" "}
                {info.uploader && `${t("by")} ${info.uploader} · `}
                {t("videosCount", { n: info.entries.length })}
              </div>
            </div>
          </div>

          {info.truncated && <div className="notice-box">{t("truncated")}</div>}

          <div className="list-tools">
            <button
              className="ghost-btn"
              onClick={() => setSelected(new Set(info.entries.map((e) => e.index)))}
            >
              {t("selectAll")}
            </button>
            <button className="ghost-btn" onClick={() => setSelected(new Set())}>
              {t("selectNone")}
            </button>
            <span className="muted">{t("selected", { n: selected.size })}</span>
          </div>

          <div className="video-list">
            {info.entries.map((entry) => (
              <label key={entry.id} className="video-row">
                <input
                  type="checkbox"
                  checked={selected.has(entry.index)}
                  onChange={() => toggle(entry.index)}
                />
                <span className="row-index">{String(entry.index).padStart(3, "0")}</span>
                {entry.thumbnail && (
                  <img className="row-thumb" src={entry.thumbnail} alt="" loading="lazy" />
                )}
                <span className="row-title">{entry.title}</span>
                <span className="row-duration">{formatDuration(entry.duration)}</span>
              </label>
            ))}
          </div>
        </div>
      )}

      {info && (
        <div className="options-panel">
          <div className="option">
            <span className="option-label">{t("format")}</span>
            <div className="seg">
              <button
                className={`seg-btn ${audioOnly ? "" : "active"}`}
                onClick={() => setAudioOnly(false)}
              >
                {t("videoFormat")}
              </button>
              <button
                className={`seg-btn ${audioOnly ? "active" : ""}`}
                onClick={() => setAudioOnly(true)}
              >
                {t("audioFormat")}
              </button>
            </div>
          </div>

          {!audioOnly && (
            <div className="option">
              <span className="option-label">{t("quality")}</span>
              <select value={quality} onChange={(event) => setQuality(event.target.value)}>
                <option value="best">{t("best")}</option>
                {heights.map((height) => (
                  <option key={height} value={String(height)}>
                    {heightLabel(height)}
                  </option>
                ))}
              </select>
            </div>
          )}

          <div className="option">
            <span className="option-label">{t("destination")}</span>
            <div className="dir-row">
              <input className="dir-input" value={dir} readOnly dir="ltr" placeholder="…" />
              <button className="btn secondary" onClick={() => void chooseDir()}>
                {t("chooseFolder")}
              </button>
            </div>
          </div>

          {isCollection && <div className="muted small">{t("numberingNote")}</div>}

          <div className="option">
            <span className="option-label">{t("perItemSchedule")}</span>
            <div className="seg" style={{ gap: 8 }}>
              <input
                type="time"
                value={scheduleStart}
                onChange={(event) => setScheduleStart(event.target.value)}
                style={{ width: 120 }}
              />
              <span style={{ alignSelf: "center" }}>{t("scheduleStart")}</span>
              <input
                type="time"
                value={scheduleEnd}
                onChange={(event) => setScheduleEnd(event.target.value)}
                style={{ width: 120 }}
              />
              <span style={{ alignSelf: "center" }}>{t("scheduleEnd")}</span>
              {(scheduleStart || scheduleEnd) && (
                <button
                  className="ghost-btn small"
                  onClick={() => {
                    setScheduleStart("");
                    setScheduleEnd("");
                  }}
                >
                  {t("clearSchedule")}
                </button>
              )}
            </div>
            <div className="muted small" style={{ marginTop: 4 }}>
              {scheduleStart && scheduleEnd
                ? `${t("scheduleStart")}: ${scheduleStart} — ${t("scheduleEnd")}: ${scheduleEnd}`
                : t("schedulerNote")}
            </div>
          </div>

          <button className="btn primary big" onClick={() => void addToQueue()}>
            {t("addToQueue", { n: selectedCount })}
          </button>
        </div>
      )}
    </div>
  );
}
