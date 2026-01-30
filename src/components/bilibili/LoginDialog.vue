<script setup lang="ts">
import { onUnmounted } from 'vue';
import { storeToRefs } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import { Refresh } from '@element-plus/icons-vue';
import { useUserStore } from '@/stores';
import type { ApiResponse } from '@/types';
import type { BilibiliQrCodeResult, BilibiliQrCodeStatus, BilibiliUserInfo } from '@/types/bilibili';

const userStore = useUserStore();
const { showLoginDialog, qrcodeBase64, qrcodeKey, qrcodeStatus } = storeToRefs(userStore);

const emit = defineEmits<{
  (e: 'login-success', user: BilibiliUserInfo): void;
}>();

// 刷新二维码
async function refreshQrCode() {
  qrcodeStatus.value = '正在获取二维码...';
  
  try {
    const result = await invoke<ApiResponse<BilibiliQrCodeResult>>('get_qrcode');
    
    if (result.success && result.data) {
      userStore.setQrcode(result.data.qrcode_base64, result.data.qrcode_key);
      qrcodeStatus.value = '请使用哔哩哔哩APP扫码登录';
      startPolling();
    } else {
      qrcodeStatus.value = result.error || '获取二维码失败';
    }
  } catch (error) {
    qrcodeStatus.value = `获取二维码失败: ${error}`;
  }
}

// 开始轮询
function startPolling() {
  userStore.startPolling(async () => {
    try {
      const result = await invoke<ApiResponse<BilibiliQrCodeStatus>>('poll_qrcode', {
        qrcodeKey: qrcodeKey.value
      });
      
      if (result.success && result.data) {
        const status = result.data.status;
        qrcodeStatus.value = result.data.message;
        
        if (status === 'success') {
          userStore.stopPolling();
          userStore.closeLoginDialog();
          // 获取用户信息
          await checkLoginStatus();
          ElMessage.success('登录成功');
        } else if (status === 'expired') {
          userStore.stopPolling();
        }
      }
    } catch (error) {
      console.error('轮询失败', error);
    }
  }, 2000);
}

// 检查登录状态
async function checkLoginStatus() {
  try {
    const result = await invoke<ApiResponse<BilibiliUserInfo>>('check_login_status');
    if (result.success && result.data) {
      userStore.setUserInfo(result.data);
      emit('login-success', result.data);
    }
  } catch (error) {
    console.error('检查登录状态失败', error);
  }
}

// 关闭对话框
function handleClose() {
  userStore.closeLoginDialog();
}

// 对话框打开时获取二维码
async function handleOpen() {
  await refreshQrCode();
}

// 组件卸载时停止轮询
onUnmounted(() => {
  userStore.stopPolling();
});
</script>

<template>
  <el-dialog
    v-model="showLoginDialog"
    title=""
    width="360px"
    :show-close="true"
    :before-close="handleClose"
    class="login-dialog"
    @open="handleOpen"
  >
    <div class="qrcode-content">
      <div class="qrcode-title">扫码登录</div>
      <div class="qrcode-wrapper">
        <img v-if="qrcodeBase64" :src="qrcodeBase64" class="qrcode-img" />
        <div v-else class="qrcode-loading">
          <el-icon class="is-loading" :size="32"><Refresh /></el-icon>
        </div>
      </div>
      <p class="qrcode-tip">{{ qrcodeStatus }}</p>
      <el-button 
        v-if="qrcodeStatus.includes('过期')" 
        type="primary" 
        @click="refreshQrCode"
      >
        刷新二维码
      </el-button>
    </div>
  </el-dialog>
</template>

<style scoped>
.login-dialog :deep(.el-dialog__header) {
  display: none;
}

.qrcode-content {
  text-align: center;
  padding: 16px;
}

.qrcode-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 20px;
}

.qrcode-wrapper {
  width: 180px;
  height: 180px;
  margin: 0 auto 16px;
  border-radius: 8px;
  overflow: hidden;
  background: var(--bg-hover);
  display: flex;
  align-items: center;
  justify-content: center;
}

.qrcode-img {
  width: 100%;
  height: 100%;
}

.qrcode-loading {
  color: #fb7299;
}

.qrcode-tip {
  color: var(--text-secondary);
  font-size: 13px;
  margin-bottom: 12px;
}
</style>
