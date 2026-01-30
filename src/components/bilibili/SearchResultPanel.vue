<script setup lang="ts">
import { Search, CaretRight } from '@element-plus/icons-vue';

// Props
defineProps<{
  keyword: string;
  results: Array<{
    bvid: string;
    title: string;
    cover: string | null;
    duration: string;
    author: string;
    play: number;
    danmaku: number;
    pubdate: number;
    description: string;
  }>;
  total: number;
  loading: boolean;
  hasMore: boolean;
  searchType: 'video' | 'media_bangumi' | 'media_ft';
}>();

const emit = defineEmits<{
  (e: 'select', bvid: string, cover: string | null): void;
  (e: 'load-more'): void;
}>();

// 格式化播放量
function formatPlayCount(count: number): string {
  if (count >= 10000) {
    return (count / 10000).toFixed(1) + '万';
  }
  return count.toString();
}

function onSelect(bvid: string, cover: string | null) {
  emit('select', bvid, cover);
}
</script>

<template>
  <div class="search-result-panel">
    <div v-if="results.length > 0" class="search-results">
      <div class="result-header">
        <span>找到 {{ total }} 个结果</span>
      </div>
      <!-- 番剧/影视用栅格布局 -->
      <div v-if="searchType === 'media_bangumi' || searchType === 'media_ft'" class="media-grid">
        <div 
          v-for="item in results" 
          :key="item.bvid"
          class="media-card"
          @click="onSelect(item.bvid, item.cover)"
        >
          <div class="media-cover">
            <el-image v-if="item.cover" :src="item.cover" fit="cover" />
          </div>
          <div class="media-title">{{ item.title }}</div>
        </div>
      </div>
      <!-- 视频用列表布局 -->
      <div v-else class="video-list">
        <div 
          v-for="item in results" 
          :key="item.bvid"
          class="video-item"
          @click="onSelect(item.bvid, item.cover)"
        >
          <div class="video-cover">
            <el-image v-if="item.cover" :src="item.cover" fit="cover" />
            <span class="duration-tag">{{ item.duration }}</span>
          </div>
          <div class="video-meta">
            <div class="video-title">{{ item.title }}</div>
            <div class="video-info-row">
              <span class="author">{{ item.author }}</span>
              <span class="stats">
                <el-icon><CaretRight /></el-icon>{{ formatPlayCount(item.play) }}
              </span>
            </div>
          </div>
        </div>
      </div>
      <div v-if="hasMore" class="load-more">
        <el-button text :loading="loading" @click="emit('load-more')">加载更多</el-button>
      </div>
    </div>
    <div v-else-if="!loading && keyword" class="empty-state">
      <el-icon :size="48"><Search /></el-icon>
      <p>暂无搜索结果</p>
    </div>
  </div>
</template>

<style scoped>
.search-result-panel {
  height: 100%;
}

.search-results {
  padding-bottom: 16px;
}

.result-header {
  padding: 8px 16px;
  font-size: 12px;
  color: var(--text-muted);
}

.media-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: 12px;
  padding: 0 16px;
}

.media-card {
  cursor: pointer;
  transition: transform 0.2s;
}

.media-card:hover {
  transform: translateY(-2px);
}

.media-cover {
  aspect-ratio: 3/4;
  border-radius: 6px;
  overflow: hidden;
  background: var(--bg-hover);
}

.media-cover .el-image {
  width: 100%;
  height: 100%;
}

.media-title {
  font-size: 12px;
  color: var(--text-primary);
  margin-top: 6px;
  line-height: 1.3;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
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

.stats {
  display: flex;
  align-items: center;
  gap: 2px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: var(--text-muted);
}

.empty-state p {
  margin-top: 16px;
}

.load-more {
  text-align: center;
  padding: 12px;
}
</style>
