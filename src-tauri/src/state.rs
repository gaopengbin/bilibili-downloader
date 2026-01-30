//! 应用全局状态

use std::collections::HashMap;
use std::sync::Mutex;

/// 应用状态
pub struct AppState {
    /// 用户 cookies
    pub cookies: Mutex<Option<HashMap<String, String>>>,
    /// cookies 文件路径
    pub cookies_file: Mutex<Option<String>>,
    /// 当前下载进程 ID
    pub current_download_pid: Mutex<Option<u32>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            cookies: Mutex::new(None),
            cookies_file: Mutex::new(None),
            current_download_pid: Mutex::new(None),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
