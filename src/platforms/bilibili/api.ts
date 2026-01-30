/**
 * B站平台 API 封装
 * 封装所有与 Tauri 后端通信的 B站相关 API 调用
 */

import { invoke } from '@tauri-apps/api/core';
import type { 
  VideoInfo, 
  UserInfo, 
  HistoryItem, 
  FavoriteFolder, 
  FavoriteItem,
  SearchResult,
  PagedData,
  ApiResponse,
  VideoEntry
} from '@/types';

/**
 * 搜索视频
 */
export async function searchVideo(
  keyword: string, 
  page: number = 1, 
  searchType: 'video' | 'media_bangumi' | 'media_ft' = 'video'
): Promise<ApiResponse<SearchResult>> {
  return invoke<ApiResponse<SearchResult>>('search_video', {
    keyword,
    page,
    searchType,
  });
}

/**
 * 获取视频信息
 */
export async function getVideoInfo(url: string): Promise<ApiResponse<VideoInfo>> {
  return invoke<ApiResponse<VideoInfo>>('get_video_info', { url });
}

/**
 * 获取视频分P信息
 */
export async function getVideoEntries(bvid: string): Promise<ApiResponse<{ entries: VideoEntry[] }>> {
  return invoke<ApiResponse<{ entries: VideoEntry[] }>>('get_video_info', { 
    url: `https://www.bilibili.com/video/${bvid}` 
  });
}

/**
 * 检查登录状态
 */
export async function checkLoginStatus(): Promise<ApiResponse<UserInfo>> {
  return invoke<ApiResponse<UserInfo>>('check_login_status');
}

/**
 * 退出登录
 */
export async function logout(): Promise<void> {
  return invoke('logout');
}

/**
 * 获取历史记录
 */
export async function getHistory(viewAt: number = 0): Promise<ApiResponse<PagedData<HistoryItem>>> {
  return invoke<ApiResponse<PagedData<HistoryItem>>>('get_history', { viewAt });
}

/**
 * 获取收藏夹列表
 */
export async function getFavoriteFolders(mid: number): Promise<ApiResponse<FavoriteFolder[]>> {
  return invoke<ApiResponse<FavoriteFolder[]>>('get_favorite_folders', { mid });
}

/**
 * 获取收藏夹内容
 */
export async function getFavoriteContent(
  folderId: number, 
  page: number = 1
): Promise<ApiResponse<PagedData<FavoriteItem>>> {
  return invoke<ApiResponse<PagedData<FavoriteItem>>>('get_favorite_content', { folderId, page });
}

/**
 * 开始下载
 */
export async function startDownload(options: {
  url: string;
  outputDir: string;
  quality?: string | null;
  videoTitle?: string | null;
  entryIndex?: number | null;
  entryTitle?: string | null;
  taskId?: string | null;
}): Promise<void> {
  return invoke('start_download', options);
}

/**
 * 取消下载
 */
export async function cancelDownload(): Promise<void> {
  return invoke('cancel_download');
}

/**
 * 构建B站视频URL
 */
export function buildVideoUrl(bvid: string): string {
  return `https://www.bilibili.com/video/${bvid}`;
}

/**
 * 从URL中提取BV号
 */
export function extractBvid(url: string): string | null {
  const match = url.match(/BV[a-zA-Z0-9]+/);
  return match ? match[0] : null;
}

/**
 * B站 URL 匹配模式
 */
export const URL_PATTERNS = [
  /bilibili\.com\/video\//,
  /bilibili\.com\/bangumi\/play\//,
  /b23\.tv\//,
  /^BV[a-zA-Z0-9]+$/,
  /^av\d+$/i,
];

/**
 * 检查URL是否是B站链接
 */
export function isBilibiliUrl(url: string): boolean {
  return URL_PATTERNS.some(pattern => pattern.test(url));
}
