// Vidaro YouTube Downloader - Content Script
// Injects download buttons into YouTube pages

(function() {
  'use strict';

  let downloadButtonsInjected = false;
  let observer = null;
  let currentVideoId = null;
  let currentPlaylistId = null;
  let currentChannelId = null;

  // Initialize when DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

  function init() {
    // Detect page type and video/playlist/channel info
    detectPageType();
    
    // Set up mutation observer for SPA navigation
    setupObserver();
    
    // Listen for messages from background script
    chrome.runtime.onMessage.addListener(handleMessage);
    
    // Initial button injection
    injectDownloadButtons();
  }

  function detectPageType() {
    const url = window.location.href;
    const urlObj = new URL(window.location.href);
    
    // Check for video page
    const videoMatch = urlObj.searchParams.get('v');
    if (videoMatch) {
      currentVideoId = videoMatch;
    }
    
    // Check for playlist
    const playlistMatch = urlObj.searchParams.get('list');
    if (playlistMatch) {
      currentPlaylistId = playlistMatch;
    }
    
    // Check for channel
    const pathParts = urlObj.pathname.split('/').filter(p => p);
    if (pathParts.length > 0) {
      const firstPart = pathParts[0];
      if (firstPart.startsWith('@') || firstPart === 'channel' || firstPart === 'c' || firstPart === 'user') {
        currentChannelId = firstPart;
      }
    }
  }

  function setupObserver() {
    // Watch for SPA navigation changes
    observer = new MutationObserver((mutations) => {
      let shouldReinject = false;
      
      for (const mutation of mutations) {
        if (mutation.type === 'childList') {
          for (const node of mutation.addedNodes) {
            if (node.nodeType === Node.ELEMENT_NODE) {
              // Check for navigation or video player changes
              if (node.matches && (node.matches('#movie_player') || node.matches('.html5-video-player') || node.querySelector('#movie_player'))) {
                shouldReinject = true;
                break;
              }
              // Check for navigation
              if (node.id === 'contents' || node.id === 'primary' || node.classList?.contains('ytd-browse')) {
                shouldReinject = true;
                break;
              }
            }
          }
        }
        
        if (shouldReinject) {
          setTimeout(() => {
            detectPageType();
            injectDownloadButtons();
          }, 500);
        }
      });
      
      observer.observe(document.body, {
        childList: true,
        subtree: true
      });
  }

  function handleMessage(message, sender, sendResponse) {
    switch (message.type) {
      case 'queueUpdate':
        // Update UI with queue status
        updateDownloadButtons(message.items);
        break;
      case 'settingsUpdate':
        // Update UI based on new settings
        break;
      case 'settingsUpdate':
        break;
      case 'ping':
        sendResponse({ status: 'pong' });
        return true;
    }
    return true;
  }

  function injectDownloadButtons() {
    // Remove existing buttons to avoid duplicates
    removeExistingButtons();
    
    // Inject button for single video page
    injectVideoPageButton();
    
    // Inject buttons for playlist items
    injectPlaylistButtons();
    
    // Inject buttons for channel videos
    injectChannelButtons();
  }

  function removeExistingButtons() {
    document.querySelectorAll('.vidaro-download-btn, .vidaro-playlist-download-btn, .vidaro-channel-download-btn').forEach(btn => btn.remove());
  }

  // Single video page - add button below video title
  function injectVideoPageButton() {
    if (!currentVideoId) return;
    
    // Try multiple selectors for video title area
    const titleSelectors = [
      '#title h1 yt-formatted-string',
      '#title h1',
      '.ytd-watch-metadata #title',
      '#title.style-scope.ytd-watch-metadata'
    ];
    
    let titleElement = null;
    for (const selector of titleSelectors) {
      titleElement = document.querySelector(selector);
      if (titleElement) break;
    }
    
    if (!titleElement) return;
    
    // Check if button already exists
    if (document.getElementById('vidaro-download-btn-main')) return;
    
    const button = createDownloadButton('vidaro-download-btn-main', 'Download with Vidaro', async () => {
      await downloadVideo(currentVideoId);
    });
    
    // Insert after title
    const container = document.createElement('div');
    container.id = 'vidaro-download-container-main';
    container.style.marginTop = '8px';
    container.appendChild(button);
    
    titleElement.parentNode.insertBefore(container, titleElement.nextSibling);
  }

  function injectPlaylistButtons() {
    if (!currentPlaylistId) return;
    
    // Find playlist items
    const playlistItems = document.querySelectorAll('ytd-playlist-video-renderer, ytd-playlist-video-list-renderer');
    
    playlistItems.forEach((item, index) => {
      if (item.querySelector('.vidaro-playlist-download-btn')) return;
      
      const videoId = item.getAttribute('video-id') || 
                     item.querySelector('a#video-title')?.href?.match(/v=([^&]+)/)?.[1];
      
      if (!videoId) return;
      
      const button = createDownloadButton(
        `vidaro-playlist-btn-${index}`,
        'Download',
        async () => await downloadVideo(videoId)
      );
      button.classList.add('vidaro-playlist-download-btn');
      button.style.fontSize = '12px';
      button.style.padding = '4px 8px';
      button.style.marginLeft = '8px';
      
      // Find title element to insert button after
      const titleEl = item.querySelector('#video-title, a#video-title');
      if (titleEl) {
        const wrapper = document.createElement('span');
        wrapper.style.display = 'inline-flex';
        wrapper.style.alignItems = 'center';
        wrapper.style.marginLeft = '8px';
        wrapper.appendChild(button);
        titleEl.parentNode.insertBefore(wrapper, titleEl.nextSibling);
      }
    });
  }

  function injectChannelButtons() {
    if (!currentChannelId) return;
    
    const videoItems = document.querySelectorAll('ytd-rich-item-renderer, ytd-grid-video-renderer');
    
    videoItems.forEach((item, index) => {
      if (item.querySelector('.vidaro-channel-download-btn')) return;
      
      const link = item.querySelector('a#video-title-link, a#thumbnail');
      const href = link?.href;
      const videoIdMatch = href?.match(/\/watch\?v=([^&]+)/);
      const videoId = videoIdMatch?.[1];
      
      if (!videoId) return;
      
      const button = createDownloadButton(
        `vidaro-channel-btn-${index}`,
        'Download',
        async () => await downloadVideo(videoId)
      );
      button.classList.add('vidaro-channel-download-btn');
      button.style.fontSize = '11px';
      button.style.padding = '2px 6px';
      
      const meta = item.querySelector('#metadata-line, #metadata');
      if (meta) {
        const wrapper = document.createElement('span');
        wrapper.style.display = 'inline-flex';
        wrapper.style.alignItems = 'center';
        wrapper.style.marginTop = '4px';
        wrapper.appendChild(button);
        meta.appendChild(wrapper);
      }
    });
  }

  function createDownloadButton(id, text, onClick) {
    const button = document.createElement('button');
    button.id = id;
    button.className = 'vidaro-download-btn';
    button.textContent = text;
    button.style.cssText = `
      background: linear-gradient(135deg, #ff0000, #cc0000);
      color: white;
      border: none;
      border-radius: 4px;
      padding: 8px 16px;
      font-size: 14px;
      font-weight: 600;
      cursor: pointer;
      box-shadow: 0 2px 4px rgba(255,0,0,0.3);
      transition: all 0.2s ease;
      display: inline-flex;
      align-items: center;
      gap: 6px;
    `;
    
    button.addEventListener('mouseenter', () => {
      button.style.transform = 'translateY(-1px)';
      button.style.boxShadow = '0 4px 8px rgba(255,0,0,0.4)';
    });
    button.addEventListener('mouseleave', () => {
      button.style.transform = 'translateY(0)';
      button.style.boxShadow = '0 2px 4px rgba(255,0,0,0.3)';
    });
    button.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      onClick();
    });
    
    return button;
  }

  async function downloadVideo(videoId) {
    const url = `https://www.youtube.com/watch?v=${videoId}`;
    
    // Send message to background script to initiate download
    return new Promise((resolve) => {
      chrome.runtime.sendMessage({
        type: 'download',
        url: `https://www.youtube.com/watch?v=${videoId}`,
        quality: 'best',
        audioOnly: false
      }, (response) => {
        if (chrome.runtime.lastError) {
          console.error('[Vidaro] Download error:', chrome.runtime.lastError);
        }
        resolve(response);
      });
    });
  }

  function updateDownloadButtons(items) {
    // Update button states based on queue status
    items.forEach(item => {
      const buttons = document.querySelectorAll(`[data-video-id="${item.video_id}"]`);
      buttons.forEach(btn => {
        if (item.status === 'completed') {
          btn.textContent = 'Downloaded';
          btn.disabled = true;
          btn.style.background = 'linear-gradient(135deg, #4CAF50, #45a049)';
        } else if (item.status === 'active') {
          btn.textContent = `${Math.round(item.percent)}%`;
          btn.disabled = true;
        } else if (item.status === 'error') {
          btn.textContent = 'Error';
          btn.disabled = false;
          btn.style.background = 'linear-gradient(135deg, #f44336, #d32f2f)';
        }
      });
    });
  }

  // Clean up on unload
  window.addEventListener('beforeunload', () => {
    if (observer) observer.disconnect();
  };
})();