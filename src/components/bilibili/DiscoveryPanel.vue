<script setup lang="ts">
import { computed, ref, watch, onUnmounted } from 'vue';
import { Refresh, CaretRight, Search } from '@element-plus/icons-vue';
import { getHotSearch, getRanking } from '@/platforms/bilibili/api';
import { bilibiliRankingCategories, type BilibiliHotSearchItem, type BilibiliRankingItem } from '@/types/bilibili';
import BilibiliCover from './BilibiliCover.vue';

const props = defineProps<{ mode: 'hot' | 'ranking'; active: boolean }>();
const emit = defineEmits<{
  (e: 'search', keyword: string): void;
  (e: 'select', bvid: string, cover: string | null): void;
}>();
const loading = ref(false);
const error = ref('');
const hotSearches = ref<BilibiliHotSearchItem[]>([]);
const videos = ref<BilibiliRankingItem[]>([]);
const rid = ref(0);
const note = ref('');
const updatedAt = ref('');
const visibleCount = ref(20);
const visibleVideos = computed(() => videos.value.slice(0, visibleCount.value));
const category = computed(() => bilibiliRankingCategories.find(c => c.value === rid.value)?.label || '全站');
const title = computed(() => props.mode === 'hot' ? 'B站热搜' : `${category.value}排行榜`);
const cache = new Map<number, { items: BilibiliRankingItem[]; note: string; updatedAt: string }>();
let requestId = 0;
let hotLoaded = false;

function formatCount(value: number) {
  if (value >= 100000000) return `${(value / 100000000).toFixed(1)}亿`;
  if (value >= 10000) return `${(value / 10000).toFixed(1)}万`;
  return value.toLocaleString('zh-CN');
}

async function load(force = false) {
  const id = ++requestId;
  error.value = '';
  visibleCount.value = 20;
  if (!force && props.mode === 'hot' && hotLoaded) return;
  const cached = cache.get(rid.value);
  if (!force && props.mode === 'ranking' && cached) {
    videos.value = cached.items;
    note.value = cached.note;
    updatedAt.value = cached.updatedAt;
    loading.value = false;
    return;
  }
  loading.value = true;
  updatedAt.value = '';
  videos.value = [];
  hotSearches.value = [];
  try {
    const fetchedAt = () => new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
    if (props.mode === 'hot') {
      const result = await getHotSearch();
      if (id !== requestId) return;
      if (!result.success || !result.data) throw new Error(result.error || '获取热搜失败');
      hotSearches.value = result.data;
      hotLoaded = true;
      updatedAt.value = fetchedAt();
    } else {
      const selectedRid = rid.value;
      const result = await getRanking(selectedRid);
      if (id !== requestId) return;
      if (!result.success || !result.data) throw new Error(result.error || '获取榜单失败');
      videos.value = result.data.items;
      note.value = result.data.note;
      updatedAt.value = fetchedAt();
      cache.set(selectedRid, { ...result.data, updatedAt: updatedAt.value });
    }
  } catch (reason) {
    if (id === requestId) error.value = reason instanceof Error ? reason.message : String(reason);
  } finally {
    if (id === requestId) loading.value = false;
  }
}

watch(() => props.active, active => { if (active && !loading.value) void load(); }, { immediate: true });
watch(rid, () => { if (props.active) void load(); });
onUnmounted(() => { requestId++; });
</script>

<template>
  <section class="discovery-panel" :aria-label="title" :aria-busy="loading">
    <header class="discovery-header">
      <div>
        <h2>{{ title }}</h2>
        <p>{{ mode === 'hot' ? '看看大家正在搜什么，点击话题搜索视频' : '发现热门视频，点击查看详情与下载' }}</p>
      </div>
      <el-button text :icon="Refresh" :loading="loading" @click="load(true)">刷新</el-button>
    </header>

    <el-select v-if="mode === 'ranking'" v-model="rid" class="category-select" aria-label="选择榜单分区">
      <el-option v-for="item in bilibiliRankingCategories" :key="item.value" :label="item.label" :value="item.value" />
    </el-select>

    <div v-if="updatedAt" class="list-caption">
      <span>{{ mode === 'hot' ? `${hotSearches.length} 个热搜话题` : `${videos.length} 个上榜视频` }}</span>
      <span>获取于 {{ updatedAt }}</span>
    </div>

    <el-skeleton v-if="loading" :rows="8" animated class="discovery-skeleton" />
    <div v-else-if="error" class="discovery-error">
      <el-alert :title="mode === 'hot' ? '热搜暂时无法加载' : '榜单暂时无法加载'" :description="error" type="warning" :closable="false" show-icon />
      <el-button :icon="Refresh" @click="load(true)">重新加载</el-button>
    </div>
    <template v-else-if="mode === 'hot'">
      <div v-if="hotSearches.length" class="hot-list">
        <el-button v-for="item in hotSearches" :key="item.rank" text class="hot-row" @click="emit('search', item.keyword)">
          <span class="rank" :class="{ 'top-rank': item.rank <= 3 }">{{ item.rank }}</span>
          <span class="hot-title">{{ item.title }}</span>
          <span v-if="item.heat" class="heat">{{ formatCount(item.heat) }}<small>热度</small></span>
          <el-icon class="search-icon"><Search /></el-icon>
        </el-button>
      </div>
      <el-empty v-else description="暂无热搜话题" :image-size="80" />
    </template>
    <template v-else>
      <p v-if="videos.length && note" class="ranking-note">{{ note }}</p>
      <div v-if="videos.length" class="ranking-list">
        <el-button v-for="item in visibleVideos" :key="item.bvid" text class="ranking-row" @click="emit('select', item.bvid, item.cover)">
          <span class="rank" :class="{ 'top-rank': item.rank <= 3 }">{{ item.rank }}</span>
          <div class="video-cover">
            <BilibiliCover :src="item.cover" />
            <span v-if="item.duration" class="duration-tag">{{ item.duration }}</span>
          </div>
          <div class="video-meta">
            <div class="video-title">{{ item.title }}</div>
            <div class="author">{{ item.author }}</div>
            <div class="video-stats"><el-icon><CaretRight /></el-icon>{{ formatCount(item.play) }}<span>弹幕 {{ formatCount(item.danmaku) }}</span></div>
          </div>
        </el-button>
        <div v-if="visibleCount < videos.length" class="load-more">
          <el-button text @click="visibleCount += 20">加载更多（{{ videos.length - visibleCount }}）</el-button>
        </div>
      </div>
      <el-empty v-else description="该分区暂无上榜视频" :image-size="80" />
    </template>
  </section>
</template>

<style scoped>
.discovery-panel { color: var(--text-primary); padding-bottom: 16px; }
.discovery-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; margin-bottom: 16px; }
h2 { font-size: 18px; font-weight: 600; margin-bottom: 6px; }
.discovery-header p, .ranking-note { font-size: 12px; line-height: 1.6; color: var(--text-secondary); }
.discovery-header .el-button { margin-top: -2px; flex-shrink: 0; }
.discovery-panel :deep(.el-button.is-text) {
  --el-fill-color-light: var(--bg-hover);
  --el-fill-color: var(--bg-hover);
  --el-button-outline-color: var(--bili-pink);
}
.discovery-header .el-button, .load-more .el-button {
  --el-button-text-color: var(--text-secondary);
  --el-button-hover-text-color: var(--text-primary);
  --el-button-active-text-color: var(--text-primary);
}
.category-select { width: 150px; margin-bottom: 12px; }
.list-caption { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 6px; color: var(--text-secondary); font-size: 12px; padding: 6px 0 12px; }
.discovery-skeleton { padding: 16px 0; }
.discovery-error { display: grid; justify-items: start; gap: 16px; padding: 12px 0; }
.hot-list, .ranking-list { display: flex; flex-direction: column; }
.hot-row.el-button, .ranking-row.el-button {
  --el-button-text-color: var(--text-primary);
  --el-button-hover-text-color: var(--text-primary);
  --el-button-active-text-color: var(--text-primary);
  width: 100%; height: auto; margin: 0; padding: 14px 8px; border-radius: 8px;
  color: var(--text-primary); text-align: left; white-space: normal;
}
.hot-row :deep(> span), .ranking-row :deep(> span) { width: 100%; display: flex; align-items: center; gap: 10px; }
.hot-row.el-button.is-text:not(.is-disabled):hover,
.ranking-row.el-button.is-text:not(.is-disabled):hover,
.hot-row.el-button.is-text:not(.is-disabled):active,
.ranking-row.el-button.is-text:not(.is-disabled):active {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}
.hot-row.el-button.is-text:not(.is-disabled):focus-visible,
.ranking-row.el-button.is-text:not(.is-disabled):focus-visible {
  outline: 2px solid var(--bili-pink); outline-offset: -2px;
}
.rank { width: 24px; flex-shrink: 0; text-align: center; font-size: 15px; font-weight: 600; color: var(--text-secondary); font-variant-numeric: tabular-nums; }
.top-rank { color: var(--el-color-primary-dark-2); }
html.dark .top-rank { color: var(--bili-pink); }
.hot-title { flex: 1; min-width: 0; font-size: 14px; line-height: 1.5; overflow-wrap: anywhere; }
.heat { color: var(--text-secondary); font-size: 12px; text-align: right; flex-shrink: 0; font-variant-numeric: tabular-nums; }
.heat small { display: block; font-size: 10px; margin-top: 4px; }
.search-icon { color: var(--text-secondary); flex-shrink: 0; }
.ranking-note { margin-bottom: 10px; }
.video-cover { width: 120px; aspect-ratio: 16 / 9; border-radius: 6px; overflow: hidden; position: relative; flex-shrink: 0; }
.duration-tag { position: absolute; bottom: 4px; right: 4px; background: rgba(0,0,0,.75); color: #fff; font-size: 11px; padding: 2px 5px; border-radius: 3px; }
.video-meta { flex: 1; min-width: 0; }
.video-title { font-size: 13px; line-height: 1.5; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.author { color: var(--text-secondary); font-size: 12px; margin-top: 5px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
.video-stats { display: flex; flex-wrap: wrap; align-items: center; gap: 4px; color: var(--text-secondary); font-size: 11px; margin-top: 5px; }
.video-stats > span { margin-left: 6px; }
.load-more { text-align: center; padding: 12px 0; }
@media (max-width: 760px) { .video-cover { width: 100px; } }
@media (prefers-reduced-motion: reduce) { .hot-row, .ranking-row { transition: none; } }
</style>
