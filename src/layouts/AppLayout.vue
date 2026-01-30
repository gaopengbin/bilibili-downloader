<script setup lang="ts">
import { ref, onMounted, provide, computed } from "vue";
import { useRouter, useRoute } from 'vue-router';
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ElMessage, ElMessageBox } from 'element-plus';
import { 
  Download, Close, Sunny, Moon, Setting, Back
} from '@element-plus/icons-vue';

// 引入组件
import { SettingsPanel, UpdateDialog } from '@/components';
import { useAppStore, useUserStore } from '@/stores';

// 引入类型
import type { UserInfo, ApiResponse } from '@/types';

const router = useRouter();
const route = useRoute();
const appStore = useAppStore();
const userStore = useUserStore();

// ==================== 状态 ====================

const userInfo = ref<UserInfo | null>(null);
const isDarkMode = ref(false);
const showSettings = ref(false);

// 当前平台信息
const currentPlatform = computed(() => {
  const name = route.name as string;
  const platformMap: Record<string, { name: string; color: string }> = {
    Bilibili: { name: '哔哩哔哩', color: '#fb7299' },
    // 未来可添加更多平台
  };
  return platformMap[name] || null;
});

// 是否在平台页面（非首页）
const isInPlatform = computed(() => {
  return route.path !== '/' && currentPlatform.value !== null;
});

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

// ==================== 导航 ====================

function goBack() {
  router.push('/');
}

function goHome() {
  router.push('/');
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
    <!-- 顶部导航栏 -->
    <header class="app-header">
      <div class="header-left">
        <!-- 返回按钮（仅在平台页面显示） -->
        <el-button 
          v-if="isInPlatform" 
          class="back-btn" 
          text 
          @click="goBack"
        >
          <el-icon><Back /></el-icon>
        </el-button>
        
        <div class="logo" @click="goHome" style="cursor: pointer;">
          <svg viewBox="0 0 24 24" width="32" height="32" fill="currentColor">
            <path d="M17.813 4.653h.854c1.51.054 2.769.578 3.773 1.574 1.004.995 1.524 2.249 1.56 3.76v7.36c-.036 1.51-.556 2.769-1.56 3.773s-2.262 1.524-3.773 1.56H5.333c-1.51-.036-2.769-.556-3.773-1.56S.036 18.858 0 17.347v-7.36c.036-1.511.556-2.765 1.56-3.76 1.004-.996 2.262-1.52 3.773-1.574h.774l-1.174-1.12a1.234 1.234 0 0 1-.373-.906c0-.356.124-.659.373-.907l.027-.027c.267-.249.573-.373.92-.373.347 0 .653.124.92.373L9.653 4.44c.071.071.134.142.187.213h4.267a.836.836 0 0 1 .16-.213l2.853-2.747c.267-.249.573-.373.92-.373.347 0 .662.151.929.4.267.249.391.551.391.907 0 .355-.124.657-.373.906l-1.174 1.12zM5.333 7.24c-.746.018-1.373.276-1.88.773-.506.498-.769 1.13-.786 1.894v7.52c.017.764.28 1.395.786 1.893.507.498 1.134.756 1.88.773h13.334c.746-.017 1.373-.275 1.88-.773.506-.498.769-1.129.786-1.893v-7.52c-.017-.765-.28-1.396-.786-1.894-.507-.497-1.134-.755-1.88-.773H5.333zM8 11.107c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c0-.373.129-.689.386-.947.258-.257.574-.386.947-.386zm8 0c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c.017-.391.15-.711.4-.96.249-.249.56-.373.933-.373z"/>
          </svg>
          <span class="logo-text">
            {{ currentPlatform?.name || '视频下载器' }}
          </span>
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
        
        <!-- 下载中心按钮（仅在平台页面显示） -->
        <el-badge 
          v-if="isInPlatform"
          :value="getDownloadingCount()" 
          :hidden="getDownloadingCount() === 0" 
          class="download-badge"
        >
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
            登录
          </el-button>
        </template>
      </div>
    </header>

    <!-- 路由内容 -->
    <main class="app-main">
      <router-view v-slot="{ Component }">
        <component 
          :is="Component" 
          :ref="(el: any) => { if (route.name === 'Bilibili') bilibiliViewRef = el; }"
        />
      </router-view>
    </main>

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

.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 56px;
  padding: 0 16px;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  -webkit-app-region: drag;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  -webkit-app-region: no-drag;
}

.back-btn {
  padding: 8px;
  color: var(--text-secondary);
}

.back-btn:hover {
  color: var(--primary-color);
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--primary-color);
}

.logo-text {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  -webkit-app-region: no-drag;
}

.header-btn {
  color: var(--text-secondary);
  background: var(--bg-hover);
  border: none;
  display: flex;
  align-items: center;
  gap: 4px;
}

.header-btn:hover {
  background: var(--bg-active);
  color: var(--text-primary);
}

.download-badge {
  margin-left: 4px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 8px;
  transition: background-color 0.2s;
}

.user-info:hover {
  background: var(--bg-hover);
}

.username {
  font-size: 14px;
  color: var(--text-primary);
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.login-btn {
  background: var(--primary-color);
  color: #fff;
  border: none;
}

.login-btn:hover {
  background: var(--primary-hover);
}

.app-main {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
</style>
