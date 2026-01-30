<script setup lang="ts">
import { computed } from 'vue';
import { 
  Download, VideoPlay, VideoPause, FolderOpened, 
  Close, Refresh, RefreshRight, Folder 
} from '@element-plus/icons-vue';

// 类型定义
interface DownloadTask {
  id: string;
  title: string;
  cover: string | null;
  status: 'downloading' | 'completed' | 'failed' | 'paused' | 'waiting';
  progress: number;
  stage: string;
  speed?: string;
  downloaded?: string;
  totalSize?: string;
  createdAt: number;
  completedAt?: number;
  error?: string;
  downloadInfo?: {
    url: string;
    outputDir: string;
    tempDir: string | null;
    finalDir: string | null;
    quality: string | null;
    videoTitle: string | null;
    isPlaylistItem: boolean;
    entryIndex: number | null;
    entryTitle: string | null;
    expectedId: string | null;
  };
  groupId?: string;
  isGroup?: boolean;
  childIds?: string[];
  totalCount?: number;
  completedCount?: number;
  failedCount?: number;
  retryCount?: number;
}

// Props
const props = defineProps<{
  tasks: DownloadTask[];
  isSelectMode: boolean;
  selectedTaskIds: string[];
  expandedGroupIds: string[];
}>();

// Emits
const emit = defineEmits<{
  (e: 'pause-task', task: DownloadTask): void;
  (e: 'resume-task', task: DownloadTask): void;
  (e: 'delete-task', task: DownloadTask): void;
  (e: 'open-folder', task: DownloadTask): void;
  (e: 'pause-group', task: DownloadTask): void;
  (e: 'resume-group', task: DownloadTask): void;
  (e: 'delete-group', task: DownloadTask): void;
  (e: 'retry-failed-children', task: DownloadTask): void;
  (e: 'retry-child', child: DownloadTask): void;
  (e: 'delete-child', groupId: string, childId: string): void;
  (e: 'toggle-group-expand', groupId: string): void;
  (e: 'toggle-select-mode'): void;
  (e: 'toggle-task-select', taskId: string): void;
  (e: 'toggle-select-all'): void;
  (e: 'confirm-delete-selected'): void;
  (e: 'pause-all'): void;
  (e: 'resume-all'): void;
  (e: 'clear-completed'): void;
  (e: 'clear-failed'): void;
}>();

// 计算属性
const topLevelTasks = computed(() => 
  props.tasks.filter(t => !t.groupId)
);

const activeTasks = computed(() => 
  topLevelTasks.value.filter(t => 
    t.status === 'downloading' || 
    t.status === 'waiting' || 
    t.status === 'paused'
  )
);

const completedTasks = computed(() => 
  topLevelTasks.value.filter(t => t.status === 'completed')
);

const failedTasks = computed(() => 
  topLevelTasks.value.filter(t => t.status === 'failed')
);

const pausedTasksCount = computed(() => 
  props.tasks.filter(t => t.status === 'paused').length
);

// 获取活动任务数（用于显示）
const activeTasksCountDisplay = computed(() => {
  const singleCount = activeTasks.value.filter(t => !t.isGroup).length;
  const groupCount = activeTasks.value
    .filter(t => t.isGroup)
    .reduce((sum, g) => sum + (g.totalCount || 0), 0);
  return singleCount + groupCount;
});

const completedTasksCountDisplay = computed(() => {
  const singleCount = completedTasks.value.filter(t => !t.isGroup).length;
  const groupCount = completedTasks.value
    .filter(t => t.isGroup)
    .reduce((sum, g) => sum + (g.totalCount || 0), 0);
  return singleCount + groupCount;
});

const failedTasksCountDisplay = computed(() => {
  const singleCount = failedTasks.value.filter(t => !t.isGroup).length;
  const groupCount = failedTasks.value
    .filter(t => t.isGroup)
    .reduce((sum, g) => sum + (g.failedCount || 0), 0);
  return singleCount + groupCount;
});

// 获取组任务的子任务
function getChildTasks(groupId: string): DownloadTask[] {
  return props.tasks.filter(t => t.groupId === groupId);
}
</script>

<template>
  <div class="download-center">
    <div v-if="tasks.length === 0" class="empty-tasks">
      <el-icon :size="48"><Download /></el-icon>
      <p>暂无下载任务</p>
    </div>
    <template v-else>
      <!-- 全局操作按钮 -->
      <div class="download-actions">
        <template v-if="!isSelectMode">
          <el-button 
            size="small" 
            :disabled="activeTasks.length === 0"
            @click="emit('pause-all')"
          >
            <el-icon><VideoPause /></el-icon>
            全部暂停
          </el-button>
          <el-button 
            size="small" 
            type="primary"
            :disabled="pausedTasksCount === 0"
            @click="emit('resume-all')"
          >
            <el-icon><VideoPlay /></el-icon>
            全部开始
          </el-button>
          <el-button 
            size="small" 
            @click="emit('toggle-select-mode')"
          >
            批量管理
          </el-button>
        </template>
        <template v-else>
          <el-button size="small" @click="emit('toggle-select-all')">
            {{ selectedTaskIds.length === topLevelTasks.length ? '取消全选' : '全选' }}
          </el-button>
          <el-button 
            size="small" 
            type="danger"
            :disabled="selectedTaskIds.length === 0"
            @click="emit('confirm-delete-selected')"
          >
            删除 ({{ selectedTaskIds.length }})
          </el-button>
          <el-button size="small" @click="emit('toggle-select-mode')">
            完成
          </el-button>
        </template>
      </div>
      
      <!-- 下载中/等待中 -->
      <div v-if="activeTasks.length > 0" class="task-section">
        <div class="section-title">下载中 ({{ activeTasksCountDisplay }})</div>
        <div class="task-list">
          <template v-for="task in activeTasks" :key="task.id">
            <!-- 组任务 -->
            <div v-if="task.isGroup" class="task-group">
              <div 
                class="task-item task-group-header"
                :class="{ 
                  'task-downloading': task.status === 'downloading',
                  'task-waiting': task.status === 'waiting',
                  'task-selected': isSelectMode && selectedTaskIds.includes(task.id)
                }"
                @click="isSelectMode ? emit('toggle-task-select', task.id) : emit('toggle-group-expand', task.id)"
              >
                <el-checkbox 
                  v-if="isSelectMode"
                  :model-value="selectedTaskIds.includes(task.id)"
                  @click.stop
                  @change="emit('toggle-task-select', task.id)"
                  class="task-checkbox"
                />
                <div class="task-cover">
                  <el-image v-if="task.cover" :src="task.cover" fit="cover" />
                  <div v-else class="task-cover-placeholder">
                    <el-icon><Folder /></el-icon>
                  </div>
                  <div class="task-type-badge">合集</div>
                </div>
                <div class="task-info">
                  <div class="task-title">{{ task.title }}</div>
                  <div class="task-group-progress">
                    <span class="group-progress-text">{{ task.completedCount }}/{{ task.totalCount }}</span>
                    <span class="group-progress-label">已完成</span>
                    <span v-if="task.failedCount && task.failedCount > 0" class="group-failed-text">
                      {{ task.failedCount }} 个失败
                    </span>
                  </div>
                </div>
                <div class="task-actions">
                  <el-button 
                    v-if="task.status === 'downloading'" 
                    type="warning" 
                    size="small" 
                    circle
                    @click.stop="emit('pause-group', task)"
                    title="暂停全部"
                  >
                    <el-icon><VideoPause /></el-icon>
                  </el-button>
                  <el-button 
                    v-else-if="task.status === 'paused'" 
                    type="primary" 
                    size="small" 
                    circle
                    @click.stop="emit('resume-group', task)"
                    title="继续下载"
                  >
                    <el-icon><VideoPlay /></el-icon>
                  </el-button>
                  <el-button 
                    v-if="task.failedCount && task.failedCount > 0"
                    type="warning" 
                    size="small" 
                    circle
                    @click.stop="emit('retry-failed-children', task)"
                    title="重试失败"
                  >
                    <el-icon><RefreshRight /></el-icon>
                  </el-button>
                  <el-button 
                    type="danger" 
                    size="small" 
                    circle
                    @click.stop="emit('delete-group', task)"
                    title="删除"
                  >
                    <el-icon><Close /></el-icon>
                  </el-button>
                </div>
              </div>
              <!-- 子任务列表 -->
              <div v-if="expandedGroupIds.includes(task.id)" class="task-children">
                <div 
                  v-for="child in getChildTasks(task.id)" 
                  :key="child.id" 
                  class="task-item task-child"
                  :class="{ 
                    'task-downloading': child.status === 'downloading',
                    'task-completed': child.status === 'completed',
                    'task-failed': child.status === 'failed'
                  }"
                >
                  <div class="task-info">
                    <div class="task-title">{{ child.title }}</div>
                    <div v-if="child.status === 'downloading'" class="task-progress">
                      <div class="task-progress-header">
                        <span class="task-stage">{{ child.stage }}</span>
                        <span class="task-size-speed">
                          <span v-if="child.downloaded && child.totalSize" class="task-size">{{ child.downloaded }} / {{ child.totalSize }}</span>
                          <span v-if="child.speed" class="task-speed">{{ child.speed }}</span>
                        </span>
                      </div>
                      <el-progress 
                        :percentage="Math.round(child.progress)" 
                        :stroke-width="4"
                        :show-text="false"
                      />
                    </div>
                    <div v-else-if="child.status === 'completed'" class="task-status-text completed">
                      ✓ 已完成
                    </div>
                    <div v-else-if="child.status === 'failed'" class="task-status-text failed">
                      ✗ {{ child.error || '下载失败' }}
                    </div>
                    <div v-else-if="child.status === 'waiting'" class="task-status-text waiting">
                      等待中...
                    </div>
                    <div v-else-if="child.status === 'paused'" class="task-status-text paused">
                      已暂停
                    </div>
                  </div>
                  <!-- 子任务操作按钮 -->
                  <div class="task-child-actions">
                    <el-button 
                      v-if="child.status === 'failed' || child.status === 'paused'"
                      type="primary" 
                      size="small" 
                      circle
                      @click.stop="emit('retry-child', child)"
                      title="重试"
                    >
                      <el-icon><RefreshRight /></el-icon>
                    </el-button>
                    <el-button 
                      v-if="child.status !== 'downloading'"
                      type="danger" 
                      size="small" 
                      circle
                      @click.stop="emit('delete-child', task.id, child.id)"
                      title="删除"
                    >
                      <el-icon><Close /></el-icon>
                    </el-button>
                  </div>
                </div>
              </div>
            </div>
            <!-- 普通任务 -->
            <div 
              v-else
              class="task-item"
              :class="{ 
                'task-downloading': task.status === 'downloading',
                'task-waiting': task.status === 'waiting',
                'task-selected': isSelectMode && selectedTaskIds.includes(task.id)
              }"
              @click="isSelectMode && emit('toggle-task-select', task.id)"
            >
              <el-checkbox 
                v-if="isSelectMode"
                :model-value="selectedTaskIds.includes(task.id)"
                @click.stop
                @change="emit('toggle-task-select', task.id)"
                class="task-checkbox"
              />
              <div class="task-cover">
                <el-image v-if="task.cover" :src="task.cover" fit="cover" />
                <div v-else class="task-cover-placeholder">
                  <el-icon><VideoPlay /></el-icon>
                </div>
                <div v-if="task.status === 'downloading'" class="task-status-badge downloading">
                  <el-icon class="is-loading"><Refresh /></el-icon>
                </div>
                <div v-else-if="task.status === 'waiting'" class="task-status-badge waiting">
                  •••
                </div>
                <div v-else-if="task.status === 'paused'" class="task-status-badge paused">
                  ⏸
                </div>
              </div>
              <div class="task-info">
                <div class="task-title">{{ task.title }}</div>
                <div v-if="task.status === 'downloading'" class="task-progress">
                  <div class="task-progress-header">
                    <span class="task-stage">{{ task.stage }}</span>
                    <span class="task-size-speed">
                      <span v-if="task.downloaded && task.totalSize" class="task-size">{{ task.downloaded }} / {{ task.totalSize }}</span>
                      <span v-if="task.speed" class="task-speed">{{ task.speed }}</span>
                    </span>
                  </div>
                  <el-progress 
                    :percentage="Math.round(task.progress)" 
                    :stroke-width="6"
                    :show-text="false"
                  />
                  <span class="task-percent">{{ Math.round(task.progress) }}%</span>
                </div>
                <div v-else-if="task.status === 'waiting'" class="task-status-text waiting">
                  等待中...
                </div>
                <div v-else-if="task.status === 'paused'" class="task-status-text paused">
                  已暂停
                </div>
              </div>
              <div class="task-actions">
                <el-button 
                  v-if="task.status === 'downloading'" 
                  type="warning" 
                  size="small" 
                  circle
                  @click.stop="emit('pause-task', task)"
                  title="暂停"
                >
                  <el-icon><VideoPause /></el-icon>
                </el-button>
                <el-button 
                  v-else-if="task.status === 'waiting'" 
                  type="info" 
                  size="small" 
                  circle
                  @click.stop="emit('pause-task', task)"
                  title="取消"
                >
                  <el-icon><Close /></el-icon>
                </el-button>
                <template v-else>
                  <el-button 
                    v-if="task.downloadInfo" 
                    type="primary" 
                    size="small" 
                    circle
                    @click.stop="emit('resume-task', task)"
                    title="继续"
                  >
                    <el-icon><VideoPlay /></el-icon>
                  </el-button>
                  <el-button 
                    type="danger" 
                    size="small" 
                    circle
                    @click.stop="emit('delete-task', task)"
                    title="删除"
                  >
                    <el-icon><Close /></el-icon>
                  </el-button>
                </template>
              </div>
            </div>
          </template>
        </div>
      </div>
      
      <!-- 已完成 -->
      <div v-if="completedTasks.length > 0" class="task-section">
        <div class="section-header">
          <div class="section-title">已完成 ({{ completedTasksCountDisplay }})</div>
          <el-button type="danger" text size="small" @click="emit('clear-completed')">
            清空
          </el-button>
        </div>
        <div class="task-list">
          <template v-for="task in completedTasks" :key="task.id">
            <!-- 组任务 -->
            <div v-if="task.isGroup" class="task-group">
              <div 
                class="task-item task-group-header task-completed"
                :class="{ 'task-selected': isSelectMode && selectedTaskIds.includes(task.id) }"
                @click="isSelectMode ? emit('toggle-task-select', task.id) : emit('toggle-group-expand', task.id)"
              >
                <el-checkbox 
                  v-if="isSelectMode"
                  :model-value="selectedTaskIds.includes(task.id)"
                  @click.stop
                  @change="emit('toggle-task-select', task.id)"
                  class="task-checkbox"
                />
                <div class="task-cover">
                  <el-image v-if="task.cover" :src="task.cover" fit="cover" />
                  <div v-else class="task-cover-placeholder">
                    <el-icon><Folder /></el-icon>
                  </div>
                  <div class="task-type-badge">合集</div>
                  <div class="task-status-badge completed">✓</div>
                </div>
                <div class="task-info">
                  <div class="task-title">{{ task.title }}</div>
                  <div class="task-status-text completed">
                    全部下载完成 ({{ task.totalCount }}个)
                  </div>
                </div>
                <div class="task-actions">
                  <el-button 
                    type="primary" 
                    size="small" 
                    circle
                    @click.stop="emit('open-folder', task)"
                    title="打开文件夹"
                  >
                    <el-icon><FolderOpened /></el-icon>
                  </el-button>
                  <el-button 
                    type="danger" 
                    size="small" 
                    circle
                    @click.stop="emit('delete-group', task)"
                    title="删除"
                  >
                    <el-icon><Close /></el-icon>
                  </el-button>
                </div>
              </div>
              <!-- 子任务列表 -->
              <div v-if="expandedGroupIds.includes(task.id)" class="task-children">
                <div 
                  v-for="child in getChildTasks(task.id)" 
                  :key="child.id" 
                  class="task-item task-child task-completed"
                >
                  <div class="task-info">
                    <div class="task-title">{{ child.title }}</div>
                    <div class="task-status-text completed">✓ 已完成</div>
                  </div>
                </div>
              </div>
            </div>
            <!-- 普通任务 -->
            <div 
              v-else
              class="task-item task-completed"
              :class="{ 'task-selected': isSelectMode && selectedTaskIds.includes(task.id) }"
              @click="isSelectMode && emit('toggle-task-select', task.id)"
            >
              <el-checkbox 
                v-if="isSelectMode"
                :model-value="selectedTaskIds.includes(task.id)"
                @click.stop
                @change="emit('toggle-task-select', task.id)"
                class="task-checkbox"
              />
              <div class="task-cover">
                <el-image v-if="task.cover" :src="task.cover" fit="cover" />
                <div v-else class="task-cover-placeholder">
                  <el-icon><VideoPlay /></el-icon>
                </div>
                <div class="task-status-badge completed">
                  ✓
                </div>
              </div>
              <div class="task-info">
                <div class="task-title">{{ task.title }}</div>
                <div class="task-status-text completed">
                  下载完成
                </div>
              </div>
              <div class="task-actions">
                <el-button 
                  type="primary" 
                  size="small" 
                  circle
                  @click.stop="emit('open-folder', task)"
                  title="打开文件夹"
                >
                  <el-icon><FolderOpened /></el-icon>
                </el-button>
                <el-button 
                  type="danger" 
                  size="small" 
                  circle
                  @click.stop="emit('delete-task', task)"
                  title="删除"
                >
                  <el-icon><Close /></el-icon>
                </el-button>
              </div>
            </div>
          </template>
        </div>
      </div>
      
      <!-- 下载失败 -->
      <div v-if="failedTasks.length > 0" class="task-section">
        <div class="section-header">
          <div class="section-title failed-title">下载失败 ({{ failedTasksCountDisplay }})</div>
          <el-button type="danger" text size="small" @click="emit('clear-failed')">
            清空
          </el-button>
        </div>
        <div class="task-list">
          <template v-for="task in failedTasks" :key="task.id">
            <!-- 组任务（合集） -->
            <div v-if="task.isGroup" class="task-group">
              <div 
                class="task-item task-group-header task-failed"
                :class="{ 'task-selected': isSelectMode && selectedTaskIds.includes(task.id) }"
                @click="isSelectMode ? emit('toggle-task-select', task.id) : emit('toggle-group-expand', task.id)"
              >
                <el-checkbox 
                  v-if="isSelectMode"
                  :model-value="selectedTaskIds.includes(task.id)"
                  @click.stop
                  @change="emit('toggle-task-select', task.id)"
                  class="task-checkbox"
                />
                <div class="task-cover">
                  <el-image v-if="task.cover" :src="task.cover" fit="cover" />
                  <div v-else class="task-cover-placeholder">
                    <el-icon><Folder /></el-icon>
                  </div>
                  <div class="task-type-badge">合集</div>
                  <div class="task-status-badge failed">✗</div>
                </div>
                <div class="task-info">
                  <div class="task-title">{{ task.title }}</div>
                  <div class="task-group-progress">
                    <span class="group-progress-text">{{ task.completedCount }}/{{ task.totalCount }}</span>
                    <span class="group-progress-label">已完成</span>
                    <span class="group-failed-text">{{ task.failedCount }} 个失败</span>
                  </div>
                </div>
                <div class="task-actions">
                  <el-button 
                    type="warning" 
                    size="small" 
                    circle
                    @click.stop="emit('retry-failed-children', task)"
                    title="重试失败"
                  >
                    <el-icon><RefreshRight /></el-icon>
                  </el-button>
                  <el-button 
                    type="danger" 
                    size="small" 
                    circle
                    @click.stop="emit('delete-group', task)"
                    title="删除"
                  >
                    <el-icon><Close /></el-icon>
                  </el-button>
                </div>
              </div>
              <!-- 子任务列表 -->
              <div v-if="expandedGroupIds.includes(task.id)" class="task-children">
                <div 
                  v-for="child in getChildTasks(task.id)" 
                  :key="child.id" 
                  class="task-item task-child"
                  :class="{ 
                    'task-completed': child.status === 'completed',
                    'task-failed': child.status === 'failed'
                  }"
                >
                  <div class="task-info">
                    <div class="task-title">{{ child.title }}</div>
                    <div v-if="child.status === 'completed'" class="task-status-text completed">
                      ✓ 已完成
                    </div>
                    <div v-else-if="child.status === 'failed'" class="task-status-text failed" :title="child.error">
                      {{ child.error || '下载失败' }}
                    </div>
                  </div>
                  <div v-if="child.status === 'failed'" class="task-child-actions">
                    <el-button 
                      type="primary" 
                      size="small" 
                      circle
                      @click.stop="emit('retry-child', child)"
                      title="重试"
                    >
                      <el-icon><RefreshRight /></el-icon>
                    </el-button>
                    <el-button 
                      type="danger" 
                      size="small" 
                      circle
                      @click.stop="emit('delete-child', task.id, child.id)"
                      title="删除"
                    >
                      <el-icon><Close /></el-icon>
                    </el-button>
                  </div>
                </div>
              </div>
            </div>
            <!-- 普通任务 -->
            <div 
              v-else
              class="task-item task-failed"
              :class="{ 'task-selected': isSelectMode && selectedTaskIds.includes(task.id) }"
              @click="isSelectMode && emit('toggle-task-select', task.id)"
            >
              <el-checkbox 
                v-if="isSelectMode"
                :model-value="selectedTaskIds.includes(task.id)"
                @click.stop
                @change="emit('toggle-task-select', task.id)"
                class="task-checkbox"
              />
              <div class="task-cover">
                <el-image v-if="task.cover" :src="task.cover" fit="cover" />
                <div v-else class="task-cover-placeholder">
                  <el-icon><VideoPlay /></el-icon>
                </div>
                <div class="task-status-badge failed">
                  ✗
                </div>
              </div>
              <div class="task-info">
                <div class="task-title">{{ task.title }}</div>
                <div class="task-status-text failed" :title="task.error">
                  {{ task.error || '下载失败' }}
                </div>
              </div>
              <div class="task-actions">
                <el-button 
                  v-if="task.downloadInfo" 
                  type="primary" 
                  size="small" 
                  circle
                  @click.stop="emit('resume-task', task)"
                  title="重试"
                >
                  <el-icon><RefreshRight /></el-icon>
                </el-button>
                <el-button 
                  type="danger" 
                  size="small" 
                  circle
                  @click.stop="emit('delete-task', task)"
                  title="删除"
                >
                  <el-icon><Close /></el-icon>
                </el-button>
              </div>
            </div>
          </template>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* 下载中心 */
.download-center {
  height: 100%;
}

.download-actions {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-color);
  flex-wrap: wrap;
}

.download-actions .el-button {
  flex: 1;
  min-width: 80px;
}

.empty-tasks {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 300px;
  color: var(--text-muted);
  gap: 12px;
}

.task-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.task-item {
  display: flex;
  gap: 12px;
  padding: 12px;
  background: var(--bg-hover);
  border-radius: 8px;
  transition: background 0.2s, border-color 0.2s;
  position: relative;
}

.task-item.task-selected {
  background: rgba(251, 114, 153, 0.15);
  border: 1px solid #fb7299;
}

.task-checkbox {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

/* 组任务样式 */
.task-group {
  display: flex;
  flex-direction: column;
}

.task-group-header {
  cursor: pointer;
}

.task-group-progress {
  display: flex;
  align-items: baseline;
  gap: 6px;
  margin-top: 4px;
}

.group-progress-text {
  font-size: 16px;
  font-weight: 600;
  color: #fb7299;
}

.group-progress-label {
  font-size: 12px;
  color: var(--text-muted);
}

.group-failed-text {
  font-size: 12px;
  color: #f56c6c;
  margin-left: 8px;
}

.task-children {
  margin-left: 20px;
  border-left: 2px solid var(--border-color);
  padding-left: 8px;
  margin-top: 4px;
}

.task-child {
  padding: 8px 12px;
  background: var(--bg-primary);
  border: none !important;
  display: flex;
  align-items: center;
  gap: 8px;
}

.task-child .task-info {
  flex: 1;
  min-width: 0;
}

.task-child .task-title {
  font-size: 12px;
  margin-bottom: 4px;
}

.task-child .task-progress {
  gap: 2px;
}

.task-child .task-status-text {
  font-size: 11px;
}

.task-child-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.2s;
}

.task-child:hover .task-child-actions {
  opacity: 1;
}

.task-child-actions .el-button {
  width: 24px !important;
  height: 24px !important;
  padding: 0 !important;
}

.task-child-actions .el-button .el-icon {
  font-size: 12px;
}

.task-item.task-downloading {
  background: rgba(251, 114, 153, 0.1);
  border: 1px solid rgba(251, 114, 153, 0.3);
}

.task-item.task-waiting {
  background: var(--bg-hover);
  border: 1px solid var(--border-color);
}

.task-item.task-completed {
  opacity: 0.8;
}

.task-item.task-failed {
  background: rgba(245, 108, 108, 0.08);
  border: 1px solid rgba(245, 108, 108, 0.3);
}

.section-title.failed-title {
  color: #f56c6c;
}

.task-cover {
  width: 80px;
  height: 50px;
  border-radius: 6px;
  overflow: hidden;
  position: relative;
  flex-shrink: 0;
  background: var(--bg-primary);
}

.task-cover .el-image {
  width: 100%;
  height: 100%;
}

.task-cover-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
}

.task-status-badge {
  position: absolute;
  bottom: 2px;
  right: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: bold;
}

.task-status-badge.downloading {
  background: #fb7299;
  color: #fff;
}

.task-status-badge.completed {
  background: #52c41a;
  color: #fff;
}

.task-status-badge.failed {
  background: #ff4d4f;
  color: #fff;
}

.task-status-badge.paused {
  background: #faad14;
  color: #fff;
}

.task-status-badge.waiting {
  background: #909399;
  color: #fff;
  font-size: 8px;
}

/* 合集类型标签 */
.task-type-badge {
  position: absolute;
  top: 2px;
  left: 2px;
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 9px;
  font-weight: 500;
  background: rgba(251, 114, 153, 0.9);
  color: #fff;
}

.task-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.task-title {
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
  margin-bottom: 6px;
}

.task-progress {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.task-progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.task-stage {
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
}

.task-size-speed {
  display: flex;
  gap: 8px;
  align-items: center;
}

.task-size {
  font-size: 11px;
  color: var(--text-muted);
}

.task-speed {
  font-size: 11px;
  color: #fb7299;
  font-weight: 500;
}

.task-progress .el-progress {
  flex: 1;
}

.task-percent {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  text-align: right;
}

.task-status-text {
  font-size: 12px;
}

.task-status-text.completed {
  color: #52c41a;
}

.task-status-text.failed {
  color: #ff4d4f;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 180px;
  cursor: help;
}

.task-status-text.paused {
  color: #faad14;
}

.task-status-text.waiting {
  color: #909399;
}

.task-section {
  margin-bottom: 20px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.section-header .section-title {
  margin-bottom: 0;
  padding-bottom: 0;
  border-bottom: none;
}

.section-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.task-actions {
  display: flex;
  align-items: center;
  margin-left: 8px;
}

.task-actions .el-button {
  width: 28px;
  height: 28px;
  padding: 0;
}
</style>
