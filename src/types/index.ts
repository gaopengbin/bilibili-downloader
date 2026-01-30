// ==================== 通用类型定义 ====================

// API 响应格式
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// 分页数据
export interface PagedData<T> {
  items: T[];
  has_more: boolean;
  cursor?: number;
}

// 视频格式（清晰度）
export interface VideoFormat {
  height: number | null;
  format_note: string | null;
  format_id: string;
}

// 视频分P信息
export interface VideoEntry {
  index: number;
  id: string;
  title: string;
  duration: number | null;
  url: string;
  thumbnail: string | null;
}

// 通用视频信息
export interface VideoInfo {
  title: string;
  uploader: string | null;
  duration: number | null;
  thumbnail: string | null;
  description: string | null;
  formats: VideoFormat[];
  is_playlist: boolean;
  entries: VideoEntry[];
  // 平台特有信息
  platform?: string;
  platformData?: any;
}

// 下载任务状态
export type DownloadStatus = 'downloading' | 'completed' | 'failed' | 'paused' | 'waiting';

// 下载任务
export interface DownloadTask {
  id: string;
  title: string;
  cover: string | null;
  status: DownloadStatus;
  progress: number;
  stage: string;
  speed?: string;
  downloaded?: string;
  totalSize?: string;
  createdAt: number;
  completedAt?: number;
  error?: string;
  // 恢复下载所需信息
  downloadInfo?: DownloadInfo;
  // 组任务支持
  groupId?: string;
  isGroup?: boolean;
  childIds?: string[];
  totalCount?: number;
  completedCount?: number;
  failedCount?: number;
  retryCount?: number;
  // 平台标识
  platform?: string;
}

// 下载信息
export interface DownloadInfo {
  url: string;
  outputDir: string;
  tempDir: string | null;
  finalDir: string | null;
  quality: string | null;
  videoTitle: string | null;
  isPlaylistItem: boolean;
  entryIndex: number | null;
  entryTitle: string | null;
  expectedId: string | null;
}

// 下载进度详情
export interface ProgressDetail {
  task_id?: string;
  percent: number;
  stage: string;
  stage_index: number;
  speed?: string;
  downloaded?: string;
  total_size?: string;
}

// 下载选项
export interface DownloadOptions {
  url: string;
  outputDir: string;
  quality?: string;
  videoTitle?: string;
}

// 用户设置
export interface UserSettings {
  maxConcurrentDownloads: number;
  defaultOutputDir: string;
  defaultQuality: string;
  aria2cConnections: number;
  preferCodec: string;
  maxRetryCount: number;
}

// 默认设置
export const defaultSettings: UserSettings = {
  maxConcurrentDownloads: 3,
  defaultOutputDir: '',
  defaultQuality: '',
  aria2cConnections: 16,
  preferCodec: '',
  maxRetryCount: 3,
};
