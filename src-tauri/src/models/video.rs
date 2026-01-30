use serde::{Deserialize, Serialize};

/// 视频格式信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoFormat {
    pub height: Option<u32>,
    pub format_note: Option<String>,
    pub format_id: String,
}

/// 多P视频中的单个分P
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoEntry {
    pub index: usize,
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
    pub url: String,
    pub thumbnail: Option<String>,
}

/// 视频信息
#[derive(Debug, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub description: Option<String>,
    pub formats: Vec<VideoFormat>,
    /// 是否是多P视频
    pub is_playlist: bool,
    pub entries: Vec<VideoEntry>,
    /// 合集信息
    pub season: Option<SeasonInfo>,
}

/// 视频合集信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeasonInfo {
    pub season_id: u64,
    pub title: String,
    pub cover: Option<String>,
    pub total: u32,
    pub mid: u64,
    pub episodes: Vec<SeasonEpisode>,
}

/// 合集中的单集
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeasonEpisode {
    pub bvid: String,
    pub aid: u64,
    pub title: String,
    pub cover: Option<String>,
    pub duration: u64,
}

/// 历史记录项
#[derive(Debug, Serialize, Clone)]
pub struct HistoryItem {
    pub bvid: String,
    pub title: String,
    pub cover: Option<String>,
    pub duration: u64,
    pub progress: u64,
    pub view_at: u64,
    pub author: String,
}

/// 收藏夹信息
#[derive(Debug, Serialize, Clone)]
pub struct FavoriteFolder {
    pub id: u64,
    pub title: String,
    pub media_count: u32,
    pub cover: Option<String>,
}

/// 收藏夹内的视频
#[derive(Debug, Serialize, Clone)]
pub struct FavoriteItem {
    pub bvid: String,
    pub title: String,
    pub cover: Option<String>,
    pub duration: u64,
    pub author: String,
    pub fav_time: u64,
}

/// 搜索结果项
#[derive(Debug, Serialize, Clone)]
pub struct SearchResultItem {
    pub bvid: String,
    pub title: String,
    pub cover: Option<String>,
    pub duration: String,
    pub author: String,
    pub play: u64,
    pub danmaku: u64,
    pub pubdate: u64,
    pub description: String,
}

/// 搜索结果
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub items: Vec<SearchResultItem>,
    pub page: u32,
    pub page_size: u32,
    pub total: u32,
    pub has_more: bool,
}

/// 分页数据
#[derive(Debug, Serialize)]
pub struct PagedData<T> {
    pub items: Vec<T>,
    pub has_more: bool,
    pub cursor: Option<u64>,
}
