<p align="center">
  <img src="docs/images/logo.png" width="120" alt="TimeSense" />
</p>

<h1 align="center">TimeSense</h1>

<p align="center">
  <strong>面向开发者的效能与健康助手</strong><br />
  看清你的时间都去哪儿了。
</p>

<p align="center">
  <strong>简体中文</strong> |
  <a href="README.md">English</a>
</p>

<p align="center">
  <img alt="license" src="https://img.shields.io/badge/license-MIT-green" />
  <img alt="release" src="https://img.shields.io/badge/release-v0.1.0-blue" />
  <img alt="platform" src="https://img.shields.io/badge/platform-Windows%2010%2F11-blue" />
  <img alt="tauri" src="https://img.shields.io/badge/Tauri-2.x-orange" />
  <img alt="vue" src="https://img.shields.io/badge/Vue-3-brightgreen" />
</p>

---

## 项目简介

TimeSense 是一款轻量级桌面应用，自动追踪你正在使用的应用和窗口，通过可自定义的规则将其归类到不同分类，并把原始活动数据转化为可执行的洞察 —— 全程无需你操心。

基于 Tauri 2 + Vue 3 + Rust + SQLite 构建，所有数据仅保存在本地。

## 功能特性

- **自动活动追踪** —— 每 2 秒监测当前活动窗口与进程，无需手动开启计时
- **规则化分类** —— 通过正则规则将进程 / 窗口标题映射到分类，未匹配的活动自动归入默认分类
- **番茄钟** —— 专注 / 休息循环，支持自动模式：检测到专注状态时自动切换分类
- **闲置检测** —— 可配置阈值，离开电脑时自动标记为闲置
- **丰富的统计分析** —— 概览看板、分类统计、每日时间轴、趋势图表、全年热力图
- **系统集成** —— 托盘常驻、开机自启、系统通知、关闭时最小化到托盘
- **隐私优先** —— 数据仅存储在本地 SQLite，不上传任何内容

## 界面截图

> TODO：待补充 看板 / 时间轴 / 热力图 截图

## 安装

1. 从 [Releases](https://github.com/sleepy-ailurus/time-sense/releases) 下载最新的 `TimeSense_0.1.0_x64-setup.exe` 安装程序
2. 运行安装程序，按向导提示完成安装
3. 启动 TimeSense —— 它会常驻系统托盘

> 需要 Windows 10 或更高版本。

## 开发

环境要求：

- [Rust](https://www.rust-lang.org/tools/install)（stable 工具链）
- [Node.js](https://nodejs.org/zh-cn) 18+

```bash
git clone https://github.com/sleepy-ailurus/time-sense.git
cd time-sense
npm install
npm run tauri dev
```

构建生产安装包：

```bash
npm run tauri build
```

## 项目结构

```
src/          # Vue 3 前端（视图、路由、API 层）
src-tauri/    # Rust 后端（监控引擎、SQLite DAO、Tauri 命令）
docs/         # 文档资源
```

## 参与贡献

欢迎在 [Issues](https://github.com/sleepy-ailurus/time-sense/issues) 提交 Bug 反馈与功能建议，也欢迎提交 Pull Request。

## 开源协议

[MIT](LICENSE) © 2026 sleepy-ailurus
