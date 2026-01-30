import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { DownloadTask, VideoEntry } from '@/types';

export const useDownloadStore = defineStore('download', () => {
  // ==================== 下载任务 ====================
  const downloadTasks = ref<DownloadTask[]>([]);
  const currentTaskId = ref<string | null>(null);
  const downloading = ref(false);
  const downloadProgress = ref(0);
  const downloadStage = ref('视频');
  const downloadSpeed = ref('');
  const isPausing = ref(false);
  const isProcessingQueue = ref(false);

  // ==================== 选择状态 ====================
  const isSelectMode = ref(false);
  const selectedTaskIds = ref<string[]>([]);
  const expandedGroupIds = ref<string[]>([]);

  // ==================== 视频分P选择 ====================
  const selectedEntries = ref<number[]>([]);
  const selectedSeasonEpisodes = ref<string[]>([]);
  const expandedSeasonItems = ref<Set<string>>(new Set());
  const seasonItemEntries = ref<Map<string, VideoEntry[]>>(new Map());
  const seasonItemLoading = ref<Set<string>>(new Set());
  const selectedSeasonEntries = ref<Map<string, number[]>>(new Map());

  // ==================== 计算属性 ====================
  
  // 顶级任务（不包含子任务）
  const topLevelTasks = computed(() => 
    downloadTasks.value.filter(t => !t.groupId)
  );

  // 活动任务（下载中 + 等待中 + 暂停）
  const activeTasks = computed(() => 
    topLevelTasks.value.filter(t => 
      t.status === 'downloading' || 
      t.status === 'waiting' || 
      t.status === 'paused'
    )
  );

  // 已完成任务
  const completedTasks = computed(() => 
    topLevelTasks.value.filter(t => t.status === 'completed')
  );

  // 失败任务
  const failedTasks = computed(() => 
    topLevelTasks.value.filter(t => t.status === 'failed')
  );

  // 活动任务数（用于徽章显示）
  const activeTaskCount = computed(() => activeTasks.value.length);

  // ==================== 方法 ====================

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

  // 创建下载任务
  function createDownloadTask(
    title: string,
    cover: string | null,
    downloadInfo: DownloadTask['downloadInfo'],
    groupId?: string
  ): DownloadTask {
    const task: DownloadTask = {
      id: `task_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      title,
      cover,
      status: 'waiting',
      progress: 0,
      stage: '等待中',
      createdAt: Date.now(),
      downloadInfo,
      groupId,
    };
    downloadTasks.value.push(task);
    saveDownloadTasks();
    return task;
  }

  // 创建组任务
  function createGroupTask(
    title: string,
    cover: string | null,
    totalCount: number,
    finalDir: string
  ): DownloadTask {
    const task: DownloadTask = {
      id: `group_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      title,
      cover,
      status: 'waiting',
      progress: 0,
      stage: '等待中',
      createdAt: Date.now(),
      isGroup: true,
      childIds: [],
      totalCount,
      completedCount: 0,
      failedCount: 0,
      downloadInfo: {
        url: '',
        outputDir: finalDir,
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
    downloadTasks.value.push(task);
    saveDownloadTasks();
    return task;
  }

  // 更新任务状态
  function updateTask(taskId: string, updates: Partial<DownloadTask>) {
    const task = downloadTasks.value.find(t => t.id === taskId);
    if (task) {
      Object.assign(task, updates);
      saveDownloadTasks();
    }
  }

  // 删除任务
  function removeTask(taskId: string) {
    const index = downloadTasks.value.findIndex(t => t.id === taskId);
    if (index !== -1) {
      const task = downloadTasks.value[index];
      
      // 如果是组任务，同时删除子任务
      if (task.isGroup && task.childIds) {
        task.childIds.forEach(childId => {
          const childIndex = downloadTasks.value.findIndex(t => t.id === childId);
          if (childIndex !== -1) {
            downloadTasks.value.splice(childIndex, 1);
          }
        });
      }
      
      downloadTasks.value.splice(index, 1);
      saveDownloadTasks();
    }
  }

  // 清空已完成任务
  function clearCompletedTasks() {
    downloadTasks.value = downloadTasks.value.filter(t => t.status !== 'completed');
    saveDownloadTasks();
  }

  // 清空失败任务
  function clearFailedTasks() {
    downloadTasks.value = downloadTasks.value.filter(t => t.status !== 'failed');
    saveDownloadTasks();
  }

  // 保存任务到 localStorage
  function saveDownloadTasks() {
    try {
      localStorage.setItem('downloadTasks', JSON.stringify(downloadTasks.value));
    } catch (error) {
      console.error('保存下载任务失败:', error);
    }
  }

  // 从 localStorage 加载任务
  function loadDownloadTasks() {
    try {
      const saved = localStorage.getItem('downloadTasks');
      if (saved) {
        const tasks = JSON.parse(saved) as DownloadTask[];
        // 将下载中/等待中的任务恢复为暂停状态
        downloadTasks.value = tasks.map(t => {
          if (t.status === 'downloading' || t.status === 'waiting') {
            return { ...t, status: 'paused' as const, stage: '已暂停' };
          }
          return t;
        });
      }
    } catch (error) {
      console.error('加载下载任务失败:', error);
    }
  }

  // 批量选择相关
  function toggleSelectMode() {
    isSelectMode.value = !isSelectMode.value;
    if (!isSelectMode.value) {
      selectedTaskIds.value = [];
    }
  }

  function toggleTaskSelection(taskId: string) {
    const index = selectedTaskIds.value.indexOf(taskId);
    if (index === -1) {
      selectedTaskIds.value.push(taskId);
    } else {
      selectedTaskIds.value.splice(index, 1);
    }
  }

  function selectAllTasks() {
    selectedTaskIds.value = topLevelTasks.value.map(t => t.id);
  }

  function clearSelection() {
    selectedTaskIds.value = [];
  }

  // 分P选择相关
  function toggleEntrySelect(index: number) {
    const idx = selectedEntries.value.indexOf(index);
    if (idx === -1) {
      selectedEntries.value.push(index);
    } else {
      selectedEntries.value.splice(idx, 1);
    }
  }

  function selectAllEntries(entries: VideoEntry[]) {
    selectedEntries.value = entries.map(e => e.index);
  }

  function clearEntrySelection() {
    selectedEntries.value = [];
  }

  // 重置视频选择状态
  function resetVideoSelection() {
    selectedEntries.value = [];
    selectedSeasonEpisodes.value = [];
    expandedSeasonItems.value.clear();
    seasonItemEntries.value.clear();
    seasonItemLoading.value.clear();
    selectedSeasonEntries.value.clear();
  }

  return {
    // 下载任务
    downloadTasks,
    currentTaskId,
    downloading,
    downloadProgress,
    downloadStage,
    downloadSpeed,
    isPausing,
    isProcessingQueue,

    // 选择状态
    isSelectMode,
    selectedTaskIds,
    expandedGroupIds,

    // 视频分P选择
    selectedEntries,
    selectedSeasonEpisodes,
    expandedSeasonItems,
    seasonItemEntries,
    seasonItemLoading,
    selectedSeasonEntries,

    // 计算属性
    topLevelTasks,
    activeTasks,
    completedTasks,
    failedTasks,
    activeTaskCount,

    // 方法
    getChildTasks,
    toggleGroupExpand,
    createDownloadTask,
    createGroupTask,
    updateTask,
    removeTask,
    clearCompletedTasks,
    clearFailedTasks,
    saveDownloadTasks,
    loadDownloadTasks,
    toggleSelectMode,
    toggleTaskSelection,
    selectAllTasks,
    clearSelection,
    toggleEntrySelect,
    selectAllEntries,
    clearEntrySelection,
    resetVideoSelection,
  };
});
