<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from '@tauri-apps/plugin-dialog';
// import { check } from '@tauri-apps/plugin-updater';
// import { relaunch } from '@tauri-apps/plugin-process';
import { ElMessage, ElMessageBox } from 'element-plus';
import { 
  Search, Download, VideoPlay, Clock, Star,
  User, Link, Close, Sunny, Moon,
  Setting
} from '@element-plus/icons-vue';

// 引入新拆分的组件
import { SettingsPanel, UpdateDialog, DownloadCenter, LoginDialog } from '@/components';
import { HistoryPanel, FavoritesPanel, SearchResultPanel, VideoDetailPanel } from '@/components/bilibili';
import { useAppStore, useUserStore } from '@/stores';

// 引入类型
import type { 
  VideoInfo, VideoEntry, UserInfo, HistoryItem, FavoriteFolder, FavoriteItem,
  SearchResultItem, SearchResult, PagedData, ApiResponse, DownloadTask, ProgressDetail,
  UserSettings, SeasonEpisode
} from '@/types';

// 初始化 store
const appStore = useAppStore();
const userStore = useUserStore();

// ==================== 状态 ====================

const searchKeyword = ref('');
const videoUrl = ref('');
const videoInfo = ref<VideoInfo | null>(null);
const selectedQuality = ref('');
const outputDir = ref('');
const loading = ref(false);
const downloading = ref(false);
const downloadProgress = ref(0);

const selectedEntries = ref<number[]>([]);
const selectedSeasonEpisodes = ref<string[]>([]); // 合集选中的bvid列表
const pendingCover = ref<string | null>(null); // 临时保存列表中的封面

// 合集项展开状态和分P信息缓存
const expandedSeasonItems = ref<Set<string>>(new Set()); // 展开的合集项bvid
const seasonItemEntries = ref<Map<string, VideoEntry[]>>(new Map()); // 每个合集项的分P列表
const seasonItemLoading = ref<Set<string>>(new Set()); // 正在加载分P的合集项
const selectedSeasonEntries = ref<Map<string, number[]>>(new Map()); // 每个合集项选中的分P

const userInfo = ref<UserInfo | null>(null);

const activeTab = ref('search');

// 搜索相关
const searchLoading = ref(false);
const searchResults = ref<SearchResultItem[]>([]);
const searchPage = ref(1);
const searchHasMore = ref(false);
const searchTotal = ref(0);
const searchType = ref<'video' | 'media_bangumi' | 'media_ft'>('video');

const historyList = ref<HistoryItem[]>([]);
const historyLoading = ref(false);
const historyHasMore = ref(false);
const historyCursor = ref(0);

const favoriteFolders = ref<FavoriteFolder[]>([]);
const selectedFolder = ref<number | null>(null);
const favoriteList = ref<FavoriteItem[]>([]);
const favoriteLoading = ref(false);
const favoriteHasMore = ref(false);
const favoritePage = ref(1);

// 下载中心
const showDownloadCenter = ref(false);
const downloadTasks = ref<DownloadTask[]>([]);
const currentTaskId = ref<string | null>(null);
const downloadStage = ref('视频'); // 当前下载阶段
const downloadSpeed = ref(''); // 下载速度
const isPausing = ref(false); // 是否正在暂停
const isProcessingQueue = ref(false); // 是否正在处理队列
const isSelectMode = ref(false); // 是否处于批量选择模式
const selectedTaskIds = ref<string[]>([]); // 选中的任务ID
const expandedGroupIds = ref<string[]>([]); // 展开的组任务ID

// 主题
const isDarkMode = ref(false);

// 设置面板
const showSettings = ref(false);

// 更新相关 - 已移至 stores/app.ts 和 components/common/UpdateDialog.vue

// 用户设置
import { defaultSettings } from '@/types';

const settings = ref<UserSettings>({
  ...defaultSettings
});

// 可拖动分栏
const leftPanelWidth = ref(50); // 左侧面板宽度百分比
const isDragging = ref(false);

// 简介折叠
const descExpanded = ref(false);

// ==================== 计算属性 ====================

// 检查是否是当前视频（用于合集列表高亮）
const isCurrentVideo = (bvid: string): boolean => {
  if (!videoUrl.value) return false;
  return videoUrl.value.includes(bvid);
};

// 顶级任务（不包含子任务，组任务或单独任务）
const topLevelTasks = computed(() => 
  downloadTasks.value.filter(t => !t.groupId)
);

// 已完成任务 - 仅显示顶级任务
const completedTasks = computed(() => 
  topLevelTasks.value.filter(t => t.status === 'completed')
);

// 失败任务 - 仅显示顶级任务
const failedTasks = computed(() => 
  topLevelTasks.value.filter(t => t.status === 'failed')
);

// 获取组任务的子任务
function getChildTasks(groupId: string): DownloadTask[] {
  return downloadTasks.value.filter(t => t.groupId === groupId);
}

// 切换组任务展开/折叠
function toggleGroupExpand(groupId: string) {
  const index = expandedGroupIds.value.indexOf(groupId);
  if (index === -1) {
    expandedGroupIds.value.push(groupId);
  } else {
    expandedGroupIds.value.splice(index, 1);
  }
}

const searchTypes = [
  { label: '视频', value: 'video' as const },
  { label: '番剧', value: 'media_bangumi' as const },
  { label: '影视', value: 'media_ft' as const },
];

// ==================== 生命周期 ====================

// 切换主题
function toggleTheme() {
  isDarkMode.value = !isDarkMode.value;
  document.documentElement.classList.toggle('dark', isDarkMode.value);
  localStorage.setItem('theme', isDarkMode.value ? 'dark' : 'light');
}

// 初始化主题
function initTheme() {
  const saved = localStorage.getItem('theme');
  if (saved === 'dark' || (!saved && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
    isDarkMode.value = true;
    document.documentElement.classList.add('dark');
  }
}

// 加载用户设置
function loadSettings() {
  try {
    const saved = localStorage.getItem('userSettings');
    if (saved) {
      const parsed = JSON.parse(saved);
      settings.value = { ...settings.value, ...parsed };
    }
  } catch (error) {
    console.error('加载设置失败:', error);
  }
}

// 选择默认下载目录 - 已移至 SettingsPanel 组件

onMounted(async () => {
  initTheme(); // 初始化主题
  loadSettings(); // 加载用户设置
  
  const splashStart = Date.now();
  const minSplashTime = 3000; // 开屏动画至少显示3秒
  
  // 初始化嵌入资源（yt-dlp, ffmpeg）
  try {
    await invoke('init_resources');
  } catch (error) {
    console.error('初始化失败:', error);
    ElMessage.error('初始化失败，请重新启动程序');
  }
  
  await checkLoginStatus();
  await loadDownloadTasks(); // 加载下载任务历史
  
  // 确保开屏动画至少显示3秒
  const elapsed = Date.now() - splashStart;
  if (elapsed < minSplashTime) {
    await new Promise(resolve => setTimeout(resolve, minSplashTime - elapsed));
  }
  
  // 初始化完成，关闭开屏窗口并显示主窗口
  await invoke('close_splashscreen');
  
  // 静默检查更新（启动后延迟3秒检查，不打扰用户）
  setTimeout(() => {
    appStore.checkForUpdate(true);
  }, 3000);
  
  // 监听窗口关闭事件
  const appWindow = getCurrentWindow();
  appWindow.onCloseRequested(async (event) => {
    // 检查是否有正在进行的下载或等待中的任务
    const hasActiveTasks = downloadTasks.value.some(
      t => t.status === 'downloading' || t.status === 'waiting'
    );
    
    if (hasActiveTasks) {
      // 先阻止默认关闭
      event.preventDefault();
      
      // 有活动任务，弹出确认
      try {
        await ElMessageBox.confirm(
          '当前有下载任务正在进行，确定要退出吗？',
          '提示',
          {
            confirmButtonText: '确定退出',
            cancelButtonText: '取消',
            type: 'warning',
          }
        );
        
        // 用户确认退出，先暂停所有进行中和等待中的任务
        for (const task of downloadTasks.value) {
          if (task.status === 'downloading' || task.status === 'waiting') {
            task.status = 'paused';
          }
        }
        
        // 取消当前下载进程
        try {
          await invoke('cancel_download');
        } catch (e) {
          // 忽略错误
        }
        
        // 保存任务状态
        await saveDownloadTasks();
        
        // 关闭窗口
        appWindow.destroy();
      } catch {
        // 用户取消，不关闭
      }
    }
  });
  
  // 监听简单进度
  await listen<number>('download-progress', (event) => {
    downloadProgress.value = event.payload;
  });
  
  // 监听更新下载进度
  await listen<number>('update-download-progress', (event) => {
    appStore.updateDownloadProgress = event.payload;
  });
  
  // 监听详细进度（支持多任务）
  await listen<ProgressDetail>('download-progress-detail', (event) => {
    const { task_id, percent, stage, speed, downloaded, total_size } = event.payload;
    downloadProgress.value = percent;
    downloadStage.value = stage;
    downloadSpeed.value = speed || '';
    
    // 根据 task_id 更新对应任务的进度
    if (task_id) {
      const task = downloadTasks.value.find(t => t.id === task_id);
      if (task && task.status === 'downloading') {
        task.progress = percent;
        task.stage = stage;
        task.speed = speed || '';
        task.downloaded = downloaded || '';
        task.totalSize = total_size || '';
        
        // 如果是子任务，更新父组任务状态
        if (task.groupId) {
          updateGroupTaskStatus(task.groupId);
        }
      }
    } else if (currentTaskId.value) {
      // 兼容旧版本（没有 task_id）
      const task = downloadTasks.value.find(t => t.id === currentTaskId.value);
      if (task) {
        task.progress = percent;
        task.stage = stage;
        task.speed = speed || '';
        task.downloaded = downloaded || '';
        task.totalSize = total_size || '';
        
        // 如果是子任务，更新父组任务状态
        if (task.groupId) {
          updateGroupTaskStatus(task.groupId);
        }
      }
    }
  });
});

onUnmounted(() => {
  // 移除拖动事件监听
  document.removeEventListener('mousemove', onDividerDrag);
  document.removeEventListener('mouseup', onDividerDragEnd);
});

// ==================== 可拖动分栏 ====================

function onDividerDragStart(e: MouseEvent) {
  isDragging.value = true;
  document.addEventListener('mousemove', onDividerDrag);
  document.addEventListener('mouseup', onDividerDragEnd);
  e.preventDefault();
}

function onDividerDrag(e: MouseEvent) {
  if (!isDragging.value) return;
  const container = document.querySelector('.app-main') as HTMLElement;
  if (!container) return;
  
  const rect = container.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const percent = (x / rect.width) * 100;
  
  // 限制范围 30% - 70%
  leftPanelWidth.value = Math.max(30, Math.min(70, percent));
}

function onDividerDragEnd() {
  isDragging.value = false;
  document.removeEventListener('mousemove', onDividerDrag);
  document.removeEventListener('mouseup', onDividerDragEnd);
}

// ==================== 登录相关 ====================

async function checkLoginStatus() {
  try {
    const result = await invoke<ApiResponse<UserInfo>>('check_login_status');
    if (result.success && result.data) {
      userInfo.value = result.data;
      userStore.setUserInfo(result.data as any); // 同步到 store
    }
  } catch (error) {
    console.error('检查登录状态失败', error);
  }
}

function openLoginDialog() {
  userStore.openLoginDialog();
}

async function handleLogout() {
  try {
    await invoke('logout');
    userInfo.value = null;
    historyList.value = [];
    favoriteList.value = [];
    favoriteFolders.value = [];
    userStore.clearUserInfo(); // 同步到 store
    ElMessage.success('已退出登录');
  } catch (error) {
    ElMessage.error(`退出失败: ${error}`);
  }
}

// 登录成功回调
function onLoginSuccess(user: UserInfo) {
  userInfo.value = user;
  // 登录成功后刷新历史记录和收藏夹
  loadHistory();
  loadFavoriteFolders();
}

// ==================== 搜索功能 ====================

async function doSearch(refresh = true) {
  if (!searchKeyword.value.trim()) {
    ElMessage.warning('请输入搜索关键词');
    return;
  }

  if (refresh) {
    searchPage.value = 1;
    searchResults.value = [];
  }

  searchLoading.value = true;

  try {
    const result = await invoke<ApiResponse<SearchResult>>('search_video', {
      keyword: searchKeyword.value,
      page: searchPage.value,
      searchType: searchType.value,
    });

    if (result.success && result.data) {
      if (refresh) {
        searchResults.value = result.data.items;
      } else {
        searchResults.value.push(...result.data.items);
      }
      searchHasMore.value = result.data.has_more;
      searchTotal.value = result.data.total;
    } else {
      ElMessage.error(result.error || '搜索失败');
    }
  } catch (error) {
    ElMessage.error(`搜索失败: ${error}`);
  } finally {
    searchLoading.value = false;
  }
}

async function loadMoreSearch() {
  searchPage.value++;
  await doSearch(false);
}

// ==================== 视频解析下载 ====================

async function parseVideo(url?: string) {
  const targetUrl = url || videoUrl.value;
  if (!targetUrl.trim()) {
    ElMessage.warning('请输入视频链接');
    return;
  }

  videoUrl.value = targetUrl;
  loading.value = true;
  videoInfo.value = null;
  selectedEntries.value = [];
  selectedSeasonEpisodes.value = [];
  
  try {
    const result = await invoke<ApiResponse<VideoInfo>>('get_video_info', {
      url: targetUrl,
    });

    if (result.success && result.data) {
      videoInfo.value = result.data;
      // 如果解析结果没有封面，使用列表中的封面
      if (!videoInfo.value.thumbnail && pendingCover.value) {
        videoInfo.value.thumbnail = pendingCover.value;
      }
      pendingCover.value = null;
      
      // 应用默认清晰度设置
      if (settings.value.defaultQuality && result.data.formats.some(f => f.height?.toString() === settings.value.defaultQuality)) {
        selectedQuality.value = settings.value.defaultQuality;
      } else if (result.data.formats.length > 0) {
        selectedQuality.value = result.data.formats[0].height?.toString() || '';
      }
      
      // 应用默认下载目录
      if (settings.value.defaultOutputDir && !outputDir.value) {
        outputDir.value = settings.value.defaultOutputDir;
      }
      
      if (result.data.is_playlist && result.data.entries.length > 0) {
        selectedEntries.value = result.data.entries.map(e => e.index);
      }
      
      // 如果有合集，初始化当前视频的分P信息并自动展开
      if (result.data.season && result.data.entries.length > 0) {
        const currentBvid = getCurrentBvid();
        if (currentBvid) {
          // 清除之前的缓存
          expandedSeasonItems.value.clear();
          seasonItemEntries.value.clear();
          selectedSeasonEntries.value.clear();
          
          // 缓存当前视频的分P
          seasonItemEntries.value.set(currentBvid, result.data.entries);
          seasonItemEntries.value = new Map(seasonItemEntries.value);
          
          // 默认全选分P
          selectedSeasonEntries.value.set(currentBvid, result.data.entries.map(e => e.index));
          selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
          
          // 自动展开当前视频
          expandedSeasonItems.value.add(currentBvid);
          expandedSeasonItems.value = new Set(expandedSeasonItems.value);
          
          // 默认选中当前视频
          selectedSeasonEpisodes.value = [currentBvid];
        }
      }
      
      ElMessage.success('解析成功');
    } else {
      ElMessage.error(result.error || '解析失败');
    }
  } catch (error) {
    ElMessage.error(`解析失败: ${error}`);
  } finally {
    loading.value = false;
  }
}

async function selectOutputDir() {
  try {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      outputDir.value = typeof selected === 'string' ? selected : '';
    }
  } catch (error) {
    ElMessage.error('选择目录失败');
  }
}

// ==================== 下载任务管理 ====================

// 加载下载任务历史
async function loadDownloadTasks() {
  try {
    const result = await invoke<ApiResponse<string>>('load_download_tasks');
    if (result.success && result.data) {
      const tasks = JSON.parse(result.data) as DownloadTask[];
      // 过滤掉超过7天的任务，并把下载中/等待中的任务标记为暂停（应用关闭中断）
      const sevenDaysAgo = Date.now() - 7 * 24 * 60 * 60 * 1000;
      downloadTasks.value = tasks
        .filter(t => t.createdAt > sevenDaysAgo)
        .map(t => (t.status === 'downloading' || t.status === 'waiting') 
          ? { ...t, status: 'paused' as const, error: '应用关闭中断' } 
          : t);
    }
  } catch (error) {
    console.error('加载下载任务失败:', error);
  }
}

// 保存下载任务（优化：不保存大的base64封面数据，但保留URL）
async function saveDownloadTasks() {
  try {
    // 只保存最近100个任务，且去掉大的base64封面数据
    const tasksToSave = downloadTasks.value.slice(0, 100).map(task => ({
      ...task,
      // 如果封面是base64格式（以data:开头）且超过1KB，不保存；URL则保留
      cover: task.cover && task.cover.startsWith('data:') && task.cover.length > 1000 
        ? null 
        : task.cover,
    }));
    await invoke('save_download_tasks', {
      tasksJson: JSON.stringify(tasksToSave),
    });
  } catch (error) {
    console.error('保存下载任务失败:', error);
  }
}

// 创建下载任务（初始状态为等待）- 不自动保存，由调用者决定何时保存
function createDownloadTask(title: string, cover: string | null, downloadInfo: DownloadTask['downloadInfo'], groupId?: string): DownloadTask {
  const task: DownloadTask = {
    id: `task_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    title,
    cover,
    status: 'waiting',
    progress: 0,
    stage: '下载中',
    createdAt: Date.now(),
    downloadInfo,
    groupId,
  };
  downloadTasks.value.unshift(task);
  // 不在这里保存，由调用者批量保存
  return task;
}

// 创建组任务
function createGroupTask(title: string, cover: string | null, totalCount: number, finalDir: string | null): DownloadTask {
  const task: DownloadTask = {
    id: `group_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    title,
    cover,
    status: 'waiting',
    progress: 0,
    stage: '下载中',
    createdAt: Date.now(),
    isGroup: true,
    childIds: [],
    totalCount,
    completedCount: 0,
    downloadInfo: {
      url: '',
      outputDir: '',
      tempDir: null,
      finalDir,
      quality: null,
      videoTitle: title,
      isPlaylistItem: false,
      entryIndex: null,
      entryTitle: null,
      expectedId: null,
    },
  };
  downloadTasks.value.unshift(task);
  return task;
}

// 更新组任务状态
function updateGroupTaskStatus(groupId: string) {
  const groupTask = downloadTasks.value.find(t => t.id === groupId);
  if (!groupTask || !groupTask.isGroup) return;
  
  const childTasks = getChildTasks(groupId);
  const completedCount = childTasks.filter(t => t.status === 'completed').length;
  const failedCount = childTasks.filter(t => t.status === 'failed').length;
  const downloadingCount = childTasks.filter(t => t.status === 'downloading').length;
  const waitingCount = childTasks.filter(t => t.status === 'waiting').length;
  const pausedCount = childTasks.filter(t => t.status === 'paused').length;
  const totalCount = childTasks.length;
  
  groupTask.completedCount = completedCount;
  groupTask.failedCount = failedCount;
  groupTask.totalCount = totalCount;
  
  // 计算总进度（只计算已完成的）
  groupTask.progress = totalCount > 0 ? (completedCount / totalCount) * 100 : 0;
  
  // 更新组任务状态
  if (completedCount === totalCount) {
    // 全部完成
    groupTask.status = 'completed';
    groupTask.completedAt = Date.now();
  } else if (failedCount === totalCount) {
    // 全部失败
    groupTask.status = 'failed';
  } else if (downloadingCount > 0) {
    // 有下载中的
    groupTask.status = 'downloading';
    groupTask.stage = `正在下载 ${completedCount}/${totalCount}`;
  } else if (waitingCount > 0) {
    // 有等待中的
    groupTask.status = 'waiting';
    if (failedCount > 0) {
      groupTask.stage = `${failedCount} 个失败，${waitingCount} 个等待中`;
    }
  } else if (failedCount > 0 && completedCount > 0) {
    // 部分完成、部分失败，没有进行中或等待的 -> 显示为部分失败
    groupTask.status = 'failed';
    groupTask.stage = `${completedCount} 个完成，${failedCount} 个失败`;
  } else if (pausedCount > 0) {
    // 有暂停的
    groupTask.status = 'paused';
  }
}

// 更新任务状态
function updateTaskStatus(taskId: string, status: 'completed' | 'failed', error?: string) {
  const task = downloadTasks.value.find(t => t.id === taskId);
  if (task) {
    // 如果任务已经是暂停状态，不要覆盖
    if (task.status === 'paused') return;
    task.status = status;
    task.completedAt = Date.now();
    if (error) task.error = error;
    if (status === 'completed') {
      task.progress = 100;
      task.retryCount = 0; // 成功后重置重试计数
    }
    
    // 如果是子任务，更新父组任务状态
    if (task.groupId) {
      updateGroupTaskStatus(task.groupId);
    }
    
    saveDownloadTasks();
  }
}

// 获取下载中的任务数量（不包括组任务）
function getDownloadingCount(): number {
  return downloadTasks.value.filter(t => t.status === 'downloading' && !t.isGroup).length;
}

// 获取等待中的任务（不包括组任务，组任务不直接下载）
function getWaitingTasks(): DownloadTask[] {
  return downloadTasks.value.filter(t => t.status === 'waiting' && !t.isGroup);
}

// 打开文件所在文件夹
async function openTaskFolder(task: DownloadTask) {
  const folderPath = task.downloadInfo?.finalDir || task.downloadInfo?.outputDir;
  if (!folderPath) {
    ElMessage.warning('无法获取文件夹路径');
    return;
  }
  try {
    await invoke('open_folder', { path: folderPath });
  } catch (error) {
    ElMessage.error('打开文件夹失败');
  }
}

// 执行单个下载任务
async function executeDownloadTask(task: DownloadTask): Promise<boolean> {
  if (!task.downloadInfo) return false;
  
  const info = task.downloadInfo;
  const taskId = task.id;
  
  // 验证 URL 有效性
  if (!info.url || info.url === 'https://www.bilibili.com/video/' || info.url.endsWith('/video/')) {
    updateTaskStatus(taskId, 'failed', '下载链接无效，缺少视频ID');
    return false;
  }
  
  task.status = 'downloading';
  task.progress = 0;
  task.stage = '下载中';
  task.error = undefined;
  currentTaskId.value = taskId;
  saveDownloadTasks();
  
  try {
    const result = await invoke<ApiResponse<string>>('download_video', {
      url: info.url,
      outputDir: info.outputDir,
      tempDir: info.tempDir,
      finalDir: info.finalDir,
      quality: info.quality,
      videoTitle: info.videoTitle,
      isPlaylistItem: info.isPlaylistItem,
      entryIndex: info.entryIndex,
      entryTitle: info.entryTitle,
      expectedId: info.expectedId,
      taskId: taskId, // 传递任务ID用于进度追踪
      aria2cConnections: settings.value.aria2cConnections,
      preferCodec: settings.value.preferCodec || null,
    });

    // 重新获取任务状态（可能已被暂停）
    const currentTask = downloadTasks.value.find(t => t.id === taskId);
    if (currentTask?.status === 'paused') {
      // 已经被暂停，不更新状态
      return false;
    }

    if (result.success) {
      updateTaskStatus(taskId, 'completed');
      return true;
    } else {
      // 检查是否需要自动重试
      const retryCount = task.retryCount || 0;
      if (retryCount < settings.value.maxRetryCount) {
        task.retryCount = retryCount + 1;
        task.status = 'waiting';
        task.stage = `重试中 (${task.retryCount}/${settings.value.maxRetryCount})`;
        task.progress = 0;
        saveDownloadTasks();
        console.log(`任务 ${taskId} 下载失败，自动重试 ${task.retryCount}/${settings.value.maxRetryCount}`);
        return false; // 返回 false，让队列继续处理
      } else {
        updateTaskStatus(taskId, 'failed', result.error);
        return false;
      }
    }
  } catch (error) {
    // 重新获取任务状态
    const currentTask = downloadTasks.value.find(t => t.id === taskId);
    if (currentTask?.status !== 'paused') {
      // 检查是否需要自动重试
      const retryCount = task.retryCount || 0;
      if (retryCount < settings.value.maxRetryCount) {
        task.retryCount = retryCount + 1;
        task.status = 'waiting';
        task.stage = `重试中 (${task.retryCount}/${settings.value.maxRetryCount})`;
        task.progress = 0;
        saveDownloadTasks();
        console.log(`任务 ${taskId} 下载异常，自动重试 ${task.retryCount}/${settings.value.maxRetryCount}`);
        return false;
      } else {
        updateTaskStatus(taskId, 'failed', String(error));
      }
    }
    return false;
  } finally {
    // 如果这是当前任务，清除
    if (currentTaskId.value === taskId) {
      currentTaskId.value = null;
    }
  }
}

// 处理下载队列（支持并行下载）
async function processDownloadQueue() {
  if (isProcessingQueue.value) return;
  isProcessingQueue.value = true;
  downloading.value = true;
  
  // 收集所有用到的临时目录
  const tempDirs = new Set<string>();
  
  try {
    while (true) {
      const waitingTasks = getWaitingTasks();
      const downloadingCount = getDownloadingCount();
      
      // 没有等待中的任务，且没有下载中的任务，退出
      if (waitingTasks.length === 0 && downloadingCount === 0) break;
      
      // 还有下载中的任务，但没有等待的，等待一下再检查
      if (waitingTasks.length === 0) {
        await new Promise(resolve => setTimeout(resolve, 500));
        continue;
      }
      
      // 已达到最大并行数，等待
      if (downloadingCount >= settings.value.maxConcurrentDownloads) {
        await new Promise(resolve => setTimeout(resolve, 500));
        continue;
      }
      
      // 可以启动新任务
      const task = waitingTasks[0];
      if (task.downloadInfo?.tempDir) {
        tempDirs.add(task.downloadInfo.tempDir);
      }
      
      // 异步启动任务（不等待完成）
      executeDownloadTask(task).catch(console.error);
      
      // 给一点时间让任务状态更新
      await new Promise(resolve => setTimeout(resolve, 100));
    }
  } finally {
    isProcessingQueue.value = false;
    downloading.value = downloadTasks.value.some(t => t.status === 'downloading');
    
    // 清理空的临时目录
    for (const tempDir of tempDirs) {
      try {
        await invoke('cleanup_temp_dir', { tempDir });
      } catch (e) {
        console.log('清理临时目录失败:', e);
      }
    }
  }
}

// 暂停下载
async function pauseDownload(task?: DownloadTask) {
  const targetTask = task || downloadTasks.value.find(t => t.id === currentTaskId.value);
  if (!targetTask) return;
  
  try {
    // 先设置暂停状态
    targetTask.status = 'paused';
    saveDownloadTasks();
    
    // 如果是当前正在下载的任务，取消进程
    if (targetTask.id === currentTaskId.value) {
      isPausing.value = true;
      await invoke('cancel_download');
      isPausing.value = false;
    }
    ElMessage.info('下载已暂停');
  } catch (error) {
    isPausing.value = false;
    console.error('暂停失败:', error);
  }
}

// 继续下载（将任务加入队列）
async function resumeDownload(task: DownloadTask) {
  if (!task.downloadInfo) {
    ElMessage.error('无法恢复，缺少下载信息');
    return;
  }
  
  // 检查 URL 是否有效
  if (!task.downloadInfo.url || task.downloadInfo.url === 'https://www.bilibili.com/video/') {
    ElMessage.error('下载链接无效');
    return;
  }
  
  // 设置为等待状态，加入队列
  task.status = 'waiting';
  task.error = undefined;
  task.progress = 0;
  
  // 如果是子任务，同时更新父组任务状态
  if (task.groupId) {
    updateGroupTaskStatus(task.groupId);
  }
  
  saveDownloadTasks();
  
  // 开始处理队列
  processDownloadQueue();
}

// 删除任务
async function deleteTask(task: DownloadTask) {
  // 如果是下载中的任务，先取消下载
  if (task.status === 'downloading') {
    isPausing.value = true;
    try {
      await invoke('cancel_download');
    } catch (e) {
      // 忽略错误
    }
    isPausing.value = false;
  }
  
  // 非已完成的任务，删除临时文件
  if (task.status !== 'completed' && task.downloadInfo?.tempDir) {
    try {
      await invoke('delete_folder', { path: task.downloadInfo.tempDir });
    } catch (e) {
      console.error('删除临时文件夹失败:', e);
    }
  }
  
  const index = downloadTasks.value.findIndex(t => t.id === task.id);
  if (index !== -1) {
    downloadTasks.value.splice(index, 1);
    saveDownloadTasks();
  }
}

// 暂停组任务
async function pauseGroupDownload(groupTask: DownloadTask) {
  if (!groupTask.isGroup) return;
  
  const childTasks = getChildTasks(groupTask.id);
  const activeChildren = childTasks.filter(t => t.status === 'downloading' || t.status === 'waiting');
  
  if (activeChildren.length === 0) return;
  
  // 取消当前下载
  isPausing.value = true;
  try {
    await invoke('cancel_download');
  } catch (e) {
    // 忽略错误
  }
  isPausing.value = false;
  
  // 将所有活动子任务设为暂停
  for (const child of activeChildren) {
    child.status = 'paused';
  }
  
  // 更新组状态
  groupTask.status = 'paused';
  saveDownloadTasks();
  ElMessage.info('已暂停组内所有任务');
}

// 继续组任务
function resumeGroupDownload(groupTask: DownloadTask) {
  if (!groupTask.isGroup) return;
  
  const childTasks = getChildTasks(groupTask.id);
  const pausedChildren = childTasks.filter(t => t.status === 'paused');
  
  if (pausedChildren.length === 0) return;
  
  // 将所有暂停子任务设为等待
  for (const child of pausedChildren) {
    child.status = 'waiting';
    child.error = undefined;
  }
  
  // 更新组状态
  groupTask.status = 'waiting';
  saveDownloadTasks();
  ElMessage.success('已恢复组内所有任务');
  
  // 开始处理队列
  processDownloadQueue();
}

// 删除组任务
async function deleteGroupTask(groupTask: DownloadTask) {
  if (!groupTask.isGroup) return;
  
  const childTasks = getChildTasks(groupTask.id);
  
  // 如果有正在下载的子任务，先取消
  const downloadingChildren = childTasks.filter(t => t.status === 'downloading');
  if (downloadingChildren.length > 0) {
    isPausing.value = true;
    try {
      await invoke('cancel_download');
    } catch (e) {
      // 忽略错误
    }
    isPausing.value = false;
  }
  
  // 删除临时文件（对于未完成的任务）
  const tempDir = childTasks[0]?.downloadInfo?.tempDir;
  if (tempDir && groupTask.status !== 'completed') {
    try {
      await invoke('delete_folder', { path: tempDir });
    } catch (e) {
      console.error('删除临时文件夹失败:', e);
    }
  }
  
  // 删除所有子任务和组任务
  const idsToDelete = [groupTask.id, ...childTasks.map(t => t.id)];
  downloadTasks.value = downloadTasks.value.filter(t => !idsToDelete.includes(t.id));
  saveDownloadTasks();
}

// 重试单个子任务
async function retryChildTask(child: DownloadTask) {
  if (!child.groupId) return;
  
  // 将子任务状态设为等待
  child.status = 'waiting';
  child.progress = 0;
  child.error = undefined;
  child.stage = '';
  child.speed = undefined;
  child.downloaded = undefined;
  child.totalSize = undefined;
  child.retryCount = 0; // 手动重试时重置计数
  
  // 更新组任务状态
  updateGroupTaskStatus(child.groupId);
  saveDownloadTasks();
  
  ElMessage.success('已加入下载队列');
  
  // 开始处理队列
  processDownloadQueue();
}

// 删除单个子任务
async function deleteChildTask(groupId: string, childId: string) {
  const groupTask = downloadTasks.value.find(t => t.id === groupId);
  const child = downloadTasks.value.find(t => t.id === childId);
  
  if (!groupTask || !child) return;
  
  // 如果子任务正在下载，先取消
  if (child.status === 'downloading') {
    isPausing.value = true;
    try {
      await invoke('cancel_download');
    } catch (e) {
      // 忽略错误
    }
    isPausing.value = false;
  }
  
  // 删除临时文件
  if (child.status !== 'completed' && child.downloadInfo?.tempDir) {
    try {
      await invoke('delete_folder', { path: child.downloadInfo.tempDir });
    } catch (e) {
      console.error('删除临时文件夹失败:', e);
    }
  }
  
  // 从任务列表中删除子任务
  const index = downloadTasks.value.findIndex(t => t.id === childId);
  if (index !== -1) {
    downloadTasks.value.splice(index, 1);
  }
  
  // 更新组任务的 childIds
  if (groupTask.childIds) {
    const childIndex = groupTask.childIds.indexOf(childId);
    if (childIndex !== -1) {
      groupTask.childIds.splice(childIndex, 1);
    }
  }
  
  // 更新组任务状态
  updateGroupTaskStatus(groupId);
  
  // 如果组任务没有子任务了，删除组任务
  const remainingChildren = getChildTasks(groupId);
  if (remainingChildren.length === 0) {
    const groupIndex = downloadTasks.value.findIndex(t => t.id === groupId);
    if (groupIndex !== -1) {
      downloadTasks.value.splice(groupIndex, 1);
    }
  }
  
  saveDownloadTasks();
}

// 重试组任务中所有失败的子任务
function retryFailedChildren(groupTask: DownloadTask) {
  if (!groupTask.isGroup) return;
  
  const childTasks = getChildTasks(groupTask.id);
  const failedChildren = childTasks.filter(t => t.status === 'failed');
  
  if (failedChildren.length === 0) return;
  
  // 将所有失败子任务设为等待
  for (const child of failedChildren) {
    child.status = 'waiting';
    child.progress = 0;
    child.error = undefined;
    child.stage = '';
    child.speed = undefined;
    child.downloaded = undefined;
    child.totalSize = undefined;
    child.retryCount = 0; // 重置重试计数
  }
  
  // 更新组任务状态
  updateGroupTaskStatus(groupTask.id);
  saveDownloadTasks();
  
  ElMessage.success(`已将 ${failedChildren.length} 个失败任务加入下载队列`);
  
  // 开始处理队列
  processDownloadQueue();
}

// 清空已完成任务
async function clearCompletedTasks() {
  const count = completedTasks.value.length;
  if (count === 0) return;
  
  try {
    await ElMessageBox.confirm(
      `确定清空 ${count} 个已完成任务？\n\n此操作仅删除下载记录，不会删除已下载的文件。`,
      '清空已完成',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'info',
      }
    );
    
    // 收集需要删除的任务ID（包括组任务的子任务）
    const idsToDelete = new Set<string>();
    for (const task of completedTasks.value) {
      idsToDelete.add(task.id);
      if (task.isGroup && task.childIds) {
        task.childIds.forEach(id => idsToDelete.add(id));
      }
    }
    
    downloadTasks.value = downloadTasks.value.filter(t => !idsToDelete.has(t.id));
    saveDownloadTasks();
    ElMessage.success(`已清空 ${count} 个已完成任务`);
  } catch {
    // 用户取消
  }
}

// 清空失败任务
async function clearFailedTasks() {
  const count = failedTasks.value.length;
  if (count === 0) return;
  
  try {
    await ElMessageBox.confirm(
      `确定清空 ${count} 个失败任务？\n\n此操作会同时删除临时文件。`,
      '清空失败任务',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    );
    
    // 删除临时文件
    for (const task of failedTasks.value) {
      if (task.downloadInfo?.tempDir) {
        try {
          await invoke('delete_folder', { path: task.downloadInfo.tempDir });
        } catch (e) {
          console.error('删除临时文件夹失败:', e);
        }
      }
    }
    
    // 收集需要删除的任务ID（包括组任务的子任务）
    const idsToDelete = new Set<string>();
    for (const task of failedTasks.value) {
      idsToDelete.add(task.id);
      if (task.isGroup && task.childIds) {
        task.childIds.forEach(id => idsToDelete.add(id));
      }
    }
    
    downloadTasks.value = downloadTasks.value.filter(t => !idsToDelete.has(t.id));
    saveDownloadTasks();
    ElMessage.success(`已清空 ${count} 个失败任务`);
  } catch {
    // 用户取消
  }
}

// 切换批量选择模式
function toggleSelectMode() {
  isSelectMode.value = !isSelectMode.value;
  if (!isSelectMode.value) {
    selectedTaskIds.value = [];
  }
}

// 切换任务选中状态
function toggleTaskSelect(taskId: string) {
  const index = selectedTaskIds.value.indexOf(taskId);
  if (index === -1) {
    selectedTaskIds.value.push(taskId);
  } else {
    selectedTaskIds.value.splice(index, 1);
  }
}

// 全选/取消全选所有顶级任务
function toggleSelectAllActive() {
  const allTopLevelIds = topLevelTasks.value.map(t => t.id);
  const allSelected = allTopLevelIds.every(id => selectedTaskIds.value.includes(id));
  if (allSelected) {
    selectedTaskIds.value = [];
  } else {
    selectedTaskIds.value = [...allTopLevelIds];
  }
}

// 批量删除选中的任务
async function deleteSelectedTasks(deleteFiles: boolean = false) {
  const count = selectedTaskIds.value.length;
  if (count === 0) return;
  
  const tasksToDelete = downloadTasks.value.filter(t => selectedTaskIds.value.includes(t.id));
  
  // 收集所有需要删除的任务ID（包括组任务的子任务）
  const allIdsToDelete = new Set<string>(selectedTaskIds.value);
  for (const task of tasksToDelete) {
    if (task.isGroup && task.childIds) {
      task.childIds.forEach(id => allIdsToDelete.add(id));
    }
  }
  
  // 获取所有要删除的任务
  const allTasksToDelete = downloadTasks.value.filter(t => allIdsToDelete.has(t.id));
  
  // 如果有正在下载的任务，先取消
  const downloadingTasks = allTasksToDelete.filter(t => t.status === 'downloading');
  if (downloadingTasks.length > 0) {
    isPausing.value = true;
    try {
      await invoke('cancel_download');
    } catch (e) {
      // 忽略错误
    }
    isPausing.value = false;
  }
  
  // 删除文件（使用Set避免重复删除同一目录）
  const deletedDirs = new Set<string>();
  for (const task of allTasksToDelete) {
    // 已完成的任务：根据 deleteFiles 参数决定是否删除
    if (task.status === 'completed') {
      if (deleteFiles && task.downloadInfo?.finalDir && !deletedDirs.has(task.downloadInfo.finalDir)) {
        try {
          await invoke('delete_folder', { path: task.downloadInfo.finalDir });
          deletedDirs.add(task.downloadInfo.finalDir);
        } catch (e) {
          console.error('删除文件夹失败:', e);
        }
      }
    } else {
      // 下载中/等待中/暂停/失败的任务：始终删除临时文件
      if (task.downloadInfo?.tempDir && !deletedDirs.has(task.downloadInfo.tempDir)) {
        try {
          await invoke('delete_folder', { path: task.downloadInfo.tempDir });
          deletedDirs.add(task.downloadInfo.tempDir);
        } catch (e) {
          console.error('删除临时文件夹失败:', e);
        }
      }
    }
  }
  
  // 删除任务记录
  downloadTasks.value = downloadTasks.value.filter(t => !allIdsToDelete.has(t.id));
  selectedTaskIds.value = [];
  isSelectMode.value = false;
  saveDownloadTasks();
  ElMessage.success(`已删除 ${count} 个任务`);
}

// 确认批量删除
async function confirmDeleteSelected() {
  const count = selectedTaskIds.value.length;
  if (count === 0) {
    ElMessage.warning('请先选择要删除的任务');
    return;
  }
  
  // 检查是否有已完成的任务（有文件可删除）
  const completedTasks = downloadTasks.value.filter(
    t => selectedTaskIds.value.includes(t.id) && t.status === 'completed'
  );
  const completedCount = completedTasks.length;
  
  if (completedCount > 0) {
    // 有已完成的任务，提供两个删除选项
    try {
      await ElMessageBox({
        title: '删除任务',
        message: `确定删除选中的 ${count} 个任务？其中 ${completedCount} 个已下载完成。`,
        showCancelButton: true,
        confirmButtonText: '同时删除文件',
        cancelButtonText: '仅删除记录',
        distinguishCancelAndClose: true,
        type: 'warning',
        confirmButtonClass: 'el-button--danger',
      });
      // 确认 - 同时删除文件
      await deleteSelectedTasks(true);
    } catch (actionResult) {
      if (actionResult === 'cancel') {
        // 仅删除记录
        await deleteSelectedTasks(false);
      }
      // close - 用户关闭弹窗，不做任何事
    }
  } else {
    // 没有已完成的任务，直接确认删除
    try {
      await ElMessageBox.confirm(
        `确定删除选中的 ${count} 个任务？`,
        '删除任务',
        {
          confirmButtonText: '删除',
          cancelButtonText: '取消',
          type: 'warning',
        }
      );
      await deleteSelectedTasks(false);
    } catch {
      // 用户取消
    }
  }
}

// 全部暂停
async function pauseAllTasks() {
  const activeTasks = downloadTasks.value.filter(
    t => t.status === 'downloading' || t.status === 'waiting'
  );
  if (activeTasks.length === 0) return;
  
  // 先取消当前下载
  isPausing.value = true;
  try {
    await invoke('cancel_download');
  } catch (e) {
    // 忽略错误
  }
  isPausing.value = false;
  
  // 将所有活动任务设为暂停
  for (const task of activeTasks) {
    task.status = 'paused';
  }
  saveDownloadTasks();
  ElMessage.info(`已暂停 ${activeTasks.length} 个任务`);
}

// 全部开始
function resumeAllTasks() {
  const pausedTasks = downloadTasks.value.filter(t => t.status === 'paused');
  if (pausedTasks.length === 0) return;
  
  // 将所有暂停任务设为等待
  for (const task of pausedTasks) {
    task.status = 'waiting';
    task.error = undefined;
  }
  saveDownloadTasks();
  ElMessage.success(`已恢复 ${pausedTasks.length} 个任务`);
  
  // 开始处理队列
  processDownloadQueue();
}

async function startDownload() {
  if (!videoInfo.value) return;
  if (!outputDir.value) {
    ElMessage.warning('请选择下载目录');
    return;
  }
  if (videoInfo.value.is_playlist && selectedEntries.value.length === 0) {
    ElMessage.warning('请选择要下载的分P');
    return;
  }

  // 创建所有任务（状态为等待）
  if (videoInfo.value.is_playlist && videoInfo.value.entries.length > 0) {
    const entriesToDownload = videoInfo.value.entries.filter(
      e => selectedEntries.value.includes(e.index)
    );
    
    // 多P视频：创建临时目录和最终目录
    const timestamp = Date.now();
    const tempDir = `${outputDir.value}\\temp_${timestamp}`;
    const finalDir = `${outputDir.value}\\${sanitizeFileName(videoInfo.value.title)}`;
    
    // 如果多于1个分P，创建组任务
    let groupId: string | undefined;
    if (entriesToDownload.length > 1) {
      const groupTask = createGroupTask(
        videoInfo.value.title,
        videoInfo.value.thumbnail,
        entriesToDownload.length,
        finalDir
      );
      groupId = groupTask.id;
      groupTask.childIds = [];
    }
    
    for (const entry of entriesToDownload) {
      const downloadUrl = entry.url || `${videoUrl.value}?p=${entry.index}`;
      const task = createDownloadTask(
        entriesToDownload.length > 1 ? `P${entry.index}. ${entry.title}` : `${videoInfo.value.title} - P${entry.index}`,
        videoInfo.value.thumbnail,
        {
          url: downloadUrl,
          outputDir: outputDir.value,
          tempDir: tempDir,
          finalDir: finalDir,
          quality: selectedQuality.value || null,
          videoTitle: videoInfo.value.title,
          isPlaylistItem: true,
          entryIndex: entry.index,
          entryTitle: entry.title,
          expectedId: entry.id || null,
        },
        groupId
      );
      
      // 添加到组任务的子任务列表
      if (groupId) {
        const groupTask = downloadTasks.value.find(t => t.id === groupId);
        if (groupTask && groupTask.childIds) {
          groupTask.childIds.push(task.id);
        }
      }
    }
    
    // 批量创建后统一保存一次
    saveDownloadTasks();
    ElMessage.success(`已添加 ${entriesToDownload.length} 个下载任务`);
  } else {
    createDownloadTask(
      videoInfo.value.title,
      videoInfo.value.thumbnail,
      {
        url: videoUrl.value,
        outputDir: outputDir.value,
        tempDir: null,
        finalDir: null,
        quality: selectedQuality.value || null,
        videoTitle: videoInfo.value.title,
        isPlaylistItem: false,
        entryIndex: null,
        entryTitle: null,
        expectedId: null,
      }
    );
    
    saveDownloadTasks();
    ElMessage.success('已添加下载任务');
  }
  
  // 开始处理队列
  processDownloadQueue();
}

// 清理文件名中的非法字符
function sanitizeFileName(name: string): string {
  return name
    .replace(/[\\/:*?"<>|]/g, '_')
    .replace(/\s+/g, ' ')
    .trim()
    .substring(0, 80);
}

function toggleSelectAll() {
  if (!videoInfo.value) return;
  if (selectedEntries.value.length === videoInfo.value.entries.length) {
    selectedEntries.value = [];
  } else {
    selectedEntries.value = videoInfo.value.entries.map(e => e.index);
  }
}

// 合集全选/取消全选
function toggleSeasonSelectAll() {
  if (!videoInfo.value?.season) return;
  if (selectedSeasonEpisodes.value.length === videoInfo.value.season.episodes.length) {
    selectedSeasonEpisodes.value = [];
    // 清空所有分P选择
    selectedSeasonEntries.value.clear();
    selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
  } else {
    selectedSeasonEpisodes.value = videoInfo.value.season.episodes.map(e => e.bvid);
    // 对于已加载分P的项，全选分P
    for (const ep of videoInfo.value.season.episodes) {
      const entries = seasonItemEntries.value.get(ep.bvid);
      if (entries && entries.length > 0) {
        selectedSeasonEntries.value.set(ep.bvid, entries.map(e => e.index));
      }
    }
    selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
  }
}

// 切换单个合集项的选中状态
function toggleSeasonEpisodeSelect(bvid: string) {
  const idx = selectedSeasonEpisodes.value.indexOf(bvid);
  if (idx >= 0) {
    // 取消选择：从外层移除，清空内层分P
    selectedSeasonEpisodes.value.splice(idx, 1);
    selectedSeasonEntries.value.delete(bvid);
    selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
  } else {
    // 选中：添加到外层，如果有分P则全选
    selectedSeasonEpisodes.value.push(bvid);
    const entries = seasonItemEntries.value.get(bvid);
    if (entries && entries.length > 0) {
      selectedSeasonEntries.value.set(bvid, entries.map(e => e.index));
      selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
    }
  }
}

// 获取当前视频的 BV 号
function getCurrentBvid(): string | null {
  const url = videoUrl.value;
  const match = url.match(/BV[a-zA-Z0-9]+/);
  return match ? match[0] : null;
}

// 展开/折叠合集项，获取分P信息
async function toggleSeasonItemExpand(bvid: string) {
  if (expandedSeasonItems.value.has(bvid)) {
    expandedSeasonItems.value.delete(bvid);
    expandedSeasonItems.value = new Set(expandedSeasonItems.value); // 触发响应式更新
    return;
  }
  
  // 如果已有缓存的分P信息，直接展开
  if (seasonItemEntries.value.has(bvid)) {
    expandedSeasonItems.value.add(bvid);
    expandedSeasonItems.value = new Set(expandedSeasonItems.value);
    return;
  }
  
  // 如果是当前视频，直接使用已有的分P信息
  const currentBvid = getCurrentBvid();
  if (currentBvid === bvid && videoInfo.value && videoInfo.value.entries.length > 0) {
    seasonItemEntries.value.set(bvid, videoInfo.value.entries);
    seasonItemEntries.value = new Map(seasonItemEntries.value);
    
    // 默认全选
    if (videoInfo.value.entries.length > 1) {
      selectedSeasonEntries.value.set(bvid, videoInfo.value.entries.map(e => e.index));
      selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
    }
    
    expandedSeasonItems.value.add(bvid);
    expandedSeasonItems.value = new Set(expandedSeasonItems.value);
    return;
  }
  
  // 其他视频需要获取分P信息
  seasonItemLoading.value.add(bvid);
  seasonItemLoading.value = new Set(seasonItemLoading.value);
  
  try {
    const result = await invoke<ApiResponse<VideoInfo>>('get_video_info', {
      url: `https://www.bilibili.com/video/${bvid}`
    });
    
    if (result.success && result.data) {
      // 缓存分P信息
      seasonItemEntries.value.set(bvid, result.data.entries);
      seasonItemEntries.value = new Map(seasonItemEntries.value);
      
      // 如果有多P，默认全选
      if (result.data.entries.length > 1) {
        selectedSeasonEntries.value.set(bvid, result.data.entries.map(e => e.index));
        selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
      }
      
      // 展开
      expandedSeasonItems.value.add(bvid);
      expandedSeasonItems.value = new Set(expandedSeasonItems.value);
    }
  } catch (error) {
    console.error('获取分P信息失败:', error);
  } finally {
    seasonItemLoading.value.delete(bvid);
    seasonItemLoading.value = new Set(seasonItemLoading.value);
  }
}

// 切换合集项内分P的选择
function toggleSeasonEntrySelect(bvid: string, entryIndex: number) {
  const current = selectedSeasonEntries.value.get(bvid) || [];
  const idx = current.indexOf(entryIndex);
  if (idx >= 0) {
    current.splice(idx, 1);
  } else {
    current.push(entryIndex);
  }
  selectedSeasonEntries.value.set(bvid, current);
  selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
  
  // 同步外层选择状态：有分P选中则外层选中，无则取消
  syncSeasonEpisodeSelection(bvid);
}

// 同步外层合集项的选中状态
function syncSeasonEpisodeSelection(bvid: string) {
  const entries = selectedSeasonEntries.value.get(bvid) || [];
  const isSelected = selectedSeasonEpisodes.value.includes(bvid);
  
  if (entries.length > 0 && !isSelected) {
    // 有分P选中但外层未选中，添加到外层
    selectedSeasonEpisodes.value.push(bvid);
  } else if (entries.length === 0 && isSelected) {
    // 无分P选中但外层选中，从外层移除
    const idx = selectedSeasonEpisodes.value.indexOf(bvid);
    if (idx >= 0) {
      selectedSeasonEpisodes.value.splice(idx, 1);
    }
  }
}

// 合集项内全选/取消全选分P
function toggleSeasonItemSelectAll(bvid: string) {
  const entries = seasonItemEntries.value.get(bvid) || [];
  const current = selectedSeasonEntries.value.get(bvid) || [];
  
  if (current.length === entries.length) {
    selectedSeasonEntries.value.set(bvid, []);
  } else {
    selectedSeasonEntries.value.set(bvid, entries.map(e => e.index));
  }
  selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
  
  // 同步外层选择
  syncSeasonEpisodeSelection(bvid);
}

// 获取合集项选中的分P数量
function getSeasonItemSelectedCount(bvid: string): number {
  return selectedSeasonEntries.value.get(bvid)?.length || 0;
}

// 判断合集项是否为半选状态（部分分P被选中）
function isSeasonItemIndeterminate(bvid: string): boolean {
  const entries = seasonItemEntries.value.get(bvid);
  if (!entries || entries.length <= 1) return false; // 没有分P或只有1个分P，不存在半选
  
  const selectedCount = selectedSeasonEntries.value.get(bvid)?.length || 0;
  return selectedCount > 0 && selectedCount < entries.length;
}

// 判断合集项是否全选（用于复选框状态）
function isSeasonItemFullySelected(bvid: string): boolean {
  const entries = seasonItemEntries.value.get(bvid);
  if (!entries || entries.length === 0) {
    // 没有加载分P信息，使用外层选择状态
    return selectedSeasonEpisodes.value.includes(bvid);
  }
  
  const selectedCount = selectedSeasonEntries.value.get(bvid)?.length || 0;
  return selectedCount === entries.length && selectedCount > 0;
}

// 下载合集中选中的视频（支持分P选择）
async function downloadSeason() {
  if (!videoInfo.value?.season) return;
  if (!outputDir.value) {
    ElMessage.warning('请选择下载目录');
    return;
  }
  if (selectedSeasonEpisodes.value.length === 0) {
    ElMessage.warning('请选择要下载的视频');
    return;
  }

  const season = videoInfo.value.season;
  const episodesToDownload = season.episodes.filter(
    ep => selectedSeasonEpisodes.value.includes(ep.bvid) && ep.bvid // 确保 bvid 不为空
  );
  
  if (episodesToDownload.length === 0) {
    ElMessage.error('选中的视频缺少有效的 BV 号');
    return;
  }
  
  // 合集下载：最终目录
  const finalDir = `${outputDir.value}\\${sanitizeFileName(season.title)}`;
  
  // 计算总任务数（考虑分P选择）
  let totalTasks = 0;
  const tasksToCreate: Array<{ep: SeasonEpisode, epIdx: number, entries?: VideoEntry[], selectedEntries?: number[]}> = [];
  
  for (const ep of episodesToDownload) {
    const epIdx = season.episodes.findIndex(e => e.bvid === ep.bvid) + 1; // 在合集中的真实序号
    const epEntries = seasonItemEntries.value.get(ep.bvid);
    const epSelectedEntries = selectedSeasonEntries.value.get(ep.bvid);
    
    if (epEntries && epEntries.length > 1 && epSelectedEntries && epSelectedEntries.length > 0) {
      // 有分P且有选择
      totalTasks += epSelectedEntries.length;
      tasksToCreate.push({ ep, epIdx, entries: epEntries, selectedEntries: epSelectedEntries });
    } else {
      // 无分P或未展开，作为单个任务
      totalTasks += 1;
      tasksToCreate.push({ ep, epIdx });
    }
  }
  
  // 如果多于1个任务，创建组任务
  let groupId: string | undefined;
  if (totalTasks > 1) {
    const groupTask = createGroupTask(
      season.title,
      videoInfo.value.thumbnail,
      totalTasks,
      finalDir
    );
    groupId = groupTask.id;
    groupTask.childIds = [];
  }
  
  // 使用时间戳基础，每个任务加唯一后缀
  const baseTimestamp = Date.now();
  let taskCounter = 0;
  
  for (const { ep, epIdx, entries, selectedEntries } of tasksToCreate) {
    if (entries && entries.length > 1 && selectedEntries && selectedEntries.length > 0) {
      // 有分P选择，为每个分P创建任务
      for (const entryIndex of selectedEntries) {
        const entry = entries.find(e => e.index === entryIndex);
        if (!entry) continue;
        
        taskCounter++;
        // 每个任务独立的临时目录
        const taskTempDir = `${outputDir.value}\\temp_${baseTimestamp}_${taskCounter}`;
        
        // 命名格式：合集序号-分P序号. 分P标题
        const taskTitle = totalTasks > 1 
          ? `${epIdx}-P${entryIndex}. ${entry.title}` 
          : `${season.title} - ${ep.title} - P${entryIndex}`;
        
        // entryTitle 用于最终文件名：合集序号-分P序号. 分P标题
        const fileTitle = `${String(epIdx).padStart(2, '0')}-P${String(entryIndex).padStart(2, '0')}. ${entry.title}`;
        
        const task = createDownloadTask(
          taskTitle,
          videoInfo.value.thumbnail,
          {
            url: `https://www.bilibili.com/video/${ep.bvid}?p=${entryIndex}`,
            outputDir: outputDir.value,
            tempDir: taskTempDir,
            finalDir: finalDir,
            quality: selectedQuality.value || null,
            videoTitle: season.title,
            isPlaylistItem: true,
            entryIndex: epIdx * 100 + entryIndex,
            entryTitle: fileTitle,
            expectedId: `${ep.bvid}_${entry.id?.split('_')[1] || entryIndex}`,
          },
          groupId
        );
        
        if (groupId) {
          const groupTask = downloadTasks.value.find(t => t.id === groupId);
          if (groupTask && groupTask.childIds) {
            groupTask.childIds.push(task.id);
          }
        }
      }
    } else {
      // 无分P或未展开，作为单个任务
      taskCounter++;
      // 每个任务独立的临时目录
      const taskTempDir = `${outputDir.value}\\temp_${baseTimestamp}_${taskCounter}`;
      
      // 命名格式：合集序号. 视频标题
      const taskTitle = totalTasks > 1 
        ? `${epIdx}. ${ep.title}` 
        : `${season.title} - ${epIdx}. ${ep.title}`;
      
      // entryTitle 用于最终文件名
      const fileTitle = `${String(epIdx).padStart(2, '0')}. ${ep.title}`;
      
      const task = createDownloadTask(
        taskTitle,
        videoInfo.value.thumbnail,
        {
          url: `https://www.bilibili.com/video/${ep.bvid}`,
          outputDir: outputDir.value,
          tempDir: taskTempDir,
          finalDir: finalDir,
          quality: selectedQuality.value || null,
          videoTitle: season.title,
          isPlaylistItem: true,
          entryIndex: epIdx,
          entryTitle: fileTitle,
          expectedId: ep.bvid,
        },
        groupId
      );
      
      if (groupId) {
        const groupTask = downloadTasks.value.find(t => t.id === groupId);
        if (groupTask && groupTask.childIds) {
          groupTask.childIds.push(task.id);
        }
      }
    }
  }
  
  // 批量创建后统一保存一次
  saveDownloadTasks();
  ElMessage.success(`已添加 ${totalTasks} 个下载任务`);
  
  // 开始处理队列
  processDownloadQueue();
}

function formatDuration(seconds: number | null): string {
  if (!seconds) return '00:00';
  const min = Math.floor(seconds / 60);
  const sec = Math.floor(seconds % 60);
  return `${min}:${sec.toString().padStart(2, '0')}`;
}

// ==================== 历史记录 ====================

async function loadHistory(refresh = false) {
  if (!userInfo.value) return;
  
  if (refresh) {
    historyCursor.value = 0;
    historyList.value = [];
  }
  
  historyLoading.value = true;
  
  try {
    const result = await invoke<ApiResponse<PagedData<HistoryItem>>>('get_history', {
      viewAt: historyCursor.value,
    });
    
    if (result.success && result.data) {
      historyList.value.push(...result.data.items);
      historyHasMore.value = result.data.has_more;
      if (result.data.cursor) {
        historyCursor.value = result.data.cursor;
      }
    }
  } catch (error) {
    ElMessage.error('获取历史记录失败');
  } finally {
    historyLoading.value = false;
  }
}

// ==================== 收藏夹 ====================

async function loadFavoriteFolders() {
  if (!userInfo.value) return;
  
  favoriteLoading.value = true;
  
  try {
    const result = await invoke<ApiResponse<FavoriteFolder[]>>('get_favorite_folders', {
      mid: userInfo.value.mid,
    });
    
    if (result.success && result.data) {
      favoriteFolders.value = result.data;
      if (result.data.length > 0 && !selectedFolder.value) {
        selectedFolder.value = result.data[0].id;
        await loadFavoriteContent(true);
      }
    }
  } catch (error) {
    ElMessage.error('获取收藏夹失败');
  } finally {
    favoriteLoading.value = false;
  }
}

async function loadFavoriteContent(refresh = false) {
  if (!selectedFolder.value) return;
  
  if (refresh) {
    favoritePage.value = 1;
    favoriteList.value = [];
  }
  
  favoriteLoading.value = true;
  
  try {
    const result = await invoke<ApiResponse<PagedData<FavoriteItem>>>('get_favorite_content', {
      folderId: selectedFolder.value,
      page: favoritePage.value,
    });
    
    if (result.success && result.data) {
      if (refresh) {
        favoriteList.value = result.data.items;
      } else {
        favoriteList.value.push(...result.data.items);
      }
      favoriteHasMore.value = result.data.has_more;
    }
  } catch (error) {
    ElMessage.error('获取收藏内容失败');
  } finally {
    favoriteLoading.value = false;
  }
}

async function onFolderChange(folderId: number) {
  selectedFolder.value = folderId;
  await loadFavoriteContent(true);
}

function selectFromList(bvid: string, cover?: string | null) {
  pendingCover.value = cover || null;
  parseVideo(bvid);
}

async function onTabChange(tab: string) {
  if (tab === 'history' && historyList.value.length === 0 && userInfo.value) {
    await loadHistory(true);
  } else if (tab === 'favorites' && favoriteFolders.value.length === 0 && userInfo.value) {
    await loadFavoriteFolders();
  }
}

async function loadMoreHistory() {
  await loadHistory(false);
}

async function loadMoreFavorites() {
  favoritePage.value++;
  await loadFavoriteContent(false);
}
</script>

<template>
  <div class="app-container">
    <!-- 顶部导航栏 -->
    <header class="app-header">
      <div class="header-left">
        <div class="logo">
          <svg viewBox="0 0 24 24" width="32" height="32" fill="currentColor">
            <path d="M17.813 4.653h.854c1.51.054 2.769.578 3.773 1.574 1.004.995 1.524 2.249 1.56 3.76v7.36c-.036 1.51-.556 2.769-1.56 3.773s-2.262 1.524-3.773 1.56H5.333c-1.51-.036-2.769-.556-3.773-1.56S.036 18.858 0 17.347v-7.36c.036-1.511.556-2.765 1.56-3.76 1.004-.996 2.262-1.52 3.773-1.574h.774l-1.174-1.12a1.234 1.234 0 0 1-.373-.906c0-.356.124-.659.373-.907l.027-.027c.267-.249.573-.373.92-.373.347 0 .653.124.92.373L9.653 4.44c.071.071.134.142.187.213h4.267a.836.836 0 0 1 .16-.213l2.853-2.747c.267-.249.573-.373.92-.373.347 0 .662.151.929.4.267.249.391.551.391.907 0 .355-.124.657-.373.906l-1.174 1.12zM5.333 7.24c-.746.018-1.373.276-1.88.773-.506.498-.769 1.13-.786 1.894v7.52c.017.764.28 1.395.786 1.893.507.498 1.134.756 1.88.773h13.334c.746-.017 1.373-.275 1.88-.773.506-.498.769-1.129.786-1.893v-7.52c-.017-.765-.28-1.396-.786-1.894-.507-.497-1.134-.755-1.88-.773H5.333zM8 11.107c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c0-.373.129-.689.386-.947.258-.257.574-.386.947-.386zm8 0c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c.017-.391.15-.711.4-.96.249-.249.56-.373.933-.373z"/>
          </svg>
          <span class="logo-text">哔哩哔哩下载器</span>
        </div>
      </div>
      <div class="header-right">
        <!-- 设置按钮 -->
        <el-button class="header-btn" size="small" @click="showSettings = true">
          <el-icon><Setting /></el-icon>
          <span>设置</span>
        </el-button>
        
        <!-- 主题切换 -->
        <el-button class="header-btn" size="small" @click="toggleTheme">
          <el-icon><Moon v-if="!isDarkMode" /><Sunny v-else /></el-icon>
          <span>{{ isDarkMode ? '浅色' : '深色' }}</span>
        </el-button>
        
        <!-- 下载中心按钮 -->
        <el-badge :value="getDownloadingCount()" :hidden="getDownloadingCount() === 0" class="download-badge">
          <el-button class="header-btn" size="small" @click="showDownloadCenter = true">
            <el-icon><Download /></el-icon>
            <span>下载</span>
          </el-button>
        </el-badge>
        
        <template v-if="userInfo">
          <el-dropdown trigger="click">
            <div class="user-info">
              <el-avatar :src="userInfo.face" :size="32" />
              <span class="username">{{ userInfo.username }}</span>
            </div>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="handleLogout">
                  <el-icon><Close /></el-icon>
                  退出登录
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </template>
        <template v-else>
          <el-button class="login-btn" size="small" @click="openLoginDialog">
            <el-icon><User /></el-icon>
            登录
          </el-button>
        </template>
      </div>
    </header>

    <!-- 主内容区 - 双栏布局 -->
    <main class="app-main" :class="{ 'is-dragging': isDragging }">
      <!-- 左侧列表区 -->
      <div class="left-panel" :style="{ width: leftPanelWidth + '%' }">
        <!-- Tab 导航 -->
      <div class="tab-nav">
        <div 
          class="tab-item" 
          :class="{ active: activeTab === 'search' }"
          @click="activeTab = 'search'"
        >
          <el-icon><Search /></el-icon>
          <span>搜索</span>
        </div>
        <div 
          class="tab-item" 
          :class="{ active: activeTab === 'link' }"
          @click="activeTab = 'link'"
        >
          <el-icon><Link /></el-icon>
          <span>链接</span>
        </div>
        <div 
          class="tab-item" 
          :class="{ active: activeTab === 'history' }"
          @click="activeTab = 'history'; onTabChange('history')"
        >
          <el-icon><Clock /></el-icon>
          <span>历史</span>
        </div>
        <div 
          class="tab-item" 
          :class="{ active: activeTab === 'favorites' }"
          @click="activeTab = 'favorites'; onTabChange('favorites')"
        >
          <el-icon><Star /></el-icon>
          <span>收藏</span>
        </div>
      </div>

      <!-- Tab 内容 -->
      <div class="tab-content">
<!-- 搜索 -->
        <div v-show="activeTab === 'search'" class="tab-pane">
          <div class="search-box">
            <el-input
              v-model="searchKeyword"
              placeholder="搜索视频、番剧..."
              size="large"
              clearable
              @keyup.enter="doSearch(true)"
            >
              <template #prefix>
                <el-icon><Search /></el-icon>
              </template>
            </el-input>
            <el-button 
              type="primary" 
              size="large" 
              :loading="searchLoading"
              @click="doSearch(true)"
            >
              搜索
            </el-button>
          </div>
          <div class="search-types">
            <span 
              v-for="t in searchTypes" 
              :key="t.value"
              class="type-tag"
              :class="{ active: searchType === t.value }"
              @click="searchType = t.value; searchResults.length && doSearch(true)"
            >
              {{ t.label }}
            </span>
          </div>
          
          <SearchResultPanel
            :keyword="searchKeyword"
            :results="searchResults"
            :total="searchTotal"
            :loading="searchLoading"
            :has-more="searchHasMore"
            :search-type="searchType"
            @select="selectFromList"
            @load-more="loadMoreSearch"
          />
        </div>

        <!-- 链接输入 -->
        <div v-show="activeTab === 'link'" class="tab-pane">
          <div class="search-box">
            <el-input
              v-model="videoUrl"
              placeholder="输入视频链接或BV号"
              size="large"
              clearable
              @keyup.enter="parseVideo()"
            >
              <template #prefix>
                <el-icon><VideoPlay /></el-icon>
              </template>
            </el-input>
            <el-button 
              type="primary" 
              size="large" 
              :loading="loading"
              @click="parseVideo()"
            >
              解析
            </el-button>
          </div>
          <div class="link-tips">
            <p>支持的链接格式：</p>
            <ul>
              <li>BV号：BV1xx411x7xx</li>
              <li>视频链接：bilibili.com/video/BVxxx</li>
              <li>番剧链接：bilibili.com/bangumi/play/epxxx</li>
            </ul>
          </div>
        </div>

        <!-- 历史记录 -->
        <div v-show="activeTab === 'history'" class="tab-pane">
          <HistoryPanel
            :history-list="historyList"
            :loading="historyLoading"
            :has-more="historyHasMore"
            @select="selectFromList"
            @load-more="loadMoreHistory"
            @login="openLoginDialog"
          />
        </div>

        <!-- 收藏夹 -->
        <div v-show="activeTab === 'favorites'" class="tab-pane">
          <FavoritesPanel
            :folders="favoriteFolders"
            :selected-folder="selectedFolder"
            :favorite-list="favoriteList"
            :loading="favoriteLoading"
            :has-more="favoriteHasMore"
            @select="selectFromList"
            @folder-change="onFolderChange"
            @load-more="loadMoreFavorites"
            @login="openLoginDialog"
          />
        </div>
      </div>
      </div>

      <!-- 可拖动分隔条 -->
      <div class="panel-divider" @mousedown="onDividerDragStart"></div>

      <!-- 右侧详情区 -->
      <div class="right-panel" :style="{ width: (100 - leftPanelWidth) + '%' }">
        <VideoDetailPanel
          :video-info="videoInfo"
          :loading="loading"
          :output-dir="outputDir"
          :selected-quality="selectedQuality"
          :selected-entries="selectedEntries"
          :selected-season-episodes="selectedSeasonEpisodes"
          :expanded-season-items="expandedSeasonItems"
          :season-item-entries="seasonItemEntries"
          :season-item-loading="seasonItemLoading"
          :selected-season-entries="selectedSeasonEntries"
          :desc-expanded="descExpanded"
          :is-current-video="isCurrentVideo"
          :is-season-item-indeterminate="isSeasonItemIndeterminate"
          :is-season-item-fully-selected="isSeasonItemFullySelected"
          :get-season-item-selected-count="getSeasonItemSelectedCount"
          :format-duration="formatDuration"
          @update:selected-quality="selectedQuality = $event"
          @update:selected-entries="selectedEntries = $event"
          @update:desc-expanded="descExpanded = $event"
          @toggle-select-all="toggleSelectAll"
          @toggle-season-select-all="toggleSeasonSelectAll"
          @toggle-season-episode-select="toggleSeasonEpisodeSelect"
          @toggle-season-item-expand="toggleSeasonItemExpand"
          @toggle-season-item-select-all="toggleSeasonItemSelectAll"
          @toggle-season-entry-select="toggleSeasonEntrySelect"
          @select-from-list="selectFromList"
          @select-output-dir="selectOutputDir"
          @download-season="downloadSeason"
          @start-download="startDownload"
        />
      </div>
    </main>

    <!-- 登录弹窗 -->
    <LoginDialog @login-success="onLoginSuccess" />

    <!-- 下载中心抽屉 -->
    <el-drawer
      v-model="showDownloadCenter"
      title="下载中心"
      direction="rtl"
      size="420px"
    >
      <DownloadCenter
        :tasks="downloadTasks"
        :is-select-mode="isSelectMode"
        :selected-task-ids="selectedTaskIds"
        :expanded-group-ids="expandedGroupIds"
        @pause-task="pauseDownload"
        @resume-task="resumeDownload"
        @delete-task="deleteTask"
        @open-folder="openTaskFolder"
        @pause-group="pauseGroupDownload"
        @resume-group="resumeGroupDownload"
        @delete-group="deleteGroupTask"
        @retry-failed-children="retryFailedChildren"
        @retry-child="retryChildTask"
        @delete-child="deleteChildTask"
        @toggle-group-expand="toggleGroupExpand"
        @toggle-select-mode="toggleSelectMode"
        @toggle-task-select="toggleTaskSelect"
        @toggle-select-all="toggleSelectAllActive"
        @confirm-delete-selected="confirmDeleteSelected"
        @pause-all="pauseAllTasks"
        @resume-all="resumeAllTasks"
        @clear-completed="clearCompletedTasks"
        @clear-failed="clearFailedTasks"
      />
    </el-drawer>

    <!-- 设置抽屉 -->
    <el-drawer
      v-model="showSettings"
      title="设置"
      direction="rtl"
      size="400px"
    >
      <SettingsPanel />
    </el-drawer>
    
    <!-- 更新提示对话框 -->
    <UpdateDialog v-model="appStore.showUpdateDialog" />
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
}

/* 滚动条美化 */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: transparent;
  border-radius: 4px;
}

::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 4px;
  transition: background 0.3s;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--text-muted);
}

/* 暗黑模式下的滚动条 */
html.dark ::-webkit-scrollbar-thumb {
  background: #4a4d50;
}

html.dark ::-webkit-scrollbar-thumb:hover {
  background: #5a5d60;
}

/* 火狐滚动条 */
* {
  scrollbar-width: thin;
  scrollbar-color: var(--border-color) transparent;
}

html.dark * {
  scrollbar-color: #4a4d50 transparent;
}

/* Element Plus 主题覆盖 - B站粉色 */
:root {
  --el-color-primary: #fb7299;
  --el-color-primary-light-3: #fc8bab;
  --el-color-primary-light-5: #fda4bd;
  --el-color-primary-light-7: #febdcf;
  --el-color-primary-light-9: #fee8ef;
  --el-color-primary-dark-2: #c95b7a;
  --bili-pink: #fb7299;
  
  /* 统一色系 - 只使用粉黑白 */
  --el-color-success: #fb7299;
  --el-color-success-light-3: #fc8bab;
  --el-color-success-light-5: #fda4bd;
  --el-color-success-light-9: #fee8ef;
  --el-color-warning: #fb7299;
  --el-color-warning-light-3: #fc8bab;
  --el-color-warning-light-5: #fda4bd;
  --el-color-warning-light-9: #fee8ef;
  --el-color-danger: #fb7299;
  --el-color-danger-light-3: #fc8bab;
  --el-color-danger-light-5: #fda4bd;
  --el-color-danger-light-9: #fee8ef;
  --el-color-info: #909399;
  --el-color-info-light-3: #a6a9ad;
  --el-color-info-light-5: #bcbec2;
  --el-color-info-light-9: #e9e9eb;
  
  /* 主题变量 - 亮色 */
  --bg-primary: #f4f5f7;
  --bg-secondary: #f8f8f8;
  --bg-card: #fff;
  --bg-hover: #f4f5f7;
  --bg-input: #f4f5f7;
  --text-primary: #18191c;
  --text-secondary: #61666d;
  --text-muted: #9499a0;
  --border-color: #e7e7e7;
  --shadow-color: rgba(0,0,0,0.06);
}

/* 暗黑主题 */
html.dark {
  --bg-primary: #18191c;
  --bg-secondary: #1f2022;
  --bg-card: #242628;
  --bg-hover: #2d2f31;
  --bg-input: #2d2f31;
  --text-primary: #e3e5e7;
  --text-secondary: #a2a7ae;
  --text-muted: #757a82;
  --border-color: #3c3f41;
  --shadow-color: rgba(0,0,0,0.3);
  
  /* 暗黑模式下的统一色系 */
  --el-color-primary-light-9: rgba(251, 114, 153, 0.15);
  --el-color-success-light-9: rgba(251, 114, 153, 0.15);
  --el-color-warning-light-9: rgba(251, 114, 153, 0.15);
  --el-color-danger-light-9: rgba(251, 114, 153, 0.15);
  --el-color-info-light-9: rgba(144, 147, 153, 0.15);
}

html.dark .el-input__wrapper,
html.dark .el-select__wrapper {
  background-color: var(--bg-input);
  box-shadow: 0 0 0 1px var(--border-color) inset;
}

html.dark .el-input__inner,
html.dark .el-select__placeholder {
  color: var(--text-primary);
}

html.dark .el-drawer {
  background-color: var(--bg-card);
}

html.dark .el-drawer__header {
  color: var(--text-primary);
}

html.dark .el-dialog {
  background-color: var(--bg-card);
}

html.dark .el-message-box {
  background-color: var(--bg-card);
}

html.dark .el-message-box__title {
  color: var(--text-primary);
}

html.dark .el-message-box__content {
  color: var(--text-secondary);
}

html.dark .el-dropdown-menu {
  background-color: var(--bg-card);
  border-color: var(--border-color);
}

html.dark .el-dropdown-menu__item {
  color: var(--text-primary);
}

html.dark .el-dropdown-menu__item:hover {
  background-color: var(--bg-hover);
}

html.dark .el-skeleton__item {
  background: var(--bg-hover);
}

html.dark .el-tabs__item {
  color: var(--text-secondary);
}

html.dark .el-tabs__item.is-active {
  color: #fb7299;
}

html.dark .el-button--default {
  background-color: var(--bg-input);
  border-color: var(--border-color);
  color: var(--text-primary);
}

html.dark .el-button--default:hover {
  color: #fb7299;
  border-color: #fb7299;
  background-color: var(--bg-hover);
}

html.dark .el-radio-button__inner {
  background-color: var(--bg-input);
  border-color: var(--border-color);
  color: var(--text-secondary);
}

.el-button--primary {
  --el-button-bg-color: #fb7299;
  --el-button-border-color: #fb7299;
  --el-button-hover-bg-color: #fc8bab;
  --el-button-hover-border-color: #fc8bab;
  --el-button-active-bg-color: #e9678a;
  --el-button-active-border-color: #e9678a;
  background-color: #fb7299 !important;
  border-color: #fb7299 !important;
  color: #fff !important;
}

.el-button--primary:hover,
.el-button--primary:focus {
  background-color: #fc8bab !important;
  border-color: #fc8bab !important;
  color: #fff !important;
}

.el-button--primary:active {
  background-color: #e9678a !important;
  border-color: #e9678a !important;
}

.el-button--primary.is-loading {
  background-color: #fc8bab !important;
  border-color: #fc8bab !important;
}

.el-radio-button__original-radio:checked + .el-radio-button__inner {
  background-color: #fb7299;
  border-color: #fb7299;
}

.el-progress-bar__inner {
  background-color: #fb7299;
}

.el-checkbox__input.is-checked .el-checkbox__inner {
  background-color: #fb7299;
  border-color: #fb7299;
}

/* 统一消息提示颜色 */
.el-message--success {
  --el-message-bg-color: #fee8ef;
  --el-message-border-color: #fda4bd;
  --el-message-text-color: #c95b7a;
}

.el-message--success .el-message__icon {
  color: #fb7299;
}

.el-message--warning {
  --el-message-bg-color: #fee8ef;
  --el-message-border-color: #fda4bd;
  --el-message-text-color: #c95b7a;
}

.el-message--warning .el-message__icon {
  color: #fb7299;
}

.el-message--error {
  --el-message-bg-color: #2d2d2d;
  --el-message-border-color: #4a4a4a;
  --el-message-text-color: #e3e5e7;
}

html.dark .el-message--success,
html.dark .el-message--warning {
  --el-message-bg-color: rgba(251, 114, 153, 0.15);
  --el-message-border-color: rgba(251, 114, 153, 0.3);
  --el-message-text-color: #fc8bab;
}

html.dark .el-message--error {
  --el-message-bg-color: rgba(50, 50, 50, 0.95);
  --el-message-border-color: rgba(80, 80, 80, 0.5);
  --el-message-text-color: #e3e5e7;
}

/* 统一消息框颜色 */
.el-message-box__btns .el-button--primary {
  background-color: #fb7299;
  border-color: #fb7299;
}

.el-message-box__btns .el-button--primary:hover {
  background-color: #fc8bab;
  border-color: #fc8bab;
}

/* 统一对话框样式 */
.el-dialog {
  border-radius: 12px;
  overflow: hidden;
}

.el-dialog__header {
  padding: 20px;
  border-bottom: 1px solid var(--border-color);
}

.el-dialog__title {
  font-weight: 600;
}

/* 输入框聚焦颜色 */
.el-input__wrapper:focus-within {
  box-shadow: 0 0 0 1px #fb7299 inset !important;
}

.el-select__wrapper:focus-within {
  box-shadow: 0 0 0 1px #fb7299 inset !important;
}

/* 开关按钮颜色 */
.el-switch.is-checked .el-switch__core {
  background-color: #fb7299;
  border-color: #fb7299;
}

/* 加载动画颜色 */
.el-loading-spinner .circular {
  stroke: #fb7299;
}

.el-loading-spinner .el-loading-text {
  color: #fb7299;
}
</style>

<style scoped>
.app-container {
  height: 100vh;
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: background 0.3s;
}

/* Header */
.app-header {
  background: var(--bg-card);
  height: 56px;
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-shadow: 0 1px 3px var(--shadow-color);
  flex-shrink: 0;
  z-index: 100;
  transition: background 0.3s;
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #fb7299;
}

.logo-text {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  transition: color 0.3s;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-btn {
  background: transparent;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  height: 32px;
  border-radius: 6px;
}

.header-btn:hover {
  color: #fb7299;
  border-color: #fb7299;
  background: rgba(251, 114, 153, 0.05);
}

.header-btn .el-icon {
  font-size: 16px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: background 0.2s;
}

.user-info:hover {
  background: var(--bg-hover);
}

.username {
  font-size: 13px;
  color: var(--text-primary);
}

.login-btn {
  background: #fb7299 !important;
  border-color: #fb7299 !important;
  color: #fff !important;
  height: 32px;
  padding: 6px 12px;
  border-radius: 6px;
}

.login-btn:hover {
  background: #fc8bab !important;
  border-color: #fc8bab !important;
}

/* Main */
.app-main {
  flex: 1;
  display: flex;
  gap: 0;
  width: 100%;
  padding: 16px;
  min-height: 0;
  overflow: hidden;
}

.app-main.is-dragging {
  user-select: none;
  cursor: col-resize;
}

/* 左侧面板 */
.left-panel {
  min-width: 280px;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  padding-right: 8px;
}

/* 可拖动分隔条 */
.panel-divider {
  width: 6px;
  cursor: col-resize;
  background: transparent;
  position: relative;
  flex-shrink: 0;
}

.panel-divider::before {
  content: '';
  position: absolute;
  left: 2px;
  top: 50%;
  transform: translateY(-50%);
  width: 2px;
  height: 40px;
  background: var(--border-color);
  border-radius: 1px;
  transition: background 0.2s, height 0.2s;
}

.panel-divider:hover::before {
  background: #fb7299;
  height: 60px;
}

/* 右侧面板 */
.right-panel {
  min-width: 300px;
  height: 100%;
  display: flex;
  flex-direction: column;
  padding-left: 8px;
}

/* Tab Nav */
.tab-nav {
  display: flex;
  background: var(--bg-card);
  border-radius: 10px;
  padding: 4px;
  margin-bottom: 12px;
  box-shadow: 0 1px 3px var(--shadow-color);
  flex-shrink: 0;
  transition: background 0.3s;
}

.tab-item {
  flex: 1;
  padding: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  cursor: pointer;
  color: var(--text-secondary);
  font-size: 13px;
  border-radius: 8px;
  transition: all 0.2s;
}

.tab-item:hover {
  color: #fb7299;
}

.tab-item.active {
  color: #fff;
  background: #fb7299;
}

/* Tab Content */
.tab-content {
  background: var(--bg-card);
  border-radius: 12px;
  padding: 16px;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  transition: background 0.3s;
}

.tab-pane {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.search-results,
.video-list-wrapper {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

/* Search Box */
.search-box {
  display: flex;
  gap: 10px;
  margin-bottom: 16px;
}

.search-box .el-input {
  flex: 1;
}

.search-types {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.type-tag {
  padding: 6px 16px;
  font-size: 13px;
  color: var(--text-secondary);
  background: var(--bg-hover);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.type-tag:hover {
  color: #fb7299;
}

.type-tag.active {
  color: #fff;
  background: #fb7299;
}

.result-header {
  font-size: 13px;
  color: var(--text-muted);
  margin-bottom: 12px;
}

/* Link Tips */
.link-tips {
  color: var(--text-muted);
  font-size: 13px;
  padding: 20px;
  background: var(--bg-hover);
  border-radius: 8px;
}

.link-tips p {
  margin-bottom: 8px;
  font-weight: 500;
  color: var(--text-secondary);
}

.link-tips ul {
  padding-left: 20px;
}

.link-tips li {
  margin: 6px 0;
}

/* Video List */
.video-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.video-item {
  display: flex;
  gap: 12px;
  padding: 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.2s;
}

.video-item:hover {
  background: var(--bg-hover);
}

.video-cover {
  width: 160px;
  height: 90px;
  border-radius: 6px;
  overflow: hidden;
  position: relative;
  flex-shrink: 0;
  background: var(--bg-hover);
}

.video-cover .el-image {
  width: 100%;
  height: 100%;
}

.duration-tag {
  position: absolute;
  bottom: 4px;
  right: 4px;
  background: rgba(0,0,0,0.75);
  color: #fff;
  font-size: 11px;
  padding: 2px 5px;
  border-radius: 3px;
}

.watch-progress {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: rgba(255,255,255,0.3);
}

.progress-inner {
  height: 100%;
  background: #fb7299;
}

.video-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.video-title {
  font-size: 14px;
  color: var(--text-primary);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  margin-bottom: 6px;
}

.video-info-row {
  font-size: 12px;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 12px;
}

.stats {
  display: flex;
  align-items: center;
  gap: 2px;
}

/* States */
.not-login, .empty-state, .loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: var(--text-muted);
  gap: 12px;
}

.load-more {
  text-align: center;
  padding: 12px;
}

/* 番剧/影视栅格布局 */
.media-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 12px;
}

.media-card {
  cursor: pointer;
  transition: transform 0.2s;
}

.media-card:hover {
  transform: translateY(-4px);
}

.media-cover {
  width: 100%;
  aspect-ratio: 3 / 4;
  border-radius: 8px;
  overflow: hidden;
  background: var(--bg-hover);
  margin-bottom: 8px;
}

.media-cover .el-image {
  width: 100%;
  height: 100%;
}

.media-title {
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-align: center;
}

.folder-select {
  width: 100%;
  margin-bottom: 12px;
}

/* 下载中心按钮 */
.download-badge {
  margin-right: 12px;
}

/* 设置面板 */
.settings-panel {
  padding: 0 4px;
}

.settings-section {
  margin-bottom: 24px;
}

.settings-section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 16px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.settings-item {
  margin-bottom: 16px;
}

.settings-label {
  font-size: 13px;
  color: var(--text-primary);
  margin-bottom: 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.settings-hint {
  font-size: 11px;
  color: var(--text-secondary);
  font-weight: normal;
}

.settings-input-group {
  display: flex;
  gap: 8px;
  align-items: center;
}

.settings-input-group .el-input {
  flex: 1;
}

.settings-about {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.8;
}

.settings-about-desc {
  font-size: 12px;
  opacity: 0.7;
}

.settings-about-link {
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.settings-about-link a,
.settings-about-link .github-link {
  color: var(--el-color-primary);
  text-decoration: none;
  cursor: pointer;
}

.settings-about-link a:hover,
.settings-about-link .github-link:hover {
  text-decoration: underline;
}

.settings-about-link .link-divider {
  color: var(--text-secondary);
  opacity: 0.5;
}

.settings-update {
  margin-top: 12px;
}

/* 更新对话框 */
.update-dialog .el-dialog__header {
  padding: 0;
  margin: 0;
}

.update-dialog .el-dialog__body {
  padding: 0 24px 20px;
}

.update-dialog .el-dialog__footer {
  padding: 0 24px 24px;
}

.update-dialog-header {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 28px 24px 20px;
  background: linear-gradient(135deg, #fb7299 0%, #fc8bab 100%);
  border-radius: 8px 8px 0 0;
  margin: -20px -20px 0;
}

.update-dialog-header .update-icon {
  font-size: 40px;
  margin-bottom: 8px;
  animation: bounce 1s ease infinite;
}

@keyframes bounce {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-8px); }
}

.update-dialog-header .update-title {
  font-size: 20px;
  font-weight: 600;
  color: #fff;
}

.update-dialog-content {
  padding: 24px 0 0;
}

.update-version-box {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 20px;
  margin-bottom: 24px;
}

.version-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px 24px;
  border-radius: 12px;
  min-width: 120px;
}

.version-item.current {
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
}

.version-item.new {
  background: linear-gradient(135deg, rgba(251, 114, 153, 0.1) 0%, rgba(252, 139, 171, 0.1) 100%);
  border: 1px solid #fb7299;
}

.version-label {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.version-item.current .version-num {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-secondary);
}

.version-item.new .version-num {
  font-size: 18px;
  font-weight: 600;
  color: #fb7299;
}

.version-arrow {
  color: #fb7299;
  display: flex;
  align-items: center;
}

.update-notes {
  background: var(--bg-secondary);
  border-radius: 12px;
  padding: 16px;
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid var(--border-color);
}

.update-notes-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  margin-bottom: 12px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border-color);
}

.update-notes-header svg {
  color: #fb7299;
}

/* 简洁的更新列表样式 */
.update-notes-list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.update-notes-list li {
  position: relative;
  padding: 8px 0 8px 20px;
  font-size: 13px;
  color: var(--text-secondary);
  border-bottom: 1px dashed var(--border-color);
}

.update-notes-list li:last-child {
  border-bottom: none;
}

.update-notes-list li::before {
  content: '✓';
  position: absolute;
  left: 0;
  color: #fb7299;
  font-weight: bold;
}

.update-progress {
  margin-top: 24px;
  padding: 20px;
  background: var(--bg-secondary);
  border-radius: 12px;
  border: 1px solid var(--border-color);
}

.update-progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.update-progress-text {
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
}

.update-progress-percent {
  font-size: 14px;
  color: #fb7299;
  font-weight: 600;
}

.update-progress-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 12px;
}

.update-progress-hint svg {
  color: var(--text-muted);
}

.update-dialog-footer {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
}

.update-dialog-footer .btn-later {
  background: var(--bg-secondary);
  border-color: var(--border-color);
  color: var(--text-secondary);
  padding: 10px 20px;
  border-radius: 8px;
}

.update-dialog-footer .btn-later:hover {
  background: var(--bg-hover);
  border-color: var(--border-color);
  color: var(--text-primary);
}

.update-dialog-footer .btn-update {
  background: linear-gradient(135deg, #fb7299 0%, #fc8bab 100%);
  border: none;
  color: #fff;
  padding: 10px 24px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  font-weight: 500;
}

.update-dialog-footer .btn-update:hover {
  background: linear-gradient(135deg, #e8688a 0%, #fb7299 100%);
}

.update-dialog-footer .btn-downloading {
  background: var(--bg-secondary);
  border-color: var(--border-color);
  color: var(--text-muted);
  padding: 10px 24px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.downloading-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid var(--border-color);
  border-top-color: #fb7299;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>