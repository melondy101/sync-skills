# UI 改进方案：竞品对标 + 逐屏优化

> **日期**：2026-08-23
> **状态**：提案（待评审）
> **范围**：全应用 UI 视觉与交互改进
> **前置依赖**：`docs/ui-review-2026-08-08.md`（18 条 finding）、`docs/market-ui-redesign.md`（市场改造）、`docs/spec-market-ux-polish.md`（Phase 4）
> **方法**：竞品截图分析 → 提取设计模式 → 映射到 Skill Manager 各屏 → 产出可执行改动清单

---

## 1. 现状评估

### 1.1 做得好的

- **设计 token 体系完整**：亮/暗双主题 CSS 变量已覆盖色彩、圆角、阴影、字体，具备良好的扩展基础
- **DM Sans + JetBrains Mono** 字体搭配符合开发者工具调性
- **Amber accent** 差异化明显，在一众蓝紫色开发者工具中有辨识度
- **市场 Tab 已完成卡片化改造**（market-ui-redesign.md），信息架构合理
- **骨架屏、toast 反馈** 等体验细节已有实现

### 1.2 核心问题

| 问题 | 影响 | 证据 |
|------|------|------|
| **界面"素"且"平"** | 卡片、按钮、分隔全靠 1px border，缺少层级感 | App.css 全局 `--shadow-sm: 0 1px 3px rgba(0,0,0,0.06)` 几乎不可见 |
| **色彩过于克制** | 除 accent 外几乎没有彩色信号，状态全靠文字传达 | 状态色 `--success/--error/--info` 只在 toast 使用，卡片上无彩色信号 |
| **间距密度不一致** | action-bar 拥挤（8+ 控件），卡片内间距偏小 | ui-review #7/#8 已指出 |
| **图标系统缺失** | unicode 图标（⬡▸▾●⟳✎×☰▦）跨平台渲染不一致 | ui-review #14，仍未修 |
| **启动页/引导页视觉弱** | OnboardingWizard 仅 modal + 文字列表，无品牌感 | `OnboardingWizard.tsx` 全文 160 行，无任何视觉元素 |
| **字体远程加载** | Google Fonts CDN 在离线/内网/Tauri 环境下可能失败 | ui-review #11，仍未修 |
| **字号碎片化** | 10px/11px/12px/13px/14px 混用无层级约束 | ui-review #12，仍未修 |

---

## 2. 竞品对标分析

### 2.1 Raycast Store — 最值得借鉴

**核心特征**：极致暗色 + 极简层级 + hairline borders + pill tabs + 渐变 icon tiles

| 设计要素 | Raycast 做法 | Skill Manager 可借鉴 |
|---------|-------------|---------------------|
| **表面层级** | 4 级表面阶梯（#07080a → #0d0d0d → #101111 → #121212），靠 2-3% 亮度差创造深度 | 暗色主题可增加 1-2 级中间表面，当前只有 3 级（bg → surface → card） |
| **边框** | hairline 1px `#242728`，不做阴影 | 当前暗色 `--border: #1f1f2e` 已接近，但卡片可加 1px top-edge highlight 做"浮起"感 |
| **Pill tabs** | `border-radius: 9999px` 的全圆角 tab，background 切换 | 当前 tab 是 `7px` 圆角的矩形段控件，可升级为 pill 形态 |
| **分类筛选** | pill chip 行，与 tab 同语言 | 当前市场筛选是 `<select>` 下拉，market-ui-redesign 已规划 chips 但尚未实现 |
| **Install 按钮** | 白色 primary 按钮（暗色下），极度克制 | 当前 `[安装]` 按钮已是 primary，保持即可 |
| **图标 tiles** | 每个 extension 有独立渐变色的 48px icon tile，创造色彩节奏 | Skill Manager 可用首字母 + 渐变背景做 skill avatar |
| **品牌标识** | 红色对角渐变 hero banner | 当前 OnboardingWizard 无任何品牌元素 |

**关键启发**：Raycast 的"营销页即产品"理念 → Skill Manager 的启动页/引导页应该展示产品价值，而不只是一个功能向导。

### 2.2 Linear — 暗色品质标杆

**核心特征**：零阴影 + 1px hairline borders + top-edge highlight + 单色系严格管控

| 设计要素 | Linear 做法 | Skill Manager 可借鉴 |
|---------|------------|---------------------|
| **深度表达** | 完全不用 `box-shadow`，靠 `1px top-edge white highlight at 5% opacity` 做表面浮起 | 暗色主题卡片加 `::before { border-top: 1px solid rgba(255,255,255,0.05) }` |
| **色彩纪律** | 单 accent（#5e6ad2 lavender-blue），状态色严格只在需要时出现 | Skill Manager 的 amber accent 应保持同样纪律：只用于 primary action + 更新强调 |
| **圆角层级** | 8px（按钮/输入）→ 12px（卡片）→ 16px（媒体面板），同心嵌套 | 当前 6/10/14 体系已接近，但需确保嵌套时外 = 内 + padding |
| **Heading** | negative letter-spacing (-0.5px) on headings, 600 weight | 当前 heading 无 letter-spacing 调整 |
| **动效** | 入场 translateY(4px) → 0 + opacity, 150ms ease-out | market-ui-redesign §5.4 已规划相同策略 |

**关键启发**：Linear 证明"少即是多"——深度不需要阴影，1px highlight 就够了。

### 2.3 VS Code Extensions — 安装状态机范本

**核心特征**：list + detail split view + 安装状态五态机 + verified publisher badge

| 设计要素 | VS Code 做法 | Skill Manager 可借鉴 |
|---------|-------------|---------------------|
| **状态机** | Install → Installing → Installed → Update available → Disabled，按钮形态随状态变化 | 当前卡片有"未安装/已安装/可更新"三态，但视觉差异不够明显 |
| **Verified badge** | 蓝色 ✓ 徽标标记已验证发布者 | 可为内置市场的 skill 加 `[官方]` 徽标 |
| **Star ratings + download counts** | 社交证据辅助决策 | 市场卡片可显示 GitHub stars（如果索引时拉取） |
| **Gear menu** | 安装后齿轮图标展开更多操作 | 已安装卡片可用 `⋯` 溢出菜单收纳 同步/卸载/编辑 |

### 2.4 Obsidian Community Plugins — 简洁列表模式

**核心特征**：单列列表 + toggle switch + sort tabs（Popular/New/Updated）

| 设计要素 | Obsidian 做法 | Skill Manager 可借鉴 |
|---------|-------------|---------------------|
| **Toggle switch** | 安装 → toggle 启用/禁用，两步都在列表行内 | 当前卡片有 toggle 但视觉权重不够 |
| **Sort tabs** | Popular / New / Updated 三个 tab | 市场可增加排序维度（名称/更新时间/来源） |
| **搜索** | 顶部大搜索框，实时过滤 | 已实现，保持 |
| **Description** | 一行描述紧跟标题 | 已规划在 market-ui-redesign |

### 2.5 Zed Extensions — 极简市场

**核心特征**：搜索 + category pill tabs + 卡片网格 + 下载计数

| 设计要素 | Zed 做法 | Skill Manager 可借鉴 |
|---------|---------|---------------------|
| **Category tabs** | All / Languages / Language Servers / MCP Servers / Themes / Snippets pill tabs | 市场筛选可改为 pill tabs（market-ui-redesign 已规划） |
| **下载计数** | 每张卡片显示 `6.3M` / `982k` 等社交证据 | 暂不适用（无下载量数据） |
| **Creator attribution** | 每张卡片底部显示作者名 | 市场卡片可增加 `owner/name` 来源行 |
| **深色模式** | `hsl(0,0%,7%)` base + `hsl(0,0%,93%)` text | 当前暗色 `#08080c` base + `#e2e2ea` text 已接近 |

### 2.6 Warp Terminal — 设置面板范本

**核心特征**：视觉主题网格 + live preview + 左侧分类导航

| 设计要素 | Warp 做法 | Skill Manager 可借鉴 |
|---------|----------|---------------------|
| **设置导航** | 左侧分类 sidebar（Appearance / Editor / Features…） | 当前 SettingsPanel 是单列表单，可改为 sidebar + content 双栏 |
| **主题选择** | 网格展示主题预览卡片，选中即切换 | 主题选择可改为三张预览卡片（Light/Dark/System） |
| **Toggle switches** | 自定义 toggle 替代 checkbox | 当前用原生 checkbox，可改为 styled toggle |

---

## 3. 逐屏优化方案

### 3.1 启动页 / Onboarding Wizard

**现状问题**：OnboardingWizard 是一个标准 modal + 文字列表，没有品牌感，没有视觉吸引力。

**参考**：Raycast 的全屏暗色 splash + 渐变 hero + 步骤引导

**改进方案**：

```
现状                                改进后
──────────────────                  ──────────────────
┌────────────────────┐              ┌──────────────────────────────┐
│ 首次启动引导    [×]│              │                              │
│ 欢迎使用…          │              │         ⬡ (64px icon)        │
│ ○ ○ ○              │              │       Skill Manager          │
│                    │              │   一处管理，多处生效            │
│ 1. Claude Code     │              │                              │
│    ~/.claude/skills│              │   ┌─────────┐ ┌─────────┐   │
│ 2. Codex CLI       │              │   │  Claude  │ │  Codex   │   │
│    ~/.codex/skills │              │   │  Code    │ │  CLI     │   │
│                    │              │   └─────────┘ └─────────┘   │
│    [全部添加]       │              │   ┌─────────┐ ┌─────────┐   │
│                    │              │   │ OpenCode │ │ +more   │   │
│                    │              │   └─────────┘ └─────────┘   │
│ [下一步] [跳过]    │              │                              │
└────────────────────┘              │     [发现并添加工具]          │
                                    │                              │
                                    │        ○  ○  ○               │
                                    └──────────────────────────────┘
```

**具体改动**：

| 改动 | 位置 | 说明 |
|------|------|------|
| 改为全屏 overlay + 大尺寸容器 | `OnboardingWizard.tsx` | modal-overlay 改为全屏深色背景（`--bg` + blur），容器宽度 600px |
| 品牌 hero 区 | 新增 CSS | 64px 品牌图标 + 15px 品牌名 + 13px tagline，居中排列 |
| 工具发现展示改为 icon grid | `wizard-tool-list` | 列表改为 2 列 grid，每项显示工具名 + 简化图标（首字母大写 + 渐变背景 tile） |
| 步骤指示器改为 dot 形态 | `wizard-steps` | 文字步骤 → 3 个 dot（active = accent 色，done = check icon） |
| 按钮居中 + 加大 | `modal-actions` | 按钮组居中，primary 按钮高度 40px，宽度 ≥ 160px |
| 跳过按钮降低权重 | `btn-secondary` → `btn-ghost` | "跳过"改为 ghost 按钮，不抢视觉焦点 |

### 3.2 主面板（Global / Projects Tab）

**现状问题**：卡片缺乏层级感，action-bar 控件密集，skill 卡片"扁平"。

**参考**：Linear 的 hairline + top-edge highlight、Raycast 的 pill tabs

**改进方案**：

#### 3.2.1 Tab 导航升级

```css
/* 现状: 矩形段控件 */
.tabs { border-radius: var(--radius-md); padding: 3px; }
.tab { border-radius: 7px; }

/* 改进: pill 形态，更现代的切换 */
.tabs {
  border-radius: 9999px;  /* 外容器全圆角 */
  padding: 2px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
}
.tab {
  border-radius: 9999px;  /* pill 内部也全圆角 */
  padding: 6px 20px;
}
```

#### 3.2.2 卡片层级提升

```css
/* 现状: 几乎无阴影 */
.skill-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
}

/* 改进: 暗色主题加 top-edge highlight (Linear 风格) */
[data-theme="dark"] .skill-card {
  border: 1px solid var(--border);
  position: relative;
}
[data-theme="dark"] .skill-card::before {
  content: '';
  position: absolute;
  top: 0; left: 0; right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.06), transparent);
  border-radius: var(--radius-lg) var(--radius-lg) 0 0;
}

/* 改进: 亮色主题加微妙阴影 */
.skill-card {
  box-shadow: 0 1px 2px rgba(0,0,0,0.04), 0 4px 12px rgba(0,0,0,0.03);
}
.skill-card:hover {
  box-shadow: 0 2px 4px rgba(0,0,0,0.06), 0 8px 24px rgba(0,0,0,0.06);
  border-color: var(--border-hover);
}
```

#### 3.2.3 Skill Avatar（首字母渐变 tile）

借鉴 Raycast 的 extension icon tile，为每个 skill 生成一个视觉锚点：

```css
.skill-avatar {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  font-weight: 600;
  color: #fff;
  flex-shrink: 0;
  /* 背景色基于 skill name hash 生成，确保同一 skill 颜色一致 */
  background: var(--avatar-bg, linear-gradient(135deg, #6366f1, #8b5cf6));
}
```

Avatar 颜色算法（JS）：取 skill name 的 charCode 之和 mod 8，映射到 8 种渐变预设。

```
卡片布局变化:
现状:                          改进后:
┌──────────────────────┐      ┌──────────────────────────────┐
│ pdf-tools            │      │ [PT] pdf-tools          ⟳   │
│ PDF 生成、拆分、合并 │      │      PDF 生成、拆分、合并页 │
│ 页面…                │      │      面…                    │
│                      │      │                             │
│ Claude Code ● Codex  │      │      Claude Code ● Codex    │
│ ──────────────────── │      │ ─────────────────────────── │
│            [同步]    │      │                   [同步]    │
└──────────────────────┘      └──────────────────────────────┘
```

#### 3.2.4 Action Bar 分组

```
现状（8+ 控件挤一行）:
[扫描] [更新] [同步全部] [健康检查] [🔍搜索] [排序▾] [▦/☰] [工具路径]

改进（分组 + 溢出收纳）:
┌─ 主操作 ──────────────┐  ┌─ 筛选 ──────────┐  ┌ 设置 ┐
│ [扫描] [更新ⁿ] [同步] │  │ [🔍搜索…] [排序▾]│  │ [⋯] │
└───────────────────────┘  └─────────────────┘  └─────┘

⋯ 溢出菜单:
  ┌──────────────────────┐
  │ ☐ 健康检查           │
  │ ⚙ 工具路径管理       │
  │ ─────────────────── │
  │ ▦ 卡片视图  ✓        │
  │ ☰ 列表视图           │
  └──────────────────────┘
```

#### 3.2.5 状态信号色彩化

借鉴 VS Code 的状态徽标系统，在卡片左侧添加彩色状态条：

```css
/* 有更新: accent 左边框 */
.skill-has-update {
  border-left: 3px solid var(--accent);
}

/* 冲突: error 左边框 */
.skill-has-conflict {
  border-left: 3px solid var(--error);
}

/* 已同步: 右上角绿色 dot */
.skill-synced-badge {
  width: 6px; height: 6px;
  border-radius: 50%;
  background: var(--success);
}
```

### 3.3 市场 Tab

**现状**：已完成卡片化改造（market-ui-redesign），但有改进空间。

**参考**：Raycast Store 的 pill filter chips + Zed 的 category tabs + VS Code 的 install 状态机

**改进方案**：

#### 3.3.1 筛选 pills 替代下拉

```
现状: [全部市场 ▾] (select dropdown)
改进: ( 全部 ) ( Anthropic Skills ) ( Superpowers ) ( + )  ← pill chips

CSS:
.market-chip {
  padding: 5px 14px;
  border-radius: 9999px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
}
.market-chip-active {
  background: var(--accent-glow);
  border-color: var(--accent);
  color: var(--accent);
}
```

#### 3.3.2 卡片增加来源归属行

```
┌──────────────────────────────┐
│ [CR] code-review         ⟳  │
│      基于证据的代码审查工作流│
│                              │
│      ⬡ anthropic-skills     │  ← 来源行：12px, muted, 带来源图标
│                              │
│ ─────────────────────────── │
│                    [安装]    │
└──────────────────────────────┘
```

#### 3.3.3 空状态设计

```
┌──────────────────────────────────────────┐
│                                          │
│              🔍 (48px icon)              │
│                                          │
│         还没有市场源                      │
│   添加 GitHub 仓库来发现和安装 Skill      │
│                                          │
│          [ 添加市场源 ]                   │
│                                          │
└──────────────────────────────────────────┘
```

### 3.4 设置面板

**现状问题**：单列长表单，无分组，视觉单调。

**参考**：Warp Terminal 的左侧分类导航 + 视觉主题选择器 + toggle switches

**改进方案**：

#### 3.4.1 双栏布局

```
现状:                              改进后:
┌──────────────────────────────┐   ┌───────────┬──────────────────────────┐
│ 外观                          │   │  外观     │                          │
│ 主题 [亮色▾]                  │   │  同步     │  主题                    │
│ 语言 [中文▾]                  │   │  代理     │  ┌────┐ ┌────┐ ┌────┐  │
│                               │   │  更新     │  │ ☀️ │ │ 🌙 │ │ 💻 │  │
│ 同步模式                      │   │           │  │亮色│ │暗色│ │系统│  │
│ ( ) 半自动 ( ) 全自动         │   │           │  └────┘ └────┘ └────┘  │
│                               │   │           │                          │
│ 代理                          │   │           │  语言                    │
│ [系统代理▾]                   │   │           │  (●) 中文  ( ) English  │
│ ...                           │   │           │                          │
└──────────────────────────────┘   └───────────┴──────────────────────────┘
```

```css
.settings-layout {
  display: grid;
  grid-template-columns: 180px 1fr;
  gap: 32px;
}

.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.settings-nav-item {
  padding: 8px 14px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 13px;
}
.settings-nav-item-active {
  background: var(--bg-elevated);
  color: var(--text);
}
```

#### 3.4.2 主题选择改为视觉卡片

```css
.theme-card {
  width: 100px;
  padding: 12px;
  border: 2px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  text-align: center;
}
.theme-card-active {
  border-color: var(--accent);
  background: var(--accent-glow);
}
.theme-preview {
  width: 100%;
  height: 48px;
  border-radius: var(--radius-sm);
  margin-bottom: 8px;
}
```

#### 3.4.3 Toggle switch 替代 checkbox

```css
.toggle-switch {
  position: relative;
  width: 40px;
  height: 22px;
  border-radius: 9999px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  cursor: pointer;
  transition: background 0.2s ease;
}
.toggle-switch::after {
  content: '';
  position: absolute;
  top: 2px; left: 2px;
  width: 16px; height: 16px;
  border-radius: 50%;
  background: var(--text-muted);
  transition: transform 0.2s ease, background 0.2s ease;
}
.toggle-switch[aria-checked="true"] {
  background: var(--accent);
  border-color: var(--accent);
}
.toggle-switch[aria-checked="true"]::after {
  transform: translateX(18px);
  background: #fff;
}
```

### 3.5 活动日志面板

**现状问题**：纯表格，缺乏时间感和层级。

**参考**：GitHub Activity Timeline、Linear 的 timeline 视图

**改进方案**：

```
现状:                              改进后:
┌──────────────────────────────┐   ┌──────────────────────────────────────┐
│ 时间 │ 动作 │ 技能 │ 状态    │   │ ● 14:32                              │
│──────┼──────┼──────┼────────│   │   ✓ 同步 pdf-tools → Claude Code    │
│ 14:32│ sync │ pdf  │ success│   │   ✓ 同步 pdf-tools → Codex CLI      │
│ 14:31│ sync │ git  │ success│   │                                      │
│ 14:30│ scan │ -    │ success│   │ ● 14:31                              │
│      │      │      │        │   │   ✓ 同步 git-workflow → Claude Code │
│      │      │      │        │   │                                      │
│                              │   │ ● 14:30                              │
│                              │   │   ℹ 扫描完成，发现 12 个 Skill       │
└──────────────────────────────┘   └──────────────────────────────────────┘
```

```css
.log-timeline {
  position: relative;
  padding-left: 28px;
}
.log-timeline::before {
  content: '';
  position: absolute;
  left: 8px;
  top: 0; bottom: 0;
  width: 1px;
  background: var(--border);
}
.log-entry {
  position: relative;
  padding: 8px 0;
}
.log-entry::before {
  content: '';
  position: absolute;
  left: -24px;
  top: 14px;
  width: 8px; height: 8px;
  border-radius: 50%;
  background: var(--info);
}
.log-entry-success::before { background: var(--success); }
.log-entry-error::before { background: var(--error); }
```

---

## 4. 全局 Token 改进

### 4.1 新增 Token

```css
:root {
  /* 新增: 表面层级细分 */
  --surface-1: var(--surface);
  --surface-2: var(--bg-elevated);
  --surface-3: #e8e7e3;  /* 亮色第三级 */

  /* 新增: 顶部高光线 (暗色主题专用) */
  --edge-highlight: transparent;

  /* 新增: avatar 渐变预设 */
  --avatar-0: linear-gradient(135deg, #6366f1, #8b5cf6);
  --avatar-1: linear-gradient(135deg, #ec4899, #f43f5e);
  --avatar-2: linear-gradient(135deg, #f59e0b, #ef4444);
  --avatar-3: linear-gradient(135deg, #10b981, #06b6d4);
  --avatar-4: linear-gradient(135deg, #3b82f6, #6366f1);
  --avatar-5: linear-gradient(135deg, #8b5cf6, #ec4899);
  --avatar-6: linear-gradient(135deg, #14b8a6, #22d3ee);
  --avatar-7: linear-gradient(135deg, #f97316, #facc15);
}

[data-theme="dark"] {
  --surface-3: #1a1a26;
  --edge-highlight: rgba(255, 255, 255, 0.05);
}
```

### 4.2 字号 Scale 规范化

```
现状:  10px / 11px / 12px / 13px / 14px / 15px (无层级约束)
改进:  严格五级制

Role           Size    Weight    Tracking    Usage
────────────────────────────────────────────────────────
Caption        11px    400       +0.02em     mono-only (timestamp, hash)
Meta           12px    400       +0.01em     元信息、来源、相对时间
Body           13px    400       0           正文描述、设置选项
Title          14px    600       -0.01em     卡片标题、区块标题
Heading        16px    600       -0.02em     面板标题、modal 标题
```

清除所有 10px 用法（当前 `sync-time` 10px → 11px mono）。

### 4.3 Unicode → SVG 图标映射

| Unicode | 语义 | SVG 替代 | 来源建议 |
|---------|------|---------|---------|
| ⬡ | 市场/仓库 | `<Hexagon>` | Lucide |
| ● | 状态 dot | `<Circle>` 或纯 CSS `::before` | — |
| ✎ | 编辑 | `<Pencil>` | Lucide |
| × | 关闭 | `<X>` | Lucide |
| ⟳ | 更新/刷新 | `<RefreshCw>` | Lucide |
| ▸ | 展开 | `<ChevronRight>` | Lucide |
| ▾ | 折叠/下拉 | `<ChevronDown>` | Lucide |
| ▦ | 卡片视图 | `<Grid3x3>` | Lucide |
| ☰ | 列表视图 | `<List>` | Lucide |

建议用内联 SVG 组件 `<Icon name="refresh-cw" size={14} />`，不引入外部图标库依赖（保持零依赖原则）。

---

## 5. 动效规范（补充）

当前 market-ui-redesign §5.4 已规划基础动效。补充以下从竞品提取的模式：

### 5.1 Page Transition

```css
/* Tab 切换时内容区 fade 过渡 */
.tab-content {
  animation: fadeSlideIn 200ms ease-out;
}
@keyframes fadeSlideIn {
  from { opacity: 0; transform: translateY(4px); }
  to   { opacity: 1; transform: translateY(0); }
}
```

### 5.2 Skill Avatar 入场

```css
/* 卡片首次渲染时 stagger 入场（仅首屏，后续不重复） */
.skill-card {
  animation: cardIn 250ms ease-out backwards;
}
.skill-card:nth-child(1) { animation-delay: 0ms; }
.skill-card:nth-child(2) { animation-delay: 30ms; }
.skill-card:nth-child(3) { animation-delay: 60ms; }
/* ... up to 8th */
.skill-card:nth-child(n+9) { animation-delay: 240ms; }

@keyframes cardIn {
  from { opacity: 0; transform: translateY(8px); }
  to   { opacity: 1; transform: translateY(0); }
}
```

### 5.3 Toggle Switch 弹簧

```css
.toggle-switch::after {
  transition: transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
  /* 轻微过冲弹簧感，但不过分 */
}
```

### 5.4 prefers-reduced-motion

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## 6. 实施优先级

### P0 · 立即可见（1-2 天）

| # | 改动 | 影响面 | 工作量 |
|---|------|--------|--------|
| 1 | Tab 改为 pill 形态 | 全局导航 | ~15 行 CSS |
| 2 | 暗色卡片加 top-edge highlight | 卡片层级感 | ~20 行 CSS |
| 3 | 亮色卡片加微妙阴影 | 卡片层级感 | ~10 行 CSS |
| 4 | 字号 scale 规范化 | 全局可读性 | ~50 行 CSS 修改 |
| 5 | 清除 10px 字号 | 可读性底线 | ~10 行 CSS |

### P1 · 核心体验（3-5 天）

| # | 改动 | 影响面 | 工作量 |
|---|------|--------|--------|
| 6 | Skill Avatar 首字母 tile | 卡片视觉锚点 | ~40 行 CSS + ~30 行 TSX |
| 7 | Action-bar 分组 + 溢出菜单 | 操作区整洁度 | ~100 行 TSX |
| 8 | 状态色彩信号（左边框 + dot） | 状态可读性 | ~30 行 CSS |
| 9 | Onboarding 全屏重构 | 第一印象 | ~200 行 TSX + ~100 行 CSS |
| 10 | 市场 pill filter chips | 市场浏览体验 | ~80 行 CSS + ~60 行 TSX |

### P2 · 体验升级（5-7 天）

| # | 改动 | 影响面 | 工作量 |
|---|------|--------|--------|
| 11 | 设置面板双栏 + 视觉主题选择 | 设置体验 | ~200 行 TSX + ~150 行 CSS |
| 12 | Toggle switch 替代 checkbox | 全局一致性 | ~60 行 CSS + ~40 行 TSX |
| 13 | 日志面板 timeline 化 | 日志可读性 | ~100 行 CSS + ~80 行 TSX |
| 14 | Unicode → SVG 图标替换 | 跨平台一致性 | ~200 行（图标组件 + 各组件替换） |
| 15 | 字体本地打包 | 离线可用性 | 构建配置 |
| 16 | 入场动效 + stagger | 品质感 | ~60 行 CSS |

### 不做（明确排除）

| 候选 | 排除原因 |
|------|---------|
| 引入 Tailwind/组件库 | 项目已有完整 CSS token 体系，迁移成本 > 收益 |
| OKLCH 色彩空间 | 现有 hex/rgba 体系运行良好 |
| 全屏 hero splash 页 | Tauri 桌面应用不适合网页式 splash |
| 3D/渐变复杂动效 | 工具类应用应保持克制 |

---

## 7. 竞品参考链接

| 产品 | 关键参考点 | 链接 |
|------|----------|------|
| Raycast | Store 卡片、pill tabs、dark mode surface ladder | https://www.raycast.com/store |
| Linear | Dark mode quality、hairline borders、top-edge highlight | https://linear.app |
| VS Code | Extension state machine、verified badge | https://marketplace.visualstudio.com |
| Obsidian | Plugin browser、toggle switch | https://obsidian.md/plugins |
| Zed | Extensions marketplace、category tabs | https://zed.dev/extensions |
| Warp | Settings panel、visual theme picker | https://www.warp.dev |
| Figma Community | Card grid、social proof | https://www.figma.com/community |

**设计系统分析文档**：
- [Raycast DESIGN.md](https://github.com/VoltAgent/awesome-design-md/blob/main/design-md/raycast/DESIGN.md) — 完整色彩/排版/间距/组件规范
- [Linear Design System](https://open-design.ai/zh/plugins/design-system-linear-app/) — 暗色模式设计哲学
- [VoltAgent Awesome Design MD](https://github.com/VoltAgent/awesome-design-md/) — 多产品设计系统合集

---

## 附录 A · 色彩对比度复测建议

每次修改 `--accent` / `--text-muted` / `--text-on-accent` 后，须运行对比度校验：

```python
# WCAG 2.1 AA: 正文 ≥ 4.5:1, 大号文本 ≥ 3:1
def contrast_ratio(fg, bg):
    """fg/bg = (r, g, b) in 0-255"""
    def luminance(r, g, b):
        rs, gs, bs = r/255, g/255, b/255
        rs = rs/12.92 if rs <= 0.03928 else ((rs+0.055)/1.055)**2.4
        gs = gs/12.92 if gs <= 0.03928 else ((gs+0.055)/1.055)**2.4
        bs = bs/12.92 if bs <= 0.03928 else ((bs+0.055)/1.055)**2.4
        return 0.2126*rs + 0.7152*gs + 0.0722*bs
    l1, l2 = luminance(*fg), luminance(*bg)
    lighter, darker = max(l1,l2), min(l1,l2)
    return (lighter + 0.05) / (darker + 0.05)
```

当前已验证通过的组合：
- `--accent: #a56a14` on `#ffffff` (white text) = 4.58:1 ✓
- `--text-muted: #6f6f80` on `#fafaf8` = 4.52:1 ✓
- 暗色 `--text-muted: #7a7a8e` on `#08080c` = 4.63:1 ✓
