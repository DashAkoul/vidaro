/**
 * Vidaro — toolbar popup.
 *
 * The popup only talks to the background worker; it never opens a native port
 * of its own (a popup closes on every click, which would tear the port down).
 */

const PREF_KEY = "vidaro:prefs";

let currentUrl = "";
let busy = false;

document.addEventListener("DOMContentLoaded", init);

async function init() {
  const tab = await activeTab();
  currentUrl = (tab && tab.url) || "";

  renderUrl();
  renderPageType();

  const downloadBtn = document.getElementById("download-btn");
  const supported = isYouTubeUrl(currentUrl);
  downloadBtn.disabled = !supported;
  downloadBtn.title = supported ? "" : "Open a YouTube video, playlist or channel";

  restorePrefs();
  bindEvents();

  if (supported) {
    const reply = await call({ type: "ping" }, 4000);
    if (!reply || reply.type === "error") {
      showStatus("Vidaro app is not running — open it to download", "error");
    }
  }
}

/* ---------------------------------------------------------------- helpers -- */

function activeTab() {
  return new Promise((resolve) => {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      resolve((tabs && tabs[0]) || null);
    });
  });
}

function call(payload, timeoutMs = 15000) {
  return new Promise((resolve) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (!settled) {
        settled = true;
        resolve({ type: "error", message: "Vidaro is not responding" });
      }
    }, timeoutMs);

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
  });
}

function isYouTubeUrl(url) {
  try {
    const { hostname } = new URL(url);
    return hostname.endsWith("youtube.com") || hostname === "youtu.be";
  } catch {
    return false;
  }
}

/* ------------------------------------------------------------------ render -- */

function renderUrl() {
  const element = document.getElementById("current-url");
  if (!element) return;

  if (!currentUrl) {
    element.textContent = "No page detected";
    return;
  }

  try {
    const url = new URL(currentUrl);
    element.textContent = url.hostname + url.pathname + url.search;
    element.title = currentUrl;
  } catch {
    element.textContent = currentUrl;
  }
}

function renderPageType() {
  const element = document.getElementById("page-type");
  if (!element) return;

  let type = "Unknown";
  if (currentUrl.includes("/watch")) type = "Video";
  else if (currentUrl.includes("/playlist")) type = "Playlist";
  else if (/\/(channel|c|user)\/|youtube\.com\/@/.test(currentUrl)) type = "Channel";

  element.textContent = type;
  element.className = `page-type-badge type-${type.toLowerCase()}`;
}

function showStatus(message, kind) {
  document.querySelectorAll(".status-message").forEach((node) => node.remove());

  const status = document.createElement("div");
  status.className = `status-message status-${kind}`;
  status.textContent = message;
  status.style.cssText = `
    position: fixed;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    padding: 9px 16px;
    border-radius: 8px;
    font-size: 12px;
    color: #fff;
    z-index: 10000;
    background: ${kind === "success" ? "#22c55e" : "#ef4444"};
  `;
  document.body.appendChild(status);
  setTimeout(() => status.remove(), 3500);
}

/* ------------------------------------------------------------------- prefs -- */

function restorePrefs() {
  let prefs = {};
  try {
    prefs = JSON.parse(localStorage.getItem(PREF_KEY) || "{}");
  } catch {
    prefs = {};
  }

  const format = document.getElementById("format-select");
  const quality = document.getElementById("quality-select");
  if (format && prefs.format) format.value = prefs.format;
  if (quality && prefs.quality) quality.value = prefs.quality;

  if (format) {
    const group = document.getElementById("quality-group");
    if (group) group.style.display = format.value === "mp3" ? "none" : "block";
  }
}

function savePrefs() {
  const format = document.getElementById("format-select");
  const quality = document.getElementById("quality-select");
  try {
    localStorage.setItem(
      PREF_KEY,
      JSON.stringify({ format: format?.value, quality: quality?.value }),
    );
  } catch {
    /* storage may be unavailable; the defaults still work */
  }
}

/* ------------------------------------------------------------------ events -- */

function bindEvents() {
  const downloadBtn = document.getElementById("download-btn");
  const format = document.getElementById("format-select");
  const openApp = document.getElementById("open-app-btn");
  const settingsLink = document.getElementById("settings-link");

  if (format) {
    format.addEventListener("change", () => {
      const group = document.getElementById("quality-group");
      if (group) group.style.display = format.value === "mp3" ? "none" : "block";
      savePrefs();
    });
  }

  const quality = document.getElementById("quality-select");
  if (quality) quality.addEventListener("change", savePrefs);

  if (downloadBtn) downloadBtn.addEventListener("click", startDownload);

  if (openApp) {
    openApp.addEventListener("click", async () => {
      await call({ type: "show" }, 5000);
      window.close();
    });
  }

  if (settingsLink) {
    settingsLink.addEventListener("click", async (event) => {
      event.preventDefault();
      await call({ type: "show" }, 5000);
      window.close();
    });
  }
}

async function startDownload() {
  if (busy) return;

  const button = document.getElementById("download-btn");
  const format = document.getElementById("format-select");
  const quality = document.getElementById("quality-select");
  const text = button?.querySelector(".btn-text");
  const loader = button?.querySelector(".btn-loader");

  busy = true;
  if (button) button.disabled = true;
  if (text) text.textContent = "Starting…";
  if (loader) loader.classList.remove("hidden");

  const reply = await call({
    type: "download",
    url: currentUrl,
    quality: quality?.value || "best",
    audioOnly: format?.value === "mp3",
  });

  busy = false;
  if (button) button.disabled = false;
  if (text) text.textContent = "Download";
  if (loader) loader.classList.add("hidden");

  if (reply && reply.type === "error") {
    showStatus(reply.message || "Could not start the download", "error");
    return;
  }

  showStatus("Added to the Vidaro queue", "success");
  setTimeout(() => window.close(), 1200);
}
