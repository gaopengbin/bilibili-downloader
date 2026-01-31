import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { getVersion } from '@tauri-apps/api/app';
import type { UserSettings } from '@/types';
import { defaultSettings } from '@/types';

export const useAppStore = defineStore('app', () => {
  // ==================== 主题 ====================
  const isDarkMode = ref(false);

  function toggleTheme() {
    isDarkMode.value = !isDarkMode.value;
    document.documentElement.classList.toggle('dark', isDarkMode.value);
    localStorage.setItem('theme', isDarkMode.value ? 'dark' : 'light');
  }

  function initTheme() {
    const saved = localStorage.getItem('theme');
    if (saved === 'dark' || (!saved && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
      isDarkMode.value = true;
      document.documentElement.classList.add('dark');
    }
  }

  // ==================== UI 状态 ====================
  const activeTab = ref('search');
  const showSettings = ref(false);
  const showDownloadCenter = ref(false);
  const leftPanelWidth = ref(50);
  const isDragging = ref(false);
  const descExpanded = ref(false);

  // ==================== 设置 ====================
  const settings = ref<UserSettings>({ ...defaultSettings });

  function loadSettings() {
    try {
      const saved = localStorage.getItem('userSettings');
      if (saved) {
        const parsed = JSON.parse(saved);
        settings.value = { ...defaultSettings, ...parsed };
      }
    } catch (error) {
      console.error('加载设置失败:', error);
    }
  }

  function saveSettings() {
    try {
      localStorage.setItem('userSettings', JSON.stringify(settings.value));
    } catch (error) {
      console.error('保存设置失败:', error);
    }
  }

  function updateSettings(newSettings: Partial<UserSettings>) {
    settings.value = { ...settings.value, ...newSettings };
    saveSettings();
  }

  // ==================== 更新 ====================
  const currentVersion = ref('0.0.0'); // 将从 Tauri 动态获取
  const isCheckingUpdate = ref(false);
  const isDownloadingUpdate = ref(false);
  const updateDownloadProgress = ref(0);
  const showUpdateDialog = ref(false);
  const updateInfo = ref<{
    version: string;
    body?: string;
    url: string;
    downloadUrl?: string;
  } | null>(null);

  // 初始化版本号（从 tauri.conf.json 获取）
  async function initVersion() {
    try {
      currentVersion.value = await getVersion();
    } catch (error) {
      console.error('获取版本号失败:', error);
      currentVersion.value = '0.0.0';
    }
  }

  // 版本号比较 (返回 1: a > b, -1: a < b, 0: a == b)
  function compareVersions(a: string, b: string): number {
    const partsA = a.split('.').map(Number);
    const partsB = b.split('.').map(Number);
    
    for (let i = 0; i < Math.max(partsA.length, partsB.length); i++) {
      const numA = partsA[i] || 0;
      const numB = partsB[i] || 0;
      if (numA > numB) return 1;
      if (numA < numB) return -1;
    }
    return 0;
  }

  // 从更新日志中提取关键更新内容
  function extractKeyUpdates(body: string): string[] {
    if (!body) return [];
    
    const updates: string[] = [];
    const lines = body.split('\n');
    
    for (const line of lines) {
      const trimmed = line.trim();
      if (trimmed.startsWith('-') || trimmed.startsWith('*')) {
        let content = trimmed.slice(1).trim();
        content = content.replace(/^[\u{1F300}-\u{1F9FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]\s*/u, '');
        const boldMatch = content.match(/\*\*(.+?)\*\*/);
        if (boldMatch) {
          updates.push(boldMatch[1]);
        } else if (content.length > 0 && content.length < 50) {
          updates.push(content);
        }
      }
    }
    
    return updates.slice(0, 5);
  }

  const keyUpdates = computed(() => {
    if (!updateInfo.value?.body) return [];
    return extractKeyUpdates(updateInfo.value.body);
  });

  async function checkForUpdate(silent = false) {
    if (isCheckingUpdate.value) return { hasUpdate: false };
    
    isCheckingUpdate.value = true;
    try {
      const response = await fetch('https://api.github.com/repos/gaopengbin/bilibili-downloader/releases/latest');
      if (!response.ok) throw new Error('获取更新信息失败');
      
      const data = await response.json();
      const latestVersion = data.tag_name.replace(/^v/, '');
      
      if (compareVersions(latestVersion, currentVersion.value) > 0) {
        const assets = data.assets || [];
        const setupAsset = assets.find((a: { name: string }) => 
          a.name.endsWith('_setup.exe') || a.name.endsWith('-setup.exe')
        );
        
        updateInfo.value = {
          version: latestVersion,
          body: data.body,
          url: data.html_url,
          downloadUrl: setupAsset?.browser_download_url
        };
        showUpdateDialog.value = true;
        return { hasUpdate: true };
      }
      return { hasUpdate: false };
    } catch (error) {
      console.error('检查更新失败:', error);
      if (!silent) throw error;
      return { hasUpdate: false, error };
    } finally {
      isCheckingUpdate.value = false;
    }
  }

  // ==================== 初始化 ====================
  function init() {
    initTheme();
    loadSettings();
    initVersion(); // 从 Tauri 获取版本号
  }

  return {
    // 主题
    isDarkMode,
    toggleTheme,
    initTheme,
    
    // UI 状态
    activeTab,
    showSettings,
    showDownloadCenter,
    leftPanelWidth,
    isDragging,
    descExpanded,
    
    // 设置
    settings,
    loadSettings,
    saveSettings,
    updateSettings,
    
    // 更新
    currentVersion,
    isCheckingUpdate,
    isDownloadingUpdate,
    updateDownloadProgress,
    showUpdateDialog,
    updateInfo,
    keyUpdates,
    checkForUpdate,
    compareVersions,
    initVersion,
    
    // 初始化
    init,
  };
});
