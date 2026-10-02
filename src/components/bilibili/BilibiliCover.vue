<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { VideoPlay } from '@element-plus/icons-vue';

const props = defineProps<{ src: string | null }>();
const root = ref<HTMLElement>();
const image = ref('');
let visible = false;
let requestId = 0;
let observer: IntersectionObserver | undefined;

async function load() {
  const id = ++requestId;
  image.value = '';
  if (!props.src || !visible) return;
  if (props.src.startsWith('data:')) {
    image.value = props.src;
    return;
  }
  try {
    const result = await invoke<string>('fetch_image_as_base64', { url: props.src });
    if (id === requestId) image.value = result;
  } catch {
    // Keep the placeholder when a cover is unavailable; video selection still works.
  }
}

watch(() => props.src, load);
onMounted(() => {
  observer = new IntersectionObserver(entries => {
    if (entries.some(entry => entry.isIntersecting)) {
      visible = true;
      observer?.disconnect();
      void load();
    }
  }, { rootMargin: '100px' });
  if (root.value) observer.observe(root.value);
});
onUnmounted(() => { requestId++; observer?.disconnect(); });
</script>

<template>
  <div ref="root" class="bilibili-cover">
    <el-image v-if="image" :src="image" fit="cover">
      <template #error><el-icon :size="24"><VideoPlay /></el-icon></template>
    </el-image>
    <el-icon v-else :size="24"><VideoPlay /></el-icon>
  </div>
</template>

<style scoped>
.bilibili-cover { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: var(--bg-hover); color: var(--text-muted); }
.el-image { width: 100%; height: 100%; }
</style>
