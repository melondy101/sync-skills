# 市场界面改造设计文档

- 日期：2026-08-10
- 状态：**已实现**（2026-08-11 完成于分支 `codex/ui_optimization`；执行手册：`docs/market-ui-impl-guide.md`；提交列表见 `git log --oneline codex/ui_optimization`）
- 范围：「市场」Tab 整体重设计 + 相关全局设计规范修正
- 方法依据：ui-designer 工作流 + [jakubkrehel/skills](https://github.com/jakubkrehel/skills)（better-interface / better-layout / better-ui / better-colors / better-typography / better-writing）
- 关联文档：`docs/ui-review-2026-08-08.md`（全量 UI 评审，本文档继承其 P0/P1 结论）、`doc/PRD.md`、`doc/design-discussion.md`

---

## 1. 背景与目标

Skill Manager 的「市场」Tab 提供从 GitHub 仓库发现、下载、同步 Skill 的能力。当前实现（v0.1.15 重构后）以**数据管理控制台**的形态呈现：市场源表格、技能表格、更新哈希表格加两行操作栏。用户的反馈是"不好看，和心智模型有差距"。

本文档通过代码级现状诊断与同类产品心智模型调研，给出市场界面的改造方案。

**改造目标（按优先级）：**

1. **心智对齐**：把"市场"从"仓库管理控制台"改造为用户熟悉的应用商店式体验——浏览、搜索、一键安装。
2. **隐藏内部机制**：SSOT、ssot_path、remote_url、hash、branch 等实现细节退出主浏览界面。
3. **视觉升级**：复用并延伸现有卡片式设计语言（全局/项目 Tab 已有 skill-card 体系），统一设计 token。
4. **不引入新技术栈**：保持手写 CSS + CSS 变量体系，不引入 Tailwind 或组件库（与 2026-08-08 评审结论一致）。

---

## 2. 现状诊断

### 2.1 页面结构现状

`src/components/SkillMarketPanel.tsx`（616 行）自上而下由 4 个区块构成：

| 区块 | 内容 | 形态 |
| --- | --- | --- |
| ① 市场源管理 | 添加市场表单（URL+分支）、市场筛选下拉、市场源表格（provider/owner/name/branch/enabled/lastIndexedAt + 停用/同步索引按钮） | 可折叠 section + `<table>` |
| ② 操作区 | 两条 action-bar 共 8 个控件：搜索、检查更新、全部扫描、installed/all 选择器、项目选择器、范围选择器、工具选择器、同步所有激活、全部标记已安装、全部取消标记 | 两行按钮+下拉 |
| ③ 技能列表 | skill_name / remote_url / ssot_path / 安装状态 / 索引时间 + 两个操作按钮 | `<table>` |
| ④ 更新列表 | market / skill / remote_url / old hash / new hash | `<table>` |

### 2.2 与用户心智模型的差距（核心问题）

| 用户对"市场"的预期 | 当前实际 |
| --- | --- |
| 进来就能**浏览和搜索**技能，卡片上有名字、描述、来源 | 进来先看到市场源配置表格和技能数据表格；技能无描述展示 |
| 点一次**「安装」**就完成 | 两步分离操作：「下载到 SSOT」→「同步到工具」，用户需要理解 SSOT 概念 |
| 安装目标在**安装时**选择 | 项目/范围/工具三个选择器常驻操作栏，选技能之前就被迫面对 |
| 市场源（仓库）是**低频设置** | 市场源表格占据页面第一屏主位 |
| 更新是**徽标+一键更新** | 原始 hash 对照表格（old/new 各 12 位） |
| 状态一眼可见（已安装/可更新） | 状态是表格中的一列文字（"已安装"/"未安装"） |

### 2.3 具体问题清单（代码证据）

**数据利用不足**

| # | 位置 | 问题 |
| --- | --- | --- |
| D1 | `src-tauri/src/commands/market.rs:253` | 索引远端技能时 description 硬编码为 `None::<String>`，从未解析 SKILL.md frontmatter——本地扫描的 `scanner.rs::parse_front_matter`（scanner.rs:112）已具备解析能力却未被复用。卡片化展示的最大数据障碍，**必须修** |
| D2 | `src/components/SkillMarketPanel.tsx:556-577` | `RemoteSkill.description` 字段在类型中存在（types.ts:239），但表格 UI 完全没有展示 |
| D3 | `src-tauri/src/commands/market.rs:17-65` | 内置市场模板带 `label`（如 "Anthropic Skills"），但 `markets` 表不持久化 label，UI 只能显示 `owner/name`，内置市场仅以 option 文本后缀 `[builtin]` 标记（SkillMarketPanel.tsx:392） |

**信息架构**

| # | 位置 | 问题 |
| --- | --- | --- |
| A1 | SkillMarketPanel.tsx:459-530 | 两条 action-bar 共 8+ 控件无分组（2026-08-08 评审 Finding #8 已指出）；安装目标选择器与浏览操作混杂 |
| A2 | SkillMarketPanel.tsx:412-454 | 市场源表格暴露 provider/branch/lastIndexedAt 等运维字段，且表头硬编码英文（Finding #9） |
| A3 | SkillMarketPanel.tsx:588-612 | 更新区展示原始 hash 片段，用户无法据此决策；且无更新动作入口（只读表格） |
| A4 | i18n.ts `remoteInstallsTab` / `skillMarketTab` | 子 Tab 方案（市场/安装管理/Skills）在 v0.1.15 重构中移除后遗留的孤儿 key，说明信息架构经历过一次摇摆 |

**视觉与样式**

| # | 位置 | 问题 |
| --- | --- | --- |
| V1 | App.css | `.market-select-field` / `.market-select-label` / `.market-select` / `.add-market-form` 在组件中使用但**没有任何对应 CSS**，依赖浏览器默认样式；另有内联样式（`marginBottom:10`、`fontSize:12`）散落（SkillMarketPanel.tsx:343,365,486） |
| V2 | SkillMarketPanel.tsx:306-313 | 搜索只匹配 skill_name 和 remote_url，不匹配 description（即使 D1 修复后也不生效） |
| V3 | 全局 | 市场 Tab 与全局/项目 Tab 的展示语言割裂：后者有卡片网格/列表切换、骨架屏，前者只有裸表格 |
| V4 | 继承 ui-review | 对比度不达标（--accent 白字 3.28:1、--text-muted 2.68:1）、无 :focus-visible、字体远程加载、unicode 图标（⬡▸▾●⟳）、`transition:all` —— 详见 `docs/ui-review-2026-08-08.md` Finding #1-#14 |

---

## 3. 心智模型调研："市场"应该长什么样

### 3.1 同类产品的共性模式

调研 VS Code Extensions Marketplace、Raycast Store、Obsidian Community Plugins、Figma Community、Homebrew/npm 等"市场/商店"类产品，提炼出高度一致的五个模式：

1. **浏览优先（Browse-first）**：首屏即内容。搜索框置顶，主体是技能卡片/列表行，而不是源配置。源（registry/repository）是设置项，不是浏览对象——VS Code 用户从不"管理 gallery"，npm 用户很少改 registry。
2. **条目即卡片（Item = Card/Row）**：每个条目展示名称、一句话描述、作者/来源、热度或新鲜度信号、状态徽标。描述是决策的核心信息，绝不缺席。
3. **单一动词安装（One Verb Install）**：主操作只有一个——Install / Add / Get。所有高级选项（安装位置、范围）在安装时刻才出现（渐进披露），而不是常驻界面。Obsidian 的 Install→Enable 两步也收敛在详情页内完成。
4. **状态即徽标（Status = Badge）**：已安装、可更新用徽标与按钮形态变化表达（Installed ✓ / Update），不用文字列。
5. **更新可操作（Actionable Updates）**：更新入口带数字徽标，更新视图每条带"更新"动作，而非只读对照表。

### 3.2 映射到 Skill Manager

Skill Manager 的特殊性在于"一处管理，多处生效"：安装一个技能涉及**目标项目（全局/项目）**与**目标工具（Claude Code / Codex 等）**两个维度。调研结论是：这两个维度应当收敛进**安装时刻的对话框**，并记忆上次选择，而不是常驻操作栏。

由此确立改造后的三层心智模型：

- **浏览层**（默认视图）：搜索 + 按市场筛选 + 技能卡片网格
- **安装层**（按需触发）：安装对话框（目标项目 + 目标工具 + 记忆选择）
- **管理层**（低频入口）：市场源管理弹窗、批量操作收进溢出菜单

---

## 4. 重设计方案

### 4.1 信息架构对比

```
现状（控制台模型）                       改造后（商店模型）
─────────────────────                  ─────────────────────
① 市场源表格（折叠）                     工具栏: [搜索] [市场筛选chips] [源管理…] [检查更新ⁿ]
② 操作栏×2（8+控件）                    技能卡片网格（浏览主体）
③ 技能表格                              安装 → 对话框（项目+工具，记忆选择）
④ 更新哈希表格                          更新 → 徽标 + 更新弹窗
                                       市场源管理 → 弹窗（增删/停用/同步索引）
                                       批量操作 → 工具栏溢出菜单（⋯）
```

组件结构对应调整：`SkillMarketPanel.tsx` 拆分为

- `MarketToolbar`（搜索、筛选 chips、检查更新、溢出菜单）
- `MarketSkillGrid` / `MarketSkillCard`（复用 `.skill-grid`/`.skill-card` 设计语言）
- `InstallDialog`（安装对话框，模态，遵循 modal 无障碍修复要求）
- `MarketSourcesModal`（市场源管理弹窗）
- `RemoteUpdatesModal`（更新结果弹窗，复用 UpdatesModal 的交互模式）

### 4.2 主界面：浏览视图

```
┌──────────────────────────────────────────────────────────────────────┐
│ [🔍 搜索技能（名称或描述）…              ] [全部市场 ▾]  [⋯] [检查更新]│
├──────────────────────────────────────────────────────────────────────┤
│ 筛选: ( 全部 ) ( anthropic-skills ) ( superpowers ) ( + )   [仅已安装]│
├──────────────────────────────────────────────────────────────────────┤
│ ┌────────────────────┐ ┌────────────────────┐ ┌────────────────────┐ │
│ │ pdf-tools          │ │ code-review     ⟳  │ │ git-workflow    ✓  │ │
│ │ PDF 生成、拆分、合 │ │ 基于证据的代码审查 │ │ Git worktree 隔离  │ │
│ │ 并页面…            │ │ 工作流…            │ │ 开发流程…          │ │
│ │                    │ │                    │ │                    │ │
│ │ anthropics/skills  │ │ superpowers/skills │ │ mattpocock/skills  │ │
│ │ ────────────────── │ │ ────────────────── │ │ ────────────────── │ │
│ │           [安装]   │ │           [更新]   │ │ [已安装] [同步]    │ │
│ └────────────────────┘ └────────────────────┘ └────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

**卡片规格**（延伸现有 `.skill-card` 语言，具体 token 见第 5 节）：

| 元素 | 规格 |
| --- | --- |
| 技能名 | 14px / 600，单行截断（ellipsis），`--text` |
| 更新指示 | 名称右侧 ⟳→SVG 图标，accent 色（替代 unicode ●） |
| 描述 | 13px / 1.5，`--text-secondary`，`line-clamp: 2`；无描述时显示来源仓库的简短说明或留白（不留"无描述"字样） |
| 来源行 | 12px，`--text-muted`，格式 `owner/name`；可附分支（仅非 main 时显示） |
| 底栏 | 与内容区间距 12px；按钮右对齐 |
| 状态形态 | 未安装：`[安装]` primary；已安装：`[已安装]` success-outline 弱化 + `[同步]` secondary；可更新：卡片加 accent 左边框（复用 `.skill-has-update::before` 模式）+ `[更新]` primary |
| 悬停 | border-color 过渡 120ms + 保留现有 `translateY(-1px)`；不做整卡阴影变化 |

**工具栏与筛选**

- 搜索：单行输入框，flex 占满剩余宽度；实时过滤 skill_name + description（依赖 D1 修复），带 `aria-label`。
- 市场筛选：chips 行（全部 / 各市场 / + 添加）。市场 ≤6 个时 chips 直接铺开（当前内置 5 个，正好适用）；超过则收进 `[全部市场 ▾]` 下拉。`+` chip 直接打开源管理弹窗的添加表单。
- `[仅已安装]`：toggle chip，替代"安装管理"子 Tab 的职责（消化孤儿概念 remoteInstallsTab）。
- `[⋯]` 溢出菜单：全部扫描（重新索引全部市场）、全部标记已安装、全部取消标记、同步所有已安装。这些是低频批量操作，从主界面移入菜单（菜单项带图标与确认语义，destructive 项红色）。
- `[检查更新]`：secondary 按钮，带数字徽标（更新数）。点击后结果以弹窗呈现（见 4.5）。

**空 / 加载 / 错误状态**

| 状态 | 呈现 |
| --- | --- |
| 首次加载 | 骨架卡片 ×6（复用 `.skeleton-card`），替代当前文字"扫描中…" |
| 无任何市场 | 空状态插画区：文案"还没有市场源"+ `[添加市场]` 按钮；提示内置市场会在首次启动自动加载 |
| 有市场但技能为空 | "该市场还没有索引过技能" + `[同步索引]` 按钮 |
| 搜索无结果 | "没有匹配 \"xx\" 的技能"（保留现有 noMatch 文案结构） |
| 索引/网络失败 | toast 报错 + 受影响市场的 chip 上显示 ⚠ 图标；不静默吞错（呼应 check_market_commits 对不可达市场静默 skip 的已知问题） |

### 4.3 安装流程：一键安装

**现状**：`installRemoteSkill`（下载到 SSOT）与 `syncRemoteSkillToTools`（同步到工具）是两个暴露给用户的分离步骤，且目标工具选择器常驻。

**改造**：卡片上只有一个 `[安装]` 按钮 → 打开 InstallDialog：

```
┌──────────────────────────────────────────┐
│  安装 pdf-tools                     [×]  │
│  anthropics/skills · main                │
│ ─────────────────────────────────────── │
│  安装到    (●) 全局   ( ) 项目: [选择 ▾] │
│                                          │
│  目标工具  [ Claude Code            ▾ ]  │
│                                          │
│  ☑ 记住我的选择                          │
│ ─────────────────────────────────────── │
│                       [取消]   [安装]    │
└──────────────────────────────────────────┘
```

- 点 `[安装]` 后前端顺序调用现有两个 API：`downloadRemoteSkillToSsot` → `syncRemoteSkillToTools`，对用户是一个动作。按钮进入 loading（spinner + "安装中…"），成功后 toast："已安装 pdf-tools 到 Claude Code（全局）"，卡片原地切换为已安装形态。
- 默认值：上次选择（localStorage `market.install.projectId` / `market.install.toolPath`）；无记忆时 = 全局 + 第一个工具。
- "项目"单选激活时才出现项目下拉，避免空下拉常驻。
- 无障碍：`role="dialog" aria-modal="true"`、Esc 关闭、焦点圈定与归还（与 ui-review Finding #3 的 modal 统一修复一起做）。
- 已安装卡片上的 `[同步]`：直接调用 `syncRemoteSkillToTools`（使用记忆的目标），不再弹窗——这是高频维护动作，保持一步。

### 4.4 市场源管理（移入弹窗）

市场源是低频运维对象，从首屏移入 `[源管理…]` 弹窗：

```
┌───────────────────────────────────────────────────────────┐
│  市场源管理                                          [×]   │
│ ──────────────────────────────────────────────────────── │
│  [ 粘贴仓库链接: https://github.com/owner/repo    ] [添加]│
│    分支自动探测（main/master），也可在添加后修改          │
│ ──────────────────────────────────────────────────────── │
│  ⬡ Anthropic Skills              [内置]                  │
│    anthropics/skills · main · 32 个技能 · 2 小时前索引   │
│                                    [同步索引]  [停用]     │
│                                                          │
│  ⬡ My Team Skills                                        │
│    myteam/skills · dev · 8 个技能 · 昨天索引             │
│                          [同步索引]  [停用]  [⋯ 删除]     │
└───────────────────────────────────────────────────────────┘
```

- 内置市场显示友好 label（前端维护 `BUILTIN_MARKET_IDS → label` 映射，数据来自 market.rs 的模板常量），带 `[内置]` 徽标，不提供删除（文案提示见 i18n `builtinMarketNotice`）。
- 每行：名称（14px/600）+ 元信息行（owner/name · branch · 技能数 · 相对时间）+ 操作。技能数来自该市场的 remote_skills 计数。
- 时间显示改为**相对时间**（"2 小时前"），配 `title` 绝对时间；数字用 tabular-nums。
- 删除走确认（destructive，红色，二次确认弹窗），不再像现在那样筛选下拉旁直接放删除按钮。
- 添加表单收纳在弹窗顶部，替代当前首屏的折叠表单；保留"分支自动探测"提示文案（addMarketUrlHint）。

### 4.5 更新视图

现状的 hash 对照表改为**更新弹窗**（检查更新的结果呈现）：

```
┌───────────────────────────────────────────────────┐
│  可更新（3）                                 [×]   │
│ ──────────────────────────────────────────────── │
│  pdf-tools                    anthropics/skills   │
│  本地 8f3a21… → 远端 c91d02…                      │
│                                  [更新] [查看差异]│
│  ...                                              │
│ ──────────────────────────────────────────────── │
│                              [全部更新]  [关闭]   │
└───────────────────────────────────────────────────┘
```

- hash 缩写为 6 位 + mono 字体 + `--text-muted`，降级为辅助信息（保留可核对性，不再是主角）。
- `[更新]` = 下载远端版本到 SSOT 并同步到已激活工具（复用现有同步链路）。
- `[查看差异]`：P2 可选项，复用 DiffView 组件对比 SSOT 当前版本与远端新版本（本地 diff 能力已存在）。
- 无更新时：toast 提示"已是最新"（现有 remoteNoUpdate），不弹空窗。

### 4.6 文案规范（better-writing）

| 现状 | 改为 | 说明 |
| --- | --- | --- |
| 下载到 SSOT | 安装 | 用户不需要知道 SSOT |
| 同步到工具 | 同步 | 已安装卡片的维护动作 |
| 同步索引 | 同步索引（保留） | 源管理弹窗语境下可理解 |
| 全部扫描 | 重新索引全部市场 | 移入溢出菜单，语义明确 |
| `[builtin]` 后缀 | `[内置]` 徽标 | 视觉徽标替代文本后缀 |
| 表头 provider/owner/name/branch/enabled、old/new | 删除表格后不再需要 | 少量保留字段全部走 i18n |
| 未安装 / 已安装（文字列） | 卡片状态徽标 | 状态不再用文字列表达 |

所有新 key 补充 zh/en 双语；按钮统一句首大写（sentence case）策略（承接 ui-review #17）。

---

## 5. 视觉规范

### 5.1 设计 token 前置修正

市场改造依赖以下全局修正（全部来自 ui-review 2026-08-08 的既有结论，随 P0 一并落地）：

| 项 | 修正 |
| --- | --- |
| `--accent` | 亮色加深至对 `--text-on-accent: #fff` ≥ 4.5:1（约 `#a56a14` 一档），暗色相应校验 |
| `--text-muted` | 亮色提至 `#6f6f80`（≥4.5:1），暗色提至 `#7a7a8e` |
| `:focus-visible` | 全局焦点环：`outline: 2px solid var(--accent); outline-offset: 2px` |
| 字体 | Google Fonts 远程加载改本地打包（Tauri 资源）或系统字体栈，消除 FOUT 与离线失效 |
| `transition: all` | 全部改为显式属性：`transition-property: color, background-color, border-color, box-shadow, transform, opacity` |
| unicode 图标 | ⬡ ▸ ▾ ● ⟳ ✎ × ☰ ▦ 全部替换为内联 SVG 图标组件（`aria-hidden="true"`），单一图标风格、1.5px stroke、`currentColor` 着色 |

### 5.2 卡片与布局（better-ui / better-layout）

- 网格：复用 `.skill-grid`（`repeat(auto-fill, minmax(300px, 1fr))`，gap 14px），与本地 Tab 视觉一致。
- 同心圆角：卡片 `--radius-lg: 14px`，卡内按钮 `--radius-sm: 6px`（padding 18px 下视觉协调）；弹窗 `--radius-lg`，弹窗内输入框 `--radius-sm`。
- 分组靠间距：卡片内"描述区"与"底栏"间距 12px；工具栏与网格间距 16px；chips 行内 gap 8px、与上下区块间距 16px（组间距 ≥ 2× 组内间距）。
- 每屏单一 primary：浏览视图中唯一 primary 是卡片的 `[安装]/[更新]`；工具栏全部 secondary/ghost。
- 按钮间距：工具栏控件 gap 10px（现状保留）；卡片底栏按钮 gap 8px。

### 5.3 排版（better-typography）

统一五级字号，清除 10-11px 碎片：

| 角色 | 规格 |
| --- | --- |
| 卡片标题 | 14px / 600 / 1.3 |
| 正文描述 | 13px / 400 / 1.5，`text-wrap: pretty`（描述区 2 行内无需 balance） |
| 元信息 | 12px / 400，`--text-muted`；时间/计数 `font-variant-numeric: tabular-nums` |
| 弹窗标题 | 15px / 600 |
| 区块标题 | 13px / 600 / uppercase / letter-spacing 0.06em（沿用 `.section-title`） |

路径、hash 等技术值用 `--font-mono`（JetBrains Mono），其余一律 `--font-sans`。

### 5.4 动效（better-ui，克制原则）

- 卡片悬停：仅 border-color + background 过渡 120ms ease-out（保留 translateY(-1px)，不新增阴影动画）。
- 按钮按下：primary 按钮 `transform: scale(0.96)`，`transition-transform 100ms`。
- 弹窗：入场 opacity + translateY(4px) → 0，150ms ease-out；出场更轻（100ms）。首屏渲染不做入场动画。
- 状态切换图标（未安装→已安装的对勾）：opacity + scale(0.25→1) 交叉过渡 150ms，`cubic-bezier(0.2,0,0,1)`，无弹跳。
- 高频操作（搜索输入、chip 切换）不加动画；加载态用骨架屏而非 spinner 文本。
- 尊重 `prefers-reduced-motion`：以上动效全部归零。

---

## 6. 所需后端 / 数据改动

| # | 改动 | 位置 | 规模 |
| --- | --- | --- | --- |
| B1 | `sync_market_index` 索引时解析 SKILL.md frontmatter 写入 description（INSERT 与 UPDATE 两条路径都要）。复用 `scanner.rs::parse_front_matter`（需将其提升为 `pub` 或抽取到共享模块）；SKILL.md 已下载到 SSOT 目录，直接读本地文件解析即可 | `src-tauri/src/commands/market.rs`、`src-tauri/src/scanner.rs` | ~20 行 |
| B2 | 已安装技能计数：源管理弹窗需要每个市场的技能数。可用现有 `listRemoteSkills(marketId)` 前端统计，无需新 API | — | 0 |
| B3 | （P2，可选）远端新版本与本地 SSOT 的 diff：需要"读取远端最新版本内容"的接口。当前 diff.rs 能力基于本地路径，若远端内容已随索引下载至 SSOT 外的缓存则可复用 | 视 P2 决策 | 未定 |

**不需要**的改动：安装链路（downloadRemoteSkillToSsot + syncRemoteSkillToTools）、市场 CRUD、检查更新——全部复用现有 IPC。

**已知风险**：`check_market_commits` 对不可达市场静默 skip（2026-08-10 网络排查确认）。改造后检查更新的错误提示必须列出未到达的市场名，避免"假成功"。此项列入 P1。

---

## 7. 实施计划

**P0 · 卡片化与一键安装（核心体验，预计 3-4 天）**

1. B1 后端：索引解析 description
2. 全局 token 修正（对比度、focus-visible、字体本地化、transition 显式化）
3. 市场 Tab 重构：工具栏 + chips 筛选 + `MarketSkillCard` 网格（含三种状态形态）
4. `InstallDialog`（安装链路合并、记忆选择、modal 无障碍）
5. 搜索覆盖 description；骨架屏与空状态
6. i18n 新 key + 表头硬编码清理

**P1 · 管理层收纳（预计 2 天）**

7. `MarketSourcesModal`（内置 label 映射、相对时间、删除确认）
8. `RemoteUpdatesModal`（更新弹窗 + 单项/全部更新动作）+ 检查更新错误明示
9. 批量操作收进溢出菜单；删除 action-bar ×2
10. 孤儿 i18n key 清理（remoteInstallsTab / skillMarketTab 复用或移除）

**P2 · 增强（可选，按节奏）**

11. `[查看差异]`：远端更新 diff 预览（依赖 B3 评估）
12. 技能详情层：点击卡片展开 SKILL.md 预览（可复用 SkillEditorModal 的 markdown 渲染只读模式）
13. 列表视图切换（与本地 Tab 的 card/list 对齐）

---

## 8. 验收清单

- [ ] 市场 Tab 首屏无表格、无 SSOT/hash/branch 等字样；搜索框与技能卡片为视觉主体
- [ ] 每个技能卡片展示名称、描述（索引后可用）、来源、状态
- [ ] 未安装技能从点击 `[安装]` 到完成，用户操作 ≤2 步（点安装 → 弹窗确认），无需理解 SSOT
- [ ] 安装目标选择被记忆，二次安装零配置
- [ ] 内置市场显示友好名称与 `[内置]` 徽标，不可删除
- [ ] 检查更新结果含动作按钮；hash 降级为 6 位辅助信息；不可达市场被明确报错
- [ ] 亮/暗双主题下所有文本对比度 ≥ 4.5:1（正文）/ 3:1（大号文本）
- [ ] 键盘可完整走通：Tab 遍历 → 打开安装弹窗 → Esc 关闭 → 焦点归还
- [ ] zh/en 双语文案完整，无硬编码表头
- [ ] `pnpm tsc --noEmit` / lint / vitest 与 cargo check / clippy / test 全绿（CONTRIBUTING.md 校验集）

---

## 附录 A · 应用的评审原则（摘自 jakubkrehel/skills）

- better-layout：用空间分组而非分隔线；组间距 ≥2× 组内间距；控件与内容视觉可区分；主操作置于稳定位置不被裁切
- better-ui：同心圆角（外=内+padding）；阴影做层级、边框做结构；可中断的 CSS transition；按下 scale(0.96)；图标 stroke 与文字字重匹配；单一 currentColor SVG；高频交互不加动画
- better-colors：保留项目既有 hex token 表示法，只修数值；每个前景/背景对双主题复测；一色一义（accent 只用于主操作与更新强调）
- better-typography：语义化字号层级；正文 1.5 行高；tabular-nums 用于变化数值；截断保留可达全文（title/tooltip）
- better-writing：按钮单一动词；空状态给出下一步动作； destructive 操作显式化
- better-interface：修复写成项目自身惯用法（手写 CSS + 变量），不引入第二套样式体系

## 附录 B · 参考资料

- jakubkrehel/skills 仓库：https://github.com/jakubkrehel/skills （better-interface / better-ui / better-layout / better-colors / better-typography / better-writing）
- 本项目全量 UI 评审：`docs/ui-review-2026-08-08.md`
- 市场功能演进：56f212b（初版）→ 5a40977（筛选与 hash 更新检查）→ 1e61e51（批量安装）→ 6d470fb（v0.1.15 重构为当前形态）
