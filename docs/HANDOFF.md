# Skill Manager — Handoff 文档

> 给接手这个项目的下一个开发者 / 下次会话的自己。
> 目标：5 分钟内理解项目现状、关键约定、当前阻塞与下一步。

---

## 1. 项目概览

**项目**：Skill Manager  
**定位**：跨 AI 编码工具的 Skill 同步管理器（桌面 GUI）  
**技术栈**：Tauri v2 + React 19 + TypeScript + SQLite (rusqlite)  
**许可证**：AGPL-3.0-only  
**上游仓库**：`github.com/huang-yi-dae/sync-skills`  

最新发布标签为 `v0.2.1`；`main` 在其之上还有未发布的提交（MarketProvider 抽象层与 GitLab 适配、全站 modal 焦点 trap 统一、全局快捷键、市场卡片重渲染优化）。发版走 `git tag vX.Y.Z && git push origin vX.Y.Z`，由 `.github/workflows/release.yml` 出包。详见 [CHANGELOG.md](CHANGELOG.md)。

---

## 2. 5 分钟快速启动

```bash
# 安装依赖（必须用 pnpm）
pnpm install

# 启动开发环境（前端 Vite + Rust 后端 + 桌面窗口）
pnpm tauri dev

# 前端检查
npx tsc --noEmit
pnpm lint
pnpm test

# Rust 检查
cd src-tauri && cargo check
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo test
```

数据库文件位置：`~/.agents/skill-manager.db`  
SSOT 位置：`~/.agents/skill-manager/ssot/`  

---

## 3. 目录结构（只记关键部分）

```
sync-skills/
├── src/
│   ├── App.tsx              # 主组件，管理全局状态与 Tab 路由
│   ├── App.css              # 手写 CSS 变量主题系统，不要引入新框架
│   ├── api.ts               # 所有 Tauri IPC 调用的统一封装
│   ├── types.ts             # TypeScript 类型定义
│   ├── i18n/
│   │   ├── index.ts          # makeT(lang) 查表 + TranslateFn 类型
│   │   ├── zh.ts             # 中文字符串
│   │   └── en.ts             # 英文字符串（缺 key 时的兜底）
│   ├── components/
│   │   ├── SkillMarketPanel.tsx   # 远程市场面板（最大组件，侧栏布局 v0.2.0）
│   │   ├── MarketSidebar.tsx     # 市场侧栏导航（可折叠，源过滤，T3 2026-08-26）
│   │   ├── MarketSkillEntry.tsx  # 市场卡片 / 列表行（React.memo，重渲染敏感）
│   │   ├── ShortcutsModal.tsx    # `?` 快捷键速查表（与 MARKET_HOTKEYS 同源）
│   │   ├── RemoteSkillDetailModal.tsx # 远程技能详情 Modal（T4）
│   │   ├── RemoteSkillFileTree.tsx    # 详情 Modal 内 SSOT 文件树（T4）
│   │   ├── MarketSourcesModal.tsx # 市场源管理弹窗
│   │   ├── RemoteUpdatesModal.tsx # 更新弹窗
│   │   ├── InstallDialog.tsx      # 远程技能安装对话框
│   │   ├── ConfirmDialog.tsx      # 二次确认弹窗
│   │   ├── ConfirmProvider.tsx    # 二次确认 context
│   │   ├── OnboardingWizard.tsx   # 首次启动引导
│   │   ├── LintModal.tsx          # Skill 健康检查弹窗
│   │   ├── SkillEditorModal.tsx   # SKILL.md 内置编辑器
│   │   ├── ConflictSection.tsx    # 冲突检测与裁决
│   │   ├── DiffView.tsx           # 差异视图（unified / 并排）
│   │   ├── UpdatesModal.tsx       # 本地更新弹窗
│   │   ├── LogsPanel.tsx          # 活动日志面板
│   │   ├── SettingsPanel.tsx      # 设置面板
│   │   ├── ToastContainer.tsx     # 全局 toast
│   │   └── ...
│   └── hooks/
│       ├── useTheme.ts
│       ├── useToasts.ts
│       ├── useFocusTrap.ts          # 全站 modal 焦点 trap：模块级 LIFO 栈 + 单一 window 监听
│       └── useHotkeys.ts            # 全局快捷键，modal 打开时自动让位（openDialogCount）
├── src-tauri/src/
│   ├── lib.rs                # Tauri 入口，注册所有 command + 启动 watcher
│   ├── commands/             # 薄命令层，每个域一个文件
│   │   ├── market.rs         # 远程市场（最大命令文件，~1000 行）
│   │   ├── syncing.rs        # 同步相关命令
│   │   ├── scan.rs           # 扫描命令
│   │   ├── projects.rs       # 项目 CRUD
│   │   ├── skills.rs         # 技能 enable/disable/操作
│   │   ├── conflicts.rs      # 冲突检测与裁决
│   │   ├── logs.rs           # 活动日志读写
│   │   ├── app_settings.rs   # 应用级设置
│   │   ├── tools.rs          # 工具路径注册
│   │   └── updater.rs        # 应用内更新
│   ├── ops/                  # 核心领域逻辑（按子域拆分）
│   │   ├── mod.rs            # 聚合 re-export + 同步/更新检测编排
│   │   ├── market.rs         # 市场索引 upsert 与哈希比对
│   │   └── remote_skill_detail.rs # 远程技能详情域逻辑（T4）
│   ├── sync.rs               # 文件复制/替换/SSOT 路径（按域隔离）
│   ├── scanner.rs            # 目录递归扫描 + front matter 解析
│   ├── diff.rs               # LCS diff 算法（阈值 5000 行）
│   ├── db.rs                 # SQLite 数据访问（~1900 行，含迁移）
│   ├── fs.rs                 # 可移植叶子模块：原子写 / 复制 / 删除（无 crate:: 依赖）
│   ├── http.rs               # 共享 reqwest 客户端工厂（要求开启 system-proxy feature）
│   ├── lock.rs               # LockManager（per-skill 锁）
│   ├── lint.rs               # SKILL.md 健康检查
│   ├── market.rs             # 远程市场领域逻辑
│   ├── models.rs             # Rust 数据结构
│   ├── settings.rs           # 设置读写
│   ├── providers.rs          # MarketProvider trait + GitHub / GitLab 适配器
│   ├── watcher.rs            # notify 文件监听 → `skill-file-changed` 事件
│   ├── discovery.rs          # 工具自动发现
│   ├── hash.rs               # content_hash / core_hash / id_hash
│   ├── mcp.rs                # 对外 MCP server：JSON-RPC 2.0 / stdio，9 个只读工具
│   └── bin/skill-manager-mcp.rs # 该 server 的独立可执行入口（GUI 进程不占用其 stdio）
├── doc/
│   ├── PRD.md                # 产品需求文档（当前最权威的规格说明）
│   ├── design-discussion.md  # 设计决策记录（含实现状态）
│   ├── phase-3.md            # 演进记录与待办
│   └── skill-manager-ddl.sql # 数据库 DDL
├── docs/
│   ├── testing-issues-triage.md
│   ├── market-ui-redesign.md
│   ├── market-ui-impl-guide.md
│   ├── market-sidebar-redesign.md  # 侧栏改造(2026-08-26)· tickets 在 .scratch/market-sidebar-redesign/issues/
│   ├── ui-review-2026-08-08.md
│   ├── HANDOFF.md            # 本文件
│   └── CHANGELOG.md          # 变更日志
└── .github/workflows/
    ├── verify.yml            # CI 质量门禁
    └── release.yml           # 多平台安装包构建
```

---

## 4. 关键约定（写代码前必看）

### 4.1 领域规则

| 规则 | 说明 | 违反后果 |
|------|------|---------|
| **名字即身份** | 同名 Skill 跨工具合并为一条记录，多份安装 | 数据不一致，UI 显示异常 |
| **SSOT 域隔离** | 全局：`~/.agents/skill-manager/ssot/<name>/`；项目：`_p<project_id>/<name>/` | 同名 Skill 互相覆盖 |
| **SSOT 必须在 skills/ 之外** | Codex/OpenCode 会扫 `~/.agents/skills/`，放里面会被重复识别 | 重复扫描，假阳性 |
| **前端不直接 invoke** | 所有 Tauri 调用必须走 `src/api.ts` | eslint 会报错 |
| **只改 Rust 标准库做文件操作** | 不用 shell 命令 | 跨平台不一致 |
| **所有源码文件带 SPDX 头** | `pnpm lint` 强制检查 | CI 失败 |
| **i18n 必须同步更新 zh + en** | 新增文案需同时提供两种语言（`src/i18n/`） | 不一致 |
| **浮层统一走 `useFocusTrap`** | 新 modal 必须带 `role` / `aria-modal` / `aria-label` / `tabIndex={-1}` 并接入该 hook | 焦点逃逸、Esc 串扰 |
| **市场卡片 props 保持引用稳定** | `MarketSkillEntry.tsx` 用 `React.memo`；行内闭包 / 每渲染新建对象都会失效 | 大列表每次键入全量重渲染 |

### 4.2 代码风格

- **前端**：函数组件 + hooks，不引入新依赖，样式写在 `App.css`
- **后端**：四层结构 — `lib.rs` 注册 → `commands/` 薄层 → `ops/`（按子域拆分的领域逻辑）→ `db.rs` 数据访问
- **数据库**：`rusqlite` + `Mutex<Connection>`，迁移逻辑在 `db.rs` 的 `init_schema()` 里
- **错误处理**：统一 `Result<T, String>`，不要 panic；字符串按字符而非字节索引

### 4.3 Git 工作流

- 分支：`feat/xxx` / `fix/xxx` / `docs/xxx` / `refactor/xxx` / `chore/xxx`
- Commit：Conventional Commits（`type(scope): subject`）
- PR：必须包含背景/动机、变更内容、测试、影响范围
- 发布：推送 `v*` 标签触发 CI，普通 push 不触发构建

---

## 5. 当前状态速查

### 5.1 已实现（核心）

| 功能 | 状态 | 关键文件 |
|------|------|---------|
| 工具管理（13+ 预设工具） | ✅ | `discovery.rs`, `commands/tools.rs` |
| Skill 扫描（递归 + front matter） | ✅ | `scanner.rs`, `commands/scan.rs` |
| SSOT 同步 + 反向同步 | ✅ | `sync.rs`, `ops/`, `commands/syncing.rs` |
| 名字即身份（name-as-identity） | ✅ | `db.rs`（UNIQUE(name, project_id)） |
| core_hash（仅 SKILL.md） | ✅ | `hash.rs`, 用于冲突检测与更新检测（`ops/mod.rs::check_single_skill`） |
| content_hash（全目录） | ✅ | `hash.rs`, 用于 diff 展示 |
| LockManager | ✅ | `lock.rs`, `ops/` 多处使用 |
| 冲突检测与裁决 | ✅ | `commands/conflicts.rs`, `ConflictSection.tsx` |
| 差异预览（unified / 并排） | ✅ | `diff.rs`, `DiffView.tsx` |
| 变更忽略 | ✅ | `db.rs` dismissed_updates 表 |
| 远程市场（GitHub + GitLab） | ✅ | `market.rs`, `providers.rs`, `commands/market.rs`, `SkillMarketPanel.tsx`, `MarketSidebar.tsx` |
| Market 详情 Modal（T4） | ✅ | `ops/remote_skill_detail.rs`, `commands/market.rs::get_remote_skill_detail`, `RemoteSkillDetailModal.tsx`, `RemoteSkillFileTree.tsx`, `useFocusTrap.ts` |
| Modal 无障碍统一（13 个浮层） | ✅ | `useFocusTrap.ts`（模块级 LIFO 栈 + 单一 window 监听），各 `*Modal.tsx` |
| 文件系统监听（full-auto 自动同步） | ✅ | `watcher.rs`（`notify = "8"`），`lib.rs::start_watcher`，事件 `skill-file-changed` |
| 全局快捷键 + `?` 速查表 | ✅ | `hooks/useHotkeys.ts`, `components/ShortcutsModal.tsx`, `SkillMarketPanel.tsx::MARKET_HOTKEYS` |
| 应用内更新 | ✅ | `commands/updater.rs` |
| 首次启动引导 | ✅ | `OnboardingWizard.tsx` |
| Skill 健康检查 / Lint | ✅ | `lint.rs`, `LintModal.tsx` |
| 内置 SKILL.md 编辑器 | ✅ | `SkillEditorModal.tsx` |
| 主题切换 + 多语言 | ✅ | `App.css`, `src/i18n/`（`index.ts` + `zh.ts` + `en.ts`） |
| 活动日志 | ✅ | `LogsPanel.tsx`, `commands/logs.rs` |

### 5.2 部分完成 / 待办

| 功能 | 状态 | 说明 |
|------|------|------|
| 时间戳驱动同步 UI | ⏳ 部分 | 时间戳已展示并用于排序，未驱动按钮状态 |
| MCP 集成 | ⏳ 进行中 | v0.7.0 第一片：只读 stdio server（`mcp.rs`）；配置管控与跨工具同步尚未开工 |
| GitLab 自建实例 | ⏳ 规划中 | `providers.rs::provider_for_reference` 目前只把 `gitlab.com` 路由到 GitLab 适配器 |
| Bitbucket / Azure DevOps 适配 | ⏳ 规划中 | `MarketProvider` 已有 GitHub + GitLab 两个真实适配器，新增只需实现该 trait |

### 5.3 已知坑与注意事项

| 问题 | 说明 | 规避方案 |
|------|------|---------|
| Windows 链接器错误 | `cargo check` 偶发 `link.exe error 1224` / `LNK1105` | 杀毒软件或残留进程占用，重试一次即可 |
| 本机双 Rust 工具链 | PATH 上的旧 cargo 会引发 E0514 | 用 `rustup run stable cargo …` |
| 远程市场 ZIP 解析 | 部分仓库 layout 判断可能不准 | 用户可手动切换 layout（subdir/root） |
| 数据库并发 | `Mutex<Connection>` 是全局锁 | 当前数据量下可接受，如需性能再优化 |
| GitLab tree 分页上限 | `providers.rs` 每页 100、最多 10 页，超出会静默截断 | 单目录 >1000 条 listing 的市场需提高 `TREE_MAX_PAGES` |
| 市场卡片重渲染 | 卡片 props 必须引用稳定，否则 `React.memo` 失效 | `t` 在 `App.tsx` 已 memo；新增回调走 `skillActions`，不要传行内闭包 |

---

## 6. 常见任务速查

### 6.1 新增 Tauri Command

1. 在 `src-tauri/src/commands/` 新建或扩展现有文件
2. 在 `lib.rs` 的 `invoke_handler` 注册
3. 在 `src/api.ts` 添加 TypeScript 封装
4. 在 `src/types.ts` 添加返回类型（如需）
5. 在 `doc/PRD.md` §17.3 补充接口清单

### 6.2 修改数据库 schema

1. 在 `db.rs` 的 `init_schema()` 的 `execute_batch` 里加 `ALTER TABLE` / `CREATE TABLE`
2. 在 `models.rs` 加对应 struct
3. 如需迁移旧数据，在 `init_schema()` 后面加 `migrate_xxx()` 并调用
4. 更新 `doc/skill-manager-ddl.sql`

### 6.3 新增前端页面/组件

- 页面级组件放在 `src/components/`
- 全局状态放在 `App.tsx`，局部状态用 `useState`/`useMemo`
- 文案必须走 `src/i18n/`，新增 key 必须同时更新 zh + en
- 样式写在 `App.css`，复用现有 CSS 变量，不引入新依赖

### 6.4 调试技巧

- **前端**：浏览器 DevTools（Tauri dev 模式下可用）
- **后端**：`println!` / `log::info!` / `log::error!`，输出在终端
- **数据库**：直接打开 `~/.agents/skill-manager.db` 用 SQLite 工具查看
- **SSOT**：查看 `~/.agents/skill-manager/ssot/` 目录结构

---

## 7. 测试策略

- **前端**：`pnpm test`（vitest），覆盖组件逻辑和 hooks
- **Rust**：`cargo test`，单元测试集中在 `src-tauri/src/` 各模块和 `edge_tests.rs`
- **E2E**：Playwright 已配置，但当前主要做冒烟测试
- **手动验证**：UI 评审见 `docs/ui-review-2026-08-08.md`

---

## 8. 重要决策记录

| 决策 | 原因 | 文档 |
|------|------|------|
| 手写 CSS 而非 Tailwind | 项目已有完整 CSS 变量体系，迁移成本高 | `docs/market-ui-redesign.md` |
| 用 rusqlite 而非 tauri-plugin-sql | Rust 侧统一处理扫描/哈希/DB 写入，事务控制更灵活 | `doc/PRD.md` §17.2 |
| 名字即身份 | 同名 Skill 跨工具合并，避免重复 | `AGENTS.md` |
| SSOT 放在 skill-manager/ 下 | 避免被 Codex/OpenCode 扫描到 | `AGENTS.md` |
| local.md 保留 | 标记"此目录由 Skill Manager 管理"，防止误删 | `doc/phase-3.md` |

---

## 9. 下一步建议

如果你要继续开发，建议按这个顺序：

1. **熟悉核心流程**：跑一遍 `pnpm tauri dev`，执行一次完整扫描 → 同步 → 检查更新 → 远程安装
2. **读 `ops/`**：它是核心领域逻辑的集中地（`ops/mod.rs` + 各子模块），理解了它就理解了 60% 的业务
3. **读 `db.rs`**：理解数据模型和迁移机制
4. **读 `commands/market.rs`**：当前最复杂的命令文件，理解远程市场流程
5. **跑测试**：`pnpm test && cd src-tauri && cargo test`，确保基线干净

---

## 10. 联系方式

- **维护者**：huang-yi-dae
- **仓库**：`github.com/huang-yi-dae/sync-skills`
- **Issues**：GitHub Issues
- **License**：AGPL-3.0-only
