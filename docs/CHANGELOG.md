# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**说明**：当前 `main` 分支已超出 `v0.1.18` 标签，以下记录包含已合入但未发布的变更。正式版本号将在下次发版时统一 bump。

---

## [Unreleased]

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
- `transition: all` 改为显式属性列表

### Fixed
- 修复市场添加时仓库不可达的错误提示（HTTP cause  surfaced）
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
