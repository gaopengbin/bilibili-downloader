<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useAppStore, useUserStore, useDownloadStore } from '@/stores';
import { 
  Download, User, Close, Sunny, Moon, Setting 
} from '@element-plus/icons-vue';

const appStore = useAppStore();
const userStore = useUserStore();
const downloadStore = useDownloadStore();

const { isDarkMode, showSettings, showDownloadCenter } = storeToRefs(appStore);
const { userInfo } = storeToRefs(userStore);
const { activeTaskCount } = storeToRefs(downloadStore);

const emit = defineEmits<{
  (e: 'logout'): void;
}>();

function toggleTheme() {
  appStore.toggleTheme();
}

function openLoginDialog() {
  userStore.openLoginDialog();
}

function handleLogout() {
  emit('logout');
}
</script>

<template>
  <header class="app-header">
    <div class="header-left">
      <div class="logo">
        <svg viewBox="0 0 24 24" width="32" height="32" fill="currentColor">
          <path d="M17.813 4.653h.854c1.51.054 2.769.578 3.773 1.574 1.004.995 1.524 2.249 1.56 3.76v7.36c-.036 1.51-.556 2.769-1.56 3.773s-2.262 1.524-3.773 1.56H5.333c-1.51-.036-2.769-.556-3.773-1.56S.036 18.858 0 17.347v-7.36c.036-1.511.556-2.765 1.56-3.76 1.004-.996 2.262-1.52 3.773-1.574h.774l-1.174-1.12a1.234 1.234 0 0 1-.373-.906c0-.356.124-.659.373-.907l.027-.027c.267-.249.573-.373.92-.373.347 0 .653.124.92.373L9.653 4.44c.071.071.134.142.187.213h4.267a.836.836 0 0 1 .16-.213l2.853-2.747c.267-.249.573-.373.92-.373.347 0 .662.151.929.4.267.249.391.551.391.907 0 .355-.124.657-.373.906l-1.174 1.12zM5.333 7.24c-.746.018-1.373.276-1.88.773-.506.498-.769 1.13-.786 1.894v7.52c.017.764.28 1.395.786 1.893.507.498 1.134.756 1.88.773h13.334c.746-.017 1.373-.275 1.88-.773.506-.498.769-1.129.786-1.893v-7.52c-.017-.765-.28-1.396-.786-1.894-.507-.497-1.134-.755-1.88-.773H5.333zM8 11.107c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c0-.373.129-.689.386-.947.258-.257.574-.386.947-.386zm8 0c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c.017-.391.15-.711.4-.96.249-.249.56-.373.933-.373z"/>
        </svg>
        <span class="logo-text">哔哩哔哩下载器</span>
      </div>
    </div>
    <div class="header-right">
      <!-- 设置按钮 -->
      <el-button class="header-btn" size="small" @click="showSettings = true">
        <el-icon><Setting /></el-icon>
        <span>设置</span>
      </el-button>
      
      <!-- 主题切换 -->
      <el-button class="header-btn" size="small" @click="toggleTheme">
        <el-icon><Moon v-if="!isDarkMode" /><Sunny v-else /></el-icon>
        <span>{{ isDarkMode ? '浅色' : '深色' }}</span>
      </el-button>
      
      <!-- 下载中心按钮 -->
      <el-badge :value="activeTaskCount" :hidden="activeTaskCount === 0" class="download-badge">
        <el-button class="header-btn" size="small" @click="showDownloadCenter = true">
          <el-icon><Download /></el-icon>
          <span>下载</span>
        </el-button>
      </el-badge>
      
      <template v-if="userInfo">
        <el-dropdown trigger="click">
          <div class="user-info">
            <el-avatar :src="userInfo.face" :size="32" />
            <span class="username">{{ userInfo.username }}</span>
          </div>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item @click="handleLogout">
                <el-icon><Close /></el-icon>
                退出登录
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </template>
      <template v-else>
        <el-button class="login-btn" size="small" @click="openLoginDialog">
          <el-icon><User /></el-icon>
          登录
        </el-button>
      </template>
    </div>
  </header>
</template>

<style scoped>
.app-header {
  background: var(--bg-card);
  height: 56px;
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-shadow: 0 1px 3px var(--shadow-color);
  flex-shrink: 0;
  z-index: 100;
  transition: background 0.3s;
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #fb7299;
}

.logo-text {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  transition: color 0.3s;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-btn {
  background: transparent;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  height: 32px;
  border-radius: 6px;
}

.header-btn:hover {
  color: #fb7299;
  border-color: #fb7299;
  background: rgba(251, 114, 153, 0.05);
}

.header-btn .el-icon {
  font-size: 16px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: background 0.2s;
}

.user-info:hover {
  background: var(--bg-hover);
}

.username {
  font-size: 13px;
  color: var(--text-primary);
}

.login-btn {
  background: #fb7299 !important;
  border-color: #fb7299 !important;
  color: #fff !important;
  height: 32px;
  padding: 6px 12px;
  border-radius: 6px;
}

.login-btn:hover {
  background: #fc8bab !important;
  border-color: #fc8bab !important;
}

.download-badge {
  margin-right: 12px;
}
</style>
