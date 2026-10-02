// ==================== B站特有类型定义 ====================

import type { VideoInfo } from './index';

export interface BilibiliHotSearchItem {
  rank: number;
  keyword: string;
  title: string;
  heat: number;
}

export interface BilibiliRankingItem {
  rank: number;
  bvid: string;
  title: string;
  cover: string | null;
  duration: string;
  author: string;
  play: number;
  danmaku: number;
}

export interface BilibiliRankingResult {
  items: BilibiliRankingItem[];
  note: string;
}

export const bilibiliRankingCategories = [
  { label: '全站', value: 0 },
  { label: '动画', value: 1 },
  { label: '音乐', value: 3 },
  { label: '游戏', value: 4 },
  { label: '知识', value: 36 },
  { label: '科技', value: 188 },
  { label: '生活', value: 160 },
  { label: '美食', value: 211 },
  { label: '影视', value: 181 },
];

// B站用户信息
export interface BilibiliUserInfo {
  username: string;
  face: string;
  level: number;
  mid: number;
}

// B站历史记录项
export interface BilibiliHistoryItem {
  bvid: string;
  title: string;
  cover: string | null;
  duration: number;
  progress: number;
  view_at: number;
  author: string;
}

// B站收藏夹
export interface BilibiliFavoriteFolder {
  id: number;
  title: string;
  media_count: number;
  cover: string | null;
}

// B站收藏项
export interface BilibiliFavoriteItem {
  bvid: string;
  title: string;
  cover: string | null;
  duration: number;
  author: string;
  fav_time: number;
}

// B站搜索结果项
export interface BilibiliSearchResultItem {
  bvid: string;
  title: string;
  cover: string | null;
  duration: string;
  author: string;
  play: number;
  danmaku: number;
  pubdate: number;
  description: string;
}

// B站搜索结果
export interface BilibiliSearchResult {
  items: BilibiliSearchResultItem[];
  page: number;
  page_size: number;
  total: number;
  has_more: boolean;
}

// B站合集单集
export interface BilibiliSeasonEpisode {
  bvid: string;
  aid: number;
  title: string;
  cover: string | null;
  duration: number;
}

// B站合集信息
export interface BilibiliSeasonInfo {
  season_id: number;
  title: string;
  cover: string | null;
  total: number;
  mid: number;
  episodes: BilibiliSeasonEpisode[];
}

// B站视频信息（扩展通用 VideoInfo）
export interface BilibiliVideoInfo extends VideoInfo {
  season: BilibiliSeasonInfo | null;
}

// B站二维码登录结果
export interface BilibiliQrCodeResult {
  qrcode_base64: string;
  qrcode_key: string;
}

// B站二维码状态
export interface BilibiliQrCodeStatus {
  status: string;
  message: string;
}

// B站搜索类型
export type BilibiliSearchType = 'video' | 'media_bangumi' | 'media_ft';

// B站搜索类型选项
export const bilibiliSearchTypes = [
  { label: '视频', value: 'video' as BilibiliSearchType },
  { label: '番剧', value: 'media_bangumi' as BilibiliSearchType },
  { label: '影视', value: 'media_ft' as BilibiliSearchType },
];
