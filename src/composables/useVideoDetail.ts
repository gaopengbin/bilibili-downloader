import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { 
  VideoInfo, VideoEntry, VideoFormat, SeasonEpisode, SeasonInfo, ApiResponse 
} from '@/types';

// 重新导出类型供外部使用
export type { VideoInfo, VideoEntry, VideoFormat, SeasonEpisode, SeasonInfo };

export function useVideoDetail() {
  // ==================== 状态 ====================
  const videoUrl = ref('');
  const videoInfo = ref<VideoInfo | null>(null);
  const selectedQuality = ref('');
  const loading = ref(false);
  const descExpanded = ref(false);
  
  // 分P选择
  const selectedEntries = ref<number[]>([]);
  
  // 合集选择
  const selectedSeasonEpisodes = ref<string[]>([]);
  const expandedSeasonItems = ref<Set<string>>(new Set());
  const seasonItemEntries = ref<Map<string, VideoEntry[]>>(new Map());
  const seasonItemLoading = ref<Set<string>>(new Set());
  const selectedSeasonEntries = ref<Map<string, number[]>>(new Map());
  
  // 临时保存封面
  const pendingCover = ref<string | null>(null);

  // ==================== 计算属性 ====================
  
  const isCurrentVideo = (bvid: string): boolean => {
    if (!videoUrl.value) return false;
    return videoUrl.value.includes(bvid);
  };

  // ==================== 方法 ====================
  
  // 获取当前视频的 BV 号
  function getCurrentBvid(): string | null {
    const url = videoUrl.value;
    const match = url.match(/BV[a-zA-Z0-9]+/);
    return match ? match[0] : null;
  }

  // 分P全选/取消全选
  function toggleSelectAll() {
    if (!videoInfo.value) return;
    if (selectedEntries.value.length === videoInfo.value.entries.length) {
      selectedEntries.value = [];
    } else {
      selectedEntries.value = videoInfo.value.entries.map(e => e.index);
    }
  }

  // 合集全选/取消全选
  function toggleSeasonSelectAll() {
    if (!videoInfo.value?.season) return;
    if (selectedSeasonEpisodes.value.length === videoInfo.value.season.episodes.length) {
      selectedSeasonEpisodes.value = [];
      selectedSeasonEntries.value.clear();
      selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
    } else {
      selectedSeasonEpisodes.value = videoInfo.value.season.episodes.map(e => e.bvid);
      for (const ep of videoInfo.value.season.episodes) {
        const entries = seasonItemEntries.value.get(ep.bvid);
        if (entries && entries.length > 0) {
          selectedSeasonEntries.value.set(ep.bvid, entries.map(e => e.index));
        }
      }
      selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
    }
  }

  // 切换单个合集项的选中状态
  function toggleSeasonEpisodeSelect(bvid: string) {
    const idx = selectedSeasonEpisodes.value.indexOf(bvid);
    if (idx >= 0) {
      selectedSeasonEpisodes.value.splice(idx, 1);
      selectedSeasonEntries.value.delete(bvid);
      selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
    } else {
      selectedSeasonEpisodes.value.push(bvid);
      const entries = seasonItemEntries.value.get(bvid);
      if (entries && entries.length > 0) {
        selectedSeasonEntries.value.set(bvid, entries.map(e => e.index));
        selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
      }
    }
  }

  // 展开/折叠合集项，获取分P信息
  async function toggleSeasonItemExpand(bvid: string) {
    if (expandedSeasonItems.value.has(bvid)) {
      expandedSeasonItems.value.delete(bvid);
      expandedSeasonItems.value = new Set(expandedSeasonItems.value);
      return;
    }
    
    if (seasonItemEntries.value.has(bvid)) {
      expandedSeasonItems.value.add(bvid);
      expandedSeasonItems.value = new Set(expandedSeasonItems.value);
      return;
    }
    
    const currentBvid = getCurrentBvid();
    if (currentBvid === bvid && videoInfo.value && videoInfo.value.entries.length > 0) {
      seasonItemEntries.value.set(bvid, videoInfo.value.entries);
      seasonItemEntries.value = new Map(seasonItemEntries.value);
      
      if (videoInfo.value.entries.length > 1) {
        selectedSeasonEntries.value.set(bvid, videoInfo.value.entries.map(e => e.index));
        selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
      }
      
      expandedSeasonItems.value.add(bvid);
      expandedSeasonItems.value = new Set(expandedSeasonItems.value);
      return;
    }
    
    seasonItemLoading.value.add(bvid);
    seasonItemLoading.value = new Set(seasonItemLoading.value);
    
    try {
      const result = await invoke<ApiResponse<VideoInfo>>('get_video_info', {
        url: `https://www.bilibili.com/video/${bvid}`
      });
      
      if (result.success && result.data) {
        seasonItemEntries.value.set(bvid, result.data.entries);
        seasonItemEntries.value = new Map(seasonItemEntries.value);
        
        if (result.data.entries.length > 1) {
          selectedSeasonEntries.value.set(bvid, result.data.entries.map(e => e.index));
          selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
        }
        
        expandedSeasonItems.value.add(bvid);
        expandedSeasonItems.value = new Set(expandedSeasonItems.value);
      }
    } catch (error) {
      console.error('获取分P信息失败:', error);
    } finally {
      seasonItemLoading.value.delete(bvid);
      seasonItemLoading.value = new Set(seasonItemLoading.value);
    }
  }

  // 同步外层合集项的选中状态
  function syncSeasonEpisodeSelection(bvid: string) {
    const entries = selectedSeasonEntries.value.get(bvid) || [];
    const isSelected = selectedSeasonEpisodes.value.includes(bvid);
    
    if (entries.length > 0 && !isSelected) {
      selectedSeasonEpisodes.value.push(bvid);
    } else if (entries.length === 0 && isSelected) {
      const idx = selectedSeasonEpisodes.value.indexOf(bvid);
      if (idx >= 0) {
        selectedSeasonEpisodes.value.splice(idx, 1);
      }
    }
  }

  // 切换合集项内分P的选择
  function toggleSeasonEntrySelect(bvid: string, entryIndex: number) {
    const current = selectedSeasonEntries.value.get(bvid) || [];
    const idx = current.indexOf(entryIndex);
    if (idx >= 0) {
      current.splice(idx, 1);
    } else {
      current.push(entryIndex);
    }
    selectedSeasonEntries.value.set(bvid, current);
    selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
    
    syncSeasonEpisodeSelection(bvid);
  }

  // 合集项内全选/取消全选分P
  function toggleSeasonItemSelectAll(bvid: string) {
    const entries = seasonItemEntries.value.get(bvid) || [];
    const current = selectedSeasonEntries.value.get(bvid) || [];
    
    if (current.length === entries.length) {
      selectedSeasonEntries.value.set(bvid, []);
    } else {
      selectedSeasonEntries.value.set(bvid, entries.map(e => e.index));
    }
    selectedSeasonEntries.value = new Map(selectedSeasonEntries.value);
    
    syncSeasonEpisodeSelection(bvid);
  }

  // 获取合集项选中的分P数量
  function getSeasonItemSelectedCount(bvid: string): number {
    return selectedSeasonEntries.value.get(bvid)?.length || 0;
  }

  // 判断合集项是否为半选状态
  function isSeasonItemIndeterminate(bvid: string): boolean {
    const entries = seasonItemEntries.value.get(bvid);
    if (!entries || entries.length <= 1) return false;
    
    const selectedCount = selectedSeasonEntries.value.get(bvid)?.length || 0;
    return selectedCount > 0 && selectedCount < entries.length;
  }

  // 判断合集项是否全选
  function isSeasonItemFullySelected(bvid: string): boolean {
    const entries = seasonItemEntries.value.get(bvid);
    if (!entries || entries.length === 0) {
      return selectedSeasonEpisodes.value.includes(bvid);
    }
    
    const selectedCount = selectedSeasonEntries.value.get(bvid)?.length || 0;
    return selectedCount === entries.length && selectedCount > 0;
  }

  // 重置选择状态
  function resetSelection() {
    selectedEntries.value = [];
    selectedSeasonEpisodes.value = [];
    expandedSeasonItems.value = new Set();
    seasonItemEntries.value = new Map();
    seasonItemLoading.value = new Set();
    selectedSeasonEntries.value = new Map();
    descExpanded.value = false;
  }

  // 设置视频信息
  function setVideoInfo(info: VideoInfo | null) {
    videoInfo.value = info;
    if (info) {
      // 设置默认清晰度
      if (info.formats.length > 0) {
        const defaultFormat = info.formats.find(f => f.height === 1080) || info.formats[0];
        selectedQuality.value = defaultFormat.height?.toString() || '';
      }
      // 如果是多P，默认全选
      if (info.is_playlist && info.entries.length > 0 && !info.season) {
        selectedEntries.value = info.entries.map(e => e.index);
      }
    } else {
      resetSelection();
    }
  }

  // 格式化时长
  function formatDuration(seconds: number | null): string {
    if (!seconds) return '--:--';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    if (h > 0) {
      return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
    }
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  return {
    // 状态
    videoUrl,
    videoInfo,
    selectedQuality,
    loading,
    descExpanded,
    selectedEntries,
    selectedSeasonEpisodes,
    expandedSeasonItems,
    seasonItemEntries,
    seasonItemLoading,
    selectedSeasonEntries,
    pendingCover,
    
    // 计算属性/方法
    isCurrentVideo,
    getCurrentBvid,
    
    // 分P选择方法
    toggleSelectAll,
    
    // 合集选择方法
    toggleSeasonSelectAll,
    toggleSeasonEpisodeSelect,
    toggleSeasonItemExpand,
    toggleSeasonEntrySelect,
    toggleSeasonItemSelectAll,
    getSeasonItemSelectedCount,
    isSeasonItemIndeterminate,
    isSeasonItemFullySelected,
    
    // 其他方法
    resetSelection,
    setVideoInfo,
    formatDuration,
  };
}
