use serde::Serialize;
use std::collections::HashMap;

/// 二维码结果
#[derive(Debug, Serialize)]
pub struct QrCodeResult {
    pub qrcode_base64: String,
    pub qrcode_key: String,
}

/// 二维码状态
#[derive(Debug, Serialize)]
pub struct QrCodeStatus {
    pub status: String,
    pub message: String,
    pub cookies: Option<HashMap<String, String>>,
}

/// 用户信息
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub username: String,
    pub face: String,
    pub level: u32,
    pub mid: u64,
}
