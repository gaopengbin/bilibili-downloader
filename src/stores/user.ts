import { defineStore } from 'pinia';
import { ref } from 'vue';
import type {
  BilibiliUserInfo,
  BilibiliHistoryItem,
  BilibiliFavoriteFolder,
  BilibiliFavoriteItem,
  BilibiliSearchResultItem,
  BilibiliSearchType,
} from '@/types/bilibili';

export const useUserStore = defineStore('user', () => {
  // ==================== 用户信息 ====================
  const userInfo = ref<BilibiliUserInfo | null>(null);
  const isLoggedIn = ref(false);

  // ==================== 登录状态 ====================
  const showLoginDialog = ref(false);
  const qrcodeBase64 = ref('');
  const qrcodeKey = ref('');
  const qrcodeStatus = ref('');
  const pollingTimer = ref<ReturnType<typeof setInterval> | null>(null);

  // ==================== 搜索 ====================
  const searchKeyword = ref('');
  const searchLoading = ref(false);
  const searchResults = ref<BilibiliSearchResultItem[]>([]);
  const searchPage = ref(1);
  const searchHasMore = ref(false);
  const searchTotal = ref(0);
  const searchType = ref<BilibiliSearchType>('video');

  // ==================== 历史记录 ====================
  const historyList = ref<BilibiliHistoryItem[]>([]);
  const historyLoading = ref(false);
  const historyHasMore = ref(false);
  const historyCursor = ref(0);

  // ==================== 收藏夹 ====================
  const favoriteFolders = ref<BilibiliFavoriteFolder[]>([]);
  const selectedFolder = ref<number | null>(null);
  const favoriteList = ref<BilibiliFavoriteItem[]>([]);
  const favoriteLoading = ref(false);
  const favoriteHasMore = ref(false);
  const favoritePage = ref(1);

  // ==================== 方法 ====================

  // 设置用户信息
  function setUserInfo(info: BilibiliUserInfo | null) {
    userInfo.value = info;
    isLoggedIn.value = !!info;
  }

  // 清除用户信息（登出）
  function clearUserInfo() {
    userInfo.value = null;
    isLoggedIn.value = false;
    // 清除相关数据
    historyList.value = [];
    favoriteFolders.value = [];
    favoriteList.value = [];
    selectedFolder.value = null;
  }

  // 打开登录对话框
  function openLoginDialog() {
    showLoginDialog.value = true;
  }

  // 关闭登录对话框
  function closeLoginDialog() {
    showLoginDialog.value = false;
    // 清除二维码状态
    qrcodeBase64.value = '';
    qrcodeKey.value = '';
    qrcodeStatus.value = '';
    // 停止轮询
    if (pollingTimer.value) {
      clearInterval(pollingTimer.value);
      pollingTimer.value = null;
    }
  }

  // 设置二维码信息
  function setQrcode(base64: string, key: string) {
    qrcodeBase64.value = base64;
    qrcodeKey.value = key;
    qrcodeStatus.value = '';
  }

  // 开始轮询
  function startPolling(callback: () => void, interval = 2000) {
    if (pollingTimer.value) {
      clearInterval(pollingTimer.value);
    }
    pollingTimer.value = setInterval(callback, interval);
  }

  // 停止轮询
  function stopPolling() {
    if (pollingTimer.value) {
      clearInterval(pollingTimer.value);
      pollingTimer.value = null;
    }
  }

  // 重置搜索状态
  function resetSearch() {
    searchResults.value = [];
    searchPage.value = 1;
    searchHasMore.value = false;
    searchTotal.value = 0;
  }

  // 重置历史记录状态
  function resetHistory() {
    historyList.value = [];
    historyHasMore.value = false;
    historyCursor.value = 0;
  }

  // 重置收藏夹状态
  function resetFavorites() {
    favoriteList.value = [];
    favoriteHasMore.value = false;
    favoritePage.value = 1;
  }

  // 格式化播放量
  function formatPlayCount(count: number): string {
    if (count >= 10000) {
      return (count / 10000).toFixed(1) + '万';
    }
    return count.toString();
  }

  return {
    // 用户信息
    userInfo,
    isLoggedIn,

    // 登录状态
    showLoginDialog,
    qrcodeBase64,
    qrcodeKey,
    qrcodeStatus,
    pollingTimer,

    // 搜索
    searchKeyword,
    searchLoading,
    searchResults,
    searchPage,
    searchHasMore,
    searchTotal,
    searchType,

    // 历史记录
    historyList,
    historyLoading,
    historyHasMore,
    historyCursor,

    // 收藏夹
    favoriteFolders,
    selectedFolder,
    favoriteList,
    favoriteLoading,
    favoriteHasMore,
    favoritePage,

    // 方法
    setUserInfo,
    clearUserInfo,
    openLoginDialog,
    closeLoginDialog,
    setQrcode,
    startPolling,
    stopPolling,
    resetSearch,
    resetHistory,
    resetFavorites,
    formatPlayCount,
  };
});
