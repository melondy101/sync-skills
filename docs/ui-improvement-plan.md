# UI 改进执行计划

> **日期**：2026-08-23
> **基于**：`docs/ui-improvement-proposals.md`
> **已预创建的文件**（直接可用）：
> - `src/components/Icon.tsx` — 23 种 SVG 图标组件
> - `src/components/SkillAvatar.tsx` — 首字母渐变 avatar
> - `src/components/ToggleSwitch.tsx` — 自定义 toggle 开关
> - `src/ui-enhancements.css` — 新增 CSS token + 样式

---

## 提交前校验命令（每完成一个任务后跑一遍）

```bash
pnpm tsc --noEmit
pnpm lint
```

---

## 任务 1：引入增强 CSS（5 分钟）

### 改动 1.1：App.tsx 第 9 行之后插入 CSS import

**文件**：`src/App.tsx`
**位置**：第 9 行 `import "./App.css";` 之后

```tsx
// 第 9 行之后插入:
import "./ui-enhancements.css";
```

### 验证

```bash
pnpm tauri dev
# 确认：Tab 变为 pill 形态，卡片有顶部高光线，亮色卡片有微妙阴影
```

---

## 任务 2：替换所有 unicode 图标为 SVG（30 分钟）

### 改动 2.1：App.tsx — 品牌图标 ⬡（第 443 行）

**文件**：`src/App.tsx`
**第 443 行**，替换：

```tsx
// Before:
<span className="brand-icon">⬡</span>

// After:
<span className="brand-icon"><Icon name="hexagon" size={22} /></span>
```

**顶部加 import**（第 32 行之后）：

```tsx
import { Icon } from "./components/Icon";
```

### 改动 2.2：App.tsx — 视图切换 ▦/☰（第 551 行）

**第 551 行**，替换：

```tsx
// Before:
{viewMode === "card" ? "☰" : "▦"}

// After:
{viewMode === "card" ? <Icon name="list" size={16} /> : <Icon name="grid-3x3" size={16} />}
```

### 改动 2.3：App.tsx — 更新指示器 ●（第 636 行）

**第 636 行**，替换：

```tsx
// Before:
{hasUpdate(skill) && <span className="update-indicator" title={t("hasUpdate")}>●</span>}

// After:
{hasUpdate(skill) && <Icon name="circle" size={8} className="update-indicator" />}
```

### 改动 2.4：App.tsx — 同步旋转器 ⟳（第 637 行）

**第 637 行**，替换：

```tsx
// Before:
{syncing.has(skill.id) && <span className="sync-spinner">⟳</span>}

// After:
{syncing.has(skill.id) && <Icon name="refresh-cw" size={14} className="sync-spinner" />}
```

### 改动 2.5：ProjectNav.tsx — 编辑 ✎ 和删除 ×（第 112、119 行）

**文件**：`src/components/ProjectNav.tsx`
**顶部加 import**：

```tsx
import { Icon } from "./Icon";
```

**第 112 行**：

```tsx
// Before:
<button className="project-edit" onClick={() => handleEditProject(p)} aria-label={t("editProject")}>✎</button>

// After:
<button className="project-edit" onClick={() => handleEditProject(p)} aria-label={t("editProject")}><Icon name="pencil" size={14} /></button>
```

**第 119 行**：

```tsx
// Before:
<button className="project-delete" onClick={() => handleDeleteProject(p)} aria-label={t("deleteProject")}>×</button>

// After:
<button className="project-delete" onClick={() => handleDeleteProject(p)} aria-label={t("deleteProject")}><Icon name="x" size={14} /></button>
```

### 改动 2.6：ToolsSection.tsx — 删除 ×（第 250 行）

**文件**：`src/components/ToolsSection.tsx`
**顶部加 import**：

```tsx
import { Icon } from "./Icon";
```

**第 250 行**：

```tsx
// Before:
<button className="btn btn-small btn-danger" onClick={() => handleDeleteTool(tool)}>×</button>

// After:
<button className="btn btn-small btn-danger" onClick={() => handleDeleteTool(tool)} aria-label={t("deleteTool")}><Icon name="x" size={12} /></button>
```

### 改动 2.7：ConflictSection.tsx — 警告 ⚠（第 67 行）

**文件**：`src/components/ConflictSection.tsx`
**顶部加 import**：

```tsx
import { Icon } from "./Icon";
```

**第 67 行**：

```tsx
// Before:
<span className="conflict-icon">⚠</span>

// After:
<Icon name="alert-triangle" size={16} className="conflict-icon" />
```

### 改动 2.8：SkillListRow.tsx — ● 和 ⟳（第 45-46 行）

**文件**：`src/components/SkillListRow.tsx`
**顶部加 import**：

```tsx
import { Icon } from "./Icon";
```

**第 45-46 行**：

```tsx
// Before:
{hasUpdate && <span className="update-indicator" title={t("hasUpdate")}>●</span>}
{syncing && <span className="sync-spinner">⟳</span>}

// After:
{hasUpdate && <Icon name="circle" size={8} className="update-indicator" />}
{syncing && <Icon name="refresh-cw" size={14} className="sync-spinner" />}
```

### 改动 2.9：UpdatesModal.tsx — ▸/▾（第 259 行）

**文件**：`src/components/UpdatesModal.tsx`
**顶部加 import**：

```tsx
import { Icon } from "./Icon";
```

**第 259 行**：

```tsx
// Before:
<span className="tool-diff-caret">{open ? "▾" : "▸"}</span>

// After:
<Icon name={open ? "chevron-down" : "chevron-right"} size={14} className="tool-diff-caret" />
```

### 改动 2.10：三个 Modal 的关闭按钮 ✕

**MarketSourcesModal.tsx 第 90 行**：

```tsx
// Before:
<button className="btn btn-small btn-ghost" onClick={onClose} aria-label="Close">✕</button>

// After:
<button className="btn btn-small btn-ghost" onClick={onClose} aria-label="Close"><Icon name="x" size={14} /></button>
```

**RemoteSkillDetailModal.tsx 第 119 行**：同上模式，替换 `✕` → `<Icon name="x" size={14} />`

**RemoteUpdatesModal.tsx 第 23 行**：同上模式

每个文件顶部加 `import { Icon } from "./Icon";`

### 改动 2.11：✓ 替换（LintModal、SettingsPanel）

**LintModal.tsx 第 87 行**：

```tsx
// Before:
<div className="lint-status lint-ok">✓ {t("lintAllGood")}</div>

// After:
<div className="lint-status lint-ok"><Icon name="check" size={14} /> {t("lintAllGood")}</div>
```

**SettingsPanel.tsx 第 259 行**：

```tsx
// Before:
<p className="settings-hint update-latest">✓ {t("upToDate")} ...</p>

// After:
<p className="settings-hint update-latest"><Icon name="check" size={14} /> {t("upToDate")} ...</p>
```

**SettingsPanel.tsx 第 292 行**：

```tsx
// Before:
<span className="settings-hint update-latest">✓ {t("downloadComplete")}</span>

// After:
<span className="settings-hint update-latest"><Icon name="check" size={14} /> {t("downloadComplete")}</span>
```

每个文件顶部加 `import { Icon } from "./Icon";`

### 改动 2.12：SkillEditorModal.tsx — ●（第 81 行）

```tsx
// Before:
{dirty && <span className="editor-dirty-dot" title={t("unsavedChanges")}>●</span>}

// After:
{dirty && <Icon name="circle" size={8} className="editor-dirty-dot" />}
```

顶部加 `import { Icon } from "./Icon";`

### 改动 2.13：LogsPanel.tsx — →/←（第 84 行）

**文件**：`src/components/LogsPanel.tsx`
**顶部加 import**：

```tsx
import { Icon } from "./Icon";
```

**第 84 行**：

```tsx
// Before:
{log.direction === "to_ssot" ? "→ SSOT" : "← SSOT"}

// After:
{log.direction === "to_ssot"
  ? <><Icon name="arrow-right" size={12} /> SSOT</>
  : <><Icon name="arrow-left" size={12} /> SSOT</>
}
```

> **注意**：→/← 在 RemoteSkillDetailModal.tsx:196、RemoteUpdatesModal.tsx:45、SkillMarketPanel.tsx:222/497 中也有出现，但这些是**文本上下文中的箭头**（hash 比较、toast 消息），可保留 unicode 或按需替换。优先级低，本任务不处理。

### 验证

```bash
pnpm tsc --noEmit
pnpm lint
pnpm tauri dev
# 检查：所有界面中 unicode 图标已变为 SVG，跨 Windows/macOS 渲染一致
```

---

## 任务 3：卡片加 SkillAvatar（15 分钟）

### 改动 3.1：App.tsx 卡片渲染区（第 631 行附近）

**文件**：`src/App.tsx`
**顶部加 import**：

```tsx
import { SkillAvatar } from "./components/SkillAvatar";
```

在 `.skill-card` 内部，`.skill-header` 之前插入 avatar。找到卡片渲染（第 631 行 `<div className="skill-grid">` 之后的 `.skill-card` map），在 `<div className="skill-header">` 前面加：

```tsx
{/* 在 .skill-card 内部最前面加 */}
<div style={{ display: "flex", gap: 12, alignItems: "flex-start", marginBottom: 8 }}>
  <SkillAvatar name={skill.name} size={36} />
  <div style={{ flex: 1, minWidth: 0 }}>
    <div className="skill-name" style={{ display: "flex", alignItems: "center", gap: 6 }}>
      {skill.name}
      {hasUpdate(skill) && <Icon name="circle" size={8} className="update-indicator" />}
      {syncing.has(skill.id) && <Icon name="refresh-cw" size={14} className="sync-spinner" />}
    </div>
    {skill.description && (
      <div className="skill-desc">{skill.description}</div>
    )}
  </div>
</div>
```

> 这会替代现有的 `.skill-header` + `.skill-name` + `.skill-desc` 区块。需要删掉原来的 `<div className="skill-header">...</div>` 和紧跟的 `{skill.description && ...}` 块。

### 验证

```bash
pnpm tsc --noEmit
pnpm tauri dev
# 检查：每张卡片左上角有渐变色首字母 tile
```

---

## 任务 4：Action Bar 分组（20 分钟）

### 改动 4.1：App.tsx action-bar 区域（第 507 行附近）

**文件**：`src/App.tsx`
**目标**：将 8+ 控件分为三组

当前结构（约第 507-555 行）：

```tsx
<div className="action-bar">
  {/* 8+ 按钮和控件混在一起 */}
</div>
```

改为：

```tsx
<div className="action-bar">
  {/* 组 1: 主操作 */}
  <div className="action-group">
    <button ... >扫描</button>
    <button ... >更新 {updateCount > 0 && <span className="badge">{updateCount}</span>}</button>
    <button ... >同步全部</button>
  </div>

  {/* 组 2: 筛选 — 搜索框占满剩余空间 */}
  <div className="search-box">
    <input className="search-input" ... />
  </div>

  {/* 组 3: 视图切换 + 溢出菜单 */}
  <div className="action-group">
    <button ... title={t("sortLabel")}>{sortLabel} <Icon name="chevron-down" size={12} /></button>
    <button ... >{viewMode === "card" ? <Icon name="list" size={16} /> : <Icon name="grid-3x3" size={16} />}</button>
    <details className="overflow-menu">
      <summary className="btn btn-ghost btn-small"><Icon name="more-horizontal" size={16} /></summary>
      <div className="overflow-menu-items">
        <button onClick={handleLint}>健康检查</button>
        <button onClick={() => setPanel("settings")}>工具路径管理</button>
      </div>
    </details>
  </div>
</div>
```

### 改动 4.2：App.css 加 action-group 样式

**文件**：`src/App.css`（或 `src/ui-enhancements.css`）

```css
.action-group {
  display: flex;
  gap: 4px;
  align-items: center;
  flex-shrink: 0;
}

.overflow-menu {
  position: relative;
}

.overflow-menu summary {
  list-style: none;
  cursor: pointer;
}

.overflow-menu summary::-webkit-details-marker {
  display: none;
}

.overflow-menu-items {
  position: absolute;
  right: 0;
  top: 100%;
  margin-top: 4px;
  min-width: 180px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  padding: 4px;
  z-index: 100;
}

.overflow-menu-items button {
  display: block;
  width: 100%;
  text-align: left;
  padding: 8px 12px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-family: var(--font-sans);
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.overflow-menu-items button:hover {
  background: var(--surface-hover);
  color: var(--text);
}
```

### 验证

```bash
pnpm tsc --noEmit
pnpm tauri dev
# 检查：action bar 分三组，溢出菜单可展开
```

---

## 任务 5：状态色彩信号（10 分钟）

### 改动 5.1：App.tsx 卡片 className 增强

找到卡片 `<div className="skill-card ...">` 处，加入条件 className：

```tsx
<div className={`skill-card${hasUpdate(skill) ? " skill-has-update" : ""}${hasConflict(skill) ? " skill-has-conflict" : ""}`}>
```

在 tool toggles 区域附近，已同步的 skill 加 dot：

```tsx
{allSynced && <span className="synced-dot" title={t("allSynced")} />}
```

### 验证

```bash
pnpm tauri dev
# 检查：有更新的卡片左边有 amber 条，有冲突的有红色条
```

---

## 任务 6：Onboarding 全屏改造（30 分钟）

### 改动 6.1：重写 OnboardingWizard.tsx

**文件**：`src/components/OnboardingWizard.tsx`

关键改动：

1. 容器从 `.modal` 改为 `.modal` + `.wizard-splash`（全屏深色背景）
2. 加品牌 hero 区（Icon hexagon 64px + 品牌名 + tagline）
3. 工具发现列表改为 2 列 grid（每项有首字母 tile）
4. 步骤指示器从文字改为 dot
5. 按钮居中，primary 加大，skip 改 ghost

```tsx
// 主要结构变化:
<div className="modal-overlay wizard-fullscreen">
  <div className="modal wizard-splash" onClick={e => e.stopPropagation()}>
    {/* Hero */}
    <div className="wizard-hero">
      <Icon name="hexagon" size={56} className="wizard-hero-icon" />
      <h2>Skill Manager</h2>
      <p>{t("wizardTagline")}</p>
    </div>

    {/* Dot steps */}
    <div className="wizard-dots">
      {steps.map((_, i) => (
        <span key={i} className={`wizard-dot${i === step ? " wizard-dot-active" : ""}${i < step ? " wizard-dot-done" : ""}`} />
      ))}
    </div>

    {/* Content per step */}
    {step === 0 && (/* 工具 grid */)}
    {step === 1 && (/* 扫描 */)}
    {step === 2 && (/* 完成 */)}

    {/* Actions */}
    <div className="wizard-actions">
      <button className="btn btn-primary btn-large">...</button>
      <button className="btn btn-ghost">跳过</button>
    </div>
  </div>
</div>
```

### 改动 6.2：新增 wizard CSS

追加到 `src/ui-enhancements.css`：

```css
.wizard-fullscreen {
  background: rgba(8, 8, 12, 0.85);
  backdrop-filter: blur(16px);
}

.wizard-splash {
  width: 560px;
  max-width: 90vw;
  padding: 40px 32px;
  text-align: center;
}

.wizard-hero {
  margin-bottom: 24px;
}

.wizard-hero-icon {
  color: var(--accent);
  filter: drop-shadow(0 0 24px var(--accent-glow-strong));
  margin-bottom: 12px;
}

.wizard-hero h2 {
  font-size: 20px;
  font-weight: 600;
  letter-spacing: -0.02em;
  margin-bottom: 4px;
}

.wizard-hero p {
  font-size: 13px;
  color: var(--text-muted);
}

.wizard-dots {
  display: flex;
  justify-content: center;
  gap: 8px;
  margin-bottom: 24px;
}

.wizard-dot {
  width: 8px; height: 8px;
  border-radius: 50%;
  background: var(--border);
  transition: background 0.2s ease;
}

.wizard-dot-active {
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent-glow);
}

.wizard-dot-done {
  background: var(--success);
}

.wizard-tool-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  margin-bottom: 20px;
}

.wizard-tool-tile {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 14px 12px;
  text-align: center;
}

.wizard-actions {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.btn-large {
  padding: 10px 28px;
  font-size: 14px;
  min-width: 160px;
}
```

### i18n 新增 key

**文件**：`src/i18n.ts`

```
zh:
  wizardTagline: "一处管理，多处生效",
  wizardDiscover: "发现并添加工具",

en:
  wizardTagline: "One place to manage, everywhere it matters",
  wizardDiscover: "Discover & Add Tools",
```

### 验证

```bash
# 删除 onboardingDone 标记来重看引导：
# Windows: 在浏览器 devtools 的 localStorage 中删 "onboardingDone"
pnpm tauri dev
# 检查：全屏 splash，品牌 hero，工具 grid，dot 步骤
```

---

## 任务 7：Settings 面板双栏改造（30 分钟）

### 改动 7.1：SettingsPanel.tsx 改为 sidebar + content 布局

**文件**：`src/components/SettingsPanel.tsx`

关键改动：

1. 引入 `useState` 管理当前 section（"appearance" | "sync" | "proxy" | "update"）
2. 外层改为 `.settings-layout`（grid 160px 1fr）
3. 左侧 `.settings-nav` 分类按钮
4. 右侧 `.settings-content` 根据 section 渲染
5. 主题选择改为 `.theme-cards`（三张预览卡片）
6. checkbox 改为 `<ToggleSwitch>`

```tsx
import { ToggleSwitch } from "./ToggleSwitch";

const [section, setSection] = useState<"appearance" | "sync" | "proxy" | "update">("appearance");

return (
  <div className="settings-layout">
    <nav className="settings-nav">
      {(["appearance", "sync", "proxy", "update"] as const).map(s => (
        <button
          key={s}
          className={`settings-nav-item${section === s ? " settings-nav-item-active" : ""}`}
          onClick={() => setSection(s)}
        >
          {t(s)}
        </button>
      ))}
    </nav>
    <div className="settings-content">
      {section === "appearance" && (/* 主题卡片 + 语言 pills */)}
      {section === "sync" && (/* ToggleSwitch 替代 checkbox */)}
      {section === "proxy" && (/* 代理设置 */)}
      {section === "update" && (/* 更新检查 */)}
    </div>
  </div>
);
```

### 改动 7.2：主题选择卡片

```tsx
<div className="theme-cards">
  {(["light", "dark", "system"] as const).map(theme => (
    <button
      key={theme}
      className={`theme-card${settings.theme === theme ? " theme-card-active" : ""}`}
      onClick={() => handleChange({ ...settings, theme })}
    >
      <div className={`theme-preview theme-preview-${theme}`} />
      <div className="theme-card-label">{t(`theme${theme.charAt(0).toUpperCase() + theme.slice(1)}`)}</div>
    </button>
  ))}
</div>
```

### 验证

```bash
pnpm tsc --noEmit
pnpm tauri dev
# 检查：左侧分类导航，主题三张卡片，toggle 替代 checkbox
```

---

## 任务 8：市场 pill filter chips（20 分钟）

### 改动 8.1：SkillMarketPanel.tsx 筛选改为 chips

**文件**：`src/components/SkillMarketPanel.tsx`

找到现有的市场筛选 `<select>` 或下拉控件，替换为：

```tsx
<div className="market-chips">
  <button
    className={`market-chip${marketFilter === null ? " market-chip-active" : ""}`}
    onClick={() => setMarketFilter(null)}
  >
    {t("all")}
  </button>
  {markets.filter(m => m.enabled).map(m => (
    <button
      key={m.id}
      className={`market-chip${marketFilter === m.id ? " market-chip-active" : ""}`}
      onClick={() => setMarketFilter(m.id)}
    >
      {m.owner}/{m.name}
    </button>
  ))}
  <button className="market-chip market-chip-add" onClick={() => setShowSourcesModal(true)}>
    <Icon name="plus" size={12} />
  </button>
</div>
```

### 验证

```bash
pnpm tsc --noEmit
pnpm tauri dev
# 检查：市场筛选变为 pill chips，点击切换高亮
```

---

## 任务 9：字号 scale 规范化 + 清除 10px（15 分钟）

### 改动 9.1：App.css 全局字号修复

**文件**：`src/App.css`

搜索所有 `font-size: 10px` 和 `font-size: 11px`，统一修改：

| 原始 | 改为 | 位置 |
|------|------|------|
| `10px` | `11px` + `font-family: var(--font-mono)` | sync-time 等 timestamp 场景 |
| `11px`（正文） | `12px` | skill-path 等元信息 |
| `11px`（mono） | 保留 `11px` + `font-family: var(--font-mono)` | hash、timestamp |

### 改动 9.2：添加字号 CSS 变量

**文件**：`src/App.css` `:root` 块内追加：

```css
/* Font size scale */
--fs-caption: 11px;   /* mono-only: timestamp, hash */
--fs-meta: 12px;      /* 元信息、来源、相对时间 */
--fs-body: 13px;      /* 正文描述、设置选项 */
--fs-title: 14px;     /* 卡片标题、区块标题 */
--fs-heading: 16px;   /* 面板标题、modal 标题 */
```

### 验证

```bash
pnpm tsc --noEmit
pnpm tauri dev
# 检查：不再有 10px 文字，所有文本可读性提升
```

---

## 任务 10：字体本地打包（15 分钟）

### 改动 10.1：下载字体文件

```bash
# 下载 DM Sans 和 JetBrains Mono 的 woff2 文件
mkdir -p src/assets/fonts

# DM Sans (400, 500, 600, 700)
# 从 Google Fonts 下载或从 npm @fontsource/dm-sans 获取
pnpm add -D @fontsource-variable/dm-sans @fontsource-variable/jetbrains-mono
```

### 改动 10.2：替换远程 @import

**文件**：`src/App.css` 第 4 行

```css
/* Before: */
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600&family=DM+Sans:opsz,wght@9..40,400;9..40,500;9..40,600;9..40,700&display=swap');

/* After: */
/* 字体已本地打包，通过 main.tsx import */
```

**文件**：`src/main.tsx` 顶部加：

```tsx
import "@fontsource-variable/dm-sans";
import "@fontsource-variable/jetbrains-mono";
```

### 验证

```bash
pnpm tauri dev
# 断网后重启应用，字体仍然正常加载
```

---

## 实施顺序总览

| # | 任务 | 预估 | 依赖 | 验收点 |
|---|------|------|------|--------|
| 1 | 引入增强 CSS | 5 min | 无 | pill tabs + edge highlight 可见 |
| 2 | 替换 unicode 图标 | 30 min | 无 | 所有 unicode 图标消失 |
| 3 | 卡片加 SkillAvatar | 15 min | #2 | 卡片有渐变 avatar |
| 4 | Action Bar 分组 | 20 min | #2 | 三组分栏 + 溢出菜单 |
| 5 | 状态色彩信号 | 10 min | #1 | 更新卡片有 amber 左边框 |
| 6 | Onboarding 全屏改造 | 30 min | #2 | 品牌 splash + grid |
| 7 | Settings 双栏改造 | 30 min | #2 | 侧栏导航 + 主题卡片 |
| 8 | 市场 pill chips | 20 min | #2 | chips 替代下拉 |
| 9 | 字号规范化 | 15 min | 无 | 无 10px 文字 |
| 10 | 字体本地打包 | 15 min | 无 | 断网字体正常 |

**总计约 3 小时**，建议分 3 次提交：

- Commit 1: 任务 1-2-9-10（基础层：CSS + 图标 + 字体 + 字号）
- Commit 2: 任务 3-4-5（卡片增强 + action bar + 状态信号）
- Commit 3: 任务 6-7-8（Onboarding + Settings + Market chips）

---

## 禁区（硬性约束）

1. **不要引入 Tailwind 或任何 CSS 框架** — 保持手写 CSS + CSS 变量体系
2. **不要引入 lucide-react 等图标库** — 用已创建的 `Icon.tsx`（零依赖）
3. **不要修改 Rust 后端代码** — 本次改进纯前端
4. **不要改变任何 IPC 接口签名** — 只改展示层
5. **不要删除任何现有功能** — 只做视觉增强
6. **不要改 `src-tauri/` 下的任何文件**
7. **每次改完跑 `pnpm tsc --noEmit`** — 确保类型安全

---

## 执行 Tickets

本计划的 10 个任务已分解为 15 张垂直切片 tickets，写入 `.scratch/ui-improvement-2026-08-23/issues/01-csse-import.md` … `15-local-fonts.md`。编号即依赖顺序（#01 无依赖，可立即开工；#02–#07、#08–#13 各依赖 #01）。执行时按以下四 commit 节奏落地：

| Commit | Tickets | 主题 |
|---|---|---|
| A | #01 + #02–#07 | CSS tokens + 图标统一（6 张票） |
| B | #08 + #09 + #10 | 卡片增强 + action bar + 状态信号 |
| C | #11 + #12 + #13 | Onboarding + Settings + Market chips |
| D | #14 + #15 | 字号 scale + 本地字体 |

每 commit 前跑 `pnpm tsc --noEmit && pnpm lint`。
