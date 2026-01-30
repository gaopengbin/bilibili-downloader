use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

/// 获取 cookies 文件路径
pub fn get_cookies_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    fs::create_dir_all(&app_data).map_err(|e| format!("Failed to create app data dir: {}", e))?;

    Ok(app_data.join("cookies.txt"))
}

/// 保存 cookies 到文件（Netscape 格式）
pub fn save_cookies_to_file(cookies: &HashMap<String, String>, path: &PathBuf) -> Result<(), String> {
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

/// 从文件加载 cookies
pub fn load_cookies_from_file(path: &PathBuf) -> Result<HashMap<String, String>, String> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(path).map_err(|e| format!("读取 cookies 失败: {}", e))?;
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

    Ok(cookies)
}
