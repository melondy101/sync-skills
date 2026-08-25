# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] - 2026-08-26

### Added
- 远程 Skill 市场功能（§5.9）：从 GitHub 仓库浏览、搜索、一键安装 Skill
- 内置 5 个默认市场源（anthropics/skills、superpowers/skills、mattpocock/skills、karpathy/skill、khazix/skills）
- 应用内更新功能（§5.10）：检测、下载、安装新版本
- 首次启动引导向导（Onboarding Wizard）
- Skill 健康检查 / Lint 功能：检查 front matter、路径等问题，支持自动修复
- 内置 SKILL.md 编辑器：直接在应用内编辑 Skill 文件
- 活动日志面板：完整审计记录
- 远程技能安装对话框（InstallDialog）：选择目标项目 + 工具
- 批量操作：全部扫描、全部标记已安装、全部取消标记、同步所有已安装
- 远程更新检测：check_remote_updates / check_remote_ssot_updates
- 市场源管理弹窗：增/删/启用/禁用/同步索引
- 远程技能 provenance badge：显示技能来源市场
- user-selectable layout：支持 subdir/root 仓库布局
- **Phase 4 / T1 — Settings 扩展**：新增两项设置项 `view_market_diff_before_update`（默认 on，Market 检测到更新时弹差异对比）和 `auto_sync_on_file_change`（默认 off，为 Phase 4 / M12 文件监听预留）。后端 `Settings` 结构新增 `effective_auto_sync_on_file_change` 助手，封装"`auto_sync_on_file_change || sync_mode == "full-auto"`"的规范 OR，供下游消费方（文件监听等）统一引用。旧版 settings.json 自动向下兼容。详见 `docs/spec-market-ux-polish.md`（GitHub issue #5）。
- **Phase 4 / T4 — Market 详情 Modal**：点击远程技能卡片打开 `RemoteSkillDetailModal`，按顺序展示描述 → `RemoteSkillFileTree`（递归 SSOT，跳过 `SKILL.md` / `local.md` / 隐藏项）→ SKILL.md 只读预览 → 来源市场（`remote_url` / `ssot_path`）→ 本地安装状态 → 哈希对比行（6 字符 mono 前缀，点击展开完整 64 字符 hash + Copy；无 hover-preview）。新 IPC `get_remote_skill_detail`；后端域逻辑在 `ops::remote_skill_detail::build_remote_skill_detail`，命令层仅适配；新 `db::Database::get_remote_skill(id)` 单行查询（避免全表扫描）；`RemoteSkillDetail` 模型新增 `local_content_hash` / `local_core_hash` 字段。无障碍：`role="dialog"` + `aria-modal` + focus trap + Esc / backdrop 关闭 + 焦点归还 + 哈希按钮 Space/Enter 激活 + `aria-keyshortcuts="Enter Space"`。详见 `docs/spec-market-ux-polish.md`（GitHub issue #6）。
- UI 设计 token 体系扩展（`src/ui-enhancements.css`）：pill tab、暗色卡片顶部高光线、亮色卡片阴影、accent / error 状态左边框、synced-dot、8 套 avatar 渐变预设、action-bar 分组、settings 双栏布局、theme cards、market chips、wizard splash
- 零依赖 SVG 图标系统：`src/components/Icon.tsx`（23 个图标），`src/components/SkillAvatar.tsx`（36px 首字母渐变 tile，颜色由 skill name hash 决定），`src/components/ToggleSwitch.tsx`
- OnboardingWizard 改造为全屏 branded splash（中心 hero + dot 步骤指示器 + 2 列工具 grid + 主 CTA 居中）
- SettingsPanel 改造为双栏布局（左侧分类导航：appearance / sync / proxy / update；右侧内容区）；主题与语言切换改为视觉卡片；boolean 设置统一使用 ToggleSwitch
- 顶部 action-bar 拆三组（主操作 / 搜索 / 视图 + 溢出菜单），健康检查 / 日志 / 设置收纳到 `⋯` 菜单
- Skill 卡片加 SkillAvatar，30+ skills 不再混成一团
- 全部工具同步的 skill 在 toggle 区出绿色 synced-dot
- 五级字号 CSS 变量（`--fs-caption` / `--fs-meta` / `--fs-body` / `--fs-title` / `--fs-heading`），替代散乱的 10/11/12/13/14/15 px；遗留的 7 处 `font-size: 10px` 全部归位
- 本地打包字体：`@fontsource-variable/dm-sans` + `@fontsource-variable/jetbrains-mono`（通过 `src/main.tsx` import），移除对 Google Fonts CDN 的依赖，离线 / 内网 / Tauri 环境也能正常显示
- 新增 i18n key：`wizardTagline`、`allSynced`、`moreActions`、`addMarketSources`（zh + en）

### Changed
- 工具预设从 5 个扩展到 13+ 个（新增 Cursor、Windsurf、Aider、Roo Code、Trae、Kiro、Augment、Qoder）
- SSOT 路径从 `~/.agents/skills/` 迁移到 `~/.agents/skill-manager/ssot/`，按 `(name, project_id)` 域隔离
- 数据库 schema 扩展：新增 `core_hash`、`ssot_updated_at`、`installation_synced_at`、`source_market_id`、`dismissed_updates`、`markets`、`remote_skills`、`remote_installations` 等字段/表
- `local.md` 语义明确为"此目录由 Skill Manager 管理"
- 前端 UI：市场 Tab 从表格控制台改造为卡片式浏览视图
- 全局/项目 Tab 的 action-bar 重构：分组 + 溢出菜单
- 对比度 tokens 修正（accent / text-muted 达到 AA 标准）
- 全局 `:focus-visible` 焦点环
- 图标从 unicode 统一为 SVG（部分组件）
- 全部 unicode 图标统一替换为 SVG（品牌 ⬡、视图切换 ▦/☰、更新 ●、同步 ⟳、编辑 ✎、关闭 ×、警告 ⚠、成功 ✓、日志方向 → / ←、modal 关闭 ✕ 等）
- `SkillMarketPanel` 筛选 chip 改为统一 `market-chip` class，末尾追加 `+` chip 打开添加市场源弹窗（避免与 modal 内的 "Add Market" 撞 aria-label）
- SettingsPanel 测试同步升级到两栏布局（用 `role="switch"` + `aria-checked` 替代 `role="checkbox"`，并要求先点击分类导航）
- `transition: all` 改为显式属性列表

### Fixed
- 修复市场筛选的「Add Market」按钮与新增 `+` chip 在无障碍名称上的冲突（前者用 `addMarket`，后者用 `addMarketSources`）
- 修复市场添加时仓库不可达的错误提示（HTTP cause surfaced）
- 修复市场添加后自动同步索引 + 每源错误条
- 修复批量操作二次确认（删除市场、sync-all）
- 修复 sync-all 进度显示 + ConfirmDialog 抽到 context
- 修复市场源按钮对齐问题（error row 保持整洁）
- 修复批量菜单 hover/chevron/open state 样式
- 修复 App.css 多余闭合括号
- 清理未使用的 market CSS 类和 i18n key
- 修复 Windows 下 `link.exe error 1224` 的兼容性问题说明
- 修复 startup deadlock（market mutex 释放顺序）
- 修复系统代理调用在非 Windows 平台的编译问题
- 修复 winreg 在非 Windows 平台的编译问题

### Security
- 信任操作系统证书存储（rustls-tls-native-roots），解决 GitHub 连接 MITM 代理的证书问题

### Design Docs
- `docs/ui-improvement-proposals.md`：竞品对标（Raycast / Linear / VS Code / Obsidian / Zed / Warp）+ 逐屏优化方案
- `docs/ui-improvement-plan.md`：10 任务执行手册 + 15 张竖切 tickets（四 commit 节奏：A tokens + icons；B cards + bar + signals；C onboarding + settings + market；D type + fonts）
- `docs/ui-mockups/before-after-comparison.html`：改造前后视觉对比

---

## [0.1.18] - 2026-08-11

### Added
- 远程市场基础功能（Market Tab）
- 市场源 CRUD + 同步索引
- 远程技能卡片浏览
- 一键安装到 SSOT + 工具
- 远程更新检测

### Changed
- 前端架构重构：Global / Projects / Market 三 Tab
- 技能卡片支持描述展示（解析 SKILL.md front matter）
- 项目级 Skill 隔离强化

### Fixed
- 多项 UI/UX 问题（见 `docs/ui-review-2026-08-08.md`）

---

## [0.1.17] - 2026-08-05

### Added
- 应用自更新基础框架（check / download / install）
- 系统代理支持（system-proxy）

### Fixed
- 非 Windows 平台编译问题（winreg、system-proxy 条件编译）

---

## [0.1.16] - 2026-08-01

### Added
- 项目编辑功能（重命名、修改路径）
- 反向同步（SSOT → 指定工具目录）

### Changed
- 同步后自动刷新 hash 并更新 source_path 到 SSOT
- 禁用 Skill 时自动从工具目录移除

---

## [0.1.15] - 2026-07-28

### Added
- 冲突检测与裁决 UI（ConflictSection）
- 冲突 Diff 预览
- 变更忽略（dismissed_updates）

### Changed
- 名字即身份模型落地（name-as-identity）
- 数据库唯一约束从 `source_path` 改为 `(name, project_id)`

---

## [0.1.14] - 2026-07-20

### Added
- 时间戳字段（ssot_updated_at、installation_synced_at）

### Changed
- 扫描结果展示优化
- 同步日志细化（direction、status、error_message）

---

## [0.1.13] - 2026-07-15

### Fixed
- 工具目录修改后同步异常（Bug #1）
- 项目级路径错误（显示全局路径而非项目路径）
- 检测机制循环误报（按表单顺序逐个检测）

### Changed
- `check_single_skill` 返回 `Vec<SkillUpdate>`，全量收集
- `do_sync_skill` 接受 `source_override` 参数

---

## [0.1.12] - 2026-07-10

### Added
- LockManager 骨架

### Changed
- 冲突检测使用 core_hash 比较

---

## [0.1.11] - 2026-07-05

### Added
- core_hash 字段（仅 SKILL.md 的 SHA-256）

### Changed
- 哈希稳定性修复（local.md 跳过）

---

## [0.1.10] - 2026-07-01

### Added
- 主题切换（亮色/暗色/跟随系统）
- 多语言支持（中文/English/日本語）

### Fixed
- 多处 UI 细节问题

---

## [0.1.9] - 2026-06-25

### Added
- 自动发现已安装工具
- 排序/筛选功能

### Changed
- 扫描性能优化

---

## [0.1.8] - 2026-06-20

### Added
- Diff 视图（unified / 并排两种模式）
- 差异检测（LCS 算法，阈值 5000 行）

### Changed
- 更新弹窗支持按工具展开 + 左视图同步定位

---

## [0.1.7] - 2026-06-15

### Added
- 检查更新按钮
- 全量扫描 / 范围扫描

### Changed
- 扫描结果展示优化

---

## [0.1.6] - 2026-06-10

### Added
- 项目级管理（Projects Tab）
- 项目 CRUD

### Changed
- 工具路径配置分全局/项目两级

---

## [0.1.5] - 2026-06-05

### Added
- 启用/禁用 Skill
- 同步所有待同步 Skill（sync-all）

### Changed
- 同步流程优化（工具 → SSOT → 其他工具）

---

## [0.1.4] - 2026-06-01

### Added
- 操作反馈（Toast / Dialog）
- 同步日志查看

### Changed
- 错误处理细化（文件操作、数据库错误）

---

## [0.1.3] - 2026-05-25

### Added
- 路径格式容错（~/、C:\、/home/）
- 隐藏目录跳过
- front matter fallback（缺少 name 时用目录名）

---

## [0.1.2] - 2026-05-20

### Added
- 递归扫描 SKILL.md
- 数据库初始化 + seed data

### Changed
- SSOT 目录结构确定

---

## [0.1.1] - 2026-05-15

### Added
- 工具路径配置（CRUD）
- 项目列表管理

---

## [0.1.0] - 2026-05-10

### Added
- 项目初始化
- Tauri v2 + React + SQLite 技术栈搭建
- 基础 UI 框架
