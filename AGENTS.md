# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## 项目概述

B站视频下载器 - 基于 Tauri 2 + Vue 3 的桌面应用，支持多平台视频下载。采用 yt-dlp + aria2 + ffmpeg 作为下载核心，内嵌二进制文件实现开箱即用。

**当前分支**: `refactor/multi-platform` - 正在重构为多平台架构（支持 B站、腾讯视频等）

## 常用命令

```bash
# 安装依赖
npm install

# 开发模式（启动 Vite + Tauri）
npm run tauri dev

# 构建生产版本
npm run tauri build

# 仅前端开发（不启动 Tauri）
npm run dev

# 类型检查
npx vue-tsc --noEmit
```

## 架构设计

### 技术栈
- **前端**: Vue 3 + TypeScript + Element Plus + Pinia
- **后端**: Tauri 2 + Rust
- **下载工具**: yt-dlp（视频解析）+ aria2c（多线程下载）+ ffmpeg（音视频合并）

### 目录结构

```
src/                      # 前端源码
├── platforms/            # 多平台抽象层（核心）
│   ├── interface.ts      # Platform 接口定义
│   ├── bilibili/         # B站平台实现
│   │   ├── index.ts      # BilibiliPlatform 类
│   │   └── api.ts        # B站 API 封装（调用 Tauri invoke）
│   └── tencent/          # 腾讯视频平台（开发中）
├── stores/               # Pinia 状态管理
│   ├── app.ts            # 应用设置、主题、更新
│   ├── download.ts       # 下载任务状态
│   └── user.ts           # 用户登录、搜索、收藏
├── composables/          # Vue Composables
│   ├── useDownloadTasks.ts  # 下载队列管理（并行下载、重试）
│   └── useVideoDetail.ts    # 视频详情获取
├── components/           # Vue 组件
│   ├── bilibili/         # B站专用组件
│   └── common/           # 通用组件
├── views/                # 页面视图
└── types/                # TypeScript 类型定义

src-tauri/                # Rust 后端
├── src/
│   ├── lib.rs            # 主要逻辑（API 调用、下载控制）
│   ├── commands/         # Tauri 命令模块（渐进重构中）
│   ├── models/           # 数据结构
│   └── utils/            # 工具函数
├── yt-dlp.exe            # 内嵌的 yt-dlp
├── ffmpeg.exe            # 内嵌的 ffmpeg
└── aria2c.exe            # 内嵌的 aria2c
```

### 核心架构

**多平台抽象 (`src/platforms/interface.ts`)**
- `Platform` 接口定义所有平台必须实现的方法
- `registerPlatform()` 注册平台到全局注册表
- `matchPlatformByUrl()` 根据 URL 自动匹配平台

**下载流程**
1. 前端调用 `platform.parseVideo(url)` 获取视频信息
2. 用户选择清晰度后，调用 Tauri `download_video` 命令
3. Rust 后端调用 yt-dlp 解析 + aria2c 下载 + ffmpeg 合并
4. 通过事件 `download-progress` 向前端推送进度

**状态管理**
- `useDownloadTasks` composable 管理下载队列
- 支持并行下载（默认3个）、自动重试、暂停/继续
- 组任务支持（批量下载多P/合集）

### 前后端通信

前端通过 `@tauri-apps/api/core` 的 `invoke` 调用 Rust 命令：

```typescript
// 前端调用示例
import { invoke } from '@tauri-apps/api/core';
const result = await invoke<ApiResponse<VideoInfo>>('get_video_info', { url });
```

主要 Tauri 命令（定义在 `src-tauri/src/lib.rs`）：
- `get_video_info` - 获取视频信息
- `download_video` - 开始下载
- `cancel_download` - 取消下载
- `get_qrcode` / `poll_qrcode` - 二维码登录
- `check_login_status` / `logout` - 登录状态管理
- `get_history` / `get_favorite_folders` / `get_favorite_content` - 用户数据
- `search_video` - 搜索

## 开发注意事项

1. **二进制文件**: yt-dlp/ffmpeg/aria2c 内嵌在 Rust 中，首次运行时解压到 `%LOCALAPPDATA%/bilibili-downloader-tools/`

2. **Cookies 管理**: B站登录态存储在 Netscape 格式的 cookies.txt，路径通过 Tauri 的 app_data_dir 获取

3. **Windows 专用**: 当前仅支持 Windows（使用 `.exe` 二进制文件），跨平台支持需替换对应平台的工具

4. **API 代理**: 图片等资源需要通过 Rust 后端代理（添加 Referer），避免跨域和防盗链

5. **多平台开发**: 添加新平台时：
   - 在 `src/platforms/` 下创建新目录
   - 实现 `Platform` 接口
   - 在 `src/platforms/index.ts` 中注册
   - 添加对应的 View 和路由

## 版本信息

- 当前版本: 0.10.1（定义在 `src/stores/app.ts` 和 `src-tauri/Cargo.toml`）
- 更新检查: 通过 GitHub Releases API
