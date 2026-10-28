// Vidaro YouTube Downloader - Popup Script

let currentTab = null;
let currentUrl = '';
let isDownloading = false;

document.addEventListener('DOMContentLoaded', async () => {
  await initPopup();
});

async function initPopup() {
  // Get current tab
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  currentTab = tabs[0];
  
  if (currentTab && currentTab.url) {
    currentUrl = currentTab.url;
    updateUrlDisplay(currentUrl);
    detectPageType(currentUrl);
  }
  
  // Set up event listeners
  setupEventListeners();
  
  // Check notification permission
  if (Notification.permission !== 'granted' && Notification.permission !== 'denied') {
    Notification.requestPermission();
  }
  
  // Load settings
  await loadSettings();
  
  // Connect to native app
  connectToNativeApp();
}

function setupEventListeners() {
  // Download button
  document.getElementById('download-btn').addEventListener('click', handleDownload);
  
  // Open app button
  document.getElementById('open-app-btn').addEventListener('click', () => {
    sendToNativeApp({ type: 'show' });
    window.close();
  });
  
  // Settings link
  document.getElementById('settings-link').addEventListener('click', (e) => {
    e.preventDefault();
    sendToNativeApp({ type: 'openSettings' });
    window.close();
  });
  
  // Format change
  document.getElementById('format-select').addEventListener('change', (e) => {
    const qualityGroup = document.getElementById('quality-group');
    qualityGroup.style.display = e.target.value === 'mp3' ? 'none' : 'block';
    saveSettings();
  });
  
  // Quality change
  document.getElementById('quality-select').addEventListener('change', saveSettings);
}

function updateUrlDisplay(url) {
  const urlEl = document.getElementById('current-url');
  try {
    const urlObj = new URL(currentUrl);
    const hostname = urlObj.hostname;
    const path = urlObj.pathname + urlObj.search;
    urlEl.innerHTML = `<strong>${hostname}</strong>${path}`;
  } catch {
    urlEl.textContent = currentUrl;
  }
}

function detectPageType(url) {
  const typeEl = document.getElementById('page-type');
  let type = 'Unknown';
  let videoId = null;
  
  try {
    const urlObj = new URL(currentUrl);
    
    if (url.includes('youtube.com/watch')) {
      const videoId = new URLSearchParams(new URL(currentUrl).search).get('v');
      if (videoId) {
        type = 'Video';
      }
    } else if (currentUrl.includes('youtube.com/playlist')) {
      type = 'Playlist';
    } else if (currentUrl.includes('youtube.com/channel') || 
               currentUrl.includes('youtube.com/@') ||
               currentUrl.includes('youtube.com/c/') ||
               currentUrl.includes('youtube.com/user/')) {
      type = 'Channel';
    }
  } catch (e) {}
  
  const typeEl = document.getElementById('page-type');
  typeEl.textContent = type;
  typeEl.className = `page-type-badge type-${type.toLowerCase()}`;
}

async function loadSettings() {
  try {
    const response = await sendToNativeApp({ type: 'getSettings' });
    if (response && response.settings) {
      applySettings(response.settings);
    }
  } catch (e) {
    console.warn('Could not load settings:', e);
  }
}

function applySettings(settings) {
  if (settings.proxyMode) {
    const proxyModeSelect = document.getElementById('proxy-mode');
    if (proxyModeSelect) proxyModeSelect.value = settings.proxyMode || 'off';
  }
  
  if (settings.proxyHost) {
    const proxyHostInput = document.getElementById('proxy-host');
    if (proxyHostInput) proxyHostInput.value = settings.proxyHost;
  }
  
  if (settings.proxyPort) {
    const proxyPortInput = document.getElementById('proxy-port');
    if (proxyPortInput) proxyPortInput.value = settings.proxyPort;
  }
}

function saveSettings() {
  const settings = {
    format: document.getElementById('format-select')?.value || 'mp4',
    quality: document.getElementById('quality-select')?.value || 'best'
  };
  
  sendToNativeApp({ type: 'updateSettings', settings });
}

async function handleDownload() {
  if (isDownloading) return;
  
  isDownloading = true;
  const btn = document.getElementById('download-btn');
  btn.disabled = true;
  btn.querySelector('.btn-text').textContent = 'Starting...';
  btn.querySelector('.btn-loader').classList.remove('hidden');
  
  const format = document.getElementById('format-select').value;
  const quality = document.getElementById('quality-select').value;
  
  try {
    const response = await sendToNativeApp({
      type: 'download',
      url: currentUrl,
      quality: quality,
      audioOnly: format === 'mp3',
      format: format
    });
    
    if (response && response.success) {
      showStatus('Download started!', 'success');
    } else {
      showStatus('Failed to start download: ' + (response?.error || 'Unknown error'), 'error');
    }
  } catch (error) {
    showStatus('Error: ' + error.message, 'error');
  } finally {
    isDownloading = false;
    btn.disabled = false;
    btn.querySelector('.btn-text').textContent = 'Download';
    btn.querySelector('.btn-loader').classList.add('hidden');
  }
}

function showStatus(message, type) {
  // Remove existing status
  const existing = document.querySelector('.status-message');
  if (existing) existing.remove();
  
  const status = document.createElement('div');
  status.className = `status-message status-${type}`;
  status.textContent = message;
  status.style.cssText = `
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    padding: 12px 24px;
    border-radius: 8px;
    color: white;
    font-weight: 500;
    z-index: 10000;
    animation: slideUp 0.3s ease;
    ${type === 'success' ? 'background: #4CAF50;' : 'background: #f44336;'}
  `;
  
  document.body.appendChild(status);
  
  setTimeout(() => {
    status.style.animation = 'slideDown 0.3s ease';
    setTimeout(() => status.remove(), 300);
  }, 3000);
}

function connectToNativeApp() {
  try {
    port = chrome.runtime.connectNative('com.vidaro.downloader');
    
    port.onMessage.addListener((message) => {
      console.log('[Vidaro Popup] Received:', message);
    });
    
    port.onDisconnect.addListener(() => {
      console.log('[Vidaro] Disconnected from native app');
      if (chrome.runtime.lastError) {
        console.error('Native messaging error:', chrome.runtime.lastError.message);
      }
    });
    
    console.log('[Vidaro] Connected to native app');
  } catch (error) {
    console.error('[Vidaro] Failed to connect:', error);
  }
}

function sendToNativeApp(message) {
  return new Promise((resolve, reject) => {
    if (!port) {
      reject(new Error('Not connected to native app'));
      return;
    }
    
    const messageId = Date.now() + Math.random();
    const messageWithId = { ...message, id: messageId };
    
    const timeout = setTimeout(() => {
      reject(new Error('Timeout waiting for response'));
    }, 10000);
    
    const listener = (message) => {
      if (message.id === messageId) {
        clearTimeout(timeout);
        port.onMessage.removeListener(listener);
        resolve(message);
      }
    };
    
    port.onMessage.addListener(listener);
    port.postMessage({ ...message, id: messageId });
  };
  
  // For fire-and-forget messages
  if (port) {
    port.postMessage(message);
  }
}

function broadcastToContentScripts(message) {
  chrome.tabs.query({ url: '*://*.youtube.com/*' }, (tabs) => {
    tabs.forEach(tab => {
      chrome.tabs.sendMessage(tab.id, message).catch(() => {});
    });
  });
}

// Add CSS animation for status messages
const style = document.createElement('style');
style.textContent = `
  @keyframes slideUp {
    from { opacity: 0; transform: translateX(-50%) translateY(20px); }
    to { opacity: 1; transform: translateX(-50%) translateY(0); }
  }
  @keyframes slideDown {
    from { opacity: 1; transform: translateX(-50%) translateY(0); }
    to { opacity: 0; transform: translateX(-50%) translateY(20px); }
  }
`;
document.head.appendChild(style);