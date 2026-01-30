<script setup lang="ts">
import { computed } from 'vue';
import { VideoPlay } from '@element-plus/icons-vue';

interface Props {
  title: string;
  cover: string | null;
  duration?: number | string;
  author?: string;
  play?: number;
  danmaku?: number;
  progress?: number; // 0-100 观看进度百分比
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'click'): void;
}>();

// 格式化时长 (秒 -> mm:ss 或 hh:mm:ss)
function formatDuration(seconds: number | string | undefined): string {
  if (seconds === undefined || seconds === null) return '';
  
  // 如果已经是字符串格式（如 "12:34"），直接返回
  if (typeof seconds === 'string') return seconds;
  
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  
  if (h > 0) {
    return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }
  return `${m}:${String(s).padStart(2, '0')}`;
}

// 格式化播放量
function formatPlayCount(count: number | undefined): string {
  if (!count) return '0';
  if (count >= 10000) {
    return (count / 10000).toFixed(1) + '万';
  }
  return count.toString();
}

const formattedDuration = computed(() => formatDuration(props.duration));
const formattedPlay = computed(() => formatPlayCount(props.play));
const formattedDanmaku = computed(() => formatPlayCount(props.danmaku));
</script>

<template>
  <div class="video-item" @click="emit('click')">
    <div class="video-cover">
      <el-image 
        :src="cover || ''" 
        fit="cover"
        lazy
      >
        <template #placeholder>
          <div class="image-placeholder">
            <el-icon :size="24"><VideoPlay /></el-icon>
          </div>
        </template>
        <template #error>
          <div class="image-error">
            <el-icon :size="24"><VideoPlay /></el-icon>
          </div>
        </template>
      </el-image>
      <span v-if="formattedDuration" class="duration-tag">{{ formattedDuration }}</span>
      <div v-if="progress !== undefined && progress > 0" class="watch-progress">
        <div class="progress-inner" :style="{ width: progress + '%' }"></div>
      </div>
    </div>
    <div class="video-meta">
      <div class="video-title">{{ title }}</div>
      <div class="video-info-row">
        <span v-if="author" class="author">{{ author }}</span>
        <template v-if="play !== undefined || danmaku !== undefined">
          <span v-if="play !== undefined" class="stats">
            <el-icon :size="12"><VideoPlay /></el-icon>
            {{ formattedPlay }}
          </span>
          <span v-if="danmaku !== undefined" class="stats">
            弹{{ formattedDanmaku }}
          </span>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
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

.image-placeholder,
.image-error {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-hover);
  color: var(--text-muted);
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

.author {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
