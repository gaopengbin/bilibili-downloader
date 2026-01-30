use tauri::Manager;
use crate::models::ApiResponse;
use crate::utils::resources::init_embedded_resources;

/// 初始化嵌入资源
#[tauri::command]
pub fn init_resources() -> Result<ApiResponse<String>, String> {
    match init_embedded_resources() {
        Ok(path) => Ok(ApiResponse::success(path.to_string_lossy().to_string())),
        Err(e) => Ok(ApiResponse::error(e)),
    }
}

/// 关闭启动屏幕并显示主窗口
#[tauri::command]
pub async fn close_splashscreen(app: tauri::AppHandle) {
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
