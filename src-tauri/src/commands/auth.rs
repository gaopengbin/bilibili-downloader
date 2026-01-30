//! 认证相关命令
//! 包括二维码登录、登录状态检查、登出等

use base64::Engine;
use std::collections::HashMap;
use std::io::Cursor;
use tauri::Manager;

use crate::models::{ApiResponse, QrCodeResult, QrCodeStatus, UserInfo};
use crate::state::AppState;
use crate::utils::cookies::{get_cookies_path, save_cookies_to_file};

/// 获取登录二维码
#[tauri::command]
pub async fn get_qrcode() -> Result<ApiResponse<QrCodeResult>, String> {
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
        return Ok(ApiResponse::error(
            json["message"].as_str().unwrap_or("获取二维码失败")
        ));
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

    Ok(ApiResponse::success(QrCodeResult {
        qrcode_base64: format!("data:image/png;base64,{}", base64_str),
        qrcode_key: qrcode_key.to_string(),
    }))
}

/// 轮询二维码状态
#[tauri::command]
pub async fn poll_qrcode(
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

    Ok(ApiResponse::success(QrCodeStatus {
        status: status.to_string(),
        message: message.to_string(),
        cookies: if code == 0 { Some(cookies) } else { None },
    }))
}

/// 获取用户信息
#[tauri::command]
pub async fn get_user_info(app_handle: tauri::AppHandle) -> Result<ApiResponse<UserInfo>, String> {
    let state = app_handle.state::<AppState>();
    let cookies_opt = state.cookies.lock().unwrap().clone();

    let cookies = match cookies_opt {
        Some(c) => c,
        None => return Ok(ApiResponse::error("未登录")),
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
        return Ok(ApiResponse::error("获取用户信息失败"));
    }

    // 转换头像为 base64
    let face_url = json["data"]["face"].as_str().unwrap_or("");
    let face_base64 = if !face_url.is_empty() {
        fetch_image_as_base64_internal(face_url).await.unwrap_or(face_url.to_string())
    } else {
        String::new()
    };

    Ok(ApiResponse::success(UserInfo {
        username: json["data"]["uname"].as_str().unwrap_or("未知").to_string(),
        face: face_base64,
        level: json["data"]["level_info"]["current_level"].as_u64().unwrap_or(0) as u32,
        mid: json["data"]["mid"].as_u64().unwrap_or(0),
    }))
}

/// 检查登录状态
#[tauri::command]
pub async fn check_login_status(app_handle: tauri::AppHandle) -> Result<ApiResponse<UserInfo>, String> {
    // 首先尝试从本地加载 cookies
    let cookies_path = get_cookies_path(&app_handle)?;
    
    if cookies_path.exists() {
        let state = app_handle.state::<AppState>();
        
        // 读取并解析 cookies 文件
        if let Ok(content) = std::fs::read_to_string(&cookies_path) {
            let mut cookies = HashMap::new();
            for line in content.lines() {
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 7 {
                    cookies.insert(parts[5].to_string(), parts[6].to_string());
                }
            }
            
            if !cookies.is_empty() {
                *state.cookies.lock().unwrap() = Some(cookies);
                *state.cookies_file.lock().unwrap() = Some(cookies_path.to_string_lossy().to_string());
            }
        }
    }
    
    // 然后获取用户信息验证 cookies 是否有效
    get_user_info(app_handle).await
}

/// 登出
#[tauri::command]
pub async fn logout(app_handle: tauri::AppHandle) -> Result<ApiResponse<()>, String> {
    let state = app_handle.state::<AppState>();
    
    // 清除内存中的 cookies
    *state.cookies.lock().unwrap() = None;
    *state.cookies_file.lock().unwrap() = None;
    
    // 删除 cookies 文件
    let cookies_path = get_cookies_path(&app_handle)?;
    if cookies_path.exists() {
        std::fs::remove_file(&cookies_path).map_err(|e| e.to_string())?;
    }
    
    Ok(ApiResponse::ok())
}

/// 内部函数：下载图片并转换为 base64
async fn fetch_image_as_base64_internal(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", "https://www.bilibili.com")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let base64_str = base64::engine::general_purpose::STANDARD.encode(&bytes);
    
    let content_type = if url.contains(".png") {
        "image/png"
    } else if url.contains(".gif") {
        "image/gif"
    } else {
        "image/jpeg"
    };
    
    Ok(format!("data:{};base64,{}", content_type, base64_str))
}

/// 图片代理 - 下载图片并转换为 base64（公开命令）
#[tauri::command]
pub async fn fetch_image_as_base64(url: String) -> Result<String, String> {
    fetch_image_as_base64_internal(&url).await
}
