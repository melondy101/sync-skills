# CLAUDE.md — Skill Manager

Tauri v2 桌面应用（Rust + React 19 + TypeScript + SQLite），用于跨 AI 编码工具同步 `SKILL.md`。
上游仓库：`github.com/huang-yi-dae/sync-skills`。

## 构建 / 开发（用 pnpm，不要用 npm）
- 安装依赖：`pnpm install`
- 开发：`pnpm tauri dev`
- 前端校验：`npx tsc --noEmit`；Rust 校验：`cd src-tauri && cargo check`
- 出安装包见下方「发布」，不要在本地手动 `tauri build` 出发布包

## 发布（Release）
- 触发方式：推送 `v*` 标签（`git tag vX.Y.Z && git push origin vX.Y.Z`）。
- CI：`.github/workflows/release.yml` 在 Windows / macOS / Linux 构建安装包，汇总到 GitHub Release 草稿。
- 普通 push 到 `main` **不会**触发构建；发布前在 Releases 页面将草稿转为 published。

## 领域规则（写代码前必看）
- SSOT 模型为「名字即身份」：同名 skill 跨工具合并为一条记录，多份安装。
- SSOT 路径按域隔离：`sync.rs` 的 `ssot_path(name, project_id)` —— 全局 `~/.agents/skills/local/<name>/`，项目 `_p<project_id>/<name>/`。改路径逻辑时注意域隔离，避免同名覆盖。
- 差异算法：`diff.rs` 基于 LCS（阈值 5000 行），前端以 unified / 并排两种视图展示。

## 文档指向
- 规划 / 设计：`doc/`（PRD、design-discussion、phase-3、DDL）
- 问题追踪与解决记录：`docs/testing-issues-triage.md`
- 用户文档：`README.md`（中 / 英 / 日）

## 不要提交
- `src-tauri/target/`、`node_modules/`、`dist/`（均已 gitignore）
