<script setup lang="ts">
import { computed } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useAppStore } from '@/stores';

const appStore = useAppStore();

// 从 store 获取响应式数据
const settings = computed(() => appStore.settings);
const currentVersion = computed(() => appStore.currentVersion);
const isCheckingUpdate = computed(() => appStore.isCheckingUpdate);

// 选择默认下载目录
async function selectDefaultOutputDir() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择默认下载目录',
  });
  if (selected) {
    appStore.updateSettings({ defaultOutputDir: selected as string });
  }
}

// 清除默认目录
function clearDefaultOutputDir() {
  appStore.updateSettings({ defaultOutputDir: '' });
}

// 检查更新
async function checkUpdate() {
  await appStore.checkForUpdate(false);
}

// 打开 GitHub
async function openGitHub() {
  try {
    await openUrl('https://github.com/gaopengbin/bilibili-downloader');
  } catch (error) {
    console.error('打开链接失败:', error);
    window.open('https://github.com/gaopengbin/bilibili-downloader', '_blank');
  }
}

// 更新设置
function updateMaxConcurrentDownloads(val: number) {
  appStore.updateSettings({ maxConcurrentDownloads: val });
}

function updateDefaultQuality(val: string) {
  appStore.updateSettings({ defaultQuality: val });
}

function updateAria2cConnections(val: number) {
  appStore.updateSettings({ aria2cConnections: val });
}

function updatePreferCodec(val: string) {
  appStore.updateSettings({ preferCodec: val });
}

function updateMaxRetryCount(val: number) {
  appStore.updateSettings({ maxRetryCount: val });
}
</script>

<template>
  <div class="settings-panel">
    <!-- 下载设置 -->
    <div class="settings-section">
      <div class="settings-section-title">下载设置</div>
      
      <div class="settings-item">
        <div class="settings-label">最大并行下载数</div>
        <el-input-number 
          :model-value="settings.maxConcurrentDownloads" 
          :min="1" 
          :max="10"
          size="small"
          @update:model-value="updateMaxConcurrentDownloads"
        />
      </div>
      
      <div class="settings-item">
        <div class="settings-label">默认下载目录</div>
        <div class="settings-input-group">
          <el-input 
            :model-value="settings.defaultOutputDir" 
            placeholder="每次下载时选择"
            size="small"
            readonly
          />
          <el-button size="small" @click="selectDefaultOutputDir">选择</el-button>
          <el-button 
            v-if="settings.defaultOutputDir"
            size="small" 
            type="danger" 
            text
            @click="clearDefaultOutputDir"
          >清除</el-button>
        </div>
      </div>
      
      <div class="settings-item">
        <div class="settings-label">默认清晰度</div>
        <el-select 
          :model-value="settings.defaultQuality" 
          placeholder="自动选择最高"
          size="small"
          clearable
          @update:model-value="updateDefaultQuality"
        >
          <el-option label="自动选择最高" value="" />
          <el-option label="4K (2160P)" value="2160" />
          <el-option label="1080P 高清" value="1080" />
          <el-option label="720P 高清" value="720" />
          <el-option label="480P 标清" value="480" />
          <el-option label="360P 流畅" value="360" />
        </el-select>
      </div>
    </div>
    
    <!-- 高级设置 -->
    <div class="settings-section">
      <div class="settings-section-title">高级设置</div>
      
      <div class="settings-item">
        <div class="settings-label">
          多线程连接数
          <span class="settings-hint">aria2c 下载连接数，越大下载越快但占用带宽越多</span>
        </div>
        <el-input-number 
          :model-value="settings.aria2cConnections" 
          :min="1" 
          :max="64"
          size="small"
          @update:model-value="updateAria2cConnections"
        />
      </div>
      
      <div class="settings-item">
        <div class="settings-label">
          视频编码偏好
          <span class="settings-hint">AVC 兼容性最好，HEVC/AV1 文件更小但需要解码支持</span>
        </div>
        <el-select 
          :model-value="settings.preferCodec" 
          placeholder="自动选择"
          size="small"
          clearable
          @update:model-value="updatePreferCodec"
        >
          <el-option label="自动选择" value="" />
          <el-option label="AVC (H.264)" value="avc" />
          <el-option label="HEVC (H.265)" value="hevc" />
          <el-option label="AV1" value="av1" />
        </el-select>
      </div>
      
      <div class="settings-item">
        <div class="settings-label">
          失败自动重试次数
          <span class="settings-hint">下载失败时自动重试的次数，设为 0 则不自动重试</span>
        </div>
        <el-input-number 
          :model-value="settings.maxRetryCount" 
          :min="0" 
          :max="10"
          size="small"
          @update:model-value="updateMaxRetryCount"
        />
      </div>
    </div>
    
    <!-- 关于 -->
    <div class="settings-section">
      <div class="settings-section-title">关于</div>
      <div class="settings-about">
        <p>哔哩哔哩下载器 v{{ currentVersion }}</p>
        <p class="settings-about-desc">基于 Tauri + Vue 3 开发</p>
        <p class="settings-about-link">
          <el-button 
            type="primary" 
            link 
            :loading="isCheckingUpdate"
            @click="checkUpdate"
          >
            {{ isCheckingUpdate ? '检查中...' : '检查更新' }}
          </el-button>
          <span class="link-divider">|</span>
          <span class="github-link" @click="openGitHub">GitHub</span>
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-panel {
  padding: 0 16px;
}

.settings-section {
  margin-bottom: 24px;
}

.settings-section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 16px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.settings-item {
  margin-bottom: 16px;
}

.settings-label {
  font-size: 13px;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.settings-hint {
  display: block;
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 4px;
}

.settings-input-group {
  display: flex;
  gap: 8px;
}

.settings-input-group .el-input {
  flex: 1;
}

.settings-about {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.8;
}

.settings-about-desc {
  color: var(--text-muted);
}

.settings-about-link {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}

.link-divider {
  color: var(--border-color);
}

.github-link {
  color: var(--primary-color);
  cursor: pointer;
}

.github-link:hover {
  text-decoration: underline;
}
</style>
