use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// 嵌入的二进制文件
const YTDLP_BINARY: &[u8] = include_bytes!("../../yt-dlp.exe");
const FFMPEG_BINARY: &[u8] = include_bytes!("../../ffmpeg.exe");
const ARIA2C_BINARY: &[u8] = include_bytes!("../../aria2c.exe");

/// 初始化嵌入资源，返回资源目录路径
/// 使用 LOCALAPPDATA 目录而不是临时目录，避免被系统清理
pub fn init_embedded_resources() -> Result<PathBuf, String> {
    let base_dir = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());

    let tools_dir = base_dir.join("bilibili-downloader-tools");

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

/// 获取 yt-dlp 路径
pub fn get_ytdlp_path() -> Result<PathBuf, String> {
    // 开发环境
    let dev_path = std::env::current_dir()
        .map(|p| p.join("src-tauri").join("yt-dlp.exe"))
        .unwrap_or_default();

    if dev_path.exists() {
        return Ok(dev_path);
    }

    // 生产环境：使用嵌入的资源
    let tools_dir = init_embedded_resources()?;
    Ok(tools_dir.join("yt-dlp.exe"))
}

/// 获取 ffmpeg 路径
pub fn get_ffmpeg_path() -> Result<PathBuf, String> {
    let dev_path = std::env::current_dir()
        .map(|p| p.join("src-tauri").join("ffmpeg.exe"))
        .unwrap_or_default();

    if dev_path.exists() {
        return Ok(dev_path);
    }

    let tools_dir = init_embedded_resources()?;
    Ok(tools_dir.join("ffmpeg.exe"))
}
