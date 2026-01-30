<script setup lang="ts">
import { computed } from 'vue';
import { User } from '@element-plus/icons-vue';
import { useUserStore } from '@/stores';

// Props
defineProps<{
  historyList: Array<{
    bvid: string;
    title: string;
    cover: string | null;
    duration: number;
    progress: number;
    view_at: number;
    author: string;
  }>;
  loading: boolean;
  hasMore: boolean;
}>();

const emit = defineEmits<{
  (e: 'select', bvid: string, cover: string | null): void;
  (e: 'load-more'): void;
  (e: 'login'): void;
}>();

const userStore = useUserStore();
const isLoggedIn = computed(() => userStore.isLoggedIn);

// 格式化时长
function formatDuration(seconds: number | null): string {
  if (!seconds) return '00:00';
  const min = Math.floor(seconds / 60);
  const sec = Math.floor(seconds % 60);
  return `${min}:${sec.toString().padStart(2, '0')}`;
}

// 格式化时间
function formatTime(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  
  if (diff < 60 * 1000) return '刚刚';
  if (diff < 60 * 60 * 1000) return `${Math.floor(diff / 60 / 1000)}分钟前`;
  if (diff < 24 * 60 * 60 * 1000) return `${Math.floor(diff / 60 / 60 / 1000)}小时前`;
  if (diff < 7 * 24 * 60 * 60 * 1000) return `${Math.floor(diff / 24 / 60 / 60 / 1000)}天前`;
  
  return `${date.getMonth() + 1}-${date.getDate()}`;
}

function onSelect(bvid: string, cover: string | null) {
  emit('select', bvid, cover);
}
</script>

<template>
  <div class="history-panel">
    <div v-if="!isLoggedIn" class="not-login">
      <el-icon :size="48"><User /></el-icon>
      <p>登录后查看历史记录</p>
      <el-button type="primary" @click="emit('login')">立即登录</el-button>
    </div>
    <div v-else-if="loading && historyList.length === 0" class="loading-state">
      <el-skeleton :rows="4" animated />
    </div>
    <div v-else class="video-list">
      <div 
        v-for="item in historyList" 
        :key="item.bvid"
        class="video-item"
        @click="onSelect(item.bvid, item.cover)"
      >
        <div class="video-cover">
          <el-image v-if="item.cover" :src="item.cover" fit="cover" />
          <span class="duration-tag">{{ formatDuration(item.duration) }}</span>
          <div v-if="item.progress > 0" class="watch-progress">
            <div 
              class="progress-inner" 
              :style="{ width: Math.min(100, item.progress / item.duration * 100) + '%' }"
            ></div>
          </div>
        </div>
        <div class="video-meta">
          <div class="video-title">{{ item.title }}</div>
          <div class="video-info-row">
            <span class="author">{{ item.author }}</span>
            <span class="time">{{ formatTime(item.view_at) }}</span>
          </div>
        </div>
      </div>
      <div v-if="historyList.length === 0" class="empty-state">暂无历史记录</div>
      <div v-if="hasMore" class="load-more">
        <el-button text :loading="loading" @click="emit('load-more')">加载更多</el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.history-panel {
  height: 100%;
}

.not-login {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: var(--text-muted);
}

.not-login p {
  margin: 16px 0;
}

.loading-state {
  padding: 16px;
}

.video-list {
  display: flex;
  flex-direction: column;
}

.video-item {
  display: flex;
  gap: 12px;
  padding: 10px 16px;
  cursor: pointer;
  transition: background 0.2s;
}

.video-item:hover {
  background: var(--bg-hover);
}

.video-cover {
  width: 120px;
  height: 68px;
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
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.video-info-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-muted);
}

.author {
  flex-shrink: 0;
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty-state {
  text-align: center;
  padding: 40px;
  color: var(--text-muted);
}

.load-more {
  text-align: center;
  padding: 12px;
}
</style>
