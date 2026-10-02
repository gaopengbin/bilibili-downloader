<script setup lang="ts">
import { nextTick, onUnmounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Refresh } from '@element-plus/icons-vue';
import Artplayer from 'artplayer';
import artplayerPluginDanmuku, { type Result as DanmakuPlugin } from 'artplayer-plugin-danmuku';
import { MediaPlayer, type MediaPlayerClass } from 'dashjs';
import type { ApiResponse } from '@/types';
import type { PlaybackInfo, VideoDanmaku } from '@/types/playback';
import { parseDanmaku } from '@/utils/danmaku';

const props = defineProps<{ modelValue: boolean; url: string; title: string }>();
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>();
const container = ref<HTMLDivElement>();
const loading = ref(false);
const error = ref('');
const info = ref<PlaybackInfo>();
const selectedPage = ref(1);
const selectedQuality = ref(80);
const danmakuEnabled = ref(true);
const danmakuLoading = ref(false);
const danmakuError = ref('');
const danmakuCount = ref(0);
const density = ref('standard');
const ready = ref(false);
let art: Artplayer | undefined;
let dash: MediaPlayerClass | undefined;
let sessionId = '';
let requestId = 0;
let danmakuRequestId = 0;
let comments: VideoDanmaku[] = [];

function releaseSession(id: string) {
  if (id) void invoke('release_playback', { sessionId: id }).catch(() => {});
}

function destroyPlayer() {
  dash?.reset();
  dash = undefined;
  art?.destroy(false);
  art = undefined;
  releaseSession(sessionId);
  sessionId = '';
  ready.value = false;
}

function closePlayer() {
  requestId++;
  danmakuRequestId++;
  destroyPlayer();
  loading.value = false;
  info.value = undefined;
  comments = [];
}

function danmakuPlugin() {
  return art?.plugins.artplayerPluginDanmuku as DanmakuPlugin | undefined;
}

async function applyDanmaku() {
  const plugin = danmakuPlugin();
  if (!plugin) return;
  const interval = density.value === 'sparse' ? 2 : density.value === 'dense' ? 0 : 0.5;
  const last = [-Infinity, -Infinity, -Infinity];
  const displayed = comments.filter(comment => {
    if (comment.time - last[comment.mode] < interval) return false;
    last[comment.mode] = comment.time;
    return true;
  });
  plugin.config({ danmuku: displayed });
  await plugin.load();
  if (!danmakuEnabled.value) plugin.hide();
}

async function loadDanmaku(cid: number) {
  const id = ++danmakuRequestId;
  danmakuLoading.value = true;
  danmakuError.value = '';
  try {
    const result = await invoke<ApiResponse<string>>('get_video_danmaku', { cid });
    if (id !== danmakuRequestId || !props.modelValue) return;
    if (!result.success || !result.data) throw new Error(result.error || '弹幕加载失败');
    comments = parseDanmaku(result.data);
    danmakuCount.value = comments.length;
    await applyDanmaku();
  } catch (reason) {
    if (id === danmakuRequestId) danmakuError.value = String(reason instanceof Error ? reason.message : reason);
  } finally {
    if (id === danmakuRequestId) danmakuLoading.value = false;
  }
}

async function load(page?: number, quality?: number, preservePosition = false) {
  const position = preservePosition ? art?.currentTime || 0 : 0;
  const volume = art?.volume ?? 0.7;
  const muted = art?.muted ?? false;
  const rate = art?.playbackRate ?? 1;
  const resume = art?.playing ?? false;
  const id = ++requestId;
  danmakuRequestId++;
  destroyPlayer();
  comments = [];
  danmakuCount.value = 0;
  danmakuError.value = '';
  danmakuLoading.value = false;
  loading.value = true;
  error.value = '';
  try {
    const result = await invoke<ApiResponse<PlaybackInfo>>('start_playback', {
      url: props.url, page: page ?? null, quality: quality ?? null,
    });
    if (id !== requestId || !props.modelValue) {
      if (result.data) releaseSession(result.data.session_id);
      return;
    }
    if (!result.success || !result.data) throw new Error(result.error || '视频播放地址获取失败');
    info.value = result.data;
    sessionId = result.data.session_id;
    selectedPage.value = result.data.page;
    selectedQuality.value = result.data.quality;
    await nextTick();
    if (id !== requestId || !container.value || !props.modelValue) return;
    art = new Artplayer({
      container: container.value,
      url: result.data.source,
      type: 'mpd',
      theme: '#fb7299',
      lang: 'zh-cn',
      autoplay: false,
      volume, muted,
      setting: true,
      playbackRate: true,
      pip: true,
      fullscreen: true,
      fullscreenWeb: true,
      hotkey: true,
      playsInline: true,
      autoPlayback: false,
      customType: {
        mpd(video, url) {
          if (id !== requestId) return;
          dash = MediaPlayer().create();
          dash.updateSettings({ debug: { logLevel: 0 } });
          dash.on('error', () => {
            if (id === requestId) {
              loading.value = false;
              error.value = '视频加载中断，请重试；若仍无法播放，请检查网络或登录状态。';
            }
          });
          dash.initialize(video, url, false);
        },
      },
      plugins: [artplayerPluginDanmuku({
        danmuku: [], emitter: false, visible: danmakuEnabled.value,
        fontSize: 22, opacity: 0.85, margin: [8, '25%'],
        antiOverlap: true, synchronousPlayback: true,
      })],
    });
    art.on('artplayerPluginDanmuku:hide', () => { if (id === requestId) danmakuEnabled.value = false; });
    art.on('artplayerPluginDanmuku:show', () => { if (id === requestId) danmakuEnabled.value = true; });
    art.on('ready', () => {
      if (id !== requestId || !art) return;
      ready.value = true;
      loading.value = false;
      error.value = '';
      art.playbackRate = rate;
      if (position > 0) art.currentTime = Math.min(position, Math.max(0, art.duration - 1));
      if (resume) void art.play().catch(() => {});
    });
    art.on('video:error', () => {
      if (id !== requestId) return;
      loading.value = false;
      error.value = '当前视频无法播放，请重试或切换清晰度。';
    });
    void loadDanmaku(result.data.cid);
  } catch (reason) {
    if (id !== requestId) return;
    loading.value = false;
    error.value = String(reason instanceof Error ? reason.message : reason);
    destroyPlayer();
  }
}

watch(danmakuEnabled, enabled => {
  if (enabled) danmakuPlugin()?.show();
  else danmakuPlugin()?.hide();
});
watch(density, () => { void applyDanmaku().catch(() => {}); });
watch(() => props.modelValue, value => { if (!value) closePlayer(); });
onUnmounted(closePlayer);
</script>

<template>
  <el-dialog :model-value="modelValue" :title="info?.title || title || '视频播放'"
    width="90%" top="5vh" class="video-player-dialog" append-to-body destroy-on-close
    @update:model-value="emit('update:modelValue', $event)" @opened="load()">
    <div class="player-frame">
      <div ref="container" class="player-container" />
      <div v-if="loading && !error" class="player-overlay" role="status">
        <el-icon class="is-loading" :size="28"><Refresh /></el-icon><span>正在加载视频…</span>
      </div>
      <div v-if="error" class="player-overlay player-failure" role="alert">
        <span>{{ error }}</span>
        <el-button type="primary" @click="load(selectedPage, selectedQuality)">重新加载</el-button>
      </div>
    </div>
    <div class="player-toolbar">
      <el-select v-if="info && info.parts.length > 1" v-model="selectedPage" class="part-select"
        aria-label="选择播放分 P" :disabled="loading" @change="load(Number($event), selectedQuality)">
        <el-option v-for="part in info.parts" :key="part.page" :value="part.page"
          :label="`P${part.page} · ${part.title}`" />
      </el-select>
      <el-select v-if="info?.qualities.length" v-model="selectedQuality" class="quality-select"
        aria-label="播放清晰度" :disabled="loading" @change="load(selectedPage, Number($event), true)">
        <el-option v-for="quality in info.qualities" :key="quality.id" :value="quality.id" :label="quality.label" />
      </el-select>
      <el-switch v-model="danmakuEnabled" active-text="弹幕" aria-label="显示弹幕" />
      <el-select v-model="density" class="density-select" aria-label="弹幕密度" :disabled="!danmakuEnabled">
        <el-option value="sparse" label="稀疏" /><el-option value="standard" label="标准" /><el-option value="dense" label="密集" />
      </el-select>
      <el-button text :icon="Refresh" :disabled="loading && !error" @click="load(selectedPage, selectedQuality, ready)">刷新播放地址</el-button>
    </div>
    <div class="danmaku-status" :class="{ 'has-error': danmakuError }" aria-live="polite">
      <span v-if="danmakuLoading">正在加载弹幕…</span>
      <template v-else-if="danmakuError"><span>弹幕暂不可用：{{ danmakuError }}</span>
        <el-button text size="small" @click="info && loadDanmaku(info.cid)">重试弹幕</el-button></template>
      <span v-else-if="info">{{ danmakuCount ? `已加载 ${danmakuCount.toLocaleString()} 条弹幕` : '该视频暂无可显示的普通弹幕' }}</span>
    </div>
  </el-dialog>
</template>

<style>
.video-player-dialog.el-dialog { max-width: min(1100px, calc((100vh - 250px) * 1.7778 + 32px)); background: var(--bg-card); }
.video-player-dialog .el-dialog__title { color: var(--text-primary); font-size: 16px; }
.video-player-dialog .el-dialog__body { padding: 16px; }
</style>
<style scoped>
.player-frame { position: relative; aspect-ratio: 16 / 9; background: #08090b; border-radius: 8px; overflow: hidden; }
.player-container { width: 100%; height: 100%; }
.player-overlay { position: absolute; inset: 0; z-index: 30; display: flex; align-items: center; justify-content: center; gap: 12px; color: #fff; background: #08090be8; }
.player-failure { flex-direction: column; padding: 24px; text-align: center; line-height: 1.6; }
.player-toolbar { display: flex; align-items: center; flex-wrap: wrap; gap: 12px; margin-top: 14px; }
.part-select { flex: 1; min-width: 180px; }
.quality-select { width: 118px; }
.density-select { width: 90px; }
.danmaku-status { display: flex; align-items: center; gap: 8px; min-height: 26px; margin-top: 6px; color: var(--text-secondary); font-size: 12px; }
.danmaku-status.has-error { color: var(--el-color-warning); }
.player-toolbar, .danmaku-status { --el-fill-color-light: var(--bg-hover); --el-fill-color: var(--bg-hover); --el-fill-color-blank: var(--bg-card); }
:deep(.art-selector-list) { max-height: 240px; overflow-y: auto; }
@media (max-width: 600px) { .part-select { flex-basis: 100%; } .player-toolbar { gap: 8px; } }
</style>
