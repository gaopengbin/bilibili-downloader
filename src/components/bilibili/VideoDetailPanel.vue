<template>
  <div class="video-detail-panel">
    <!-- 加载中状态 -->
    <div v-if="loading" class="loading-card">
      <el-icon class="is-loading" :size="40"><Refresh /></el-icon>
      <p>正在解析视频...</p>
    </div>
    
    <div v-else-if="videoInfo" class="video-detail-card">
      <div class="detail-content">
      <div class="detail-cover">
        <el-image v-if="videoInfo.thumbnail" :src="videoInfo.thumbnail" fit="cover">
          <template #error>
            <div class="image-error">
              <el-icon :size="40"><VideoPlay /></el-icon>
            </div>
          </template>
        </el-image>
        <div v-else class="image-placeholder">
          <el-icon :size="40"><VideoPlay /></el-icon>
        </div>
      </div>
      <div class="detail-info">
        <h2 class="detail-title">{{ videoInfo.title }}</h2>
        <div class="detail-meta">
          <span v-if="videoInfo.uploader" class="meta-item">
            <el-icon><User /></el-icon>
            {{ videoInfo.uploader }}
          </span>
          <span v-if="videoInfo.duration" class="meta-item">
            <el-icon><Clock /></el-icon>
            {{ formatDuration(videoInfo.duration) }}
          </span>
          <el-tag v-if="videoInfo.is_playlist" type="warning" size="small">
            {{ videoInfo.entries.length }}P
          </el-tag>
        </div>
        
        <!-- 简介 -->
        <div v-if="videoInfo.description" class="detail-desc" :class="{ expanded: descExpanded }">
          <div class="desc-content">{{ videoInfo.description }}</div>
          <div class="desc-toggle" @click="$emit('update:descExpanded', !descExpanded)">
            {{ descExpanded ? '收起' : '展开' }}
          </div>
        </div>
        
        <div v-if="videoInfo.formats.length > 0" class="quality-selector">
          <span class="label">清晰度</span>
          <el-radio-group :model-value="selectedQuality" @update:model-value="$emit('update:selectedQuality', $event)" size="small">
            <el-radio-button
              v-for="format in videoInfo.formats"
              :key="format.format_id"
              :value="format.height?.toString() || ''"
            >
              {{ format.height }}p
            </el-radio-button>
          </el-radio-group>
        </div>

        <!-- 分P选择（仅当没有合集时显示） -->
        <div v-if="videoInfo.is_playlist && videoInfo.entries.length > 0 && !videoInfo.season" class="episode-section">
          <div class="episode-header">
            <span>选集列表</span>
            <el-button size="small" text @click="$emit('toggle-select-all')">
              {{ selectedEntries.length === videoInfo.entries.length ? '取消全选' : '全选' }}
            </el-button>
            <span class="select-count">{{ selectedEntries.length }}/{{ videoInfo.entries.length }}</span>
          </div>
          <div class="episode-list">
            <el-checkbox-group :model-value="selectedEntries" @update:model-value="$emit('update:selectedEntries', $event)">
              <div 
                v-for="entry in videoInfo.entries" 
                :key="entry.index"
                class="episode-item"
              >
                <el-checkbox :value="entry.index">
                  <div class="episode-content">
                    <span class="episode-index">P{{ entry.index }}</span>
                    <span class="episode-title">{{ entry.title || `第${entry.index}集` }}</span>
                    <span v-if="entry.duration" class="episode-duration">{{ formatDuration(entry.duration) }}</span>
                  </div>
                </el-checkbox>
              </div>
            </el-checkbox-group>
          </div>
        </div>
        
        <!-- 合集信息 -->
        <div v-if="videoInfo.season" class="season-section">
          <div class="season-header">
            <el-tag type="success" size="small">合集</el-tag>
            <span class="season-title">{{ videoInfo.season.title }}</span>
            <el-button size="small" text @click="$emit('toggle-season-select-all')">
              {{ selectedSeasonEpisodes.length === videoInfo.season.episodes.length ? '取消全选' : '全选' }}
            </el-button>
            <span class="select-count">{{ selectedSeasonEpisodes.length }}/{{ videoInfo.season.episodes.length }}</span>
          </div>
          <div class="season-list">
            <div 
              v-for="(ep, idx) in videoInfo.season.episodes" 
              :key="ep.bvid"
              class="season-item-wrapper"
            >
              <!-- 合集项主行 -->
              <div class="season-item" :class="{ 'current': isCurrentVideo(ep.bvid), 'expanded': expandedSeasonItems.has(ep.bvid) }">
                <el-checkbox 
                  :model-value="isSeasonItemFullySelected(ep.bvid) || selectedSeasonEpisodes.includes(ep.bvid)" 
                  :indeterminate="isSeasonItemIndeterminate(ep.bvid)"
                  @click.stop
                  @change="$emit('toggle-season-episode-select', ep.bvid)"
                />
                <div class="season-item-content" @click="$emit('select-from-list', ep.bvid, ep.cover)">
                  <span class="season-item-index">{{ idx + 1 }}</span>
                  <span class="season-item-title">{{ ep.title }}</span>
                  <span class="season-item-duration">{{ formatDuration(ep.duration) }}</span>
                </div>
                <!-- 展开/折叠按钮 -->
                <el-button 
                  class="expand-btn"
                  :icon="expandedSeasonItems.has(ep.bvid) ? 'ArrowDown' : 'ArrowRight'"
                  size="small"
                  text
                  :loading="seasonItemLoading.has(ep.bvid)"
                  @click.stop="$emit('toggle-season-item-expand', ep.bvid)"
                  title="查看分P"
                >
                  <el-icon v-if="!seasonItemLoading.has(ep.bvid)">
                    <CaretRight :class="{ 'rotated': expandedSeasonItems.has(ep.bvid) }" />
                  </el-icon>
                </el-button>
              </div>
              
              <!-- 分P子列表 -->
              <div 
                v-if="expandedSeasonItems.has(ep.bvid) && seasonItemEntries.get(ep.bvid)?.length" 
                class="season-item-entries"
              >
                <div class="entries-header" v-if="(seasonItemEntries.get(ep.bvid)?.length || 0) > 1">
                  <span class="entries-count">共 {{ seasonItemEntries.get(ep.bvid)?.length }} 个分P</span>
                  <el-button size="small" text @click.stop="$emit('toggle-season-item-select-all', ep.bvid)">
                    {{ getSeasonItemSelectedCount(ep.bvid) === (seasonItemEntries.get(ep.bvid)?.length || 0) ? '取消全选' : '全选' }}
                  </el-button>
                </div>
                <div 
                  v-for="entry in seasonItemEntries.get(ep.bvid)" 
                  :key="entry.index"
                  class="entry-item"
                  :class="{ 'selected': (selectedSeasonEntries.get(ep.bvid) || []).includes(entry.index) }"
                  @click.stop="$emit('toggle-season-entry-select', ep.bvid, entry.index)"
                >
                  <el-checkbox 
                    :model-value="(selectedSeasonEntries.get(ep.bvid) || []).includes(entry.index)"
                    @click.stop
                    @change="$emit('toggle-season-entry-select', ep.bvid, entry.index)"
                  />
                  <span class="entry-index">P{{ entry.index }}</span>
                  <span class="entry-title">{{ entry.title }}</span>
                  <span v-if="entry.duration" class="entry-duration">{{ formatDuration(entry.duration) }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      </div>
      
      <!-- 下载区域 -->
      <div class="download-section">
        <div class="output-dir">
          <el-input :model-value="outputDir" placeholder="选择保存位置" readonly size="small">
            <template #prefix>
              <el-icon><FolderOpened /></el-icon>
            </template>
            <template #append>
              <el-button @click="$emit('select-output-dir')">浏览</el-button>
            </template>
          </el-input>
        </div>
        
        <el-button 
          v-if="videoInfo?.season && selectedSeasonEpisodes.length > 0"
          type="primary" 
          size="large"
          class="download-btn"
          :disabled="!outputDir"
          @click="$emit('download-season')"
        >
          <el-icon><Download /></el-icon>
          下载合集 {{ selectedSeasonEpisodes.length }} 个视频
        </el-button>
        <el-button 
          v-else
          type="primary" 
          size="large"
          class="download-btn"
          :disabled="!outputDir || (videoInfo?.is_playlist && selectedEntries.length === 0)"
          @click="$emit('start-download')"
        >
          <el-icon><Download /></el-icon>
          {{ videoInfo?.is_playlist && selectedEntries.length > 0 
            ? `下载 ${selectedEntries.length} 个视频` 
            : '开始下载' }}
        </el-button>
      </div>
    </div>
    
    <!-- 空状态 -->
    <div v-else class="empty-detail">
      <el-icon :size="64"><VideoPlay /></el-icon>
      <p>选择视频开始下载</p>
      <p class="tip">从左侧搜索或输入链接</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { 
  VideoPlay, User, Clock, Refresh, CaretRight, 
  FolderOpened, Download 
} from '@element-plus/icons-vue';
import type { VideoInfo, VideoEntry } from '@/types';

defineProps<{
  videoInfo: VideoInfo | null;
  loading: boolean;
  outputDir: string;
  selectedQuality: string;
  selectedEntries: number[];
  selectedSeasonEpisodes: string[];
  expandedSeasonItems: Set<string>;
  seasonItemEntries: Map<string, VideoEntry[]>;
  seasonItemLoading: Set<string>;
  selectedSeasonEntries: Map<string, number[]>;
  descExpanded: boolean;
  isCurrentVideo: (bvid: string) => boolean;
  isSeasonItemIndeterminate: (bvid: string) => boolean;
  isSeasonItemFullySelected: (bvid: string) => boolean;
  getSeasonItemSelectedCount: (bvid: string) => number;
  formatDuration: (seconds: number | null) => string;
}>();

defineEmits<{
  (e: 'update:selectedQuality', value: string): void;
  (e: 'update:selectedEntries', value: number[]): void;
  (e: 'update:descExpanded', value: boolean): void;
  (e: 'toggle-select-all'): void;
  (e: 'toggle-season-select-all'): void;
  (e: 'toggle-season-episode-select', bvid: string): void;
  (e: 'toggle-season-item-expand', bvid: string): void;
  (e: 'toggle-season-item-select-all', bvid: string): void;
  (e: 'toggle-season-entry-select', bvid: string, entryIndex: number): void;
  (e: 'select-from-list', bvid: string, cover: string | null): void;
  (e: 'select-output-dir'): void;
  (e: 'download-season'): void;
  (e: 'start-download'): void;
}>();
</script>

<style scoped>
.video-detail-panel {
  height: 100%;
  overflow-y: auto;
  background: var(--bg-card);
  border-radius: 12px;
  box-shadow: 0 2px 8px var(--shadow-color);
}

.loading-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: var(--text-secondary);
}

.loading-card p {
  margin-top: 16px;
  font-size: 14px;
}

.video-detail-card {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.detail-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 16px;
  padding-bottom: 5px;
}

.detail-cover {
  width: 100%;
  aspect-ratio: 16/9;
  border-radius: 8px;
  overflow: hidden;
  background: var(--bg-hover);
  margin-bottom: 16px;
}

.detail-cover .el-image {
  width: 100%;
  height: 100%;
}

.image-error,
.image-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
}

.detail-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.detail-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 10px;
  line-height: 1.4;
}

.detail-meta {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 12px;
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--text-secondary);
  font-size: 13px;
}

.detail-desc {
  position: relative;
  margin-bottom: 16px;
  padding: 12px;
  background: var(--bg-hover);
  border-radius: 8px;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.6;
}

.desc-content {
  max-height: 60px;
  overflow: hidden;
}

.detail-desc.expanded .desc-content {
  max-height: none;
}

.desc-toggle {
  color: #fb7299;
  cursor: pointer;
  margin-top: 8px;
  font-size: 12px;
}

.quality-selector {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.quality-selector .label {
  font-size: 13px;
  color: var(--text-secondary);
}

/* 分P选择 */
.episode-section {
  margin-top: 12px;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.episode-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.select-count {
  margin-left: auto;
  color: var(--text-secondary);
  font-size: 12px;
}

.episode-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  border: 1px solid var(--border-color);
  border-radius: 8px;
}

.episode-item {
  padding: 10px 12px;
  border-bottom: 1px solid var(--border-color);
}

.episode-item:last-child {
  border-bottom: none;
}

.episode-content {
  display: flex;
  align-items: center;
  gap: 8px;
}

.episode-index {
  color: #fb7299;
  font-weight: 500;
  font-size: 12px;
  min-width: 32px;
}

.episode-title {
  flex: 1;
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.episode-duration {
  color: var(--text-secondary);
  font-size: 12px;
}

/* 合集样式 */
.season-section {
  margin-top: 12px;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.season-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.season-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.season-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  border: 1px solid var(--border-color);
  border-radius: 8px;
}

.season-item-wrapper {
  border-bottom: 1px solid var(--border-color);
}

.season-item-wrapper:last-child {
  border-bottom: none;
}

.season-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  cursor: pointer;
  transition: background 0.2s;
}

.season-item:hover {
  background: var(--bg-hover);
}

.season-item.current {
  background: rgba(251, 114, 153, 0.1);
}

.season-item.expanded {
  background: var(--bg-hover);
}

.season-item-content {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.season-item-index {
  color: var(--text-secondary);
  font-size: 12px;
  min-width: 24px;
}

.season-item-title {
  flex: 1;
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.season-item-duration {
  color: var(--text-secondary);
  font-size: 12px;
}

.expand-btn {
  padding: 4px;
}

.expand-btn .el-icon {
  transition: transform 0.2s;
}

.expand-btn .rotated {
  transform: rotate(90deg);
}

/* 分P子列表 */
.season-item-entries {
  background: var(--bg-secondary);
  padding: 8px 12px 8px 40px;
  border-top: 1px solid var(--border-color);
}

.entries-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0 8px;
  font-size: 12px;
  color: var(--text-secondary);
}

.entry-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.2s;
}

.entry-item:hover {
  background: var(--bg-hover);
}

.entry-item.selected {
  background: rgba(251, 114, 153, 0.1);
}

.entry-index {
  color: #fb7299;
  font-size: 11px;
  min-width: 28px;
}

.entry-title {
  flex: 1;
  font-size: 12px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.entry-duration {
  color: var(--text-secondary);
  font-size: 11px;
}

/* 下载区域 */
.download-section {
  flex-shrink: 0;
  padding: 16px 20px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-card);
}

.output-dir {
  margin-bottom: 12px;
}

.download-btn {
  width: 100%;
  height: 44px;
  font-size: 15px;
}

/* 空状态 */
.empty-detail {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  color: var(--text-secondary);
}

.empty-detail p {
  margin-top: 16px;
  font-size: 14px;
}

.empty-detail .tip {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-tertiary);
}
</style>
