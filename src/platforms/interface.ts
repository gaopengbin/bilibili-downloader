/**
 * 平台接口定义
 * 所有平台（B站、YouTube等）都需要实现这个接口
 */

import type { VideoInfo, DownloadTask } from '@/types';

// 平台基本信息
export interface PlatformInfo {
  id: string;           // 平台唯一标识，如 'bilibili', 'youtube'
  name: string;         // 平台显示名称
  icon: string;         // 平台图标（可以是 SVG 字符串或图标名）
  color: string;        // 平台主题色
  urlPatterns: RegExp[]; // URL 匹配模式
}

// 平台用户信息（各平台可扩展）
export interface PlatformUserInfo {
  id: string | number;
  username: string;
  avatar: string;
  [key: string]: unknown; // 允许平台特有字段
}

// 视频解析结果
export interface ParseResult {
  success: boolean;
  data?: VideoInfo;
  error?: string;
}

// 下载选项
export interface DownloadOptions {
  url: string;
  outputDir: string;
  quality?: string;
  videoTitle?: string;
  entryIndex?: number;
  entryTitle?: string;
}

// 平台接口
export interface Platform {
  // 平台信息
  readonly info: PlatformInfo;
  
  // 检查 URL 是否属于该平台
  matchUrl(url: string): boolean;
  
  // 解析视频信息
  parseVideo(url: string): Promise<ParseResult>;
  
  // 获取可用清晰度列表
  getQualities(videoInfo: VideoInfo): Array<{ label: string; value: string }>;
  
  // 开始下载
  startDownload(options: DownloadOptions): Promise<DownloadTask>;
  
  // 取消下载
  cancelDownload(taskId: string): Promise<void>;
  
  // 用户相关（可选）
  isLoggedIn?(): boolean;
  login?(): Promise<PlatformUserInfo>;
  logout?(): Promise<void>;
  getUserInfo?(): PlatformUserInfo | null;
}

// 平台注册表
const platformRegistry = new Map<string, Platform>();

// 注册平台
export function registerPlatform(platform: Platform): void {
  platformRegistry.set(platform.info.id, platform);
}

// 获取平台
export function getPlatform(id: string): Platform | undefined {
  return platformRegistry.get(id);
}

// 获取所有平台
export function getAllPlatforms(): Platform[] {
  return Array.from(platformRegistry.values());
}

// 根据 URL 匹配平台
export function matchPlatformByUrl(url: string): Platform | undefined {
  for (const platform of platformRegistry.values()) {
    if (platform.matchUrl(url)) {
      return platform;
    }
  }
  return undefined;
}
