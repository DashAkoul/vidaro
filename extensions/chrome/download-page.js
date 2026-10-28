// Vidaro YouTube Downloader - Download Page Script
// Handles the minimal download page that opens from browser extension

let currentDownloadId = null;
let progressPollInterval = null;
let currentVideoInfo = null;

document.addEventListener('DOMContentLoaded', async () => {
  await initDownloadPage();
});

async function initDownloadPage() {
  // Get URL from query parameters
  const urlParams = new URLSearchParams(window.location.search);
  const url = urlParams.get('url');
  const quality = urlParams.get('quality') || 'best';
  const format = urlParams.get('format') || 'mp4';
  const audioOnly = urlParams.get('audioOnly') === 'true';
  const embedThumbnail = urlParams.get('embedThumbnail') === 'true';
  const embedMetadata = urlParams.get('embedMetadata') === 'true';
  const writeSubs = urlParams.get('writeSubs') === 'true';
  const subLangs = urlParams.get('subLangs') || '';
  const embedThumbnailParam = urlParams.get('embedThumbnail') === 'true';
  const embedMetadata = urlParams.get('embedMetadata') === 'true';

  if (!url) {
    showError('No URL provided');
    return;
  }

  // Initialize UI
  initUI();
  
  // Load video info
  await loadVideoInfo(url);
  
  // Set up event listeners
  setupEventListeners(url, quality, format, audioOnly, embedThumbnail, embedMetadata, writeSubs, subLangs);
  
  // Connect to native app
  connectToNativeApp();
}

function initUI() {
  // Initialize format change handler
  const formatSelect = document.getElementById('format-select');
  if (formatSelect) {
    formatSelect.addEventListener('change', (e) => {
      const qualityField = document.getElementById('quality-field');
      qualityField.style.display = e.target.value === 'mp3' ? 'none' : 'block';
    });
  }
  
  // Format change
  const formatSelectEl = document.getElementById('format-select');
  if (formatSelectEl) {
    formatSelectEl.addEventListener('change', (e) => {
      const qualityField = document.getElementById('quality-field');
      qualityField.style.display = e.target.value === 'mp3' ? 'none' : 'block';
    });
  }
  
  // Subtitles checkbox
  const writeSubsCheckbox = document.getElementById('write-subs');
  const subLangsField = document.getElementById('sub-langs-field');
  if (writeSubsCheckbox && subLangsField) {
    writeSubsCheckbox.addEventListener('change', (e) => {
      subLangsField.style.display = e.target.checked ? 'block' : 'none';
    });
  }
  
  // Subtitles checkbox handler
  const writeSubsCheckboxEl = document.getElementById('write-subs');
  const subLangsFieldEl = document.getElementById('sub-langs-field');
  if (writeSubsCheckbox && subLangsField) {
    writeSubsCheckbox.addEventListener('change', (e) => {
      subLangsField.style.display = e.target.checked ? 'block' : 'none';
    });
  }
}

async function loadVideoInfo(url) {
  try {
    const response = await sendToNativeApp({
      type: 'getVideoInfo',
      url: url
    });
    
    if (response && response.success && response.videoInfo) {
      displayVideoInfo(response.videoInfo);
    } else {
      // Fallback: extract video ID from URL and show basic info
      const videoId = extractVideoId(window.location.search);
      if (videoId) {
        document.getElementById('video-title').textContent = 'Video ID: ' + videoId;
        document.getElementById('video-url').textContent = `https://www.youtube.com/watch?v=${videoId}`;
      }
    }
  } catch (error) {
    console.error('Failed to load video info:', error);
  }
}

function displayVideoInfo(videoInfo) {
  document.getElementById('video-title').textContent = videoInfo.title || 'Unknown Title';
  document.getElementById('video-url').textContent = videoInfo.url || '';
  document.getElementById('video-author').textContent = videoInfo.author || 'Unknown';
  document.getElementById('video-duration').textContent = formatDuration(videoInfo.duration);
  document.getElementById('video-views').textContent = formatViews(videoInfo.viewCount);
  
  if (videoInfo.thumbnail) {
    document.getElementById('video-thumbnail').src = videoInfo.thumbnail;
    document.getElementById('video-preview').style.display = 'flex';
  }
}

function formatDuration(seconds) {
  if (!seconds) return '--:--';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  }
  return `${minutes}:${String(secs).padStart(2, '0')}`;
}

function formatViews(count) {
  if (!count) return '--';
  if (count >= 1000000) return (count / 1000000).toFixed(1) + 'M';
  if (count >= 1000) return (count / 1000).toFixed(1) + 'K';
  return count.toString();
}

async function sendToNativeApp(message) {
  return new Promise((resolve, reject) => {
    try {
      chrome.runtime.sendNativeMessage('com.vidaro.downloader', message, (response) => {
        if (chrome.runtime.lastError) {
          reject(new Error(chrome.runtime.lastError.message));
        } else {
          resolve(response);
        }
      });
    } catch (error) {
      reject(error);
    }
  });
}

let port = null;

function connectToNativeApp() {
  try {
    const port = chrome.runtime.connectNative('com.vidaro.downloader');
    
    port.onMessage.addListener((message) => {
      handleNativeMessage(message);
    });
    
    port.onDisconnect.addListener(() => {
      console.log('[Vidaro] Disconnected from native app');
      if (chrome.runtime.lastError) {
        console.error('Native messaging error:', chrome.runtime.lastError.message);
      }
      // Try to reconnect
      setTimeout(() => connectToNativeApp(), 5000);
    });
    
    console.log('[Vidaro Download Page] Connected to native app');
  } catch (error) {
    console.error('[Vidaro] Failed to connect to native app:', error);
  }
}

function sendToNativeApp(message) {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendNativeMessage('com.vidaro.downloader', message, (response) => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
      } else {
        resolve(response);
      }
    });
  }

  // For fire-and-forget messages
  if (port) {
    port.postMessage(message);
  }
}

async function sendToNativeApp(message) {
  return new Promise((resolve, reject) => {
    try {
      chrome.runtime.sendNativeMessage('com.vidaro.downloader', message, (response) => {
        if (chrome.runtime.lastError) {
          reject(new Error(chrome.runtime.lastError.message));
        } else {
          resolve(response);
        }
      });
    } catch (error) {
      reject(error);
    }
  });
}

function extractVideoId(url) {
  try {
    const urlObj = new URL(url);
    return urlObj.searchParams.get('v');
  } catch {
    return null;
  }
}

function setupEventListeners(url, quality, format, audioOnly, embedThumbnail, embedMetadata, writeSubs, subLangs) {
  // Download button
  const downloadBtn = document.getElementById('download-btn');
  const cancelBtn = document.getElementById('cancel-btn');
  const closeBtn = document.getElementById('close-btn');
  const backBtn = document.getElementById('back-btn');
  const formatSelect = document.getElementById('format-select');
  const qualitySelect = document.getElementById('quality-select');
  const writeSubsCheckbox = document.getElementById('write-subs');
  const subLangsField = document.getElementById('sub-langs-field');
  
  // Download button
  document.getElementById('download-btn').addEventListener('click', async () => {
    await startDownload();
  });
  
  // Cancel button
  cancelBtn?.addEventListener('click', () => {
    window.close();
  });
  
  // Close button
  closeBtn?.addEventListener('click', () => {
    window.close();
  });
  
  // Back button
  backBtn?.addEventListener('click', () => {
    window.close();
  });
  
  // Format change
  formatSelect?.addEventListener('change', (e) => {
    const qualityField = document.getElementById('quality-field');
    qualityField.style.display = e.target.value === 'mp3' ? 'none' : 'block';
  });
  
  // Subtitles checkbox
  const writeSubsCheckbox = document.getElementById('write-subs');
  const subLangsField = document.getElementById('sub-langs-field');
  if (writeSubsCheckbox && subLangsField) {
    writeSubsCheckbox.addEventListener('change', (e) => {
      subLangsField.style.display = e.target.checked ? 'block' : 'none';
    });
  }
}

async function startDownload() {
  const downloadBtn = document.getElementById('download-btn');
  const cancelBtn = document.getElementById('cancel-btn');
  const format = formatSelect.value;
  const quality = qualitySelect.value;
  const writeSubs = writeSubsCheckbox.checked;
  const subLangs = subLangsField.value.trim();
  const embedThumbnail = document.getElementById('embed-thumbnail')?.checked;
  const embedMetadata = document.getElementById('embed-metadata')?.checked;
  const writeSubs = writeSubsCheckbox.checked;
  const subLangs = subLangsField.value.trim();
  
  // Disable download button
  const downloadBtn = document.getElementById('download-btn');
  downloadBtn.disabled = true;
  downloadBtn.classList.add('downloading');
  downloadBtn.querySelector('.btn-text').textContent = 'Starting...';
  downloadBtn.querySelector('.btn-loader').classList.remove('hidden');
  
  // Show progress
  document.getElementById('download-progress').style.display = 'block';
  document.getElementById('download-options').style.display = 'none';
  
  // Build options
  const options = {
    quality: quality,
    format: format,
    audioOnly: false,
    embedThumbnail: document.getElementById('embed-thumbnail')?.checked,
    embedMetadata: document.getElementById('embed-metadata')?.checked,
    writeSubs: writeSubs,
    subLangs: subLangs
  };
  
  try {
    const response = await sendToNativeApp({
      type: 'download',
      url: window.location.search.substring(1).split('=')[1] || window.location.search.substring(4),
      options: options
    });
    
    if (response && response.success) {
      // Show progress
      document.getElementById('download-progress').style.display = 'block';
      document.getElementById('download-options').style.display = 'none';
      
      // Start polling for progress
      startProgressPolling(response.downloadId);
      
      // Show cancel button
      document.getElementById('cancel-btn').style.display = 'inline-flex';
      document.getElementById('download-btn').disabled = true;
      document.getElementById('download-btn').querySelector('.btn-text').textContent = 'Downloading...';
      document.getElementById('download-btn').querySelector('.btn-loader').classList.remove('hidden');
    } else {
      showError(response?.error || 'Failed to start download');
      resetDownloadButton();
    }
  } catch (error) {
    console.error('Download error:', error);
    showError('Failed to start download: ' + error.message);
    resetDownloadButton();
  }
}

function startProgressPolling(downloadId) {
  if (progressPollInterval) {
    clearInterval(progressPollInterval);
  }
  
  progressPollInterval = setInterval(async () => {
    try {
      const response = await sendToNativeApp({
        type: 'getItemStatus',
        id: downloadId
      });
      
      if (response && response.item) {
        updateProgress(response.item);
        
        if (response.item.status === 'completed' || response.item.status === 'error' || response.item.status === 'cancelled') {
          clearInterval(progressPollInterval);
          
          if (response.item.status === 'completed') {
            showSuccess('Download completed!');
            setTimeout(() => window.close(), 3000);
          } else if (response.item.status === 'error') {
            showError(response.item.error || 'Download failed');
            resetDownloadUI();
          }
        }
      } catch (error) {
        console.error('Progress polling error:', error);
      }
    }, 1000);
  }

  function updateProgress(item) {
    const percent = Math.round(item.percent || 0);
    document.getElementById('progress-fill').style.width = percent + '%';
    document.getElementById('progress-percent').textContent = percent + '%';
    
    if (item.speedBps) {
      document.getElementById('progress-speed').textContent = formatSpeed(item.speedBps);
    }
    
    if (item.etaSecs) {
      document.getElementById('progress-eta').textContent = 'ETA: ' + formatDuration(item.etaSecs);
    }
    
    document.getElementById('progress-status').textContent = getStatusText(item.status);
    
    const progressFill = document.getElementById('progress-fill');
    progressFill.style.width = (item.percent || 0) + '%';
    
    document.getElementById('progress-percent').textContent = Math.round(item.percent || 0) + '%';
  }

  function showError(message) {
    alert('Error: ' + message);
    resetDownloadUI();
  }

  function showSuccess(message) {
    alert(message);
  }

  function resetDownloadUI() {
    document.getElementById('download-btn').disabled = false;
    document.getElementById('download-btn').classList.remove('downloading');
    document.getElementById('download-btn').querySelector('.btn-text').textContent = 'Start Download';
    document.getElementById('download-btn').querySelector('.btn-loader').classList.add('hidden');
    document.getElementById('cancel-btn').style.display = 'none';
    document.getElementById('download-progress').style.display = 'none';
    document.getElementById('download-options').style.display = 'block';
  }

  function formatDuration(seconds) {
    if (!seconds || seconds < 0) return '--:--';
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    if (hours > 0) {
      return `${hours}:${String(minutes).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    }
    return `${minutes}:${String(secs).padStart(2, '0')}`;
  }

  function formatSpeed(bytesPerSecond) {
    if (!bytesPerSecond) return '-- KB/s';
    const units = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    let i = 0;
    let speed = bytesPerSecond;
    while (speed >= 1024 && i < 3) {
      speed /= 1024;
      i++;
    }
    return speed.toFixed(1) + ' ' + units[i];
  }

  function formatDuration(seconds) {
    if (!seconds || seconds < 0) return '--:--';
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    if (hours > 0) {
      return `${hours}:${String(minutes).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    }
    return `${minutes}:${String(secs).padStart(2, '0')}`;
  }

  function formatSpeed(bytesPerSecond) {
    if (!bytesPerSecond) return '-- KB/s';
    const units = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    let i = 0;
    let speed = bytesPerSecond;
    while (speed >= 1024 && i < units.length - 1) {
      speed /= 1024;
      i++;
    }
    return speed.toFixed(1) + ' ' + units[i];
  }

  function formatViews(count) {
    if (!count) return '--';
    if (count >= 1000000) return (count / 1000000).toFixed(1) + 'M';
    if (count >= 1000) return (count / 1000).toFixed(1) + 'K';
    return count.toString();
  }

  function getStatusText(status) {
    switch (status) {
      case 'queued': return 'Queued';
      case 'active': return 'Downloading...';
      case 'paused': return 'Paused';
      case 'completed': return 'Completed';
      case 'error': return 'Error';
      case 'cancelled': return 'Cancelled';
      default: return status;
    }
  }

  function showError(message) {
    alert('Error: ' + message);
    resetDownloadUI();
  }

  function resetDownloadUI() {
    document.getElementById('download-btn').disabled = false;
    document.getElementById('download-btn').classList.remove('downloading');
    document.getElementById('download-btn').querySelector('.btn-text').textContent = 'Start Download';
    document.getElementById('download-btn').querySelector('.btn-loader').classList.add('hidden');
    document.getElementById('cancel-btn').style.display = 'none';
    document.getElementById('download-progress').style.display = 'none';
    document.getElementById('download-options').style.display = 'block';
  }

  function showError(message) {
    // Create toast notification
    const toast = document.createElement('div');
    toast.className = 'toast toast-error';
    toast.textContent = message;
    toast.style.cssText = `
      position: fixed;
      bottom: 20px;
      right: 20px;
      background: #f44336;
      color: white;
      padding: 12px 24px;
      border-radius: 8px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.2);
      z-index: 10000;
      animation: slideIn 0.3s ease;
    `;
    
    document.body.appendChild(toast);
    setTimeout(() => {
      toast.style.animation = 'slideOut 0.3s ease forwards';
      setTimeout(() => toast.remove(), 300);
    }, 5000);
  }