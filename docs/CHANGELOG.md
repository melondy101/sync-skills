# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

（暂无未发布条目）

---

## [0.3.0] - 2026-09-19

### Added
- MarketProvider 抽象层：`providers.rs` 定义 `MarketProvider` trait，市场扫描/索引/安装统一走 provider 分发，不再把 GitHub 语义硬编码进 `ops/market.rs`
- GitLab 适配器：`providers.rs::GitlabProvider` 走 gitlab.com REST v4（tree API 分页游标 + 单层失败降级），URL 解析接受 https / http / SSH 与子组路径；自建实例见下条
- 对外 MCP stdio server（v0.7.0 第一片）：`src-tauri/src/mcp.rs` + 独立二进制 `skill-manager-mcp`，JSON-RPC 2.0 over stdin/stdout，暴露 9 个只读工具（`list_skills` / `get_skill` / `list_tools` / `list_projects` / `list_markets` / `list_market_skills` / `list_conflicts` / `get_sync_logs` / `get_stats`）读取与 GUI 相同的 SQLite 索引；协议派发是纯 line-in / line-out 函数，17 个单元测试无需进程即可覆盖全部契约
- 全局快捷键层：`hooks/useHotkeys.ts` 注册 Tab 切换、搜索聚焦、`?` 速查表（`ShortcutsModal.tsx`）等键位，市场卡片的 roving focus 复用同一选择器
- 市场卡片可访问性与键盘路径：13 个浮层统一走 `useFocusTrap`
- PRD §14 路径规则集中落地：`src-tauri/src/paths.rs` 一处判定 `~` 展开、环境变量、相对路径与目录是否存在，新 IPC `check_path`；`add_tool` / `update_tool_path` / `add_project` / `update_project` 全部改走同一套校验，工具与项目表单失焦即给出本地化提示（`i18n` 的 `localizeApiError` 按错误码翻译，不解析散文）
- 索引数据库损坏自愈：`Database::new` 先跑一次完整性检查，损坏的 `skill-manager.db` 连同 `-wal` / `-shm` 一起改名隔离为 `skill-manager.db.corrupt-<ts>`（绝不删除），重建后由 `get_db_recovery` 供前端显示一次性横幅
- 工作区自动发现：`src-tauri/src/workspaces.rs` 读 `~/.claude.json` 的 `projects` 键与 Cursor `workspaceStorage/*/workspace.json`，去掉已注册与已不存在的路径后经 `discover_workspaces` 交给项目面板逐条或一键导入
- 定时检查更新：`hooks/useUpdateSchedule.ts` + 设置项 `update_check_interval_minutes`（默认 0＝关闭），只在窗口打开时检测、不同步，发现更新才提示
- MCP 服务登记：`src-tauri/src/mcp_config.rs` 把 `skill-manager` 这一条幂等写入 Claude Code / Claude Desktop / Cursor / Qoder / Gemini CLI / Windsurf 的 JSON 配置（整份文件按 `serde_json::Value` 往返，其他键与服务不受影响；未创建配置目录的工具跳过且不代为建目录），设置面板新增「MCP 服务」分区展示每个工具的登记状态并提供登记/移除
- Bitbucket Cloud 市场源：`providers.rs::BitbucketProvider` 走 `api.bitbucket.org/2.0`（`src/{ref}/{path}` 单端点兼作目录列表与原始文件、`mainbranch.name` 取默认分支、`next` 游标原样跟随），URL 解析接受 https / http / SSH 形式；命令层、数据库与 UI 零改动
- GitLab 自建实例市场源：`GitlabProvider` 改为持有实例 `base`，主机地址复用已存在的 `markets.remote_url` 列（无 schema 变更）；`add_market_by_url` 对无法按名字命中的主机先匿名探测 `GET /api/v4/projects`，是 GitLab 就走该实例，否则回落 GitHub；索引/检查更新/下载三条路径统一改走 `provider_for_market`，自建市场重启后仍指向原实例
- Azure DevOps 市场源：`providers.rs::AzureDevopsProvider` 走 `dev.azure.com` REST 7.1（`_apis/git/.../items?recursionLevel=Full` 列目录 + `objects/{objectId}` 取原始字节 + `defaultBranch` 解析分支），凭据只从环境变量 `AZURE_DEVOPS_PAT` 读取，绝不写入 `settings.json`；URL 解析接受 `https://dev.azure.com/{org}/{proj}/_git/{repo}?version=GB{branch}` 与 `ssh://git@ssh.dev.azure.com/v3/{org}/{proj}/{repo}`。匿名访问受组织策略限制（实测 `azure-sdk` 返回登录页、`dnceng` 返回 `TF401019`），故本项仅有单元测试证据，真实抓取需自备 PAT
- `skill-manager-mcp` 进入安装包：`tauri.conf.json` 声明 `bundle.externalBin`，`build.beforeBundleCommand` 调 `scripts/stage-sidecar.mjs`（`cargo metadata` + `rustc -vV` 求出 target 目录与 triple，把刚构建的 server 复制成 `src-tauri/bin/skill-manager-mcp-<triple>[.exe]`），安装后二进制与主程序同目录，设置面板登记的命令路径因此真实可达；`src-tauri/bin/` 已加入 `.gitignore`
- MCP 服务登记支持 Codex：`mcp_config.rs` 由"只写 JSON"改为按目标格式分派，新增 `~/.codex/config.toml` 的 `[mcp_servers.skill-manager]`，用 `toml_edit` 往返以保留用户的注释、其余服务表与格式；写入仍只碰这一个键，无法解析或 `[mcp_servers]` 不是表时该目标报错而其余目标继续
- 冲突自动裁决（M5 补全）：`commands/conflicts.rs` 抽出 `promote_version`，手动与自动共用同一条"提升为 SSOT 并广播"的路径；新 IPC `auto_resolve_conflicts` 提供 `newest`（比 `SKILL.md` 修改时间）与 `preferred-tool`（按工具列表顺序取第一个确实持有版本的工具）两种策略，时间戳读不到或并列时**不裁决**并把原因返回，冲突留在面板等手动处理；自动裁决在 `skill_conflicts.resolved_by` 记为 `auto:<strategy>:<tool>` 以区分人工点击；冲突横幅新增策略下拉 + 带确认对话框的「自动裁决」按钮

### Fixed
- Linux 上 `~\...` 形式的 Windows 路径被静默解析成一个畸形目录名：`scanner::expand_path` 的 `~` 分支写的是 `home.join(&path[2..])`，而 Unix 把反斜杠当普通字符，于是 `~\.claude\skills\` 展开成 `/home/<user>/\.claude\skills\`（一个真实存在但不该有的名字）；同一输入在 `paths::looks_absolute` 里却按分隔符处理（它无条件 `replace('\\', "/")`），两处对"什么算分隔符"的判断不一致。改为按 `/` 与 `\` 两种分隔符切成组件逐段 `join`，`edge_tests.rs` 补多段断言把这条固定在 scanner 这一层
- 全新 clone 根本构建不了（chicken-and-egg）：`bundle.externalBin` 由 tauri 的构建脚本在**每一次** cargo 调用时校验 `src-tauri/bin/skill-manager-mcp-<triple>[.exe]`，而该文件只能由构建产物 stage 出来，于是 `cargo build` / `tauri dev` 一律报 `resource path ... doesn't exist`。`pnpm stage:sidecar` 现在带 `--allow-placeholder`——无产物时先写空占位打破循环，首次构建后再跑一次即替换为真二进制（`beforeBundleCommand` 保持严格：缺真二进制就报错，绝不把占位打进安装包）。启动步骤同步补进 `AGENTS.md`、`CONTRIBUTING.md`、README ×3 与 `docs/HANDOFF.md`
- 全自动模式下文件监听只重扫、不同步：`App.tsx` 的 `skill-file-changed` 处理改为按 1.5s debounce 触发 `fullScan` + 同步，半自动模式仍只提示
- 本地开发/打包无法启动：加入第二个 `[[bin]]`（`skill-manager-mcp`）之后，`cargo run` 报 `unable to find binary 'skill-manager'`/`could not determine which binary to run`，而 `tauri dev` 与 `tauri build` 都经它启动。`src-tauri/Cargo.toml` 补 `default-run = "skill-manager"`（MCP 面板此前无法在真实窗口点测即源于此）
- 设置面板的语言选项显示原始键名 `langZH` / `langEN`：`SettingsPanel` 取 `t(\`lang${lang.toUpperCase()}\`)`，词典里却只有 `langZh` / `langEn`，`t()` 找不到键就回显键名。两侧词典改为 `langZH: "简体中文"` / `langEN: "English"`（语言名按惯例用自称，两种界面语言下都一样），并删除无人引用的 `langZh` / `langEn`
- 「移除登记」会重写本就没有我们这一条的工具配置：JSON 走 `serde_json` 往返，保存即把用户的排版重排一遍。`write_target` 现在在没有可移除的键时直接返回状态而不写盘（Codex 的 TOML 往返实测字节不变，无需此保护）

### Changed
- 持久化数据根统一按代码实际路径记载为 `~/.skill-manager/`（数据库 `skill-manager.db`、SSOT `ssot/`）：`AGENTS.md`、README ×3 的架构图与功能条目、`docs/HANDOFF.md`、`watcher.rs` 文档注释此前仍写作旧的 `~/.agents/skill-manager/`。工具共享的 `~/.agents/skills/` 未变，两者必须继续区分（SSOT 必须落在任何工具扫描的 `skills/` 树之外）
- `useFocusTrap` 改为模块级 LIFO 栈 + 单一 `window` 监听，叠加弹窗的 Esc 不再互相穿透
- i18n 目录化（`src/i18n/` index + zh + en）收尾修复：清理拆分遗留的错误 label 与类型漂移
- 市场卡片/列表行（`MarketSkillEntry.tsx`）与 `t`、source badge、动作回调全部保持引用稳定；搜索输入不再触发可见卡片的重渲染（同一技能集下按键：12 次渲染 → 0 次，`SkillMarketPanel.test.tsx` 有断言）
- `docs/HANDOFF.md`、`README.md`（中/英/日）、`CONTRIBUTING.md` 的目录树、版本徽章与路线图状态同步到当前代码

### Removed
- `db.rs` 中无任何调用方的 `update_content_hash`（哈希写入统一走 `update_skill_hashes`）

## [0.2.1] - 2026-08-29

### Added
- 市场侧栏布局重构：MarketTab 从单栏控制台改造为侧栏导航 + 主内容双栏布局；新 `MarketSidebar` 组件（240px 可折叠至 48px，带源过滤输入）；多选 filter chips（仅"所有源"模式显示）；Source Header（单源选中时展示分支/技能数/索引时间 + 同步/启用/禁用/删除操作）；Actions 折叠抽屉；卡片 source badge；差异化空状态（无技能 / 无匹配）；11 个新 SVG 图标；21 个新 i18n key
- Agent-facing scaffolding 文档：`docs/agents/domain.md`（单上下文仓库的域模型说明）、`docs/agents/issue-tracker.md`（GitHub Issues 与本地 `.scratch/` 镜像机制）
- 设计文档：`doc/local-sync-loop-design.md`（本地同步循环设计 spec）、`docs/market-sidebar-redesign.md`（侧栏改造设计 + 执行手册）

### Changed
- `AGENTS.md` 补充市场侧栏改造文档指向和 GitHub issues 范围 (#22–#28)
- `HANDOFF.md` 补充 `MarketSidebar.tsx` 组件说明和市场侧栏改造文档引用
- CHANGELOG 格式修正：非首条改动行去除 **加粗标记**（保持与 Keep a Changelog 格式一致）

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
- 市场侧栏布局重构：MarketTab 从单栏控制台改造为侧栏导航 + 主内容双栏布局；新 `MarketSidebar` 组件（240px 可折叠至 48px，带源过滤输入）；多选 filter chips（仅"所有源"模式显示）；Source Header（单源选中时展示分支/技能数/索引时间 + 同步/启用/禁用/删除操作）；Actions 折叠抽屉；卡片 source badge；差异化空状态（无技能 / 无匹配）；11 个新 SVG 图标；21 个新 i18n key
- user-selectable layout：支持 subdir/root 仓库布局
- Phase 4 / T1 — Settings 扩展：新增两项设置项 `view_market_diff_before_update`（默认 on，Market 检测到更新时弹差异对比）和 `auto_sync_on_file_change`（默认 off，为 Phase 4 / M12 文件监听预留）。后端 `Settings` 结构新增 `effective_auto_sync_on_file_change` 助手，封装"`auto_sync_on_file_change || sync_mode == "full-auto"`"的规范 OR，供下游消费方（文件监听等）统一引用。旧版 settings.json 自动向下兼容。详见 `docs/spec-market-ux-polish.md`（GitHub issue #5）。
- Phase 4 / T4 — Market 详情 Modal：点击远程技能卡片打开 `RemoteSkillDetailModal`，按顺序展示描述 → `RemoteSkillFileTree`（递归 SSOT，跳过 `SKILL.md` / `local.md` / 隐藏项）→ SKILL.md 只读预览 → 来源市场（`remote_url` / `ssot_path`）→ 本地安装状态 → 哈希对比行（6 字符 mono 前缀，点击展开完整 64 字符 hash + Copy；无 hover-preview）。新 IPC `get_remote_skill_detail`；后端域逻辑在 `ops::remote_skill_detail::build_remote_skill_detail`，命令层仅适配；新 `db::Database::get_remote_skill(id)` 单行查询（避免全表扫描）；`RemoteSkillDetail` 模型新增 `local_content_hash` / `local_core_hash` 字段。无障碍：`role="dialog"` + `aria-modal` + focus trap + Esc / backdrop 关闭 + 焦点归还 + 哈希按钮 Space/Enter 激活 + `aria-keyshortcuts="Enter Space"`。详见 `docs/spec-market-ux-polish.md`（GitHub issue #6）。
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
