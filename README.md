<p align="center">
  <strong>中文</strong> | <a href="README.en.md">English</a> | <a href="README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.2.1-blue?style=flat-square" alt="version">
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
- **SSOT 同步** — 以 `~/.skill-manager/ssot/` 为中心，hub-and-spoke 模型分发到各工具
- **反向同步** — 从 SSOT 推送到指定工具目录，覆盖本地更改
- **冲突管理** — 检测不同工具间的版本冲突，支持 diff 预览、手动裁决与自动裁决（最新修改 / 首选工具，证据不足时留给人）
- **变更忽略** — 持久化忽略特定工具的变更，hash 匹配则不再提示
- **项目级管理** — 为不同项目配置独立的 Skill 集合，支持编辑
- **差异检测** — 内置 LCS diff 视图（并排 / 统一两种模式），精确展示文件级变更
- **Skill 市场** — 从 GitHub / GitLab（含自建实例）/ Bitbucket / Azure DevOps 仓库浏览、搜索、一键安装 Skill，内置市场源管理与批量操作（私有 Azure 组织需环境变量 `AZURE_DEVOPS_PAT`）
- **文件监听** — 监听 SSOT 目录，磁盘变更后自动同步（full-auto）或提示（semi-auto）
- **应用内更新** — 一键检测并下载安装新版本
- **引导与健康检查** — 首次启动引导向导、内置 Lint 检查与自动修复、SKILL.md 内置编辑器
- **键盘快捷键** — 全局搜索 / 导航 / 同步 / 源管理，`?` 随时查看速查表
- **MCP 服务** — 对外提供只读 stdio server（`skill-manager-mcp`，与 GUI 共用同一份索引），设置面板一键把 `skill-manager` 登记 / 移除出 Claude Code、Claude Desktop、Cursor、Qoder、Gemini CLI、Windsurf 与 Codex 的 MCP 配置
- **主题与多语言** — 亮色 / 暗色 / 跟随系统，中文 / English / 日本語
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
       └────────►│ ~/.skill-manager/│◄──────────┘
                 │ ssot/ (SSOT Hub) │
                 └────────┬─────────┘
                          │
                 ┌────────┴─────────┐
                 │  skill-manager.db │
                 │  (SQLite)        │
                 └──────────────────┘
```

**技术栈：** Tauri v2 (Rust + React 19 + TypeScript + SQLite)

主界面分为三个 Tab：**Global**（全局 Skill 管理）、**Projects**（项目级 Skill 管理）、**Market**（远程 Skill 市场）。
右上角可进入 **Settings** 和 **Logs** 面板。应用支持最小化到系统托盘。

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
pnpm stage:sidecar   # 首次必须：见下方说明
pnpm tauri dev
```

`tauri.conf.json` 的 `bundle.externalBin` 会让 tauri 的构建脚本在**每次** cargo 调用时校验 `src-tauri/bin/skill-manager-mcp-<target-triple>[.exe]` 是否存在，所以全新 clone 必须先跑一次 `pnpm stage:sidecar`（此时还没有构建产物，它会写一个占位文件打破这个环）；首次构建完成后**再跑一次**同样的命令，把占位换成真正的 server 二进制。跳过这一步的话 `pnpm tauri dev` 会报 `resource path ... doesn't exist`。

### 发布（Release）

本项目通过 GitHub Actions 自动构建多平台安装包，**无需本地打包**：

1. 确保 `main` 已更新；
2. 打版本标签并推送：

   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

3. CI（`.github/workflows/release.yml`）在 Windows / macOS / Linux 三个 runner 上分别构建
   `.msi` / `.nsis.exe`、`.dmg` / `.app`（universal）、`.deb` / `.AppImage`，
   并汇总到一个 **GitHub Release 草稿**；
4. 在仓库 Releases 页面将草稿发布即可。

> 当前安装包未做代码签名，Windows / macOS 会提示 SmartScreen / Gatekeeper 警告。

**注意**：README 的 version 徽章跟随最新已发布标签；`main` 上的未发布提交见 [docs/CHANGELOG.md](docs/CHANGELOG.md) 的 Unreleased 段落。

## 开发

```
sync-skills/
├── src/                    # 前端 (React + TypeScript)
│   ├── App.tsx             # 主组件与路由
│   ├── App.css             # 样式（CSS 变量主题系统）
│   ├── api.ts              # Tauri IPC 封装
│   ├── i18n/               # 多语言文本
│   ├── types.ts            # TypeScript 类型定义
│   ├── main.tsx            # 入口
│   ├── components/         # UI 组件
│   └── hooks/              # 自定义 Hooks
├── src-tauri/src/          # Rust 后端
│   ├── lib.rs              # 入口与命令注册
│   ├── commands/           # Tauri 命令薄层
│   ├── ops/                # 领域逻辑
│   ├── sync.rs             # 文件同步
│   ├── scanner.rs          # 目录扫描
│   ├── diff.rs             # LCS 差异算法
│   ├── db.rs               # SQLite 数据访问
│   ├── lock.rs             # LockManager
│   ├── lint.rs             # SKILL.md 健康检查
│   ├── market.rs           # 远程市场
│   ├── mcp.rs              # 对外 MCP stdio server（只读工具）
│   ├── watcher.rs          # 技能目录文件监听
│   └── ...
├── doc/                    # 规划文档（PRD、设计、阶段计划）
├── docs/                   # 问题追踪、UI 评审、市场改造文档
└── .github/workflows/      # CI：verify.yml + release.yml
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
| v0.5.0 | ✅ 已完成 | LockManager 接入、core_hash 变更检测 |
| v0.6.0 | ✅ 已完成 | 应用内更新、引导向导、健康检查/Lint、内置编辑器、市场批量操作、文件系统监听自动同步、GitLab 市场源、全站 Modal 无障碍统一、全局快捷键与 `?` 速查表、市场列表渲染优化、PRD §14 路径规则集中校验与表单提示、索引库损坏自愈、工作区自动发现与一键导入、定时检测更新（默认关闭）、全自动监听真正执行同步的修复 |
| v0.7.0 | ✅ 已完成 | MCP 集成：`skill-manager-mcp` 只读 stdio server（9 个工具，与 GUI 共用 SQLite 索引）+ 设置面板「MCP 服务」把 `skill-manager` 幂等登记进 Claude Code / Claude Desktop / Cursor / Qoder / Gemini CLI / Windsurf 的 JSON 与 Codex 的 TOML（`toml_edit` 保留注释），二进制随安装包分发（`bundle.externalBin` + `pnpm stage:sidecar`）；市场源补齐 Bitbucket Cloud、GitLab 自建实例与 Azure DevOps；冲突自动裁决（最新修改 / 首选工具） |

> 表中版本号是路线图阶段标号，与已发布的包版本相互独立（当前最新标签为 `v0.3.0`）。

## 许可证

[AGPL-3.0](LICENSE)
