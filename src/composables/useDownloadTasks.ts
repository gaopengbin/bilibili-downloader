import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage, ElMessageBox } from 'element-plus';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import type { DownloadTask, ProgressDetail, UserSettings } from '@/types';

export function useDownloadTasks(settings: { value: UserSettings }) {
  // ==================== 状态 ====================
  const downloadTasks = ref<DownloadTask[]>([]);
  const currentTaskId = ref<string | null>(null);
  const downloadStage = ref('视频');
  const downloadSpeed = ref('');
  const downloadProgress = ref(0);
  const downloading = ref(false);
  const isPausing = ref(false);
  const isProcessingQueue = ref(false);
  
  // 批量选择
  const isSelectMode = ref(false);
  const selectedTaskIds = ref<string[]>([]);
  const expandedGroupIds = ref<string[]>([]);

  // ==================== 计算属性 ====================
  
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

  // ==================== 基础方法 ====================
  
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

  // 获取下载中的任务数量（不包括组任务）
  function getDownloadingCount(): number {
    return downloadTasks.value.filter(t => t.status === 'downloading' && !t.isGroup).length;
  }

  // 获取等待中的任务（不包括组任务，组任务不直接下载）
  function getWaitingTasks(): DownloadTask[] {
    return downloadTasks.value.filter(t => t.status === 'waiting' && !t.isGroup);
  }

  // ==================== 存储相关 ====================
  
  // 加载下载任务历史
  async function loadDownloadTasks() {
    try {
      const result = await invoke<{ success: boolean; data?: string }>('load_download_tasks');
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

  // ==================== 任务创建 ====================
  
  // 创建下载任务（初始状态为等待）
  function createDownloadTask(
    title: string, 
    cover: string | null, 
    downloadInfo: DownloadTask['downloadInfo'], 
    groupId?: string
  ): DownloadTask {
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
    return task;
  }

  // 创建组任务
  function createGroupTask(
    title: string, 
    cover: string | null, 
    totalCount: number, 
    finalDir: string | null
  ): DownloadTask {
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

  // ==================== 状态更新 ====================
  
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
    const wasNotCompleted = groupTask.status !== 'completed';
    if (completedCount === totalCount) {
      groupTask.status = 'completed';
      groupTask.completedAt = Date.now();
      // 组任务全部完成时发送通知
      if (wasNotCompleted) {
        sendDownloadNotification('B站视频下载完成', `${groupTask.title} (${totalCount}个视频)`);
      }
    } else if (failedCount === totalCount) {
      groupTask.status = 'failed';
    } else if (downloadingCount > 0) {
      groupTask.status = 'downloading';
      groupTask.stage = `正在下载 ${completedCount}/${totalCount}`;
    } else if (waitingCount > 0) {
      groupTask.status = 'waiting';
      if (failedCount > 0) {
        groupTask.stage = `${failedCount} 个失败，${waitingCount} 个等待中`;
      }
    } else if (failedCount > 0 && completedCount > 0) {
      groupTask.status = 'failed';
      groupTask.stage = `${completedCount} 个完成，${failedCount} 个失败`;
    } else if (pausedCount > 0) {
      groupTask.status = 'paused';
    }
  }

  // 发送系统通知
  async function sendDownloadNotification(title: string, body: string) {
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        const permission = await requestPermission();
        granted = permission === 'granted';
      }
      if (granted) {
        sendNotification({ 
          title, 
          body,
          sound: 'Default', // Windows 通知声音
        });
      }
    } catch (error) {
      console.debug('发送通知失败:', error);
    }
  }

  // 更新任务状态
  function updateTaskStatus(taskId: string, status: 'completed' | 'failed', error?: string) {
    const task = downloadTasks.value.find(t => t.id === taskId);
    if (task) {
      if (task.status === 'paused') return;
      task.status = status;
      task.completedAt = Date.now();
      if (error) task.error = error;
      if (status === 'completed') {
        task.progress = 100;
        task.retryCount = 0;
        
        // 发送下载完成通知（仅单独任务，组任务在全部完成时通知）
        if (!task.groupId) {
          sendDownloadNotification('B站视频下载完成', task.title);
        }
      }
      
      if (task.groupId) {
        updateGroupTaskStatus(task.groupId);
      }
      
      saveDownloadTasks();
    }
  }

  // 处理进度更新
  function handleProgressUpdate(detail: ProgressDetail) {
    const { task_id, percent, stage, speed, downloaded, total_size } = detail;
    downloadProgress.value = percent;
    downloadStage.value = stage;
    downloadSpeed.value = speed || '';
    
    if (task_id) {
      const task = downloadTasks.value.find(t => t.id === task_id);
      if (task && task.status === 'downloading') {
        task.progress = percent;
        task.stage = stage;
        task.speed = speed || '';
        task.downloaded = downloaded || '';
        task.totalSize = total_size || '';
        
        if (task.groupId) {
          updateGroupTaskStatus(task.groupId);
        }
      }
    } else if (currentTaskId.value) {
      const task = downloadTasks.value.find(t => t.id === currentTaskId.value);
      if (task) {
        task.progress = percent;
        task.stage = stage;
        task.speed = speed || '';
        task.downloaded = downloaded || '';
        task.totalSize = total_size || '';
        
        if (task.groupId) {
          updateGroupTaskStatus(task.groupId);
        }
      }
    }
  }

  // ==================== 下载执行 ====================
  
  // 执行单个下载任务
  async function executeDownloadTask(task: DownloadTask): Promise<boolean> {
    if (!task.downloadInfo) return false;
    
    const info = task.downloadInfo;
    const taskId = task.id;
    
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
      const result = await invoke<{ success: boolean; error?: string }>('download_video', {
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
        taskId: taskId,
        aria2cConnections: settings.value.aria2cConnections,
        preferCodec: settings.value.preferCodec || null,
        audioOnly: info.audioOnly || false,
      });

      const currentTask = downloadTasks.value.find(t => t.id === taskId);
      if (currentTask?.status === 'paused') {
        return false;
      }

      if (result.success) {
        updateTaskStatus(taskId, 'completed');
        return true;
      } else {
        const retryCount = task.retryCount || 0;
        if (retryCount < settings.value.maxRetryCount) {
          task.retryCount = retryCount + 1;
          task.status = 'waiting';
          task.stage = `重试中 (${task.retryCount}/${settings.value.maxRetryCount})`;
          task.progress = 0;
          saveDownloadTasks();
          return false;
        } else {
          updateTaskStatus(taskId, 'failed', result.error);
          return false;
        }
      }
    } catch (error) {
      const currentTask = downloadTasks.value.find(t => t.id === taskId);
      if (currentTask?.status !== 'paused') {
        const retryCount = task.retryCount || 0;
        if (retryCount < settings.value.maxRetryCount) {
          task.retryCount = retryCount + 1;
          task.status = 'waiting';
          task.stage = `重试中 (${task.retryCount}/${settings.value.maxRetryCount})`;
          task.progress = 0;
          saveDownloadTasks();
          return false;
        } else {
          updateTaskStatus(taskId, 'failed', String(error));
        }
      }
      return false;
    } finally {
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
    
    const tempDirs = new Set<string>();
    
    try {
      while (true) {
        const waitingTasks = getWaitingTasks();
        const downloadingCount = getDownloadingCount();
        
        if (waitingTasks.length === 0 && downloadingCount === 0) break;
        
        if (waitingTasks.length === 0) {
          await new Promise(resolve => setTimeout(resolve, 500));
          continue;
        }
        
        if (downloadingCount >= settings.value.maxConcurrentDownloads) {
          await new Promise(resolve => setTimeout(resolve, 500));
          continue;
        }
        
        const task = waitingTasks[0];
        if (task.downloadInfo?.tempDir) {
          tempDirs.add(task.downloadInfo.tempDir);
        }
        
        executeDownloadTask(task).catch(console.error);
        
        await new Promise(resolve => setTimeout(resolve, 100));
      }
    } finally {
      isProcessingQueue.value = false;
      downloading.value = downloadTasks.value.some(t => t.status === 'downloading');
      
      for (const tempDir of tempDirs) {
        try {
          await invoke('cleanup_temp_dir', { tempDir });
        } catch (e) {
          console.log('清理临时目录失败:', e);
        }
      }
    }
  }

  // ==================== 任务控制 ====================
  
  // 暂停下载
  async function pauseDownload(task?: DownloadTask) {
    const targetTask = task || downloadTasks.value.find(t => t.id === currentTaskId.value);
    if (!targetTask) return;
    
    try {
      targetTask.status = 'paused';
      saveDownloadTasks();
      
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

  // 继续下载
  async function resumeDownload(task: DownloadTask) {
    if (!task.downloadInfo) {
      ElMessage.error('无法恢复，缺少下载信息');
      return;
    }
    
    if (!task.downloadInfo.url || task.downloadInfo.url === 'https://www.bilibili.com/video/') {
      ElMessage.error('下载链接无效');
      return;
    }
    
    task.status = 'waiting';
    task.error = undefined;
    task.progress = 0;
    
    if (task.groupId) {
      updateGroupTaskStatus(task.groupId);
    }
    
    saveDownloadTasks();
    processDownloadQueue();
  }

  // 删除任务
  async function deleteTask(task: DownloadTask) {
    if (task.status === 'downloading') {
      isPausing.value = true;
      try {
        await invoke('cancel_download');
      } catch (e) {
        // 忽略错误
      }
      isPausing.value = false;
    }
    
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

  // ==================== 组任务控制 ====================
  
  // 暂停组任务
  async function pauseGroupDownload(groupTask: DownloadTask) {
    if (!groupTask.isGroup) return;
    
    const childTasks = getChildTasks(groupTask.id);
    const activeChildren = childTasks.filter(t => t.status === 'downloading' || t.status === 'waiting');
    
    if (activeChildren.length === 0) return;
    
    isPausing.value = true;
    try {
      await invoke('cancel_download');
    } catch (e) {
      // 忽略错误
    }
    isPausing.value = false;
    
    for (const child of activeChildren) {
      child.status = 'paused';
    }
    
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
    
    for (const child of pausedChildren) {
      child.status = 'waiting';
      child.error = undefined;
    }
    
    groupTask.status = 'waiting';
    saveDownloadTasks();
    ElMessage.success('已恢复组内所有任务');
    
    processDownloadQueue();
  }

  // 删除组任务
  async function deleteGroupTask(groupTask: DownloadTask) {
    if (!groupTask.isGroup) return;
    
    const childTasks = getChildTasks(groupTask.id);
    
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
    
    const tempDir = childTasks[0]?.downloadInfo?.tempDir;
    if (tempDir && groupTask.status !== 'completed') {
      try {
        await invoke('delete_folder', { path: tempDir });
      } catch (e) {
        console.error('删除临时文件夹失败:', e);
      }
    }
    
    const idsToDelete = [groupTask.id, ...childTasks.map(t => t.id)];
    downloadTasks.value = downloadTasks.value.filter(t => !idsToDelete.includes(t.id));
    saveDownloadTasks();
  }

  // 重试单个子任务
  async function retryChildTask(child: DownloadTask) {
    if (!child.groupId) return;
    
    child.status = 'waiting';
    child.progress = 0;
    child.error = undefined;
    child.stage = '';
    child.speed = undefined;
    child.downloaded = undefined;
    child.totalSize = undefined;
    child.retryCount = 0;
    
    updateGroupTaskStatus(child.groupId);
    saveDownloadTasks();
    
    ElMessage.success('已加入下载队列');
    processDownloadQueue();
  }

  // 删除单个子任务
  async function deleteChildTask(groupId: string, childId: string) {
    const groupTask = downloadTasks.value.find(t => t.id === groupId);
    const child = downloadTasks.value.find(t => t.id === childId);
    
    if (!groupTask || !child) return;
    
    if (child.status === 'downloading') {
      isPausing.value = true;
      try {
        await invoke('cancel_download');
      } catch (e) {
        // 忽略错误
      }
      isPausing.value = false;
    }
    
    if (child.status !== 'completed' && child.downloadInfo?.tempDir) {
      try {
        await invoke('delete_folder', { path: child.downloadInfo.tempDir });
      } catch (e) {
        console.error('删除临时文件夹失败:', e);
      }
    }
    
    const index = downloadTasks.value.findIndex(t => t.id === childId);
    if (index !== -1) {
      downloadTasks.value.splice(index, 1);
    }
    
    if (groupTask.childIds) {
      const childIndex = groupTask.childIds.indexOf(childId);
      if (childIndex !== -1) {
        groupTask.childIds.splice(childIndex, 1);
      }
    }
    
    updateGroupTaskStatus(groupId);
    
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
    
    for (const child of failedChildren) {
      child.status = 'waiting';
      child.progress = 0;
      child.error = undefined;
      child.stage = '';
      child.speed = undefined;
      child.downloaded = undefined;
      child.totalSize = undefined;
      child.retryCount = 0;
    }
    
    updateGroupTaskStatus(groupTask.id);
    saveDownloadTasks();
    
    ElMessage.success(`已将 ${failedChildren.length} 个失败任务加入下载队列`);
    processDownloadQueue();
  }

  // ==================== 批量操作 ====================
  
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
      
      for (const task of failedTasks.value) {
        if (task.downloadInfo?.tempDir) {
          try {
            await invoke('delete_folder', { path: task.downloadInfo.tempDir });
          } catch (e) {
            console.error('删除临时文件夹失败:', e);
          }
        }
      }
      
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
    
    const allIdsToDelete = new Set<string>(selectedTaskIds.value);
    for (const task of tasksToDelete) {
      if (task.isGroup && task.childIds) {
        task.childIds.forEach(id => allIdsToDelete.add(id));
      }
    }
    
    const allTasksToDelete = downloadTasks.value.filter(t => allIdsToDelete.has(t.id));
    
    const downloadingTasksList = allTasksToDelete.filter(t => t.status === 'downloading');
    if (downloadingTasksList.length > 0) {
      isPausing.value = true;
      try {
        await invoke('cancel_download');
      } catch (e) {
        // 忽略错误
      }
      isPausing.value = false;
    }
    
    const deletedDirs = new Set<string>();
    for (const task of allTasksToDelete) {
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
    
    const completedTasksList = downloadTasks.value.filter(
      t => selectedTaskIds.value.includes(t.id) && t.status === 'completed'
    );
    const completedCount = completedTasksList.length;
    
    if (completedCount > 0) {
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
        await deleteSelectedTasks(true);
      } catch (actionResult) {
        if (actionResult === 'cancel') {
          await deleteSelectedTasks(false);
        }
      }
    } else {
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
    
    isPausing.value = true;
    try {
      await invoke('cancel_download');
    } catch (e) {
      // 忽略错误
    }
    isPausing.value = false;
    
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
    
    for (const task of pausedTasks) {
      task.status = 'waiting';
      task.error = undefined;
    }
    saveDownloadTasks();
    ElMessage.success(`已恢复 ${pausedTasks.length} 个任务`);
    
    processDownloadQueue();
  }

  return {
    // 状态
    downloadTasks,
    currentTaskId,
    downloadStage,
    downloadSpeed,
    downloadProgress,
    downloading,
    isPausing,
    isProcessingQueue,
    isSelectMode,
    selectedTaskIds,
    expandedGroupIds,
    
    // 计算属性
    topLevelTasks,
    completedTasks,
    failedTasks,
    
    // 基础方法
    getChildTasks,
    toggleGroupExpand,
    getDownloadingCount,
    getWaitingTasks,
    
    // 存储
    loadDownloadTasks,
    saveDownloadTasks,
    
    // 任务创建
    createDownloadTask,
    createGroupTask,
    
    // 状态更新
    updateGroupTaskStatus,
    updateTaskStatus,
    handleProgressUpdate,
    
    // 下载执行
    executeDownloadTask,
    processDownloadQueue,
    
    // 任务控制
    pauseDownload,
    resumeDownload,
    deleteTask,
    openTaskFolder,
    
    // 组任务控制
    pauseGroupDownload,
    resumeGroupDownload,
    deleteGroupTask,
    retryChildTask,
    deleteChildTask,
    retryFailedChildren,
    
    // 批量操作
    clearCompletedTasks,
    clearFailedTasks,
    toggleSelectMode,
    toggleTaskSelect,
    toggleSelectAllActive,
    deleteSelectedTasks,
    confirmDeleteSelected,
    pauseAllTasks,
    resumeAllTasks,
  };
}
