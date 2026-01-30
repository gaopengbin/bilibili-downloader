<script setup lang="ts">
import { computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { ElMessage } from 'element-plus';
import { useAppStore } from '@/stores';

const appStore = useAppStore();

// Props
const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
}>();

// 计算属性
const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
});

const currentVersion = computed(() => appStore.currentVersion);
const updateInfo = computed(() => appStore.updateInfo);
const keyUpdates = computed(() => appStore.keyUpdates);
const isDownloadingUpdate = computed(() => appStore.isDownloadingUpdate);
const updateDownloadProgress = computed(() => appStore.updateDownloadProgress);

// 打开下载页面
async function openReleasePage() {
  if (updateInfo.value?.url) {
    try {
      await openUrl(updateInfo.value.url);
    } catch (error) {
      window.open(updateInfo.value.url, '_blank');
    }
  }
  visible.value = false;
}

// 下载并安装更新
async function downloadAndInstallUpdate() {
  if (!updateInfo.value?.downloadUrl) {
    openReleasePage();
    return;
  }
  
  appStore.isDownloadingUpdate = true;
  appStore.updateDownloadProgress = 0;
  
  try {
    await invoke('download_and_install_update', {
      url: updateInfo.value.downloadUrl,
      version: updateInfo.value.version
    });
  } catch (error: unknown) {
    console.error('下载更新失败:', error);
    const errorMsg = error instanceof Error ? error.message : String(error);
    ElMessage.error(`下载更新失败: ${errorMsg}`);
    appStore.isDownloadingUpdate = false;
  }
}
</script>

<template>
  <el-dialog
    v-model="visible"
    width="440px"
    :close-on-click-modal="false"
    :close-on-press-escape="!isDownloadingUpdate"
    :show-close="!isDownloadingUpdate"
    class="update-dialog"
  >
    <template #header>
      <div class="update-dialog-header">
        <div class="update-icon">🎉</div>
        <div class="update-title">发现新版本</div>
      </div>
    </template>
    
    <div class="update-dialog-content">
      <div class="update-version-box">
        <div class="version-item current">
          <span class="version-label">当前版本</span>
          <span class="version-num">v{{ currentVersion }}</span>
        </div>
        <div class="version-arrow">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
            <path d="M5 12h14M13 6l6 6-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <div class="version-item new">
          <span class="version-label">最新版本</span>
          <span class="version-num">v{{ updateInfo?.version }}</span>
        </div>
      </div>
      
      <div v-if="keyUpdates.length > 0" class="update-notes">
        <div class="update-notes-header">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
            <path d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span>更新内容</span>
        </div>
        <ul class="update-notes-list">
          <li v-for="(item, index) in keyUpdates" :key="index">{{ item }}</li>
        </ul>
      </div>
      
      <div v-if="isDownloadingUpdate" class="update-progress">
        <div class="update-progress-header">
          <span class="update-progress-text">正在下载更新...</span>
          <span class="update-progress-percent">{{ updateDownloadProgress }}%</span>
        </div>
        <el-progress 
          :percentage="updateDownloadProgress" 
          :stroke-width="8"
          :show-text="false"
          color="#fb7299"
        />
        <div class="update-progress-hint">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
            <path d="M12 6v6l4 2" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          下载完成后将自动启动安装程序
        </div>
      </div>
    </div>
    
    <template #footer>
      <div class="update-dialog-footer">
        <template v-if="!isDownloadingUpdate">
          <el-button class="btn-later" @click="visible = false">稍后再说</el-button>
          <el-button class="btn-update" type="primary" @click="downloadAndInstallUpdate">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" style="margin-right: 6px;">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            {{ updateInfo?.downloadUrl ? '立即更新' : '前往下载' }}
          </el-button>
        </template>
        <template v-else>
          <el-button class="btn-downloading" disabled>
            <span class="downloading-spinner"></span>
            下载中...
          </el-button>
        </template>
      </div>
    </template>
  </el-dialog>
</template>

<style scoped>
.update-dialog-header {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px 0;
}

.update-icon {
  font-size: 40px;
  margin-bottom: 8px;
}

.update-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.update-dialog-content {
  padding: 0 8px;
}

.update-version-box {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 20px;
  margin-bottom: 20px;
  padding: 16px;
  background: var(--bg-secondary);
  border-radius: 12px;
}

.version-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.version-label {
  font-size: 12px;
  color: var(--text-muted);
}

.version-num {
  font-size: 16px;
  font-weight: 600;
}

.version-item.current .version-num {
  color: var(--text-secondary);
}

.version-item.new .version-num {
  color: #fb7299;
}

.version-arrow {
  color: var(--text-muted);
}

.update-notes {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 12px 16px;
  margin-bottom: 16px;
}

.update-notes-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.update-notes-list {
  margin: 0;
  padding-left: 20px;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.8;
}

.update-notes-list li {
  margin-bottom: 2px;
}

.update-progress {
  padding: 16px;
  background: var(--bg-secondary);
  border-radius: 8px;
}

.update-progress-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 8px;
}

.update-progress-text {
  font-size: 13px;
  color: var(--text-primary);
}

.update-progress-percent {
  font-size: 13px;
  font-weight: 600;
  color: #fb7299;
}

.update-progress-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 12px;
  font-size: 12px;
  color: var(--text-muted);
}

.update-dialog-footer {
  display: flex;
  justify-content: center;
  gap: 12px;
}

.btn-later {
  min-width: 100px;
}

.btn-update {
  min-width: 120px;
}

.btn-downloading {
  min-width: 120px;
}

.downloading-spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-right: 6px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
