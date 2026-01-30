<script setup lang="ts">
import { useRouter } from 'vue-router';

const router = useRouter();

const platforms = [
  {
    id: 'bilibili',
    name: '哔哩哔哩',
    icon: '📺',
    color: '#fb7299',
    description: '支持视频、番剧、合集下载',
  },
  // 未来可添加更多平台
  // {
  //   id: 'youtube',
  //   name: 'YouTube',
  //   icon: '▶️',
  //   color: '#ff0000',
  //   description: '支持视频、播放列表下载',
  // },
];

function selectPlatform(platformId: string) {
  router.push(`/${platformId}`);
}
</script>

<template>
  <div class="home-container">
    <div class="home-header">
      <h1 class="home-title">视频下载器</h1>
      <p class="home-subtitle">选择平台开始下载</p>
    </div>
    
    <div class="platform-grid">
      <div 
        v-for="platform in platforms" 
        :key="platform.id"
        class="platform-card"
        :style="{ '--platform-color': platform.color }"
        @click="selectPlatform(platform.id)"
      >
        <div class="platform-icon">{{ platform.icon }}</div>
        <div class="platform-info">
          <div class="platform-name">{{ platform.name }}</div>
          <div class="platform-desc">{{ platform.description }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.home-container {
  min-height: 100vh;
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
}

.home-header {
  text-align: center;
  margin-bottom: 48px;
}

.home-title {
  font-size: 32px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.home-subtitle {
  font-size: 16px;
  color: var(--text-secondary);
}

.platform-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 20px;
  max-width: 800px;
  width: 100%;
}

.platform-card {
  background: var(--bg-card);
  border-radius: 16px;
  padding: 24px;
  display: flex;
  align-items: center;
  gap: 16px;
  cursor: pointer;
  transition: all 0.3s;
  border: 2px solid transparent;
}

.platform-card:hover {
  transform: translateY(-4px);
  border-color: var(--platform-color);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.1);
}

.platform-icon {
  font-size: 48px;
  width: 72px;
  height: 72px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-hover);
  border-radius: 12px;
}

.platform-info {
  flex: 1;
}

.platform-name {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.platform-desc {
  font-size: 14px;
  color: var(--text-secondary);
}
</style>
