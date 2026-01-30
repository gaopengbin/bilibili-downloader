// 模块声明（渐进式重构用）
pub mod models;
pub mod utils;
// pub mod commands; // 待完全迁移后启用

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// 嵌入二进制文件
const YTDLP_BINARY: &[u8] = include_bytes!("../yt-dlp.exe");
const FFMPEG_BINARY: &[u8] = include_bytes!("../ffmpeg.exe");
const ARIA2C_BINARY: &[u8] = include_bytes!("../aria2c.exe");

// 初始化嵌入资源，返回资源目录路径
// 使用 LOCALAPPDATA 目录而不是临时目录，避免被系统清理
fn init_embedded_resources() -> Result<PathBuf, String> {
    // 使用 LOCALAPPDATA 目录
    let base_dir = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    
    let tools_dir = base_dir.join("bilibili-downloader-tools");
    
    // 创建目录
    fs::create_dir_all(&tools_dir).map_err(|e| e.to_string())?;
    
    // 解压 yt-dlp
    let ytdlp_path = tools_dir.join("yt-dlp.exe");
    if !ytdlp_path.exists() {
        let mut file = fs::File::create(&ytdlp_path).map_err(|e| e.to_string())?;
        file.write_all(YTDLP_BINARY).map_err(|e| e.to_string())?;
    }
    
    // 解压 ffmpeg
    let ffmpeg_path = tools_dir.join("ffmpeg.exe");
    if !ffmpeg_path.exists() {
        let mut file = fs::File::create(&ffmpeg_path).map_err(|e| e.to_string())?;
        file.write_all(FFMPEG_BINARY).map_err(|e| e.to_string())?;
    }
    
    // 解压 aria2c
    let aria2c_path = tools_dir.join("aria2c.exe");
    if !aria2c_path.exists() {
        let mut file = fs::File::create(&aria2c_path).map_err(|e| e.to_string())?;
        file.write_all(ARIA2C_BINARY).map_err(|e| e.to_string())?;
    }
    
    Ok(tools_dir)
}

// Tauri 命令：初始化资源
#[tauri::command]
fn init_resources() -> Result<ApiResponse<String>, String> {
    match init_embedded_resources() {
        Ok(path) => Ok(ApiResponse {
            success: true,
            data: Some(path.to_string_lossy().to_string()),
            error: None,
        }),
        Err(e) => Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(e),
        }),
    }
}

// Tauri 命令：关闭开屏窗口并显示主窗口
#[tauri::command]
async fn close_splashscreen(app: tauri::AppHandle) {
    // 关闭开屏窗口
    if let Some(splash) = app.get_webview_window("splashscreen") {
        let _ = splash.close();
    }
    // 显示主窗口
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

// ==================== 数据结构 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoFormat {
    pub height: Option<u32>,
    pub format_note: Option<String>,
    pub format_id: String,
}

// 多P视频中的单个分P
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoEntry {
    pub index: usize,
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
    pub url: String,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub description: Option<String>,
    pub formats: Vec<VideoFormat>,
    // 多P视频支持
    pub is_playlist: bool,
    pub entries: Vec<VideoEntry>,
    // 合集信息
    pub season: Option<SeasonInfo>,
}

// 视频合集信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeasonInfo {
    pub season_id: u64,
    pub title: String,
    pub cover: Option<String>,
    pub total: u32,
    pub mid: u64,           // UP主 ID
    pub episodes: Vec<SeasonEpisode>,
}

// 合集中的单集
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeasonEpisode {
    pub bvid: String,
    pub aid: u64,
    pub title: String,
    pub cover: Option<String>,
    pub duration: u64,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QrCodeResult {
    pub qrcode_base64: String,
    pub qrcode_key: String,
}

#[derive(Debug, Serialize)]
pub struct QrCodeStatus {
    pub status: String,
    pub message: String,
    pub cookies: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub username: String,
    pub face: String,
    pub level: u32,
    pub mid: u64, // 用户ID，用于获取收藏夹
}

// 历史记录项
#[derive(Debug, Serialize, Clone)]
pub struct HistoryItem {
    pub bvid: String,
    pub title: String,
    pub cover: Option<String>,
    pub duration: u64,
    pub progress: u64, // 观看进度（秒）
    pub view_at: u64,  // 观看时间戳
    pub author: String,
}

// 收藏夹信息
#[derive(Debug, Serialize, Clone)]
pub struct FavoriteFolder {
    pub id: u64,
    pub title: String,
    pub media_count: u32,
    pub cover: Option<String>,
}

// 收藏夹内的视频
#[derive(Debug, Serialize, Clone)]
pub struct FavoriteItem {
    pub bvid: String,
    pub title: String,
    pub cover: Option<String>,
    pub duration: u64,
    pub author: String,
    pub fav_time: u64, // 收藏时间戳
}

// 搜索结果项
#[derive(Debug, Serialize, Clone)]
pub struct SearchResultItem {
    pub bvid: String,
    pub title: String,
    pub cover: Option<String>,
    pub duration: String,
    pub author: String,
    pub play: u64,      // 播放量
    pub danmaku: u64,   // 弹幕数
    pub pubdate: u64,   // 发布时间
    pub description: String,
}

// 搜索结果
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub items: Vec<SearchResultItem>,
    pub page: u32,
    pub page_size: u32,
    pub total: u32,
    pub has_more: bool,
}

// 分页数据
#[derive(Debug, Serialize)]
pub struct PagedData<T> {
    pub items: Vec<T>,
    pub has_more: bool,
    pub cursor: Option<u64>, // 用于历史记录的cursor分页
}

// ==================== 全局状态 ====================

struct AppState {
    cookies: Mutex<Option<HashMap<String, String>>>,
    cookies_file: Mutex<Option<String>>,
    // 当前下载进程 ID，用于暂停
    current_download_pid: Mutex<Option<u32>>,
}

// ==================== 工具函数 ====================

fn get_ytdlp_path(_app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    // 开发环境
    let dev_path = std::env::current_dir()
        .map(|p| p.join("src-tauri").join("yt-dlp.exe"))
        .unwrap_or_default();
    
    if dev_path.exists() {
        return Ok(dev_path);
    }

    // 生产环境：使用嵌入的资源
    let temp_dir = init_embedded_resources()?;
    Ok(temp_dir.join("yt-dlp.exe"))
}

fn get_cookies_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    fs::create_dir_all(&app_data).map_err(|e| format!("Failed to create app data dir: {}", e))?;

    Ok(app_data.join("cookies.txt"))
}

// ==================== 二维码登录 ====================

#[tauri::command]
async fn get_qrcode() -> Result<ApiResponse<QrCodeResult>, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get("https://passport.bilibili.com/x/passport-login/web/qrcode/generate")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json["code"].as_i64() != Some(0) {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(json["message"].as_str().unwrap_or("获取二维码失败").to_string()),
        });
    }

    let qrcode_url = json["data"]["url"].as_str().unwrap_or("");
    let qrcode_key = json["data"]["qrcode_key"].as_str().unwrap_or("");

    // 生成二维码图片
    let qr = qrcode::QrCode::new(qrcode_url.as_bytes()).map_err(|e| e.to_string())?;
    let image = qr.render::<image::Luma<u8>>().build();

    // 转为 PNG 并 base64 编码
    let mut png_data = Cursor::new(Vec::new());
    image
        .write_to(&mut png_data, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    let base64_str = base64::engine::general_purpose::STANDARD.encode(png_data.into_inner());

    Ok(ApiResponse {
        success: true,
        data: Some(QrCodeResult {
            qrcode_base64: format!("data:image/png;base64,{}", base64_str),
            qrcode_key: qrcode_key.to_string(),
        }),
        error: None,
    })
}

#[tauri::command]
async fn poll_qrcode(
    app_handle: tauri::AppHandle,
    qrcode_key: String,
) -> Result<ApiResponse<QrCodeStatus>, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .cookie_store(true)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get("https://passport.bilibili.com/x/passport-login/web/qrcode/poll")
        .query(&[("qrcode_key", &qrcode_key)])
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 提取 cookies
    let cookies: HashMap<String, String> = resp
        .cookies()
        .map(|c| (c.name().to_string(), c.value().to_string()))
        .collect();

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let code = json["data"]["code"].as_i64().unwrap_or(-1);

    let (status, message) = match code {
        0 => {
            // 登录成功，保存 cookies
            if !cookies.is_empty() {
                let cookies_path = get_cookies_path(&app_handle)?;
                save_cookies_to_file(&cookies, &cookies_path)?;

                // 更新状态
                let state = app_handle.state::<AppState>();
                *state.cookies.lock().unwrap() = Some(cookies.clone());
                *state.cookies_file.lock().unwrap() = Some(cookies_path.to_string_lossy().to_string());
            }
            ("success", "登录成功")
        }
        86038 => ("expired", "二维码已过期，请刷新"),
        86090 => ("scanned", "已扫描，请在手机上确认"),
        86101 => ("waiting", "等待扫描"),
        _ => ("unknown", "未知状态"),
    };

    Ok(ApiResponse {
        success: true,
        data: Some(QrCodeStatus {
            status: status.to_string(),
            message: message.to_string(),
            cookies: if code == 0 { Some(cookies) } else { None },
        }),
        error: None,
    })
}

fn save_cookies_to_file(cookies: &HashMap<String, String>, path: &PathBuf) -> Result<(), String> {
    let mut lines = vec![
        "# Netscape HTTP Cookie File".to_string(),
        "# https://curl.haxx.se/docs/http-cookies.html".to_string(),
        "".to_string(),
    ];

    let expires = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 86400 * 365)
        .to_string();

    for (name, value) in cookies {
        lines.push(format!(
            ".bilibili.com\tTRUE\t/\tFALSE\t{}\t{}\t{}",
            expires, name, value
        ));
    }

    fs::write(path, lines.join("\n")).map_err(|e| format!("保存 cookies 失败: {}", e))
}

// 图片代理 - 下载图片并转换为 base64
#[tauri::command]
async fn fetch_image_as_base64(url: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let base64_str = base64::engine::general_purpose::STANDARD.encode(&bytes);
    
    // 检测图片类型
    let content_type = if url.contains(".png") {
        "image/png"
    } else if url.contains(".gif") {
        "image/gif"
    } else {
        "image/jpeg"
    };
    
    Ok(format!("data:{};base64,{}", content_type, base64_str))
}

#[tauri::command]
async fn get_user_info(app_handle: tauri::AppHandle) -> Result<ApiResponse<UserInfo>, String> {
    let state = app_handle.state::<AppState>();
    let cookies_opt = state.cookies.lock().unwrap().clone();

    let cookies = match cookies_opt {
        Some(c) => c,
        None => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some("未登录".to_string()),
            })
        }
    };

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let cookie_header: String = cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");

    let resp = client
        .get("https://api.bilibili.com/x/web-interface/nav")
        .header("Cookie", cookie_header)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json["code"].as_i64() != Some(0) {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some("获取用户信息失败".to_string()),
        });
    }

    // 转换头像为 base64
    let face_url = json["data"]["face"].as_str().unwrap_or("");
    let face_base64 = if !face_url.is_empty() {
        fetch_image_as_base64(face_url.to_string()).await.unwrap_or(face_url.to_string())
    } else {
        String::new()
    };

    Ok(ApiResponse {
        success: true,
        data: Some(UserInfo {
            username: json["data"]["uname"].as_str().unwrap_or("未知").to_string(),
            face: face_base64,
            level: json["data"]["level_info"]["current_level"].as_u64().unwrap_or(0) as u32,
            mid: json["data"]["mid"].as_u64().unwrap_or(0),
        }),
        error: None,
    })
}

// 获取观看历史
#[tauri::command]
async fn get_history(
    app_handle: tauri::AppHandle,
    view_at: u64, // cursor分页，传0表示第一页
) -> Result<ApiResponse<PagedData<HistoryItem>>, String> {
    let state = app_handle.state::<AppState>();
    let cookies_opt = state.cookies.lock().unwrap().clone();

    let cookies = match cookies_opt {
        Some(c) => c,
        None => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some("未登录".to_string()),
            })
        }
    };

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let cookie_header: String = cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");

    // B站历史记录API - 使用cursor分页
    let ps = "20".to_string();
    let type_str = "archive".to_string();
    let view_at_str = view_at.to_string();
    let resp = client
        .get("https://api.bilibili.com/x/web-interface/history/cursor")
        .query(&[
            ("ps", &ps),
            ("type", &type_str),
            ("view_at", &view_at_str),
        ])
        .header("Cookie", cookie_header)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json["code"].as_i64() != Some(0) {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(json["message"].as_str().unwrap_or("获取历史记录失败").to_string()),
        });
    }

    let mut items = Vec::new();
    let mut last_view_at: u64 = 0;
    
    if let Some(list) = json["data"]["list"].as_array() {
        for item in list {
            // 只处理视频类型
            if item["history"]["business"].as_str() != Some("archive") {
                continue;
            }
            
            let item_view_at = item["view_at"].as_u64().unwrap_or(0);
            last_view_at = item_view_at;
            
            // 直接返回封面URL，让前端加载，避免同步下载导致变慢
            let cover_url = item["cover"].as_str().unwrap_or("").to_string();
            
            items.push(HistoryItem {
                bvid: item["history"]["bvid"].as_str().unwrap_or("").to_string(),
                title: item["title"].as_str().unwrap_or("").to_string(),
                cover: if cover_url.is_empty() { None } else { Some(cover_url) },
                duration: item["duration"].as_u64().unwrap_or(0),
                progress: item["progress"].as_u64().unwrap_or(0),
                view_at: item_view_at,
                author: item["author_name"].as_str().unwrap_or("").to_string(),
            });
        }
    }

    // 使用cursor的max判断是否还有更多
    let cursor_max = json["data"]["cursor"]["max"].as_u64().unwrap_or(0);
    let has_more = cursor_max > 0 && !items.is_empty();

    Ok(ApiResponse {
        success: true,
        data: Some(PagedData { 
            items, 
            has_more,
            cursor: if has_more { Some(last_view_at) } else { None },
        }),
        error: None,
    })
}

// 获取收藏夹列表
#[tauri::command]
async fn get_favorite_folders(
    app_handle: tauri::AppHandle,
    mid: u64,
) -> Result<ApiResponse<Vec<FavoriteFolder>>, String> {
    let state = app_handle.state::<AppState>();
    let cookies_opt = state.cookies.lock().unwrap().clone();

    let cookies = match cookies_opt {
        Some(c) => c,
        None => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some("未登录".to_string()),
            })
        }
    };

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let cookie_header: String = cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");

    let resp = client
        .get("https://api.bilibili.com/x/v3/fav/folder/created/list-all")
        .query(&[("up_mid", &mid.to_string())])
        .header("Cookie", cookie_header)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json["code"].as_i64() != Some(0) {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(json["message"].as_str().unwrap_or("获取收藏夹失败").to_string()),
        });
    }

    let mut folders = Vec::new();
    if let Some(list) = json["data"]["list"].as_array() {
        for folder in list {
            let cover_url = folder["cover"].as_str().unwrap_or("");
            let cover_base64 = if !cover_url.is_empty() {
                fetch_image_as_base64(cover_url.to_string()).await.ok()
            } else {
                None
            };
            
            folders.push(FavoriteFolder {
                id: folder["id"].as_u64().unwrap_or(0),
                title: folder["title"].as_str().unwrap_or("").to_string(),
                media_count: folder["media_count"].as_u64().unwrap_or(0) as u32,
                cover: cover_base64,
            });
        }
    }

    Ok(ApiResponse {
        success: true,
        data: Some(folders),
        error: None,
    })
}

// 搜索视频
#[tauri::command]
async fn search_video(
    app_handle: tauri::AppHandle,
    keyword: String,
    page: u32,
    search_type: Option<String>, // video, bangumi, live 等
) -> Result<ApiResponse<SearchResult>, String> {
    let state = app_handle.state::<AppState>();
    let cookies_opt = state.cookies.lock().unwrap().clone();

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    // 构建 cookie header（可选，未登录也能搜索）
    let cookie_header: String = cookies_opt
        .map(|c| c.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("; "))
        .unwrap_or_default();

    let search_type = search_type.unwrap_or("video".to_string());
    let ps = "20".to_string();
    
    let resp = client
        .get("https://api.bilibili.com/x/web-interface/search/type")
        .query(&[
            ("search_type", &search_type),
            ("keyword", &keyword),
            ("page", &page.to_string()),
            ("page_size", &ps),
        ])
        .header("Cookie", cookie_header)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Referer", "https://search.bilibili.com")
        .header("Origin", "https://search.bilibili.com")
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    // 检查响应状态
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("搜索请求失败: {} - {}", status, text.chars().take(100).collect::<String>())),
        });
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json["code"].as_i64() != Some(0) {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(json["message"].as_str().unwrap_or("搜索失败").to_string()),
        });
    }

    let mut items = Vec::new();
    let total = json["data"]["numResults"].as_u64().unwrap_or(0) as u32;
    let page_size = json["data"]["pagesize"].as_u64().unwrap_or(20) as u32;
    let is_media = search_type == "media_bangumi" || search_type == "media_ft";
    
    if let Some(results) = json["data"]["result"].as_array() {
        for item in results {
            // 番剧/影视和视频的字段不同
            let cover_url = if is_media {
                item["cover"].as_str().unwrap_or("")
            } else {
                item["pic"].as_str().unwrap_or("")
            };
            let full_cover_url = if cover_url.starts_with("//") {
                format!("https:{}", cover_url)
            } else if cover_url.is_empty() {
                String::new()
            } else {
                cover_url.to_string()
            };
            
            // 直接返回图片 URL，不再预先下载（加速搜索）
            let cover = if full_cover_url.is_empty() { None } else { Some(full_cover_url) };
            
            // 清理标题中的高亮标签
            let title = item["title"].as_str().unwrap_or("")
                .replace("<em class=\"keyword\">", "")
                .replace("</em>", "");
            
            if is_media {
                // 番剧/影视：使用 season_id 构建链接
                let season_id = item["season_id"].as_u64().unwrap_or(0);
                let url = format!("https://www.bilibili.com/bangumi/play/ss{}", season_id);
                
                // 获取评分
                let score = item["media_score"]["score"].as_f64().unwrap_or(0.0);
                let score_str = if score > 0.0 { format!("{:.1}分", score) } else { String::new() };
                
                items.push(SearchResultItem {
                    bvid: url, // 番剧没有bvid，用完整url代替
                    title,
                    cover,
                    duration: item["index_show"].as_str().unwrap_or("").to_string(), // "全12集"
                    author: item["areas"].as_str().unwrap_or("").to_string(), // "日本"/"中国大陆"
                    play: 0,
                    danmaku: 0,
                    pubdate: item["pubtime"].as_u64().unwrap_or(0),
                    description: format!("{} {}", item["styles"].as_str().unwrap_or(""), score_str).trim().to_string(),
                });
            } else {
                // 视频
                items.push(SearchResultItem {
                    bvid: item["bvid"].as_str().unwrap_or("").to_string(),
                    title,
                    cover,
                    duration: item["duration"].as_str().unwrap_or("0:00").to_string(),
                    author: item["author"].as_str().unwrap_or("").to_string(),
                    play: item["play"].as_u64().unwrap_or(0),
                    danmaku: item["danmaku"].as_u64().unwrap_or(0),
                    pubdate: item["pubdate"].as_u64().unwrap_or(0),
                    description: item["description"].as_str().unwrap_or("").chars().take(100).collect(),
                });
            }
        }
    }

    let has_more = (page * page_size) < total;

    Ok(ApiResponse {
        success: true,
        data: Some(SearchResult {
            items,
            page,
            page_size,
            total,
            has_more,
        }),
        error: None,
    })
}

// 获取收藏夹内容
#[tauri::command]
async fn get_favorite_content(
    app_handle: tauri::AppHandle,
    folder_id: u64,
    page: u32,
) -> Result<ApiResponse<PagedData<FavoriteItem>>, String> {
    let state = app_handle.state::<AppState>();
    let cookies_opt = state.cookies.lock().unwrap().clone();

    let cookies = match cookies_opt {
        Some(c) => c,
        None => {
            return Ok(ApiResponse {
                success: false,
                data: None,
                error: Some("未登录".to_string()),
            })
        }
    };

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let cookie_header: String = cookies
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("; ");

    let ps = "20".to_string();
    let platform = "web".to_string();
    let resp = client
        .get("https://api.bilibili.com/x/v3/fav/resource/list")
        .query(&[
            ("media_id", &folder_id.to_string()),
            ("pn", &page.to_string()),
            ("ps", &ps),
            ("platform", &platform),
        ])
        .header("Cookie", cookie_header)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json["code"].as_i64() != Some(0) {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(json["message"].as_str().unwrap_or("获取收藏内容失败").to_string()),
        });
    }

    let mut items = Vec::new();
    if let Some(medias) = json["data"]["medias"].as_array() {
        for item in medias {
            // 直接返回封面URL，让前端加载，避免同步下载导致变慢
            let cover_url = item["cover"].as_str().unwrap_or("").to_string();
            
            items.push(FavoriteItem {
                bvid: item["bvid"].as_str().unwrap_or("").to_string(),
                title: item["title"].as_str().unwrap_or("").to_string(),
                cover: if cover_url.is_empty() { None } else { Some(cover_url) },
                duration: item["duration"].as_u64().unwrap_or(0),
                author: item["upper"]["name"].as_str().unwrap_or("").to_string(),
                fav_time: item["fav_time"].as_u64().unwrap_or(0),
            });
        }
    }

    let has_more = json["data"]["has_more"].as_bool().unwrap_or(false);

    Ok(ApiResponse {
        success: true,
        data: Some(PagedData { items, has_more, cursor: None }),
        error: None,
    })
}

#[tauri::command]
async fn logout(app_handle: tauri::AppHandle) -> Result<ApiResponse<()>, String> {
    let state = app_handle.state::<AppState>();
    *state.cookies.lock().unwrap() = None;
    *state.cookies_file.lock().unwrap() = None;

    // 删除 cookies 文件
    if let Ok(path) = get_cookies_path(&app_handle) {
        let _ = fs::remove_file(path);
    }

    Ok(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

#[tauri::command]
async fn check_login_status(app_handle: tauri::AppHandle) -> Result<ApiResponse<UserInfo>, String> {
    // 尝试从文件加载 cookies
    let cookies_path = get_cookies_path(&app_handle)?;
    if cookies_path.exists() {
        if let Ok(content) = fs::read_to_string(&cookies_path) {
            let mut cookies = HashMap::new();
            for line in content.lines() {
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 7 {
                    cookies.insert(parts[5].to_string(), parts[6].to_string());
                }
            }
            if !cookies.is_empty() {
                let state = app_handle.state::<AppState>();
                *state.cookies.lock().unwrap() = Some(cookies);
                *state.cookies_file.lock().unwrap() = Some(cookies_path.to_string_lossy().to_string());
                
                // 验证 cookies 是否有效
                return get_user_info(app_handle).await;
            }
        }
    }

    Ok(ApiResponse {
        success: false,
        data: None,
        error: Some("未登录".to_string()),
    })
}

// ==================== 视频解析和下载 ====================

/// 通过 B站 API 获取视频合集信息
/// 直接从 ugc_season.sections[].episodes 获取合集列表
async fn fetch_season_info(bvid: &str) -> Option<SeasonInfo> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .ok()?;

    // 获取视频详情，查看是否属于合集
    let resp = client
        .get("https://api.bilibili.com/x/web-interface/view")
        .query(&[("bvid", bvid)])
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .ok()?;

    let json: serde_json::Value = resp.json().await.ok()?;
    
    if json["code"].as_i64() != Some(0) {
        return None;
    }

    // 检查是否有 ugc_season 字段
    let ugc_season = &json["data"]["ugc_season"];
    if ugc_season.is_null() {
        return None;
    }

    let season_id = ugc_season["id"].as_u64()?;
    let title = ugc_season["title"].as_str()?.to_string();
    let mid = ugc_season["mid"].as_u64().unwrap_or(0);
    let cover = ugc_season["cover"].as_str().map(|s| s.to_string());
    let ep_count = ugc_season["ep_count"].as_u64().unwrap_or(0) as u32;

    // 直接从 ugc_season.sections[].episodes 获取合集视频列表
    let mut episodes = Vec::new();
    if let Some(sections) = ugc_season["sections"].as_array() {
        for section in sections {
            if let Some(eps) = section["episodes"].as_array() {
                for ep in eps {
                    let ep_bvid = ep["bvid"].as_str().unwrap_or("").to_string();
                    
                    // 跳过没有有效 bvid 的剧集
                    if ep_bvid.is_empty() || !ep_bvid.starts_with("BV") {
                        continue;
                    }
                    
                    let ep_aid = ep["aid"].as_u64().unwrap_or(0);
                    // 优先使用 arc.title（视频原标题），如果没有则用 ep.title
                    let ep_title = ep["arc"]["title"].as_str()
                        .filter(|s| !s.is_empty())
                        .or_else(|| ep["title"].as_str())
                        .unwrap_or("")
                        .to_string();
                    let ep_cover = ep["arc"]["pic"].as_str().map(|s| s.to_string());
                    let ep_duration = ep["arc"]["duration"].as_u64()
                        .or_else(|| ep["page"]["duration"].as_u64())
                        .unwrap_or(0);

                    episodes.push(SeasonEpisode {
                        bvid: ep_bvid,
                        aid: ep_aid,
                        title: ep_title,
                        cover: ep_cover,
                        duration: ep_duration,
                    });
                }
            }
        }
    }

    Some(SeasonInfo {
        season_id,
        title,
        cover,
        total: ep_count,
        mid,
        episodes,
    })
}

/// 从完整标题中提取分P名称
/// 输入格式一般是: "主标题 pXX 分P名" 或 "主标题 pXX. 分P名"
/// 返回格式: "分P名" (保留原始序号如 "01. XXX")
#[allow(dead_code)]
fn extract_part_title(full_title: &str, main_title: &str, _index: usize) -> String {
    // 如果完整标题以主标题开头，尝试移除主标题部分
    // 使用字符边界安全的方式截取
    let remaining = if !main_title.is_empty() && full_title.starts_with(main_title) {
        // 找到 main_title 之后的字符边界
        let char_count = main_title.chars().count();
        full_title.chars().skip(char_count).collect::<String>()
    } else {
        full_title.to_string()
    };
    let remaining = remaining.trim_start();
    
    let chars: Vec<char> = remaining.chars().collect();
    
    // 检查是否以 p/P + 数字 开头，如 "p01 00. XXX"
    if chars.len() > 1 && (chars[0] == 'p' || chars[0] == 'P') && chars[1].is_ascii_digit() {
        // 找到 pXX 后的数字结束位置
        let mut digit_end = 1;
        for (i, &c) in chars.iter().enumerate().skip(1) {
            if c.is_ascii_digit() {
                digit_end = i + 1;
            } else {
                break;
            }
        }
        
        // 只跳过 pXX 后的空格，保留后面的 "01. XXX" 格式
        let mut content_start = digit_end;
        while content_start < chars.len() && chars[content_start] == ' ' {
            content_start += 1;
        }
        
        // 提取剩余内容 (保留 "01. XXX" 格式)
        if content_start < chars.len() {
            let part_title: String = chars[content_start..].iter().collect();
            if !part_title.is_empty() {
                return part_title;
            }
        }
    }
    
    // 如果都没有匹配到模式，返回 remaining
    if !remaining.is_empty() {
        remaining.to_string()
    } else {
        // 返回完整标题
        full_title.to_string()
    }
}

#[tauri::command]
async fn get_video_info(
    app_handle: tauri::AppHandle,
    url: String,
) -> Result<ApiResponse<VideoInfo>, String> {
    // 从 URL 中提取视频ID
    let (bvid, aid, season_id, episode_id) = extract_video_id(&url);
    
    // 获取 cookies
    let state = app_handle.state::<AppState>();
    let cookies_str = state.cookies.lock().unwrap().as_ref().map(|c| {
        c.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("; ")
    });

    // 如果是番剧，使用番剧 API
    if season_id.is_some() || episode_id.is_some() {
        return get_bangumi_info(season_id, episode_id, &cookies_str).await;
    }
    
    if bvid.is_none() && aid.is_none() {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("无法识别视频ID: '{}'，请输入BV号或完整链接", url)),
        });
    }

    // 构建 API URL
    let api_url = if let Some(ref bv) = bvid {
        format!("https://api.bilibili.com/x/web-interface/view?bvid={}", bv)
    } else {
        format!("https://api.bilibili.com/x/web-interface/view?aid={}", aid.unwrap())
    };

    // 请求 B站 API
    let client = reqwest::Client::new();
    let mut req = client.get(&api_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header("Referer", "https://www.bilibili.com/");
    
    if let Some(ref c) = cookies_str {
        req = req.header("Cookie", c);
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    // 检查 API 返回
    if json["code"].as_i64() != Some(0) {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(json["message"].as_str().unwrap_or("获取视频信息失败").to_string()),
        });
    }

    let data = &json["data"];
    
    // 解析分P列表
    let mut entries: Vec<VideoEntry> = Vec::new();
    let is_playlist = data["pages"].as_array().map(|p| p.len() > 1).unwrap_or(false);
    
    if let Some(pages) = data["pages"].as_array() {
        let bv = data["bvid"].as_str().unwrap_or("");
        for page in pages {
            let cid = page["cid"].as_u64().unwrap_or(0);
            let idx = page["page"].as_u64().unwrap_or(1) as usize;
            entries.push(VideoEntry {
                index: idx,
                id: format!("{}_{}", bv, cid),
                title: page["part"].as_str().unwrap_or(&format!("P{}", idx)).to_string(),
                duration: page["duration"].as_f64(),
                url: format!("https://www.bilibili.com/video/{}?p={}", bv, idx),
                thumbnail: None, // 分P不单独获取缩略图，加快速度
            });
        }
    }

    // 获取可用清晰度（从 playurl API）
    let formats = fetch_available_formats(
        data["bvid"].as_str().unwrap_or(""),
        data["cid"].as_u64().unwrap_or(0),
        &cookies_str,
    ).await.unwrap_or_default();

    // 获取封面 URL（不再下载转 base64，加快响应）
    let thumbnail_url = data["pic"].as_str().unwrap_or("");
    let thumbnail = if !thumbnail_url.is_empty() {
        Some(thumbnail_url.to_string())
    } else {
        None
    };

    // 获取合集信息
    let clean_bvid = data["bvid"].as_str().unwrap_or("");
    let season = if !clean_bvid.is_empty() {
        fetch_season_info(clean_bvid).await
    } else {
        None
    };

    Ok(ApiResponse {
        success: true,
        data: Some(VideoInfo {
            title: data["title"].as_str().unwrap_or("未知标题").to_string(),
            uploader: data["owner"]["name"].as_str().map(|s| s.to_string()),
            duration: data["duration"].as_f64(),
            thumbnail,
            description: data["desc"].as_str().map(|s| s.to_string()),
            formats,
            is_playlist,
            entries,
            season,
        }),
        error: None,
    })
}

// 获取番剧信息
async fn get_bangumi_info(
    season_id: Option<u64>,
    episode_id: Option<u64>,
    cookies: &Option<String>,
) -> Result<ApiResponse<VideoInfo>, String> {
    // 构建 API URL
    let api_url = if let Some(ssid) = season_id {
        format!("https://api.bilibili.com/pgc/view/web/season?season_id={}", ssid)
    } else if let Some(epid) = episode_id {
        format!("https://api.bilibili.com/pgc/view/web/season?ep_id={}", epid)
    } else {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some("无效的番剧ID".to_string()),
        });
    };

    let client = reqwest::Client::new();
    let mut req = client.get(&api_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header("Referer", "https://www.bilibili.com/")
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Origin", "https://www.bilibili.com");
    
    if let Some(ref c) = cookies {
        req = req.header("Cookie", c);
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    
    // 检查 HTTP 状态码
    if resp.status() == 412 {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some("请求被拒绝(412)，请稍后重试或登录后再试".to_string()),
        });
    }
    
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    // 检查 API 返回
    if json["code"].as_i64() != Some(0) {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(json["message"].as_str().unwrap_or("获取番剧信息失败").to_string()),
        });
    }

    let result = &json["result"];
    
    // 解析剧集列表
    let mut entries: Vec<VideoEntry> = Vec::new();
    let episodes = result["episodes"].as_array();
    
    // 检查是否有剧集
    let has_episodes = episodes.map(|e| !e.is_empty()).unwrap_or(false);
    
    if let Some(eps) = episodes {
        for (idx, ep) in eps.iter().enumerate() {
            let ep_id = ep["id"].as_u64().unwrap_or(0);
            let _bvid = ep["bvid"].as_str().unwrap_or("");
            let _cid = ep["cid"].as_u64().unwrap_or(0);
            
            entries.push(VideoEntry {
                index: idx + 1,
                id: format!("ep{}", ep_id),
                title: ep["share_copy"].as_str()
                    .or_else(|| ep["long_title"].as_str())
                    .or_else(|| ep["title"].as_str())
                    .unwrap_or(&format!("第{}集", idx + 1))
                    .to_string(),
                duration: ep["duration"].as_f64().map(|d| d / 1000.0), // 毫秒转秒
                url: format!("https://www.bilibili.com/bangumi/play/ep{}", ep_id),
                thumbnail: None,
            });
        }
    }
    
    // 如果没有剧集且未登录，提示用户
    if !has_episodes && cookies.is_none() {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some("该影视需要登录后才能获取剧集列表，请先登录".to_string()),
        });
    }

    // 获取可用清晰度（从第一集）
    let formats = if let Some(eps) = episodes {
        if let Some(first_ep) = eps.first() {
            let bvid = first_ep["bvid"].as_str().unwrap_or("");
            let cid = first_ep["cid"].as_u64().unwrap_or(0);
            if !bvid.is_empty() && cid > 0 {
                fetch_available_formats(bvid, cid, cookies).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // 获取封面 URL（不再下载转 base64）
    let cover_url = result["cover"].as_str().unwrap_or("");
    let thumbnail = if !cover_url.is_empty() {
        Some(cover_url.to_string())
    } else {
        None
    };

    let is_playlist = entries.len() > 1;

    Ok(ApiResponse {
        success: true,
        data: Some(VideoInfo {
            title: result["title"].as_str().unwrap_or("未知番剧").to_string(),
            uploader: result["actors"].as_str()
                .or_else(|| result["staff"].as_str())
                .map(|s| s.to_string()),
            duration: result["total_time"].as_f64(),
            thumbnail,
            description: result["evaluate"].as_str().map(|s| s.to_string()),
            formats,
            is_playlist,
            entries,
            season: None, // 番剧本身就是season
        }),
        error: None,
    })
}

// 从 URL 中提取视频ID (BV号, av号, 番剧ss, 番剧ep)
fn extract_video_id(url: &str) -> (Option<String>, Option<u64>, Option<u64>, Option<u64>) {
    let url = url.trim();
    
    // 纯BV号输入 (如 BV1xx411c7mD)
    if url.starts_with("BV") {
        let bv: String = url.chars().take_while(|c| c.is_alphanumeric()).collect();
        if bv.len() >= 12 {
            return (Some(bv), None, None, None);
        }
    }
    
    // 从URL中提取BV号 (如 https://www.bilibili.com/video/BV1xx411c7mD)
    if let Some(pos) = url.find("BV") {
        let bv: String = url[pos..].chars().take_while(|c| c.is_alphanumeric()).collect();
        if bv.len() >= 12 {
            return (Some(bv), None, None, None);
        }
    }
    
    // b23.tv 短链接中的BV号
    if url.contains("b23.tv") || url.contains("bili2233.cn") {
        // 短链接需要先解析，这里暂时返回空，让前端处理
        // 或者可以在这里做重定向获取真实URL
    }
    
    // 纯av号输入 (如 av170001)
    if url.to_lowercase().starts_with("av") {
        let num_str: String = url[2..].chars().take_while(|c| c.is_numeric()).collect();
        if let Ok(aid) = num_str.parse::<u64>() {
            return (None, Some(aid), None, None);
        }
    }
    
    // 从URL中提取av号 (如 https://www.bilibili.com/video/av170001)
    if let Some(pos) = url.to_lowercase().find("/av") {
        let num_str: String = url[pos+3..].chars().take_while(|c| c.is_numeric()).collect();
        if let Ok(aid) = num_str.parse::<u64>() {
            return (None, Some(aid), None, None);
        }
    }
    
    // 番剧 season_id (如 ss47534 或 https://www.bilibili.com/bangumi/play/ss47534)
    if let Some(pos) = url.to_lowercase().find("ss") {
        let num_str: String = url[pos+2..].chars().take_while(|c| c.is_numeric()).collect();
        if let Ok(ssid) = num_str.parse::<u64>() {
            return (None, None, Some(ssid), None);
        }
    }
    
    // 番剧 episode_id (如 ep123456 或 https://www.bilibili.com/bangumi/play/ep123456)
    if let Some(pos) = url.to_lowercase().find("ep") {
        let num_str: String = url[pos+2..].chars().take_while(|c| c.is_numeric()).collect();
        if let Ok(epid) = num_str.parse::<u64>() {
            return (None, None, None, Some(epid));
        }
    }
    
    (None, None, None, None)
}

// 获取可用清晰度
async fn fetch_available_formats(bvid: &str, cid: u64, cookies: &Option<String>) -> Option<Vec<VideoFormat>> {
    if bvid.is_empty() || cid == 0 {
        return None;
    }
    
    let api_url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn=127&fnval=16&fourk=1",
        bvid, cid
    );

    let client = reqwest::Client::new();
    let mut req = client.get(&api_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header("Referer", "https://www.bilibili.com/")
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Origin", "https://www.bilibili.com");
    
    if let Some(ref c) = cookies {
        req = req.header("Cookie", c);
    }

    let resp = req.send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;

    if json["code"].as_i64() != Some(0) {
        return None;
    }

    // 获取当前权限下可用的最高清晰度
    // B站返回的 quality 字段表示当前用户权限下实际可播放的最高清晰度
    let max_available_qn = json["data"]["quality"].as_u64().unwrap_or(64) as u32;
    
    let mut formats = Vec::new();
    
    if let Some(support_formats) = json["data"]["support_formats"].as_array() {
        for fmt in support_formats {
            let qn_val = fmt["quality"].as_u64().unwrap_or(0) as u32;
            
            // 只添加当前权限可用的清晰度（小于等于 max_available_qn）
            if qn_val > max_available_qn {
                continue;
            }
            
            let desc = fmt["new_description"].as_str()
                .or_else(|| fmt["display_desc"].as_str())
                .map(|s| s.to_string());
            
            let height = match qn_val {
                127 => 2160, // 8K (实际4K HDR)
                126 => 2160, // 杜比视界
                125 => 2160, // HDR
                120 => 2160, // 4K
                116 => 1080, // 1080P60
                112 => 1080, // 1080P+
                80 => 1080,  // 1080P
                74 => 720,   // 720P60
                64 => 720,   // 720P
                32 => 480,   // 480P
                16 => 360,   // 360P
                _ => 0,
            };
            
            if height > 0 {
                formats.push(VideoFormat {
                    height: Some(height),
                    format_note: desc,
                    format_id: qn_val.to_string(),
                });
            }
        }
    }

    // 去重并排序
    formats.sort_by(|a, b| b.height.cmp(&a.height));
    formats.dedup_by(|a, b| a.height == b.height);
    
    Some(formats)
}

#[tauri::command]
async fn download_video(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
    url: String,
    output_dir: String,
    temp_dir: Option<String>,      // 临时下载目录（多P用）
    final_dir: Option<String>,     // 最终目录（多P用）
    quality: Option<String>,
    video_title: Option<String>,   // 视频标题，用于重命名
    is_playlist_item: Option<bool>, // 是否是多P视频的分P
    entry_index: Option<u32>,      // 分P序号，用于重命名
    entry_title: Option<String>,   // 分P标题，用于重命名
    expected_id: Option<String>,   // 期望的yt-dlp生成的ID，用于检测文件
    task_id: Option<String>,       // 任务ID，用于进度追踪
    aria2c_connections: Option<u32>, // aria2c 并发连接数
    prefer_codec: Option<String>,  // 编码偏好: avc / hevc / av1
) -> Result<ApiResponse<String>, String> {
    let ytdlp_path = get_ytdlp_path(&app_handle)?;
    let is_multi_p = is_playlist_item.unwrap_or(false);
    let task_id = task_id.unwrap_or_default();
    let connections = aria2c_connections.unwrap_or(16);
    
    // 处理 BV号/av号，转换为完整URL
    let full_url = if url.starts_with("BV") {
        format!("https://www.bilibili.com/video/{}", url)
    } else if url.starts_with("av") {
        format!("https://www.bilibili.com/video/{}", url)
    } else if !url.starts_with("http") {
        format!("https://www.bilibili.com/video/{}", url)
    } else {
        url.clone()
    };
    
    // 清理文件名中的非法字符的辅助函数
    fn sanitize_filename(name: &str) -> String {
        // 替换非法字符
        let mut s: String = name
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                // 控制字符
                c if c as u32 <= 31 => '_',
                _ => c,
            })
            .collect();
        // 去掉首尾空格和点
        while s.ends_with(' ') || s.ends_with('.') { s.pop(); }
        while s.starts_with(' ') || s.starts_with('.') { s.remove(0); }
        // 限长（按字符数而非字节数，避免在 UTF-8 多字节字符中间截断）
        if s.chars().count() > 80 {
            s = s.chars().take(80).collect();
        }
        // Windows保留名处理（不区分大小写）
        let reserved = [
            "CON","PRN","AUX","NUL","COM1","COM2","COM3","COM4","COM5","COM6","COM7","COM8","COM9",
            "LPT1","LPT2","LPT3","LPT4","LPT5","LPT6","LPT7","LPT8","LPT9"
        ];
        let upper = s.to_uppercase();
        if reserved.contains(&upper.as_str()) { s.push('_'); }
        if s.is_empty() { s = "video".to_string(); }
        s
    }
    
    // 处理文件名
    let safe_title = video_title.as_ref().map(|t| sanitize_filename(t));
    let safe_entry_title = entry_title.as_ref().map(|t| sanitize_filename(t));
    
    // 多P视频使用临时目录下载
    let actual_output_dir = if is_multi_p {
        if let Some(ref temp) = temp_dir {
            // 创建临时目录
            let _ = fs::create_dir_all(temp);
            temp.clone()
        } else {
            output_dir.clone()
        }
    } else {
        output_dir.clone()
    };
    
    // 同时创建最终目录（多P）
    if is_multi_p {
        if let Some(ref final_d) = final_dir {
            let _ = fs::create_dir_all(final_d);
        }
    }
    
    // 使用简单的 ID 模板
    let output_template = "%(id)s.%(ext)s".to_string();
    
    // 获取 ffmpeg 路径
    let dev_ffmpeg = std::env::current_dir()
        .ok()
        .map(|p| p.join("src-tauri").join("ffmpeg.exe"))
        .filter(|p| p.exists());
    
    let ffmpeg_path = if let Some(dev_path) = dev_ffmpeg {
        dev_path
    } else {
        // 生产环境：使用嵌入的资源
        init_embedded_resources()?.join("ffmpeg.exe")
    };
    
    // 获取 aria2c 路径
    let dev_aria2c = std::env::current_dir()
        .ok()
        .map(|p| p.join("src-tauri").join("aria2c.exe"))
        .filter(|p| p.exists());
    
    let aria2c_path = if let Some(dev_path) = dev_aria2c {
        dev_path
    } else {
        init_embedded_resources()?.join("aria2c.exe")
    };

    let mut args = vec![
        "-P".to_string(),
        actual_output_dir.clone(),
        "-o".to_string(),
        output_template,
        "--no-warnings".to_string(),
        "--newline".to_string(),
        // 断点续传支持
        "--continue".to_string(),
        // 编码设置 - 设置响应解码编码
        "--encoding".to_string(),
        "utf-8".to_string(),
        // 忽略错误继续下载
        "--ignore-errors".to_string(),
        // 使用 aria2c 作为外部下载器，支持多线程下载
        "--external-downloader".to_string(),
        aria2c_path.to_string_lossy().to_string(),
        "--external-downloader-args".to_string(),
        format!("aria2c:-x {} -s {} -k 1M --file-allocation=none --summary-interval=1", connections, connections),
        "--progress-template".to_string(),
        "download:PROGRESS:%(progress._percent_str)s:%(info.ext)s:%(progress._speed_str)s:%(progress._downloaded_bytes_str)s:%(progress._total_bytes_str)s".to_string(),
        "--progress-template".to_string(),
        "postprocess:POSTPROCESS:%(info.ext)s".to_string(),
        "--merge-output-format".to_string(),
        "mp4".to_string(),
        // 确保音视频都下载完成后再合并
        "--postprocessor-args".to_string(),
        "ffmpeg:-y".to_string(), // 强制覆盖，避免合并失败
        "--no-check-certificate".to_string(),
        "--windows-filenames".to_string(),
        "--restrict-filenames".to_string(), // 限制文件名只包含ASCII字符
        "--ffmpeg-location".to_string(),
        ffmpeg_path.to_string_lossy().to_string(),
        // 添加请求头解决412错误
        "--add-header".to_string(),
        "User-Agent:Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
        "--add-header".to_string(),
        "Referer:https://www.bilibili.com/".to_string(),
        "--add-header".to_string(),
        "Origin:https://www.bilibili.com".to_string(),
        "--add-header".to_string(),
        "Accept:text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8".to_string(),
        "--add-header".to_string(),
        "Accept-Language:zh-CN,zh;q=0.9,en;q=0.8".to_string(),
        "--add-header".to_string(),
        "Sec-Ch-Ua:\"Google Chrome\";v=\"131\", \"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\"".to_string(),
        "--add-header".to_string(),
        "Sec-Ch-Ua-Mobile:?0".to_string(),
        "--add-header".to_string(),
        "Sec-Ch-Ua-Platform:\"Windows\"".to_string(),
        "--add-header".to_string(),
        "Sec-Fetch-Dest:document".to_string(),
        "--add-header".to_string(),
        "Sec-Fetch-Mode:navigate".to_string(),
        "--add-header".to_string(),
        "Sec-Fetch-Site:same-origin".to_string(),
    ];

    // 构建格式选择字符串，考虑画质和编码偏好
    let codec_filter = match prefer_codec.as_deref() {
        Some("avc") => "[vcodec^=avc]",
        Some("hevc") => "[vcodec^=hev]",
        Some("av1") => "[vcodec^=av01]",
        _ => "",
    };

    if let Some(q) = quality {
        args.push("-f".to_string());
        if codec_filter.is_empty() {
            args.push(format!(
                "bestvideo[height<={}]+bestaudio/best[height<={}]/best",
                q, q
            ));
        } else {
            // 有编码偏好时，优先选择指定编码，fallback 到任意编码
            args.push(format!(
                "bestvideo[height<={}]{}+bestaudio/bestvideo[height<={}]+bestaudio/best[height<={}]/best",
                q, codec_filter, q, q
            ));
        }
    } else {
        args.push("-f".to_string());
        if codec_filter.is_empty() {
            args.push("bestvideo+bestaudio/best".to_string());
        } else {
            args.push(format!(
                "bestvideo{}+bestaudio/bestvideo+bestaudio/best",
                codec_filter
            ));
        }
    }

    // 添加 cookies
    let state = app_handle.state::<AppState>();
    if let Some(ref cookies_file) = *state.cookies_file.lock().unwrap() {
        args.push("--cookies".to_string());
        args.push(cookies_file.clone());
    }

    args.push(full_url.clone());

    // 调试日志：写入文件
    let log_path = std::env::temp_dir().join("bilibili_download_debug.log");
    let log_content = format!(
        "args: {:?}\noutput_dir: {}\nurl: {}\n---\n",
        args, actual_output_dir, full_url
    );
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(log_content.as_bytes())
        });

    // 清除代理环境变量，设置 UTF-8 编码
    let mut child = Command::new(ytdlp_path)
        .args(&args)
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("http_proxy")
        .env_remove("https_proxy")
        .env_remove("ALL_PROXY")
        .env_remove("all_proxy")
        // Python UTF-8 模式
        .env("PYTHONIOENCODING", "utf-8:replace")  // 遇到无法编码的字符用替换而非报错
        .env("PYTHONUTF8", "1")
        .env("PYTHONLEGACYWINDOWSSTDIO", "0")  // 禁用 Windows 传统 stdio
        .env("LANG", "en_US.UTF-8")
        .env("LC_ALL", "en_US.UTF-8")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| format!("Failed to start download: {}", e))?;

    // 保存进程 ID，用于暂停
    let pid = child.id();
    {
        let state = app_handle.state::<AppState>();
        *state.current_download_pid.lock().unwrap() = Some(pid);
    }

    // 同时读取 stdout 和 stderr，避免管道阻塞
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    
    // 在单独线程读取 stderr，解析 aria2c 进度（aria2c 用 \r 而不是 \n）
    let window_clone = window.clone();
    let task_id_clone = task_id.clone();
    let stderr_handle = std::thread::spawn(move || {
        let mut error_output = String::new();
        if let Some(stderr) = stderr {
            use std::io::BufReader;
            let mut reader = BufReader::new(stderr);
            let mut line_buf = String::new();
            let mut current_stage = 0;
            
            // 按字符读取，\r 或 \n 都作为行结束
            loop {
                line_buf.clear();
                let mut buf = [0u8; 1];
                loop {
                    use std::io::Read;
                    match reader.read(&mut buf) {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            let c = buf[0] as char;
                            if c == '\r' || c == '\n' {
                                break;
                            }
                            line_buf.push(c);
                        }
                        Err(_) => break,
                    }
                }
                
                if line_buf.is_empty() {
                    // 检查是否 EOF
                    use std::io::Read;
                    let mut check = [0u8; 1];
                    if reader.read(&mut check).unwrap_or(0) == 0 {
                        break;
                    }
                    continue;
                }
                
                let line = &line_buf;
                
                // 解析 aria2c 进度: [#xxxx 50MiB/100MiB(50%) CN:16 DL:10MiB]
                if line.contains('%') {
                    if let Some(pct_pos) = line.find('%') {
                        let before = &line[..pct_pos];
                        let num_start = before.rfind(|c: char| c == '(' || c == ' ' || c == '[').map(|i| i + 1).unwrap_or(0);
                        if let Ok(p) = before[num_start..].trim().parse::<f64>() {
                            // 检测阶段切换（第二次下载开始时进度会重新从0开始）
                            if p < 5.0 && current_stage == 0 {
                                // 可能是第一次或切换到第二个文件
                            }
                            
                            let mut speed = String::new();
                            let mut downloaded = String::new();
                            let mut total_size = String::new();
                            
                            // 提取速度 DL:xxMiB
                            if let Some(dl_pos) = line.find("DL:") {
                                let after = &line[dl_pos + 3..];
                                let end = after.find(|c: char| c == ']' || c == ' ').unwrap_or(after.len());
                                speed = after[..end].to_string();
                            }
                            
                            // 提取已下载/总大小 50MiB/100MiB
                            // 格式: [#xxxx 50MiB/100MiB(50%)
                            if let Some(size_match) = line.find('/') {
                                // 向前找已下载大小
                                let before_slash = &line[..size_match];
                                let dl_start = before_slash.rfind(' ').map(|i| i + 1).unwrap_or(0);
                                downloaded = before_slash[dl_start..].to_string();
                                
                                // 向后找总大小
                                let after_slash = &line[size_match + 1..];
                                let total_end = after_slash.find('(').unwrap_or(after_slash.len());
                                total_size = after_slash[..total_end].to_string();
                            }
                            
                            // 统一显示"下载中"，不区分视频/音频（顺序不固定）
                            let stage_name = "下载中";
                            
                            let progress_info = serde_json::json!({
                                "task_id": task_id_clone,
                                "percent": p,
                                "stage": stage_name,
                                "stage_index": current_stage,
                                "speed": speed,
                                "downloaded": downloaded,
                                "total_size": total_size
                            });
                            let _ = window_clone.emit("download-progress-detail", progress_info);
                            let _ = window_clone.emit("download-progress", p);
                            
                            // 100% 完成后切换到下一阶段
                            if p >= 99.0 {
                                current_stage += 1;
                            }
                        }
                    }
                }
                
                error_output.push_str(line);
                error_output.push('\n');
            }
        }
        error_output
    });
    
    // 主线程读取 stdout 并发送进度
    // 跟踪当前下载阶段：0=视频, 1=音频, 2=合并
    let mut current_stage = 0;
    let mut stage_count = 0; // 计数器，用于检测阶段切换
    
    if let Some(stdout) = stdout {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stdout);

        for line in reader.lines().map_while(Result::ok) {
            // aria2c 进度格式: [#xxxx 50MiB/100MiB(50%) CN:16 DL:10MiB]
            // 或者: [DL:10MiB][#xxxx 50%]
            if line.contains("DL:") && (line.contains("%)" ) || line.contains("%]")) {
                // 提取百分比
                let mut percent: Option<f64> = None;
                let mut speed = String::new();
                
                // 提取进度 xx%
                if let Some(pct_pos) = line.find('%') {
                    // 向前找数字
                    let before = &line[..pct_pos];
                    let num_start = before.rfind(|c: char| !c.is_ascii_digit() && c != '.').map(|i| i + 1).unwrap_or(0);
                    if let Ok(p) = before[num_start..].parse::<f64>() {
                        percent = Some(p);
                    }
                }
                
                // 提取速度 DL:xxMiB
                if let Some(dl_pos) = line.find("DL:") {
                    let after = &line[dl_pos + 3..];
                    let end = after.find(|c: char| c == ']' || c == ' ').unwrap_or(after.len());
                    speed = after[..end].to_string();
                }
                
                // 提取已下载/总大小
                let mut downloaded = String::new();
                let mut total_size = String::new();
                if let Some(size_match) = line.find('/') {
                    let before_slash = &line[..size_match];
                    let dl_start = before_slash.rfind(' ').map(|i| i + 1).unwrap_or(0);
                    downloaded = before_slash[dl_start..].to_string();
                    
                    let after_slash = &line[size_match + 1..];
                    let total_end = after_slash.find('(').unwrap_or(after_slash.len());
                    total_size = after_slash[..total_end].to_string();
                }
                
                if let Some(p) = percent {
                    stage_count += 1;
                    // 检测阶段切换
                    if p < 10.0 && stage_count > 5 {
                        current_stage += 1;
                        stage_count = 0;
                    }
                    
                    // 统一显示"下载中"，不区分视频/音频
                    let stage_name = "下载中";
                    
                    let progress_info = serde_json::json!({
                        "task_id": task_id,
                        "percent": p,
                        "stage": stage_name,
                        "stage_index": current_stage,
                        "speed": speed,
                        "downloaded": downloaded,
                        "total_size": total_size
                    });
                    let _ = window.emit("download-progress-detail", progress_info);
                    let _ = window.emit("download-progress", p);
                }
            }
            // yt-dlp 原生进度格式
            else if line.starts_with("PROGRESS:") {
                // 格式: PROGRESS:xx.x%:ext:speed:downloaded:total
                let parts: Vec<&str> = line.splitn(6, ':').collect();
                if parts.len() >= 3 {
                    let percent_str = parts[1].trim().replace('%', "");
                    let _ext = parts[2].trim();
                    let speed = if parts.len() >= 4 { parts[3].trim() } else { "" };
                    let downloaded = if parts.len() >= 5 { parts[4].trim() } else { "" };
                    let total_size = if parts.len() >= 6 { parts[5].trim() } else { "" };
                    
                    if let Ok(percent) = percent_str.trim().parse::<f64>() {
                        // 检测阶段切换：当进度从高变低时，说明开始新的下载
                        if percent < 10.0 && stage_count > 5 {
                            current_stage += 1;
                            stage_count = 0;
                        }
                        stage_count += 1;
                        
                        // 统一显示"下载中"，不区分视频/音频（顺序不固定）
                        let stage_name = "下载中";
                        
                        // 发送详细进度信息（包含速度和大小）
                        let progress_info = serde_json::json!({
                            "task_id": task_id,
                            "percent": percent,
                            "stage": stage_name,
                            "stage_index": current_stage,
                            "speed": speed,
                            "downloaded": downloaded,
                            "total_size": total_size
                        });
                        let _ = window.emit("download-progress-detail", progress_info);
                        // 同时发送旧的简单进度以保持兼容
                        let _ = window.emit("download-progress", percent);
                    }
                }
            } else if line.starts_with("POSTPROCESS:") {
                // 后处理阶段（合并）
                let progress_info = serde_json::json!({
                    "task_id": task_id,
                    "percent": 100.0,
                    "stage": "合并中",
                    "stage_index": 2
                });
                let _ = window.emit("download-progress-detail", progress_info);
            }
        }
    }
    
    // 等待 stderr 读取完成
    let error_output = stderr_handle.join().unwrap_or_default();

    let status = child.wait().map_err(|e| e.to_string())?;
    
    // 清除进程 ID
    {
        let state = app_handle.state::<AppState>();
        *state.current_download_pid.lock().unwrap() = None;
    }

    // 等待一小段时间确保文件系统同步（特别是合并操作后）
    std::thread::sleep(std::time::Duration::from_millis(500));

    // 查找下载的文件（无论成功与否都尝试查找）
    let mut downloaded_file: Option<std::path::PathBuf> = None;
    
    // 策略1：优先精确匹配预期的文件名
    if let Some(ref id) = expected_id {
        // 尝试多种可能的文件名格式
        let mut possible_ids = vec![id.clone()];
        
        // BVxxx_cid 格式 -> 尝试 BVxxx
        if id.contains('_') {
            if let Some(bv_part) = id.split('_').next() {
                possible_ids.push(bv_part.to_string());
            }
        }
        // ep87842 -> 87842
        if id.starts_with("ep") {
            possible_ids.push(id.trim_start_matches("ep").to_string());
        }
        
        for try_id in possible_ids {
            if try_id.is_empty() { continue; }
            let candidate = std::path::Path::new(&actual_output_dir).join(format!("{}.mp4", try_id));
            if candidate.exists() {
                // 验证文件不是空的或损坏的（至少 100KB）
                if let Ok(meta) = candidate.metadata() {
                    if meta.len() > 100 * 1024 {
                        downloaded_file = Some(candidate);
                        break;
                    }
                }
            }
        }
    }

    // 策略2：如果是独立临时目录，直接找目录中唯一的完整 mp4
    if downloaded_file.is_none() {
        if let Ok(entries) = fs::read_dir(&actual_output_dir) {
            let mut mp4_files: Vec<std::path::PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().map(|e| e.eq_ignore_ascii_case("mp4")).unwrap_or(false)
                })
                .filter(|p| {
                    // 排除临时流文件（包含 .f 后跟数字）
                    let filename = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if filename.contains(".f") {
                        let parts: Vec<&str> = filename.rsplitn(2, ".f").collect();
                        if parts.len() == 2 && parts[0].chars().all(|c| c.is_ascii_digit()) {
                            return false;
                        }
                    }
                    // 文件大小至少 100KB
                    p.metadata().ok().map(|m| m.len() > 100 * 1024).unwrap_or(false)
                })
                .collect();
            
            // 按文件大小排序，取最大的（合并后的文件通常最大）
            mp4_files.sort_by(|a, b| {
                let size_a = a.metadata().ok().map(|m| m.len()).unwrap_or(0);
                let size_b = b.metadata().ok().map(|m| m.len()).unwrap_or(0);
                size_b.cmp(&size_a)
            });
            
            if let Some(largest) = mp4_files.first() {
                downloaded_file = Some(largest.clone());
            }
        }
    }

    // 回退：扫描目录查找最近生成的完整 mp4（排除临时流文件）
    if downloaded_file.is_none() {
        if let Ok(entries) = fs::read_dir(&actual_output_dir) {
            // 记录下载开始前的时间戳（用于过滤旧文件）
            let now = std::time::SystemTime::now();
            let five_minutes_ago = now - std::time::Duration::from_secs(300);
            
            // 找到最近修改的 mp4，但排除临时流文件
            let mut newest: Option<(std::time::SystemTime, std::path::PathBuf, u64)> = None;
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|e| e.eq_ignore_ascii_case("mp4")).unwrap_or(false) {
                    let filename = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    
                    // 跳过临时流文件（包含 .f 后跟数字，如 BV123.f303.mp4）
                    if filename.contains(".f") {
                        let parts: Vec<&str> = filename.rsplitn(2, ".f").collect();
                        if parts.len() == 2 && parts[0].chars().all(|c| c.is_ascii_digit()) {
                            continue;
                        }
                    }
                    
                    if let Ok(metadata) = path.metadata() {
                        let file_size = metadata.len();
                        // 跳过小于 100KB 的文件（可能是损坏的）
                        if file_size < 100 * 1024 {
                            continue;
                        }
                        
                        if let Ok(modified) = metadata.modified() {
                            // 只考虑最近 5 分钟内修改的文件
                            if modified < five_minutes_ago {
                                continue;
                            }
                            
                            if let Some((best_time, _, best_size)) = &newest {
                                // 优先选择更大的文件（合并后的文件通常比临时文件大）
                                // 如果大小相近，选择更新的
                                if file_size > *best_size + 1024 * 1024 || 
                                   (file_size > *best_size - 1024 * 1024 && modified > *best_time) {
                                    newest = Some((modified, path, file_size));
                                }
                            } else {
                                newest = Some((modified, path, file_size));
                            }
                        }
                    }
                }
            }
            if let Some((_, p, _)) = newest { downloaded_file = Some(p); }
        }
    }

    // 如果找到了文件，等待文件大小稳定（确保写入完成）
    if let Some(ref path) = downloaded_file {
        let mut last_size = 0u64;
        let mut stable_count = 0;
        for _ in 0..10 {  // 最多等待 5 秒
            if let Ok(meta) = path.metadata() {
                let current_size = meta.len();
                if current_size == last_size && current_size > 0 {
                    stable_count += 1;
                    if stable_count >= 2 {
                        break;  // 连续两次大小相同，文件稳定
                    }
                } else {
                    stable_count = 0;
                    last_size = current_size;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    // 简单验证 MP4 文件：只检查文件头部是否有效
    fn is_valid_mp4(path: &std::path::Path) -> bool {
        use std::io::Read;
        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut reader = std::io::BufReader::new(file);
        
        // 检查文件头部是否是有效的 MP4 (ftyp box)
        let mut header = [0u8; 12];
        if reader.read_exact(&mut header).is_err() {
            return false;
        }
        // ftyp 标识应该在字节 4-7
        &header[4..8] == b"ftyp"
    }

    // 判断成功：进程成功退出，并且找到了有效的 MP4 文件
    let actually_success = status.success() && downloaded_file.is_some() && {
        downloaded_file.as_ref()
            .map(|p| {
                // 文件大于 1MB 且头部是有效的 MP4
                p.metadata().ok().map(|m| m.len() > 1024 * 1024).unwrap_or(false)
                    && is_valid_mp4(p)
            })
            .unwrap_or(false)
    };

    if actually_success {
        // 下载成功后，重命名并移动文件
        if let Some(path) = downloaded_file {
            let target_name = if is_multi_p {
                // 多P视频：有标题用标题，没有就用 Pxx
                let idx = entry_index.unwrap_or(1);
                if let Some(ref t) = safe_entry_title {
                    format!("{}.mp4", t)
                } else {
                    format!("P{:02}.mp4", idx)
                }
            } else {
                // 单视频：用视频标题
                if let Some(ref t) = safe_title {
                    format!("{}.mp4", t)
                } else {
                    String::new() // 不重命名
                }
            };
            
            if !target_name.is_empty() {
                // 多P视频：移动到最终目录
                let target_dir = if is_multi_p {
                    if let Some(ref final_d) = final_dir {
                        final_d.as_str()
                    } else {
                        &actual_output_dir
                    }
                } else {
                    &actual_output_dir
                };
                
                let new_path = std::path::Path::new(target_dir).join(&target_name);
                if !new_path.exists() {
                    // 先尝试 rename（同磁盘快速移动）
                    if fs::rename(&path, &new_path).is_err() {
                        // rename 失败（可能跨磁盘），改用 copy + delete
                        if let Ok(_) = fs::copy(&path, &new_path) {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
        
        // 清理独立的临时目录（如果是多P下载）
        if is_multi_p {
            if let Some(ref temp) = temp_dir {
                // 尝试删除临时目录（只有空目录才能删除成功）
                let _ = fs::remove_dir(temp);
            }
        }
        
        Ok(ApiResponse {
            success: true,
            data: Some("下载完成".to_string()),
            error: None,
        })
    } else {
        // 进程被终止或失败，不重命名文件，保持原样以便断点续传
        Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(if error_output.is_empty() {
                "下载失败".to_string()
            } else {
                error_output
            }),
        })
    }
}

// ==================== 暂停下载 ====================

#[tauri::command]
fn cancel_download(app_handle: tauri::AppHandle) -> Result<ApiResponse<()>, String> {
    let state = app_handle.state::<AppState>();
    let pid = state.current_download_pid.lock().unwrap().take();
    
    if let Some(pid) = pid {
        // Windows: 使用 taskkill 结束进程树
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill")
                .args(&["/F", "/T", "/PID", &pid.to_string()])
                .creation_flags(0x08000000)
                .output();
        }
        
        Ok(ApiResponse {
            success: true,
            data: Some(()),
            error: None,
        })
    } else {
        Ok(ApiResponse {
            success: false,
            data: None,
            error: Some("没有正在进行的下载".to_string()),
        })
    }
}

// ==================== 文件夹重命名 ====================

#[tauri::command]
fn rename_folder(old_path: String, new_name: String) -> Result<ApiResponse<()>, String> {
    let old_path = std::path::Path::new(&old_path);
    
    if !old_path.exists() {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some("文件夹不存在".to_string()),
        });
    }
    
    // 清理新名称中的非法字符
    let safe_name: String = new_name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c as u32 <= 31 => '_',
            _ => c,
        })
        .collect();
    let safe_name = safe_name.trim();
    
    if safe_name.is_empty() {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some("无效的文件夹名".to_string()),
        });
    }
    
    // 构建新路径
    let parent = old_path.parent().unwrap_or(std::path::Path::new("."));
    let new_path = parent.join(safe_name);
    
    // 如果新路径已存在，跳过
    if new_path.exists() {
        return Ok(ApiResponse {
            success: true,
            data: Some(()),
            error: None,
        });
    }
    
    match fs::rename(old_path, &new_path) {
        Ok(_) => Ok(ApiResponse {
            success: true,
            data: Some(()),
            error: None,
        }),
        Err(e) => Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("重命名失败: {}", e)),
        }),
    }
}

// ==================== 清理临时目录 ====================

#[tauri::command]
fn cleanup_temp_dir(temp_dir: String) -> Result<ApiResponse<()>, String> {
    let path = std::path::Path::new(&temp_dir);
    
    if !path.exists() {
        return Ok(ApiResponse {
            success: true,
            data: Some(()),
            error: None,
        });
    }
    
    // 检查目录是否为空
    if let Ok(mut entries) = fs::read_dir(path) {
        if entries.next().is_none() {
            // 目录为空，删除
            let _ = fs::remove_dir(path);
        }
    }
    
    Ok(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

// ==================== 下载任务持久化 ====================

fn get_tasks_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    fs::create_dir_all(&app_data).map_err(|e| format!("Failed to create app data dir: {}", e))?;
    Ok(app_data.join("download_tasks.json"))
}

#[tauri::command]
fn save_download_tasks(app_handle: tauri::AppHandle, tasks_json: String) -> Result<ApiResponse<()>, String> {
    let path = get_tasks_path(&app_handle)?;
    match fs::write(&path, tasks_json) {
        Ok(_) => Ok(ApiResponse {
            success: true,
            data: Some(()),
            error: None,
        }),
        Err(e) => Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("保存失败: {}", e)),
        }),
    }
}

#[tauri::command]
fn load_download_tasks(app_handle: tauri::AppHandle) -> Result<ApiResponse<String>, String> {
    let path = get_tasks_path(&app_handle)?;
    if !path.exists() {
        return Ok(ApiResponse {
            success: true,
            data: Some("[]".to_string()),
            error: None,
        });
    }
    match fs::read_to_string(&path) {
        Ok(content) => Ok(ApiResponse {
            success: true,
            data: Some(content),
            error: None,
        }),
        Err(e) => Ok(ApiResponse {
            success: true,
            data: Some("[]".to_string()),
            error: Some(format!("读取失败: {}", e)),
        }),
    }
}

// ==================== 打开文件夹 ====================

#[tauri::command]
fn open_folder(path: String) -> Result<ApiResponse<()>, String> {
    let path = std::path::Path::new(&path);
    
    // 如果是文件，获取其所在目录
    let folder_path = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    
    if !folder_path.exists() {
        return Ok(ApiResponse {
            success: false,
            data: None,
            error: Some("文件夹不存在".to_string()),
        });
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(folder_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(folder_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(folder_path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

// ==================== 删除文件夹 ====================

#[tauri::command]
fn delete_folder(path: String) -> Result<ApiResponse<()>, String> {
    let folder_path = std::path::Path::new(&path);
    
    if !folder_path.exists() {
        return Ok(ApiResponse {
            success: true,
            data: Some(()),
            error: None, // 文件夹不存在也算成功
        });
    }
    
    match fs::remove_dir_all(folder_path) {
        Ok(_) => Ok(ApiResponse {
            success: true,
            data: Some(()),
            error: None,
        }),
        Err(e) => Ok(ApiResponse {
            success: false,
            data: None,
            error: Some(format!("删除失败: {}", e)),
        }),
    }
}

// ==================== 下载并安装更新 ====================

#[tauri::command]
async fn download_and_install_update(
    app: tauri::AppHandle,
    url: String,
    version: String,
) -> Result<(), String> {
    // 获取临时目录
    let temp_dir = std::env::temp_dir();
    let setup_filename = format!("bilibili-downloader_{}_setup.exe", version);
    let setup_path = temp_dir.join(&setup_filename);
    
    // 下载文件
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "bilibili-downloader")
        .send()
        .await
        .map_err(|e| format!("下载失败: {}", e))?;
    
    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    
    // 创建文件
    let mut file = fs::File::create(&setup_path)
        .map_err(|e| format!("创建文件失败: {}", e))?;
    
    // 流式下载并报告进度
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载失败: {}", e))?;
        file.write_all(&chunk).map_err(|e| format!("写入失败: {}", e))?;
        
        downloaded += chunk.len() as u64;
        
        // 计算进度并发送事件
        if total_size > 0 {
            let progress = ((downloaded as f64 / total_size as f64) * 100.0) as u32;
            let _ = app.emit("update-download-progress", progress);
        }
    }
    
    // 确保写入完成
    drop(file);
    
    // 发送完成进度
    let _ = app.emit("update-download-progress", 100u32);
    
    // 等待一小段时间让用户看到100%
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // 启动安装程序
    #[cfg(target_os = "windows")]
    {
        Command::new(&setup_path)
            .creation_flags(0x00000008) // DETACHED_PROCESS
            .spawn()
            .map_err(|e| format!("启动安装程序失败: {}", e))?;
    }
    
    // 退出当前应用
    app.exit(0);
    
    Ok(())
}

// ==================== 应用入口 ====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            cookies: Mutex::new(None),
            cookies_file: Mutex::new(None),
            current_download_pid: Mutex::new(None),
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            init_resources,
            close_splashscreen,
            fetch_image_as_base64,
            get_qrcode,
            poll_qrcode,
            get_user_info,
            logout,
            check_login_status,
            get_video_info,
            download_video,
            get_history,
            get_favorite_folders,
            get_favorite_content,
            search_video,
            rename_folder,
            cancel_download,
            save_download_tasks,
            load_download_tasks,
            cleanup_temp_dir,
            open_folder,
            delete_folder,
            download_and_install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
