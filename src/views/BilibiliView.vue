<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from '@tauri-apps/plugin-dialog';
import { ElMessage } from 'element-plus';
import { 
  Search, VideoPlay, Clock, Star, Link
} from '@element-plus/icons-vue';

// 引入组件
import { DownloadCenter, LoginDialog } from '@/components';
import { HistoryPanel, FavoritesPanel, SearchResultPanel, VideoDetailPanel } from '@/components/bilibili';
import { useUserStore } from '@/stores';
import { useDownloadTasks } from '@/composables';

// 引入类型
import type { 
  VideoInfo, VideoEntry, UserInfo, HistoryItem, FavoriteFolder, FavoriteItem,
  SearchResultItem, SearchResult, PagedData, ApiResponse, ProgressDetail,
  UserSettings, SeasonEpisode
} from '@/types';
import { defaultSettings } from '@/types';

// 初始化 store
const userStore = useUserStore();

// ==================== 用户设置 ====================

const settings = ref<UserSettings>({
  ...defaultSettings
});

// ==================== 下载任务管理 (使用 composable) ====================

const {
  downloadTasks,
  downloadProgress,
  isSelectMode,
  selectedTaskIds,
  expandedGroupIds,
  toggleGroupExpand,
  getDownloadingCount,
  loadDownloadTasks,
  saveDownloadTasks,
  createDownloadTask,
  createGroupTask,
  handleProgressUpdate,
  processDownloadQueue,
  pauseDownload,
  resumeDownload,
  deleteTask,
  openTaskFolder,
  pauseGroupDownload,
  resumeGroupDownload,
  deleteGroupTask,
  retryChildTask,
  deleteChildTask,
  retryFailedChildren,
  clearCompletedTasks,
  clearFailedTasks,
  toggleSelectMode,
  toggleTaskSelect,
  toggleSelectAllActive,
  confirmDeleteSelected,
  pauseAllTasks,
  resumeAllTasks,
} = useDownloadTasks(settings);

// ==================== 状态 ====================

const searchKeyword = ref('');
const videoUrl = ref('');
const videoInfo = ref<VideoInfo | null>(null);
const selectedQuality = ref('');
const outputDir = ref('');
const loading = ref(false);

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

const searchTypes = [
  { label: '视频', value: 'video' as const },
  { label: '番剧', value: 'media_bangumi' as const },
  { label: '影视', value: 'media_ft' as const },
];

// ==================== 生命周期 ====================

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

// 监听事件的清理函数
let unlistenProgress: (() => void) | null = null;
let unlistenDetailProgress: (() => void) | null = null;

onMounted(async () => {
  loadSettings(); // 加载用户设置
  
  // 从 store 恢复用户信息
  if (userStore.userInfo) {
    userInfo.value = userStore.userInfo as UserInfo;
  } else {
    await checkLoginStatus();
  }
  
  await loadDownloadTasks(); // 加载下载任务历史
  
  // 监听简单进度
  const unlisten1 = await listen<number>('download-progress', (event) => {
    downloadProgress.value = event.payload;
  });
  unlistenProgress = unlisten1;
  
  // 监听详细进度（支持多任务）
  const unlisten2 = await listen<ProgressDetail>('download-progress-detail', (event) => {
    handleProgressUpdate(event.payload);
  });
  unlistenDetailProgress = unlisten2;
});

onUnmounted(() => {
  // 移除拖动事件监听
  document.removeEventListener('mousemove', onDividerDrag);
  document.removeEventListener('mouseup', onDividerDragEnd);
  
  // 移除进度监听
  if (unlistenProgress) unlistenProgress();
  if (unlistenDetailProgress) unlistenDetailProgress();
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
  const container = document.querySelector('.bilibili-main') as HTMLElement;
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
      userStore.setUserInfo(result.data as any);
    }
  } catch (error) {
    console.error('检查登录状态失败', error);
  }
}

function openLoginDialog() {
  userStore.openLoginDialog();
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

// ==================== 下载功能 ====================

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

// ==================== 暴露给父组件 ====================

// 暴露下载中心相关状态和方法
defineExpose({
  showDownloadCenter,
  getDownloadingCount,
  downloadTasks,
  isSelectMode,
  selectedTaskIds,
  expandedGroupIds,
  pauseDownload,
  resumeDownload,
  deleteTask,
  openTaskFolder,
  pauseGroupDownload,
  resumeGroupDownload,
  deleteGroupTask,
  retryChildTask,
  deleteChildTask,
  retryFailedChildren,
  toggleGroupExpand,
  toggleSelectMode,
  toggleTaskSelect,
  toggleSelectAllActive,
  confirmDeleteSelected,
  pauseAllTasks,
  resumeAllTasks,
  clearCompletedTasks,
  clearFailedTasks,
});
</script>

<template>
  <div class="bilibili-view">
    <!-- 主内容区 - 双栏布局 -->
    <main class="bilibili-main" :class="{ 'is-dragging': isDragging }">
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
  </div>
</template>

<style scoped>
.bilibili-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-primary);
}

.bilibili-main {
  flex: 1;
  display: flex;
  min-height: 0;
  position: relative;
}

.bilibili-main.is-dragging {
  user-select: none;
}

.left-panel {
  display: flex;
  flex-direction: column;
  min-width: 300px;
  background: var(--bg-primary);
}

.right-panel {
  display: flex;
  flex-direction: column;
  min-width: 300px;
  overflow: hidden;
}

.panel-divider {
  width: 4px;
  background: var(--border-color);
  cursor: col-resize;
  flex-shrink: 0;
  transition: background-color 0.2s;
}

.panel-divider:hover {
  background: var(--primary-color);
}

/* Tab 导航 */
.tab-nav {
  display: flex;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-card);
  padding: 0 8px;
}

.tab-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 12px 16px;
  cursor: pointer;
  color: var(--text-secondary);
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
  font-size: 14px;
}

.tab-item:hover {
  color: var(--text-primary);
}

.tab-item.active {
  color: var(--primary-color);
  border-bottom-color: var(--primary-color);
}

.tab-item .el-icon {
  font-size: 16px;
}

/* Tab 内容 */
.tab-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.tab-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 16px;
  overflow: hidden;
}

/* 搜索框 */
.search-box {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.search-box .el-input {
  flex: 1;
}

.search-types {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.type-tag {
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
  color: var(--text-secondary);
  background: var(--bg-hover);
  transition: all 0.2s;
}

.type-tag:hover {
  color: var(--primary-color);
}

.type-tag.active {
  color: #fff;
  background: var(--primary-color);
}

/* 链接输入提示 */
.link-tips {
  margin-top: 16px;
  padding: 16px;
  background: var(--bg-hover);
  border-radius: 8px;
  color: var(--text-secondary);
  font-size: 13px;
}

.link-tips p {
  margin-bottom: 8px;
  font-weight: 500;
}

.link-tips ul {
  margin: 0;
  padding-left: 20px;
}

.link-tips li {
  margin: 4px 0;
}
</style>
