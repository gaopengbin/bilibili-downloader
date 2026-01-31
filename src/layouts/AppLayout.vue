<script setup lang="ts">
import { ref, onMounted, provide, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ElMessage, ElMessageBox } from 'element-plus';
import { 
  Download, Close, Sunny, Moon, Setting, Minus, FullScreen, CloseBold, User
} from '@element-plus/icons-vue';

// 引入组件
import { SettingsPanel, UpdateDialog } from '@/components';
import { useAppStore, useUserStore } from '@/stores';

// 引入类型
import type { UserInfo, ApiResponse } from '@/types';

const appStore = useAppStore();
const userStore = useUserStore();

// ==================== 状态 ====================

const userInfo = ref<UserInfo | null>(null);
const isDarkMode = ref(false);
const showSettings = ref(false);

// ==================== 主题 ====================

function toggleTheme() {
  isDarkMode.value = !isDarkMode.value;
  document.documentElement.classList.toggle('dark', isDarkMode.value);
  localStorage.setItem('theme', isDarkMode.value ? 'dark' : 'light');
}

function initTheme() {
  const saved = localStorage.getItem('theme');
  if (saved === 'dark' || (!saved && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
    isDarkMode.value = true;
    document.documentElement.classList.add('dark');
  }
}

// ==================== 登录相关 ====================

async function checkLoginStatus() {
  try {
    const result = await invoke<ApiResponse<UserInfo>>('check_login_status');
    if (result.success && result.data) {
      userInfo.value = result.data;
      userStore.setUserInfo(result.data as any);
    }
  } catch (error) {
    console.error('检查登录状态失败', error);
  }
}

function openLoginDialog() {
  userStore.openLoginDialog();
}

async function handleLogout() {
  try {
    await invoke('logout');
    userInfo.value = null;
    userStore.clearUserInfo();
    ElMessage.success('已退出登录');
  } catch (error) {
    ElMessage.error(`退出失败: ${error}`);
  }
}

// ==================== 下载中心相关 ====================

// 下载中心通过子组件暴露的方法
const bilibiliViewRef = ref<any>(null);

const showDownloadCenter = computed({
  get: () => bilibiliViewRef.value?.showDownloadCenter ?? false,
  set: (val) => {
    if (bilibiliViewRef.value) {
      bilibiliViewRef.value.showDownloadCenter = val;
    }
  }
});

const getDownloadingCount = () => {
  return bilibiliViewRef.value?.getDownloadingCount?.() ?? 0;
};

// 提供给子组件
provide('bilibiliViewRef', bilibiliViewRef);

// ==================== 窗口控制 ====================

const isMaximized = ref(false);

async function minimizeWindow() {
  const appWindow = getCurrentWindow();
  await appWindow.minimize();
}

async function toggleMaximize() {
  const appWindow = getCurrentWindow();
  if (await appWindow.isMaximized()) {
    await appWindow.unmaximize();
    isMaximized.value = false;
  } else {
    await appWindow.maximize();
    isMaximized.value = true;
  }
}

async function closeWindow() {
  const appWindow = getCurrentWindow();
  // 最小化到托盘而不是关闭
  await appWindow.hide();
}

// ==================== 生命周期 ====================

onMounted(async () => {
  initTheme();
  
  const splashStart = Date.now();
  const minSplashTime = 3000; // 开屏动画至少显示3秒
  
  // 初始化嵌入资源（yt-dlp, ffmpeg）
  try {
    await invoke('init_resources');
  } catch (error) {
    console.error('初始化失败:', error);
    ElMessage.error('初始化失败，请重新启动程序');
  }
  
  try {
    await checkLoginStatus();
  } catch (error) {
    console.error('检查登录状态失败:', error);
  }
  
  // 确保开屏动画至少显示3秒
  const elapsed = Date.now() - splashStart;
  if (elapsed < minSplashTime) {
    await new Promise(resolve => setTimeout(resolve, minSplashTime - elapsed));
  }
  
  // 初始化完成，关闭开屏窗口并显示主窗口
  try {
    await invoke('close_splashscreen');
  } catch (error) {
    console.error('关闭启动屏幕失败:', error);
  }
  
  // 静默检查更新（启动后延迟3秒检查，不打扰用户）
  setTimeout(() => {
    appStore.checkForUpdate(true);
  }, 3000);
  
  // 监听窗口关闭事件
  const appWindow = getCurrentWindow();
  appWindow.onCloseRequested(async (event) => {
    // 检查是否有正在进行的下载或等待中的任务
    const tasks = bilibiliViewRef.value?.downloadTasks ?? [];
    const hasActiveTasks = tasks.some(
      (t: any) => t.status === 'downloading' || t.status === 'waiting'
    );
    
    if (hasActiveTasks) {
      // 先阻止默认关闭
      event.preventDefault();
      
      // 有活动任务，弹出确认
      try {
        await ElMessageBox.confirm(
          '当前有下载任务正在进行，确定要退出吗？',
          '提示',
          {
            confirmButtonText: '确定退出',
            cancelButtonText: '取消',
            type: 'warning',
          }
        );
        
        // 用户确认退出，先暂停所有进行中和等待中的任务
        for (const task of tasks) {
          if (task.status === 'downloading' || task.status === 'waiting') {
            task.status = 'paused';
          }
        }
        
        // 取消当前下载进程
        try {
          await invoke('cancel_download');
        } catch (e) {
          // 忽略错误
        }
        
        // 保存任务状态
        if (bilibiliViewRef.value?.saveDownloadTasks) {
          await bilibiliViewRef.value.saveDownloadTasks();
        }
        
        // 关闭窗口
        appWindow.destroy();
      } catch {
        // 用户取消，不关闭
      }
    }
  });
  
  // 监听更新下载进度
  await listen<number>('update-download-progress', (event) => {
    appStore.updateDownloadProgress = event.payload;
  });
});

// 监听 store 中的用户信息变化
userStore.$subscribe((_mutation, state) => {
  if (state.userInfo) {
    userInfo.value = state.userInfo as UserInfo;
  } else {
    userInfo.value = null;
  }
});
</script>

<template>
  <div class="app-layout">
    <!-- 顶部标题栏 -->
    <header class="app-header">
      <div class="header-left">
        <div class="logo">
          <svg viewBox="0 0 24 24" width="28" height="28" fill="currentColor">
            <path d="M17.813 4.653h.854c1.51.054 2.769.578 3.773 1.574 1.004.995 1.524 2.249 1.56 3.76v7.36c-.036 1.51-.556 2.769-1.56 3.773s-2.262 1.524-3.773 1.56H5.333c-1.51-.036-2.769-.556-3.773-1.56S.036 18.858 0 17.347v-7.36c.036-1.511.556-2.765 1.56-3.76 1.004-.996 2.262-1.52 3.773-1.574h.774l-1.174-1.12a1.234 1.234 0 0 1-.373-.906c0-.356.124-.659.373-.907l.027-.027c.267-.249.573-.373.92-.373.347 0 .653.124.92.373L9.653 4.44c.071.071.134.142.187.213h4.267a.836.836 0 0 1 .16-.213l2.853-2.747c.267-.249.573-.373.92-.373.347 0 .662.151.929.4.267.249.391.551.391.907 0 .355-.124.657-.373.906l-1.174 1.12zM5.333 7.24c-.746.018-1.373.276-1.88.773-.506.498-.769 1.13-.786 1.894v7.52c.017.764.28 1.395.786 1.893.507.498 1.134.756 1.88.773h13.334c.746-.017 1.373-.275 1.88-.773.506-.498.769-1.129.786-1.893v-7.52c-.017-.765-.28-1.396-.786-1.894-.507-.497-1.134-.755-1.88-.773H5.333zM8 11.107c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c0-.373.129-.689.386-.947.258-.257.574-.386.947-.386zm8 0c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c.017-.391.15-.711.4-.96.249-.249.56-.373.933-.373z"/>
          </svg>
          <span class="logo-text">哔哩哔哩</span>
        </div>
      </div>
      
      <div class="header-right">
        <!-- 窗口控制按钮 -->
        <div class="window-controls">
          <button class="window-btn" @click="minimizeWindow" title="最小化">
            <el-icon><Minus /></el-icon>
          </button>
          <button class="window-btn" @click="toggleMaximize" :title="isMaximized ? '还原' : '最大化'">
            <el-icon><FullScreen /></el-icon>
          </button>
          <button class="window-btn close-btn" @click="closeWindow" title="最小化到托盘">
            <el-icon><CloseBold /></el-icon>
          </button>
        </div>
      </div>
    </header>

    <!-- 主体内容区 -->
    <div class="app-body">
      <!-- 左侧边栏 -->
      <aside class="app-sidebar">
        <div class="sidebar-bottom">
          <!-- 下载中心 -->
          <el-tooltip content="下载中心" placement="right">
            <el-badge 
              :value="getDownloadingCount()" 
              :hidden="getDownloadingCount() === 0" 
              class="sidebar-badge"
            >
              <button class="sidebar-btn" @click="showDownloadCenter = true">
                <el-icon :size="20"><Download /></el-icon>
              </button>
            </el-badge>
          </el-tooltip>
          
          <!-- 主题切换 -->
          <el-tooltip :content="isDarkMode ? '切换浅色' : '切换深色'" placement="right">
            <button class="sidebar-btn" @click="toggleTheme">
              <el-icon :size="20"><Moon v-if="!isDarkMode" /><Sunny v-else /></el-icon>
            </button>
          </el-tooltip>
          
          <!-- 设置 -->
          <el-tooltip content="设置" placement="right">
            <button class="sidebar-btn" @click="showSettings = true">
              <el-icon :size="20"><Setting /></el-icon>
            </button>
          </el-tooltip>
          
          <!-- 用户头像 -->
          <el-tooltip :content="userInfo ? userInfo.username : '点击登录'" placement="right">
            <el-dropdown v-if="userInfo" trigger="click" placement="right-end">
              <button class="sidebar-btn user-btn">
                <el-avatar :src="userInfo.face" :size="28" />
              </button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item @click="handleLogout">
                    <el-icon><Close /></el-icon>
                    退出登录
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
            <button v-else class="sidebar-btn" @click="openLoginDialog">
              <el-icon :size="20"><User /></el-icon>
            </button>
          </el-tooltip>
        </div>
      </aside>

      <!-- 路由内容 -->
      <main class="app-main">
        <router-view v-slot="{ Component }">
          <component 
            :is="Component" 
            ref="bilibiliViewRef"
          />
        </router-view>
      </main>
    </div>

    <!-- 设置抽屉 -->
    <el-drawer
      v-model="showSettings"
      title="设置"
      direction="rtl"
      size="400px"
    >
      <SettingsPanel />
    </el-drawer>
    
    <!-- 更新提示对话框 -->
    <UpdateDialog v-model="appStore.showUpdateDialog" />
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
  background: var(--bg-primary);
}

/* 顶部标题栏 */
.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 40px;
  padding: 0 8px 0 16px;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  -webkit-app-region: drag;
}

.header-left {
  display: flex;
  align-items: center;
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #fb7299;
}

.logo-text {
  font-size: 15px;
  font-weight: 600;
  color: #fb7299;
}

.header-right {
  display: flex;
  align-items: center;
  -webkit-app-region: no-drag;
}

/* 窗口控制按钮 */
.window-controls {
  display: flex;
  align-items: center;
  gap: 0;
}

.window-btn {
  width: 46px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.window-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.window-btn.close-btn:hover {
  background: #e81123;
  color: #fff;
}

/* 主体区域 */
.app-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* 左侧边栏 */
.app-sidebar {
  width: 48px;
  background: var(--bg-card);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  flex-shrink: 0;
}

.sidebar-bottom {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 8px 0 12px;
  gap: 4px;
}

.sidebar-btn {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: 8px;
  transition: all 0.15s;
}

.sidebar-btn:hover {
  background: var(--bg-hover);
  color: #fb7299;
}

.sidebar-btn.user-btn {
  padding: 0;
}

.sidebar-badge {
  display: flex;
}

.sidebar-badge :deep(.el-badge__content) {
  top: 4px;
  right: 8px;
}

/* 主内容区 */
.app-main {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
</style>
