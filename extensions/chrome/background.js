// Vidaro YouTube Downloader - Background Script for Chrome Extension
// Handles native messaging communication with the native app

let port = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 5;
const RECONNECT_DELAY = 2000;

// Connect to native messaging host
function connectToNativeApp() {
  try {
    port = chrome.runtime.connectNative('com.vidaro.downloader');
    
    port.onMessage.addListener((message) => {
      handleNativeMessage(message);
    });
    
    port.onDisconnect.addListener(() => {
      console.log('[Vidaro] Disconnected from native app');
      if (chrome.runtime.lastError) {
        console.error('[Vidaro] Native messaging error:', chrome.runtime.lastError.message);
      }
      
      // Attempt to reconnect
      if (reconnectAttempts < 5) {
        reconnectAttempts++;
        console.log(`[Vidaro] Attempting to reconnect (${reconnectAttempts}/5)...`);
        setTimeout(connectToNativeApp, 2000 * reconnectAttempts);
      }
    });
    
    console.log('[Vidaro] Connected to native messaging host');
    reconnectAttempts = 0;
  } catch (error) {
    console.error('[Vidaro] Failed to connect to native app:', error);
  }
}

// Handle messages from native app
function handleNativeMessage(message) {
  console.log('[Vidaro] Received from native app:', message);
  
  switch (message.type) {
    case 'queueUpdate':
      // Forward queue updates to content scripts
      broadcastToContentScripts({ type: 'queueUpdate', items: message.items });
      break;
    case 'settingsUpdate':
      broadcastToContentScripts({ type: 'settingsUpdate', settings: message.settings });
      break;
    case 'historyUpdate':
      broadcastToContentScripts({ type: 'historyUpdate', items: message.items });
      break;
    case 'itemStatus':
      broadcastToContentScripts({ type: 'itemStatus', item: message.item });
      break;
    case 'downloadComplete':
      // Show notification when download completes
      showNotification('Download Complete', 'Your download has finished');
      break;
    case 'downloadError':
      showNotification('Download Error', message.error);
      break;
    default:
      console.log('[Vidaro] Unknown message type:', message.type);
  }
}

// Broadcast message to all YouTube content scripts
function broadcastToContentScripts(message) {
  chrome.tabs.query({ url: '*://*.youtube.com/*' }, (tabs) => {
    tabs.forEach(tab => {
      chrome.tabs.sendMessage(tab.id, message).catch(() => {
        // Tab might not have content script loaded yet
      });
    });
  });
}

// Send message to native app
function sendToNativeApp(message) {
  if (port) {
    port.postMessage(message);
  } else {
    console.warn('[Vidaro] No connection to native app');
  }
}

// Initialize connection on startup
chrome.runtime.onStartup.addListener(() => {
  connectToNativeApp();
});

chrome.runtime.onInstalled.addListener(() => {
  connectToNativeApp();
  
  // Create context menu items
  chrome.contextMenus.create({
    id: 'vidaro-download-video',
    title: 'Download with Vidaro',
    contexts: ['link', 'video', 'page'],
    documentUrlPatterns: ['*://*.youtube.com/*']
  });
});

// Handle context menu clicks
chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === 'vidaro-download-video') {
    const url = info.linkUrl || info.pageUrl || info.frameUrl;
    if (url && isYouTubeUrl(url)) {
      sendToNativeApp({
        type: 'download',
        url: url,
        quality: 'best',
        audioOnly: false
      });
    }
  }
});

// Listen for messages from content scripts
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  switch (message.type) {
    case 'getSettings':
      sendToNativeApp({ type: 'getSettings' });
      break;
    case 'updateSettings':
      sendToNativeApp({ type: 'updateSettings', settings: message.settings });
      break;
    case 'download':
      sendToNativeApp({
        type: 'download',
        url: message.url,
        quality: message.quality || 'best',
        audioOnly: message.audioOnly || false,
        format: message.format || 'mp4'
      });
      break;
    case 'getQueue':
      sendToNativeApp({ type: 'getQueue' });
      break;
    case 'getHistory':
      sendToNativeApp({ type: 'getHistory' });
      break;
    case 'controlDownload':
      sendToNativeApp({
        type: 'controlDownload',
        id: message.id,
        action: message.action
      });
      break;
    case 'getItemStatus':
      sendToNativeApp({ type: 'getItemStatus', id: message.id });
      break;
    case 'getSettings':
      // Already handled
      break;
    default:
      console.log('[Vidaro] Unknown message type:', message.type);
  }
  
  return true; // Keep channel open for async response
});

// Helper: Check if URL is a YouTube URL
function isYouTubeUrl(url) {
  try {
    const urlObj = new URL(url);
    return urlObj.hostname.includes('youtube.com') || urlObj.hostname.includes('youtu.be');
  } catch {
    return false;
  }
}

// Show system notification
function showNotification(title, body) {
  if (Notification.permission === 'granted') {
    new Notification(title, { body });
  } else if (Notification.permission !== 'denied') {
    Notification.requestPermission().then(permission => {
      if (permission === 'granted') {
        new Notification(title, { body });
      }
    });
  }
}

// Initialize connection when extension loads
connectToNativeApp();

// Listen for tab updates to inject content script if needed
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status === 'complete' && tab.url && tab.url.includes('youtube.com')) {
    // Content script will auto-inject via manifest
  }
});

console.log('[Vidaro] Background script loaded');