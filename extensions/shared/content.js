/**
 * Vidaro — content script for YouTube.
 *
 * Adds a "Download with Vidaro" button next to the video title (and on
 * playlist/channel rows). Every button reflects the real queue state: after a
 * click it polls the app until the item finishes, fails, or is cancelled.
 */

(() => {
  "use strict";

  const BUTTON_CLASS = "vidaro-download-btn";
  const STYLE_ID = "vidaro-injected-style";
  const POLL_MS = 1500;

  /** videoId -> { status, percent } so re-injection keeps the current state. */
  const state = new Map();
  const watched = new Map(); // videoId -> interval id

  /* ------------------------------------------------------------- messaging -- */

  function call(payload, timeoutMs = 20000) {
    return new Promise((resolve) => {
      let settled = false;
      const timer = setTimeout(() => {
        if (!settled) {
          settled = true;
          resolve({ type: "error", message: "Vidaro is not responding" });
        }
      }, timeoutMs);

      try {
        chrome.runtime.sendMessage({ type: "vidaro:call", payload }, (reply) => {
          if (settled) return;
          settled = true;
          clearTimeout(timer);
          if (chrome.runtime.lastError) {
            resolve({ type: "error", message: chrome.runtime.lastError.message });
            return;
          }
          resolve(reply || { type: "error", message: "no reply" });
        });
      } catch (error) {
        settled = true;
        clearTimeout(timer);
        resolve({ type: "error", message: String(error) });
      }
    });
  }

  /* ------------------------------------------------------------ video ids -- */

  function currentVideoId() {
    try {
      const url = new URL(window.location.href);
      if (url.pathname === "/watch") return url.searchParams.get("v");
      if (url.pathname.startsWith("/shorts/")) return url.pathname.split("/")[2] || null;
    } catch {
      /* ignore */
    }
    return null;
  }

  function videoIdFromHref(href) {
    if (!href) return null;
    try {
      const url = new URL(href, window.location.origin);
      return url.searchParams.get("v");
    } catch {
      return null;
    }
  }

  /* ----------------------------------------------------------------- style -- */

  function injectStyle() {
    if (document.getElementById(STYLE_ID)) return;
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = `
      .${BUTTON_CLASS} {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        margin-inline-start: 8px;
        padding: 7px 14px;
        border: 1px solid rgba(0, 168, 255, 0.45);
        border-radius: 18px;
        background: rgba(0, 168, 255, 0.12);
        color: #0af;
        font: 500 13px/1.2 "Roboto", Arial, sans-serif;
        cursor: pointer;
        transition: background 0.15s ease, color 0.15s ease;
      }
      .${BUTTON_CLASS}:hover:not(:disabled) {
        background: rgba(0, 168, 255, 0.22);
      }
      .${BUTTON_CLASS}:disabled {
        cursor: default;
        opacity: 0.85;
      }
      .${BUTTON_CLASS}[data-state="done"] {
        border-color: rgba(34, 197, 94, 0.5);
        background: rgba(34, 197, 94, 0.14);
        color: #22c55e;
      }
      .${BUTTON_CLASS}[data-state="error"] {
        border-color: rgba(239, 68, 68, 0.5);
        background: rgba(239, 68, 68, 0.14);
        color: #ef4444;
      }
      .${BUTTON_CLASS}[data-state="busy"] {
        color: #8b93a7;
        border-color: rgba(139, 147, 167, 0.4);
        background: transparent;
      }
    `;
    document.documentElement.appendChild(style);
  }

  /* ---------------------------------------------------------------- button -- */

  function makeButton(videoId, compact) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = BUTTON_CLASS;
    button.dataset.videoId = videoId;
    if (compact) {
      button.style.padding = "3px 10px";
      button.style.fontSize = "11px";
      button.style.marginInlineStart = "6px";
    }

    const label = () => {
      const current = state.get(videoId);
      if (!current) {
        button.textContent = compact ? "⬇ Vidaro" : "⬇ Download with Vidaro";
        button.disabled = false;
        button.dataset.state = "idle";
        return;
      }
      if (current.status === "completed") {
        button.textContent = "✓ Downloaded";
        button.disabled = true;
        button.dataset.state = "done";
      } else if (current.status === "error") {
        button.textContent = "↻ Retry";
        button.disabled = false;
        button.dataset.state = "error";
      } else if (current.status === "cancelled") {
        button.textContent = "⬇ Download again";
        button.disabled = false;
        button.dataset.state = "idle";
      } else if (current.status === "paused") {
        button.textContent = `❚❚ Paused ${Math.floor(current.percent || 0)}%`;
        button.disabled = false;
        button.dataset.state = "busy";
      } else {
        button.textContent = `↓ ${Math.floor(current.percent || 0)}%`;
        button.disabled = true;
        button.dataset.state = "busy";
      }
    };

    button.addEventListener("click", async (event) => {
      event.preventDefault();
      event.stopPropagation();

      const current = state.get(videoId);
      if (current && (current.status === "paused" || current.status === "cancelled")) {
        await call({ type: "controlDownload", id: current.id, action: "resume" });
        return;
      }

      button.disabled = true;
      button.dataset.state = "busy";
      button.textContent = "Starting…";

      const reply = await call({
        type: "download",
        url: `https://www.youtube.com/watch?v=${videoId}`,
        quality: "best",
        audioOnly: false,
      });

      if (reply && reply.type === "error") {
        button.disabled = false;
        button.dataset.state = "error";
        button.textContent = "Vidaro unavailable";
        button.title = reply.message || "";
        setTimeout(label, 4000);
        return;
      }

      const id = reply && reply.data && reply.data.id;
      if (id) {
        state.set(videoId, { id, status: "queued", percent: 0 });
        watch(videoId, id);
      }
      label();
    });

    label();
    return button;
  }

  /** Poll the queue until this item reaches a final state. */
  function watch(videoId, itemId) {
    if (watched.has(videoId)) return;
    const timer = setInterval(async () => {
      const reply = await call({ type: "getItemStatus", id: itemId }, 8000);
      const item = reply && reply.item;

      if (!item) {
        clearInterval(timer);
        watched.delete(videoId);
        state.delete(videoId);
        refresh(videoId);
        return;
      }

      state.set(videoId, { id: itemId, status: item.status, percent: item.percent });
      refresh(videoId);

      if (["completed", "error", "cancelled"].includes(item.status)) {
        clearInterval(timer);
        watched.delete(videoId);
      }
    }, POLL_MS);
    watched.set(videoId, timer);
  }

  function refresh(videoId) {
    document.querySelectorAll(`.${BUTTON_CLASS}`).forEach((button) => {
      if (button.dataset.videoId !== videoId) return;
      // Rebuild the label from state without re-binding events.
      const current = state.get(videoId);
      if (!current) return;
      if (current.status === "completed") {
        button.textContent = "✓ Downloaded";
        button.disabled = true;
        button.dataset.state = "done";
      } else if (current.status === "error") {
        button.textContent = "↻ Retry";
        button.disabled = false;
        button.dataset.state = "error";
      } else if (current.status === "active" || current.status === "queued") {
        button.textContent = `↓ ${Math.floor(current.percent || 0)}%`;
        button.disabled = true;
        button.dataset.state = "busy";
      }
    });
  }

  /* ------------------------------------------------------------- injection -- */

  function findTitleBar() {
    const selectors = [
      "ytd-watch-metadata #title",
      "#title h1",
      "h1.ytd-watch-metadata",
      "#above-the-fold #title",
    ];
    for (const selector of selectors) {
      const element = document.querySelector(selector);
      if (element) return element;
    }
    return null;
  }

  function injectWatchButton() {
    const videoId = currentVideoId();
    if (!videoId) return;
    if (document.querySelector(`.${BUTTON_CLASS}[data-video-id="${videoId}"]`)) return;

    const anchor = findTitleBar();
    if (!anchor) return;

    const button = makeButton(videoId, false);
    const holder = document.createElement("span");
    holder.style.display = "inline-flex";
    holder.style.verticalAlign = "middle";
    holder.appendChild(button);

    // Place it after the title text so it never covers YouTube's own controls.
    anchor.appendChild(holder);
  }

  function injectListButtons() {
    const rows = document.querySelectorAll(
      "ytd-playlist-video-renderer, ytd-rich-item-renderer, ytd-grid-video-renderer, ytd-video-renderer",
    );

    for (const row of rows) {
      if (row.querySelector(`.${BUTTON_CLASS}`)) continue;

      const link = row.querySelector('a#video-title, a#thumbnail, a#video-title-link');
      const videoId = videoIdFromHref(link && link.href);
      if (!videoId) continue;

      const button = makeButton(videoId, true);
      const holder = document.createElement("span");
      holder.style.display = "inline-flex";
      holder.appendChild(button);

      const target = row.querySelector("#metadata-line, #metadata, .inline-metadata-item");
      if (target && target.parentNode) {
        target.parentNode.insertBefore(holder, target.nextSibling);
      } else if (link && link.parentNode) {
        link.parentNode.insertBefore(holder, link.nextSibling);
      }
    }
  }

  function injectAll() {
    injectStyle();
    try {
      injectWatchButton();
      injectListButtons();
    } catch (error) {
      console.warn("[Vidaro] injection failed", error);
    }
  }

  /* ------------------------------------------------------------------- spa -- */

  let pending = null;
  function scheduleInject() {
    if (pending) clearTimeout(pending);
    pending = setTimeout(injectAll, 400);
  }

  function start() {
    injectAll();

    // YouTube is a single-page app: navigation replaces the DOM without a load
    // event, so re-run on any mutation batch (debounced).
    const observer = new MutationObserver(scheduleInject);
    observer.observe(document.body, { childList: true, subtree: true });

    window.addEventListener("yt-navigate-finish", scheduleInject);
    window.addEventListener("beforeunload", () => {
      observer.disconnect();
      for (const timer of watched.values()) clearInterval(timer);
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start, { once: true });
  } else {
    start();
  }
})();
