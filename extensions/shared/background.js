/**
 * Vidaro — background service worker (Chrome MV3) / event page (Firefox MV3).
 *
 * The native host is a single connection owned by this script. Popup and
 * content scripts never open their own port: they ask here through
 * `runtime.sendMessage`, and every call is matched to its reply by requestId.
 */

const HOST = "com.vidaro.downloader";
const CALL_TIMEOUT_MS = 20_000;
const NOTIFICATION_ID = "vidaro-status";

let port = null;
let requestCounter = 0;
const pending = new Map();

/* ------------------------------------------------------------ native port -- */

function ensurePort() {
  if (port) return port;

  port = chrome.runtime.connectNative(HOST);

  port.onMessage.addListener((message) => {
    const id = message && message.requestId;
    if (id != null && pending.has(id)) {
      const entry = pending.get(id);
      pending.delete(id);
      entry.settle(message);
      return;
    }
    // Unsolicited traffic pushed by the app.
    broadcast(message);
  });

  port.onDisconnect.addListener(() => {
    const reason = chrome.runtime.lastError?.message || "Vidaro closed the connection";
    port = null;
    for (const entry of pending.values()) entry.fail(new Error(reason));
    pending.clear();
  });

  return port;
}

function callNative(message, timeoutMs = CALL_TIMEOUT_MS) {
  return new Promise((resolve, reject) => {
    let connection;
    try {
      connection = ensurePort();
    } catch {
      reject(new Error("Vidaro is not installed or not running"));
      return;
    }

    const requestId = ++requestCounter;
    const timer = setTimeout(() => {
      pending.delete(requestId);
      reject(new Error("Vidaro did not answer in time"));
    }, timeoutMs);

    pending.set(requestId, {
      settle(value) {
        clearTimeout(timer);
        resolve(value);
      },
      fail(error) {
        clearTimeout(timer);
        reject(error);
      },
    });

    try {
      connection.postMessage({ ...message, requestId });
    } catch (error) {
      pending.delete(requestId);
      clearTimeout(timer);
      reject(error);
    }
  });
}

/* --------------------------------------------------------------- helpers -- */

function broadcast(message) {
  chrome.tabs.query({ url: "*://*.youtube.com/*" }, (tabs) => {
    for (const tab of tabs || []) {
      if (tab.id != null) {
        // Callback form keeps this working in both Chromium and Firefox.
        chrome.tabs.sendMessage(tab.id, message, () => void chrome.runtime.lastError);
      }
    }
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

function notify(title, message) {
  chrome.notifications.create(NOTIFICATION_ID, {
    type: "basic",
    iconUrl: "icons/icon128.png",
    title,
    message,
  });
}

function updateBadge(items) {
  const active = (items || []).filter(
    (item) => item.status === "active" || item.status === "queued",
  ).length;
  chrome.action.setBadgeText({ text: active ? String(active) : "" });
  chrome.action.setBadgeBackgroundColor({ color: "#00a8ff" });
}

function startDownload(url, quality, audioOnly) {
  return callNative({
    type: "download",
    url,
    quality: quality || "best",
    audioOnly: Boolean(audioOnly),
  }).then((reply) => {
    if (reply && reply.type === "error") {
      throw new Error(reply.message || "the download could not be started");
    }
    return reply;
  });
}

/* ------------------------------------------------------------ context menu -- */

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create(
    {
      id: "vidaro-download",
      title: "Download with Vidaro",
      contexts: ["link", "video", "page"],
      targetUrlPatterns: ["*://*.youtube.com/*", "*://youtu.be/*"],
    },
    () => void chrome.runtime.lastError,
  );
});

chrome.contextMenus.onClicked.addListener((info) => {
  const url = info.linkUrl || info.pageUrl;
  if (!isYouTubeUrl(url)) return;

  startDownload(url, "best", false)
    .then(() => notify("Vidaro", "Download added to the queue"))
    .catch((error) => notify("Vidaro", error.message));
});

/* ------------------------------------------------- messages from the UI -- */

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (!message || message.type !== "vidaro:call") return false;

  callNative(message.payload)
    .then((reply) => {
      if (reply && Array.isArray(reply.items)) updateBadge(reply.items);
      sendResponse(reply);
    })
    .catch((error) => sendResponse({ type: "error", message: error.message }));

  // Keep the channel open for the async reply.
  return true;
});

chrome.runtime.onStartup.addListener(() => {
  // Warm the connection so the first click is instant.
  callNative({ type: "ping" }, 4000).catch(() => {});
});
