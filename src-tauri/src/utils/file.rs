use std::fs;
use std::path::PathBuf;
use tauri::Manager;

/// 获取下载任务存储路径
pub fn get_tasks_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;

    fs::create_dir_all(&app_data).map_err(|e| format!("创建目录失败: {}", e))?;

    Ok(app_data.join("download_tasks.json"))
}

/// 删除文件夹（递归）
pub fn delete_folder_recursive(path: &str) -> Result<(), String> {
    let path = PathBuf::from(path);
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| format!("删除失败: {}", e))?;
    }
    Ok(())
}

/// 重命名文件夹
pub fn rename_folder(old_path: &str, new_name: &str) -> Result<(), String> {
    let old_path = PathBuf::from(old_path);
    if !old_path.exists() {
        return Err("原文件夹不存在".to_string());
    }

    let parent = old_path.parent().ok_or("无法获取父目录")?;
    let new_path = parent.join(new_name);

    fs::rename(&old_path, &new_path).map_err(|e| format!("重命名失败: {}", e))
}

/// 清理临时目录
pub fn cleanup_temp_dir(temp_dir: &str) -> Result<(), String> {
    let path = PathBuf::from(temp_dir);
    if path.exists() && path.to_string_lossy().contains("temp_") {
        fs::remove_dir_all(&path).map_err(|e| format!("清理临时目录失败: {}", e))?;
    }
    Ok(())
}

/// 打开文件夹（系统资源管理器）
#[cfg(target_os = "windows")]
pub fn open_folder_in_explorer(path: &str) -> Result<(), String> {
    use std::process::Command;
    
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("文件夹不存在".to_string());
    }

    Command::new("explorer")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("打开文件夹失败: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn open_folder_in_explorer(path: &str) -> Result<(), String> {
    use std::process::Command;
    
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("文件夹不存在".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {}", e))?;
    }

    Ok(())
}
