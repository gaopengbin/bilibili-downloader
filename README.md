# B站视频下载器

一个基于 Tauri + Vue 3 开发的 B站视频下载工具。

![screenshot](./docs/screenshot.png)

## ✨ 功能特性

- 🎬 支持下载单个视频、分P视频、合集视频
- 📺 支持从收藏夹、历史记录中批量下载
- 🔍 支持搜索视频
- 🎨 多种清晰度选择（4K/1080P/720P/480P/360P）
- 🎵 支持选择视频编码（AVC/HEVC/AV1）
- ⚡ 多线程加速下载（aria2c）
- 🔄 下载失败自动重试
- 📋 下载任务管理（暂停/继续/删除/重试）
- 🌙 深色模式
- ⬆️ 应用内检查更新

## 📦 下载安装

前往 [Releases](../../releases) 页面下载最新版本。

- Windows: 下载 `.msi` 或 `.exe` 安装包

## 🔧 开发

### 环境要求

- Node.js 18+
- Rust 1.70+
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [ffmpeg](https://ffmpeg.org/)
- [aria2](https://aria2.github.io/)

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

## 📝 发布新版本

1. 在 GitHub 仓库设置中添加 Secret：
   - 名称：`TAURI_SIGNING_PRIVATE_KEY`
   - 值：你的私钥内容

2. 运行发版脚本：
   ```powershell
   .\scripts\release.ps1 -Version "0.2.0"
   ```

3. 推送到 GitHub：
   ```bash
   git push origin main --tags
   ```

GitHub Actions 会自动构建并创建 Release。

## 📄 许可证

MIT License

