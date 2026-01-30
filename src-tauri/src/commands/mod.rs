//! 命令模块
//! 
//! 注意：当前命令仍在 lib.rs 中定义，此模块用于新命令的组织
//! 后续可逐步将 lib.rs 中的命令迁移到此处

pub mod app;
pub mod auth;

// 重新导出命令（当前仅包含新模块中的命令）
pub use app::*;
// auth 命令暂未启用，因为依赖 state 模块
// pub use auth::*;
