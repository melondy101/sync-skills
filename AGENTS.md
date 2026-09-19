# AGENTS.md — Skill Manager

Tauri v2 桌面应用（Rust + React 19 + TypeScript + SQLite），用于跨 AI 编码工具同步 `SKILL.md`。
上游仓库：`github.com/huang-yi-dae/sync-skills`。

## 构建 / 开发（用 pnpm，不要用 npm）
- 安装依赖：`pnpm install`
- 开发：`pnpm stage:sidecar` → `pnpm tauri dev`。`bundle.externalBin` 会让 tauri 的构建脚本在**每一次** cargo 调用时校验 `src-tauri/bin/skill-manager-mcp-<target-triple>[.exe]` 是否存在，所以全新 clone 必须先 stage（此时没有构建产物，脚本写占位文件打破这个环），首次构建完成后**再 stage 一次**换成真二进制。跳过就会报 `resource path ... doesn't exist`（`tauri dev`、`tauri build`、`cargo build` 都会）
- 提交前校验：命令清单以 [CONTRIBUTING.md](CONTRIBUTING.md)「提交前请确保以下校验全部通过」一节为准（前端类型检查 / lint / 测试 + Rust check / clippy / 测试），此处不重复命令以避免漂移；CI 的 `.github/workflows/verify.yml` 执行同一组检查
- 出安装包见下方「发布」，不要在本地手动 `tauri build` 出发布包

## 发布（Release）
- 触发方式：推送 `v*` 标签（`git tag vX.Y.Z && git push origin vX.Y.Z`）。
- CI：`.github/workflows/release.yml` 在 **Windows 与 Linux** 两个 runner 构建安装包（macOS 自 `4eeb791` 起移出矩阵，`tauri.conf.json` 仍声明 `app`/`dmg`，要出 macOS 包就得把 `macos-latest` 加回 matrix），汇总到 GitHub Release 草稿。两个 job 都要先 `pnpm stage:sidecar`，否则 `bundle.externalBin` 会让 cargo 在空 `src-tauri/bin/` 上直接失败。
- 普通 push 到 `main` **不会**触发构建；发布前在 Releases 页面将草稿转为 published。

## 领域规则（写代码前必看）
- SSOT 模型为「名字即身份」：同名 skill 跨工具合并为一条记录，多份安装。
- SSOT 路径按域隔离：`sync.rs` 的 `ssot_path(name, project_id)` —— 全局 `~/.skill-manager/ssot/<name>/`，项目 `_p<project_id>/<name>/`。改路径逻辑时注意域隔离，避免同名覆盖；SSOT 必须位于任何工具扫描的 `skills/` 目录树之外（Codex/OpenCode 会扫 `~/.agents/skills/`，放里面会被重复识别为 skill）。
- 差异算法：`diff.rs` 基于 LCS（阈值 5000 行），前端以 unified / 并排两种视图展示。
- 市场 provider 共 4 家，统一走 `providers.rs` 的 `MarketProvider` seam（分发用 `provider_for_market`，不要再用 `provider_for(&market.provider)`——那会丢掉自建 GitLab 的实例地址）。凭据一律走环境变量：Azure DevOps 只读 `AZURE_DEVOPS_PAT`，**绝不**写进 `settings.json` 或任何配置快照。
- MCP 登记的写入面固定为「每家配置里的 `skill-manager` 一个键」：JSON 走 `serde_json` 往返（保留其他键，但会规范化排版），Codex 的 TOML 走 `toml_edit`（实测字节级保注释与格式）；目标工具的 config 目录不存在时跳过，绝不代为创建。

## 文档指向
- 贡献流程与代码规范：`CONTRIBUTING.md`
- 规划 / 设计：`doc/`（PRD、design-discussion、phase-3、phase4_design、market-ux-decisions）
- 问题追踪与解决记录：`docs/testing-issues-triage.md`
- 市场 UI 改造设计与执行手册：`docs/market-ui-redesign.md`、`docs/market-ui-impl-guide.md`
- 市场侧栏改造(2026-08-26 后续优化):`docs/market-sidebar-redesign.md`;执行用 tickets 在 `.scratch/market-sidebar-redesign/issues/`,GitHub issues #22–#28
- 当前进行中的设计 spec：`docs/spec-market-ux-polish.md`（Market UX 打磨 + Phase 4 范围，GitHub issues #4–#10）
- UI 评审报告：`docs/ui-review-2026-08-08.md`
- 接手 / 5 分钟入门：`docs/HANDOFF.md`
- 变更日志：`docs/CHANGELOG.md`
- 用户文档：`README.md`（中 / 英 / 日）
- UI 改进设计提案与执行计划：`docs/ui-improvement-proposals.md`、`docs/ui-improvement-plan.md`；执行用 tickets 在 `.scratch/ui-improvement-2026-08-23/issues/`

## Agent skills

Agent-facing scaffolding the engineering skills consume. Update these docs directly; re-run `/setup-matt-pocock-skills` only to switch trackers.

### Issue tracker

GitHub Issues on `huang-yi-dae/sync-skills`; `.scratch/<feature>/issues/` is a local working copy that mirrors GitHub issues (not a separate tracker). See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context repo — root-level `CONTEXT.md` and `docs/adr/`. Created lazily by `/domain-modeling`; not required to exist. See `docs/agents/domain.md`.

## 不要提交
- `src-tauri/target/`、`node_modules/`、`dist/`、`.pnpm-store/`、`src-tauri/bin/`（暂存用的 server 二进制）（均已 gitignore）
- 一次性 patch 脚本（`patch_*.py`）、scratch 笔记（`scratch/`、`.scratch/`）、临时截图（`artifacts/`）—— 历史 commit `22c64ef` 误把这些塞进了仓库，已删除该 commit，新提交务必保持工作目录干净
