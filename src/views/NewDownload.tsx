import { useContext, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  cancelFetch,
  enqueueBatch,
  fetchInfo,
  formatDuration,
  onInfoProgress,
} from "../api";
import { I18nContext } from "../i18n";
import type { InfoResult, VideoEntry } from "../types";
import { AppContext } from "../App";

const STANDARD_HEIGHTS = [2160, 1440, 1080, 720, 480, 360];

function heightLabel(h: number): string {
  if (h >= 2160) return `${h}p (4K)`;
  if (h >= 1440) return `${h}p (2K)`;
  return `${h}p`;
}

export default function NewDownload({ pendingUrl }: { pendingUrl: string | null }) {
  const { t } = useContext(I18nContext);
  const app = AppContext.current;

  const [url, setUrl] = useState("");
  const [kindHint, setKindHint] = useState<"auto" | "video" | "playlist" | "channel">("auto");
  const [fetching, setFetching] = useState(false);
  const [infoMsg, setInfoMsg] = useState("");
  const [info, setInfo] = useState<InfoResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [audioOnly, setAudioOnly] = useState(false);
  const [quality, setQuality] = useState("best");
  const [dir, setDir] = useState(app?.settings.defaultDir ?? "");
  const [notice, setNotice] = useState<string | null>(null);
  const [scheduleStart, setScheduleStart] = useState("");
  const [scheduleEnd, setScheduleEnd] = useState("");

  useEffect(() => {
    const un = onInfoProgress((p) => {
      if (p.message === "done") setInfoMsg("");
      else setInfoMsg(p.message);
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  // a URL handed over from the clipboard toast / deep open
  useEffect(() => {
    if (pendingUrl) {
      setUrl(pendingUrl);
      setError(null);
      setNotice(null);
      void doFetchWith(pendingUrl);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pendingUrl]);

  async function doFetchWith(u: string) {
    if (!u.trim() || fetching) return;
    setFetching(true);
    setError(null);
    setInfo(null);
    setNotice(null);
    setInfoMsg("");
    try {
      const res = await fetchInfo(u.trim(), kindHint);
      setInfo(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setFetching(false);
    }
  }

  // when info arrives, select all entries
  useEffect(() => {
    if (info && info.kind === "collection") {
      setSelected(new Set(info.entries.map((e) => e.index)));
    }
    if (info && info.kind === "video" && info.heights.length > 0) {
      // default quality: best available up to 1080p, else the top one
      const pick = info.heights.find((h) => h <= 1080) ?? info.heights[0];
      setQuality(String(pick));
    }
  }, [info]);

  const heights = useMemo(() => {
    if (info && info.kind === "video") return info.heights;
    return STANDARD_HEIGHTS;
  }, [info]);

  async function doFetch() {
    await doFetchWith(url);
  }

  async function chooseDir(): Promise<string> {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string" && picked) {
      setDir(picked);
      app?.updateSettings({ defaultDir: picked });
      return picked;
    }
    return dir;
  }

  async function addToQueue() {
    if (!info) return;
    if (!dir) {
      const picked = await chooseDir();
      if (!picked) {
        setNotice(t("chooseFolderFirst"));
        return;
      }
    }
    setNotice(null);

    let payloadItems: { videoId: string; title: string; url: string; index: number | null }[] = [];
    let batchName = "";
    let numbering = false;

    if (info.kind === "video") {
      payloadItems = [
        {
          videoId: info.id,
          title: info.title,
          url: url.trim(),
          index: null,
        },
      ];
    } else {
      const entries = info.entries.filter((e) => selected.has(e.index));
      if (entries.length === 0) {
        setNotice(t("noSelection"));
        return;
      }
      payloadItems = entries.map((e: VideoEntry) => ({
        videoId: e.id,
        title: e.title,
        url: e.url,
        index: e.index,
      }));
      batchName = info.title;
      numbering = true;
    }

    try {
      await enqueueBatch({
        items: payloadItems,
        dir,
        batchName,
        quality,
        audioOnly,
        numbering,
        scheduleStart: scheduleStart || undefined,
        scheduleEnd: scheduleEnd || undefined,
      });
      // reset view
      setInfo(null);
      setUrl("");
      setSelected(new Set());
      setScheduleStart("");
      setScheduleEnd("");
      app?.goTab("queue");
    } catch (e) {
      setError(String(e));
    }
  }

  const toggle = (idx: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) next.delete(idx);
      else next.add(idx);
      return next;
    });
  };

  const isCollection = info?.kind === "collection";
  const selectedCount = isCollection ? selected.size : info ? 1 : 0;

  return (
    <div className="view">
      <h1>{t("tabNew")}</h1>

      <div className="url-row">
        <input
          className="url-input"
          placeholder={t("urlPlaceholder")}
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && doFetch()}
          dir="ltr"
        />
        {fetching ? (
          <button className="btn secondary" onClick={() => cancelFetch()} disabled>
            {t("fetching")}
          </button>
        ) : (
          <button className="btn primary" onClick={doFetch} disabled={!url.trim()}>
            {t("fetchBtn")}
          </button>
        )}
      </div>

      <div className="kind-row">
        {(["auto", "video", "playlist", "channel"] as const).map((k) => (
          <button
            key={k}
            className={`chip ${kindHint === k ? "active" : ""}`}
            onClick={() => setKindHint(k)}
          >
            {k === "auto" ? "Auto" : t(k === "video" ? "typeVideo" : k === "playlist" ? "typePlaylist" : "typeChannel")}
          </button>
        ))}
        {infoMsg && <span className="muted small">{infoMsg}</span>}
      </div>

      {error && <div className="error-box">{t("fetchFailed")} <code>{error}</code></div>}
      {notice && <div className="notice-box">{notice}</div>}

      {info && info.kind === "video" && (
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

      {isCollection && (
        <div className="collection">
          <div className="collection-head">
            {info.kind === "collection" && info.thumbnail && (
              <img className="thumb small" src={info.thumbnail} alt="" />
            )}
            <div>
              <div className="video-title">
                {info.kind === "collection" && info.title}
              </div>
              <div className="muted small">
                <span className="badge-chip">
                  {info.kind === "collection" && info.collType === "channel"
                    ? t("typeChannel")
                    : t("typePlaylist")}
                </span>{" "}
                {info.kind === "collection" && info.uploader && `${t("by")} ${info.uploader} · `}
                {info.kind === "collection" && t("videosCount", { n: info.entries.length })}
              </div>
            </div>
          </div>

          {info.kind === "collection" && info.truncated && (
            <div className="notice-box">{t("truncated")}</div>
          )}

          <div className="list-tools">
            <button className="ghost-btn" onClick={() => setSelected(new Set(info.entries.map((e) => e.index)))}>
              {t("selectAll")}
            </button>
            <button className="ghost-btn" onClick={() => setSelected(new Set())}>
              {t("selectNone")}
            </button>
            <span className="muted">{t("selected", { n: selected.size })}</span>
          </div>

          <div className="video-list">
            {info.kind === "collection" &&
              info.entries.map((e) => (
                <label key={e.id} className="video-row">
                  <input
                    type="checkbox"
                    checked={selected.has(e.index)}
                    onChange={() => toggle(e.index)}
                  />
                  <span className="row-index">{String(e.index).padStart(3, "0")}</span>
                  {e.thumbnail && <img className="row-thumb" src={e.thumbnail} alt="" loading="lazy" />}
                  <span className="row-title">{e.title}</span>
                  <span className="row-duration">{formatDuration(e.duration)}</span>
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
              <button className={`seg-btn ${!audioOnly ? "active" : ""}`} onClick={() => setAudioOnly(false)}>
                {t("videoFormat")}
              </button>
              <button className={`seg-btn ${audioOnly ? "active" : ""}`} onClick={() => setAudioOnly(true)}>
                {t("audioFormat")}
              </button>
            </div>
          </div>

          {!audioOnly && (
            <div className="option">
              <span className="option-label">{t("quality")}</span>
              <select value={quality} onChange={(e) => setQuality(e.target.value)}>
                <option value="best">{t("best")}</option>
                {heights.map((h) => (
                  <option key={h} value={String(h)}>
                    {heightLabel(h)}
                  </option>
                ))}
              </select>
            </div>
          )}

          <div className="option">
            <span className="option-label">{t("destination")}</span>
            <div className="dir-row">
              <input className="dir-input" value={dir} readOnly dir="ltr" placeholder="…" />
              <button className="btn secondary" onClick={chooseDir}>
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
                onChange={(e) => setScheduleStart(e.target.value)}
                style={{ width: 120 }}
              />
              <span style={{ alignSelf: "center" }}>{t("scheduleStart")}</span>
              <input
                type="time"
                value={scheduleEnd}
                onChange={(e) => setScheduleEnd(e.target.value)}
                style={{ width: 120 }}
              />
              <span style={{ alignSelf: "center" }}>{t("scheduleEnd")}</span>
              {(scheduleStart || scheduleEnd) && (
                <button className="ghost-btn small" onClick={() => { setScheduleStart(""); setScheduleEnd(""); }}>
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

          <button className="btn primary big" onClick={addToQueue}>
            {t("addToQueue", { n: selectedCount })}
          </button>
        </div>
      )}
    </div>
  );
}
