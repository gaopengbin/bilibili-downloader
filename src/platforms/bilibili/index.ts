/**
 * B站平台实现
 * 实现 Platform 接口，封装所有 B站相关功能
 */

import type { VideoInfo, UserInfo, DownloadTask } from '@/types';
import type { 
  Platform, 
  PlatformInfo, 
  PlatformUserInfo, 
  ParseResult, 
  DownloadOptions 
} from '../interface';
import * as api from './api';

// B站平台信息
const BILIBILI_INFO: PlatformInfo = {
  id: 'bilibili',
  name: '哔哩哔哩',
  icon: '📺', // 可以替换为 SVG
  color: '#fb7299',
  urlPatterns: api.URL_PATTERNS,
};

/**
 * B站平台类
 */
export class BilibiliPlatform implements Platform {
  readonly info = BILIBILI_INFO;
  
  private _userInfo: UserInfo | null = null;

  /**
   * 检查 URL 是否属于 B站
   */
  matchUrl(url: string): boolean {
    return api.isBilibiliUrl(url);
  }

  /**
   * 解析视频信息
   */
  async parseVideo(url: string): Promise<ParseResult> {
    try {
      const result = await api.getVideoInfo(url);
      
      if (result.success && result.data) {
        return {
          success: true,
          data: result.data,
        };
      }
      
      return {
        success: false,
        error: result.error || '解析失败',
      };
    } catch (error) {
      return {
        success: false,
        error: String(error),
      };
    }
  }

  /**
   * 获取可用清晰度
   */
  getQualities(videoInfo: VideoInfo): Array<{ label: string; value: string }> {
    return videoInfo.formats
      .filter(f => f.height)
      .map(f => ({
        label: `${f.height}p`,
        value: f.height?.toString() || '',
      }));
  }

  /**
   * 开始下载
   */
  async startDownload(options: DownloadOptions): Promise<DownloadTask> {
    await api.startDownload({
      url: options.url,
      outputDir: options.outputDir,
      quality: options.quality || null,
      videoTitle: options.videoTitle || null,
      entryIndex: options.entryIndex || null,
      entryTitle: options.entryTitle || null,
    });
    
    // 返回一个占位任务，实际任务管理由 useDownloadTasks 处理
    return {
      id: Date.now().toString(),
      title: options.videoTitle || 'Download',
      status: 'waiting',
      progress: 0,
      stage: '',
      cover: null,
      downloadInfo: {
        url: options.url,
        outputDir: options.outputDir,
        tempDir: null,
        finalDir: options.outputDir,
        quality: options.quality || null,
        videoTitle: options.videoTitle || null,
        isPlaylistItem: false,
        entryIndex: options.entryIndex || null,
        entryTitle: options.entryTitle || null,
        expectedId: null,
      },
      createdAt: Date.now(),
      platform: 'bilibili',
    };
  }

  /**
   * 取消下载
   */
  async cancelDownload(_taskId: string): Promise<void> {
    await api.cancelDownload();
  }

  /**
   * 检查是否已登录
   */
  isLoggedIn(): boolean {
    return this._userInfo !== null;
  }

  /**
   * 获取用户信息
   */
  getUserInfo(): PlatformUserInfo | null {
    if (!this._userInfo) return null;
    
    return {
      id: this._userInfo.mid,
      username: this._userInfo.username,
      avatar: this._userInfo.face,
      // B站特有字段
      mid: this._userInfo.mid,
      face: this._userInfo.face,
    };
  }

  /**
   * 检查登录状态
   */
  async checkLoginStatus(): Promise<UserInfo | null> {
    try {
      const result = await api.checkLoginStatus();
      if (result.success && result.data) {
        this._userInfo = result.data;
        return result.data;
      }
      this._userInfo = null;
      return null;
    } catch {
      this._userInfo = null;
      return null;
    }
  }

  /**
   * 退出登录
   */
  async logout(): Promise<void> {
    await api.logout();
    this._userInfo = null;
  }

  /**
   * 设置用户信息（登录成功后调用）
   */
  setUserInfo(userInfo: UserInfo): void {
    this._userInfo = userInfo;
  }

  // ==================== B站特有功能 ====================

  /**
   * 搜索视频
   */
  async search(
    keyword: string, 
    page: number = 1, 
    searchType: 'video' | 'media_bangumi' | 'media_ft' = 'video'
  ) {
    return api.searchVideo(keyword, page, searchType);
  }

  /**
   * 获取历史记录
   */
  async getHistory(viewAt: number = 0) {
    return api.getHistory(viewAt);
  }

  /**
   * 获取收藏夹列表
   */
  async getFavoriteFolders() {
    if (!this._userInfo) {
      throw new Error('未登录');
    }
    return api.getFavoriteFolders(this._userInfo.mid);
  }

  /**
   * 获取收藏夹内容
   */
  async getFavoriteContent(folderId: number, page: number = 1) {
    return api.getFavoriteContent(folderId, page);
  }

  /**
   * 获取视频分P信息
   */
  async getVideoEntries(bvid: string) {
    return api.getVideoEntries(bvid);
  }

  /**
   * 构建视频URL
   */
  buildVideoUrl(bvid: string): string {
    return api.buildVideoUrl(bvid);
  }

  /**
   * 提取BV号
   */
  extractBvid(url: string): string | null {
    return api.extractBvid(url);
  }
}

// 导出单例实例
export const bilibiliPlatform = new BilibiliPlatform();
