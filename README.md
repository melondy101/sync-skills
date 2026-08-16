<p align="center">
  <strong>中文</strong> | <a href="README.en.md">English</a> | <a href="README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.18-blue?style=flat-square" alt="version">
  <img src="https://img.shields.io/badge/Tauri-v2-orange?style=flat-square&logo=tauri" alt="tauri">
  <img src="https://img.shields.io/badge/Rust-2021-brown?style=flat-square&logo=rust" alt="rust">
  <img src="https://img.shields.io/badge/React-19-blue?style=flat-square&logo=react" alt="react">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="license">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey?style=flat-square" alt="platform">
</p>

<h1 align="center">⬡ Skill Manager</h1>

<p align="center">
  <strong>跨 AI 编码工具的 Skill 同步管理器</strong><br>
  一处编辑，多处生效。
</p>

<p align="center">
  <a href="#功能">功能</a> •
  <a href="#架构">架构</a> •
  <a href="#安装">安装</a> •
  <a href="#开发">开发</a> •
  <a href="#路线图">路线图</a>
</p>

---

## 为什么需要 Skill Manager

AI 编码助手（Claude Code、Cursor、Windsurf、Cline 等）都使用 `SKILL.md` 文件来定义可复用的知识和流程。当你同时使用多个工具时，Skill 文件散落在不同目录，手动同步既繁琐又容易出错。

Skill Manager 提供一个桌面 GUI，让你在一个地方管理所有 Skill，自动同步到所有已配置的 AI 工具。

## 功能

- **工具管理** — 注册 AI 编码工具路径，支持 13+ 已知工具自动发现
- **Skill 扫描** — 递归扫描目录，识别所有包含 `SKILL.md` 的技能目录
- **SSOT 同步** — 以 `~/.agents/skill-manager/ssot/` 为中心，hub-and-spoke 模型分发到各工具
- **反向同步** — 从 SSOT 推送到指定工具目录，覆盖本地更改
- **冲突管理** — 检测不同工具间的版本冲突，支持 diff 预览和裁决
- **变更忽略** — 持久化忽略特定工具的变更，hash 匹配则不再提示
- **项目级管理** — 为不同项目配置独立的 Skill 集合，支持编辑
- **差异检测** — 内置 LCS diff 视图（并排 / 统一两种模式），精确展示文件级变更
- **主题切换** — 亮色 / 暗色 / 跟随系统
- **多语言** — 中文 / English / 日本語
- **活动日志** — 完整的操作审计记录

## 架构

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Claude Code │     │   Cursor    │     │  Windsurf   │
│  .claude/    │     │  .cursor/   │     │  .windsurf/ │
└──────┬───────┘     └──────┬──────┘     └──────┬──────┘
       │                    │                    │
       │     Skill Manager (Tauri Desktop)      │
       │         ┌──────────────────┐           │
       └────────►│  ~/.agents/      │◄──────────┘
                 │  skill-manager/  │
                 │  ssot/ (SSOT Hub)│
                 └────────┬─────────┘
                          │
                 ┌────────┴─────────┐
                 │  skill-manager.db │
                 │  (SQLite)        │
                 └──────────────────┘
```

**技术栈：** Tauri v2 (Rust + React 19 + TypeScript + SQLite)

## 安装

### 从源码构建

**前置要求：**

- [Rust](https://www.rust-lang.org/tools/install) (2021 edition)
- [Node.js](https://nodejs.org/) >= 22 与 [pnpm](https://pnpm.io/) >= 11（**请使用 pnpm，不要使用 npm / yarn**）
- [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)

**构建步骤：**

```bash
git clone https://github.com/huang-yi-dae/sync-skills.git
cd sync-skills
pnpm install
pnpm tauri build
```

构建产物（安装包）位于 `src-tauri/target/release/bundle/`。

### 开发模式

```bash
pnpm install
pnpm tauri dev
```

### 发布（Release）

本项目通过 GitHub Actions 自动构建多平台安装包，**无需本地打包**：

1. 确保 `main` 已更新；
2. 打版本标签并推送：

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. CI（`.github/workflows/release.yml`）在 Windows / macOS / Linux 三个 runner 上分别构建
   `.msi` / `.nsis.exe`、`.dmg` / `.app`（universal）、`.deb` / `.AppImage`，
   并汇总到一个 **GitHub Release 草稿**；
4. 在仓库 Releases 页面将草稿发布即可。

> 当前安装包未做代码签名，Windows / macOS 会提示 SmartScreen / Gatekeeper 警告。

## 开发

```
sync-skills/
├── src/                    # 前端 (React + TypeScript)
│   ├── App.tsx             # 主组件
│   ├── App.css             # 样式（CSS 变量主题系统）
│   ├── types.ts            # TypeScript 类型定义
│   └── main.tsx            # 入口
├── src-tauri/src/          # Rust 后端（lib.rs / commands/ / ops.rs / sync.rs / db.rs / scanner.rs / diff.rs / lock.rs / lint.rs 等）
├── doc/                    # 规划文档（PRD、设计、阶段计划、DDL）
├── docs/                   # 问题清单与已解决记录（如 testing-issues-triage.md）
└── .github/workflows/      # CI：verify.yml 质量门禁 + release.yml 多平台安装包构建
```

### 验证

提交前的完整校验清单（类型检查 / lint / 单元测试，前端 + Rust 共 6 条命令）见 [CONTRIBUTING.md](CONTRIBUTING.md)；CI（`.github/workflows/verify.yml`）会在 push / PR 时强制执行同一组检查。

## 路线图

| 版本 | 状态 | 内容 |
|------|------|------|
| v0.1.0 | ✅ 已完成 | 核心功能：工具管理、Skill 扫描、SSOT 同步、项目管理 |
| v0.2.0 | ✅ 已完成 | 自动发现、排序筛选、差异检测、diff 视图 |
| v0.3.0 | ✅ 已完成 | 主题切换、多语言支持、哈希稳定性修复 |
| v0.4.0 | ✅ 已完成 | 名字即身份、冲突检测/裁决、时间戳、项目编辑、反向同步、变更忽略 |
| v0.5.0 | 计划中 | LockManager 接入、core_hash 变更检测、文件监听 |
| v0.6.0 | 进行中 | 应用内更新一键下载安装、用户体验提升（引导优化、快捷键、批量操作、列表性能优化） — 批量操作（市场一键同步 / 批量标记）已落地，分支 `codex/ui_optimization` |
| v0.7.0 | 规划中 | MCP 集成：MCP Server 配置管控与跨工具同步、对外提供 MCP 接口供 Agent 调用 |

## 许可证

[AGPL-3.0](LICENSE)
