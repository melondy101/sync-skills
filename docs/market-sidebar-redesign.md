# Market Sidebar Redesign — 执行计划

> **Status**: Ready for implementation（2026-08-26）
> **Date**: 2026-08-26
> **Design mockup**: `outputs/market-ui-full-spec.html`（三 Tab：Icon Gallery / Interaction Spec / Interactive Mockup）
> **Competitive references**: Raycast Store, Obsidian Plugins, Glama.ai, Smithery, mcp.so, Composio
> **Scope**: 市场页布局重构（侧栏导航 + 可折叠操作栏 + 多选过滤）
> **Dependencies**: 无后端改动，纯前端（React + CSS）
> **Estimated effort**: 5 tasks × ~30min = ~2.5h

---

## 0. 硬性规则

1. 只用 **pnpm**，禁止 npm/yarn，禁止安装新依赖。
2. 样式全部写进 `src/App.css`，复用现有 CSS 变量（`--accent`, `--border`, `--card` 等）。禁止 Tailwind / CSS-in-JS / 新字体。
3. 禁止修改 Rust 后端、数据库 schema、`src/api.ts` 中现有函数签名。
4. 每个新建 `.tsx` / `.ts` 文件必须带文件头：
   ```
   // Copyright (c) 2026 Skill Manager Contributors
   // SPDX-License-Identifier: AGPL-3.0-only
   ```
5. i18n 改动**同时**更新 `src/i18n.ts` 中 zh 和 en，key 一致。
6. 图标只用 `<Icon name="..." />` 组件，SVG 路径写在 `Icon.tsx` 的 `ICON_PATHS` 中。
7. 每个任务完成后跑验证命令，全部通过才提交。

---

## 验证命令清单（每个任务结束后执行）

```bash
npx tsc --noEmit          # 前端类型检查
pnpm lint                  # ESLint
pnpm test                  # vitest
```

提交格式：`feat(market): <简述>` 或 `style(market): <简述>`

---

## 改动总览

| 任务 | 文件 | 类型 | 说明 |
|------|------|------|------|
| T1 | `src/components/Icon.tsx` | 改 | 新增 11 个 SVG 图标 |
| T2 | `src/i18n.ts` | 改 | 新增 ~15 个 i18n key |
| T3 | `src/components/MarketSidebar.tsx` | 新建 | 侧栏导航组件 |
| T4 | `src/components/SkillMarketPanel.tsx` | 重写 | 主布局改为 sidebar + main 双栏 |
| T5 | `src/App.css` | 追加 | 侧栏/折叠/过滤/卡片新样式 |

---

## T1 · 新增图标（5 分钟）

### 改动文件
`src/components/Icon.tsx`

### 1.1 扩展 `IconName` 类型（约第 19 行）

在 `| "info"` 后面追加：

```typescript
  | "panel-left-close"  // sidebar collapse
  | "panel-left-open"   // sidebar expand
  | "layers"            // all sources
  | "sliders-horizontal" // actions toggle
  | "check-circle"      // mark installed
  | "x-circle"          // unmark installed
  | "power"             // enable/disable toggle
  | "clock"             // timestamps
  | "git-branch"        // branch info
  | "chevron-up"        // collapsible drawer open
  | "filter"            // filter label
```

### 1.2 追加 SVG 路径到 `ICON_PATHS`（约第 185 行 `};` 之前）

```tsx
  "panel-left-close": (
    <>
      <rect width="18" height="18" x="3" y="3" rx="2" />
      <path d="M9 3v18" />
      <path d="m14 9-3 3 3 3" />
    </>
  ),
  "panel-left-open": (
    <>
      <rect width="18" height="18" x="3" y="3" rx="2" />
      <path d="M9 3v18" />
      <path d="m10 9 3 3-3 3" />
    </>
  ),
  layers: (
    <>
      <path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.84Z" />
      <path d="m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65" />
      <path d="m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65" />
    </>
  ),
  "sliders-horizontal": (
    <>
      <line x1="21" y1="4" x2="14" y2="4" />
      <line x1="10" y1="4" x2="3" y2="4" />
      <line x1="21" y1="12" x2="12" y2="12" />
      <line x1="8" y1="12" x2="3" y2="12" />
      <line x1="21" y1="20" x2="16" y2="20" />
      <line x1="12" y1="20" x2="3" y2="20" />
      <circle cx="12" cy="4" r="2" />
      <circle cx="10" cy="12" r="2" />
      <circle cx="14" cy="20" r="2" />
    </>
  ),
  "check-circle": (
    <>
      <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
      <path d="m9 11 3 3L22 4" />
    </>
  ),
  "x-circle": (
    <>
      <circle cx="12" cy="12" r="10" />
      <path d="m15 9-6 6" />
      <path d="m9 9 6 6" />
    </>
  ),
  power: (
    <>
      <path d="M12 2v10" />
      <path d="M18.4 6.6a9 9 0 1 1-6.4 2.5" />
    </>
  ),
  clock: (
    <>
      <circle cx="12" cy="12" r="10" />
      <path d="M12 6v6l4 2" />
    </>
  ),
  "git-branch": (
    <>
      <path d="M6 3v12" />
      <circle cx="18" cy="6" r="3" />
      <circle cx="6" cy="18" r="3" />
      <path d="M18 9a9 9 0 0 1-9 9" />
    </>
  ),
  "chevron-up": <path d="m18 15-6-6-6 6" />,
  filter: <path d="M22 3H2l8 9.46V19l4 2v-8.54L22 3z" />,
```

### 验证
```bash
npx tsc --noEmit
```
确认无 `IconName` 类型错误。提交：`feat(market): add 11 new SVG icons for sidebar redesign`

---

## T2 · i18n 新增 key（5 分钟）

### 改动文件
`src/i18n.ts`

### 2.1 在 zh 对象的 `// Skill Market` 区域末尾（约 `skillsTab` 之后）追加

```typescript
    // Market sidebar redesign
    sidebarSources: "源",
    sidebarAllSources: "所有源",
    sidebarFilterPlaceholder: "过滤源…",
    sidebarAddSource: "添加源",
    sidebarCollapse: "收起侧栏",
    sidebarExpand: "展开侧栏",
    actionsToggle: "操作",
    actionsReindexAll: "重新索引全部",
    actionsSyncAll: "同步所有已安装",
    actionsMarkAll: "标记全部已安装",
    actionsUnmarkAll: "取消全部已安装",
    actionsManageSources: "管理源",
    filterLabel: "过滤:",
    sourceHeaderSync: "同步索引",
    sourceHeaderDisable: "禁用",
    sourceHeaderEnable: "启用",
    sourceHeaderDelete: "删除",
    sourceHeaderBranch: "分支 {0}",
    sourceHeaderIndexed: "索引于 {0}",
    sourceHeaderSkills: "{0} 个技能",
    emptyNoSkills: "该源还没有索引过技能",
    emptyNoMatch: "没有匹配的技能",
    cardSourceBadge: "{0}",
```

### 2.2 在 en 对象对应位置追加

```typescript
    // Market sidebar redesign
    sidebarSources: "Sources",
    sidebarAllSources: "All Sources",
    sidebarFilterPlaceholder: "Filter sources…",
    sidebarAddSource: "Add Source",
    sidebarCollapse: "Collapse sidebar",
    sidebarExpand: "Expand sidebar",
    actionsToggle: "Actions",
    actionsReindexAll: "Reindex All",
    actionsSyncAll: "Sync All Installed",
    actionsMarkAll: "Mark All Installed",
    actionsUnmarkAll: "Unmark All",
    actionsManageSources: "Manage Sources",
    filterLabel: "Filter:",
    sourceHeaderSync: "Sync Index",
    sourceHeaderDisable: "Disable",
    sourceHeaderEnable: "Enable",
    sourceHeaderDelete: "Delete",
    sourceHeaderBranch: "branch: {0}",
    sourceHeaderIndexed: "indexed {0}",
    sourceHeaderSkills: "{0} skills",
    emptyNoSkills: "No skills indexed for this source",
    emptyNoMatch: "No skills match your filters",
    cardSourceBadge: "{0}",
```

### 验证
```bash
npx tsc --noEmit
```
提交：`feat(market): add i18n keys for sidebar redesign`

---

## T3 · 新建 MarketSidebar 组件（20 分钟）

### 新建文件
`src/components/MarketSidebar.tsx`

### 3.1 完整组件代码

```tsx
// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useMemo, useState } from "react";
import { Icon } from "./Icon";
import type { Market, RemoteSkill } from "../types";

type Props = {
  markets: Market[];
  remoteSkills: RemoteSkill[];
  selectedMarketFilter: string; // "all" | String(market.id)
  onSelectFilter: (filter: string) => void;
  onAddSource: () => void;
  marketTitle: (m: Market) => string;
  t: (key: string) => string;
};

export default function MarketSidebar({
  markets,
  remoteSkills,
  selectedMarketFilter,
  onSelectFilter,
  onAddSource,
  marketTitle,
  t,
}: Props) {
  const [collapsed, setCollapsed] = useState(false);
  const [filterQuery, setFilterQuery] = useState("");

  // Count skills per market
  const skillsByMarket = useMemo(() => {
    const counts = new Map<number, number>();
    for (const skill of remoteSkills) {
      counts.set(skill.market_id, (counts.get(skill.market_id) ?? 0) + 1);
    }
    return counts;
  }, [remoteSkills]);

  const totalSkills = remoteSkills.length;
  const enabledMarkets = markets.filter((m) => m.enabled);

  // Filter markets by search query
  const filteredMarkets = useMemo(() => {
    const q = filterQuery.trim().toLowerCase();
    if (!q) return enabledMarkets;
    return enabledMarkets.filter((m) =>
      marketTitle(m).toLowerCase().includes(q)
    );
  }, [enabledMarkets, filterQuery, marketTitle]);

  return (
    <aside className={`m-sidebar ${collapsed ? "m-sidebar-collapsed" : ""}`}>
      {/* Header */}
      <div className="m-sidebar-head">
        {!collapsed && (
          <h3 className="m-sidebar-title">{t("sidebarSources")}</h3>
        )}
        <button
          className="m-sidebar-collapse-btn"
          onClick={() => setCollapsed((v) => !v)}
          title={collapsed ? t("sidebarExpand") : t("sidebarCollapse")}
          aria-label={collapsed ? t("sidebarExpand") : t("sidebarCollapse")}
        >
          <Icon
            name={collapsed ? "panel-left-open" : "panel-left-close"}
            size={16}
          />
        </button>
      </div>

      {/* Filter input */}
      {!collapsed && (
        <div className="m-sidebar-filter">
          <input
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder={t("sidebarFilterPlaceholder")}
            className="m-sidebar-filter-input"
          />
        </div>
      )}

      {/* Source list */}
      <div className="m-sidebar-list">
        {/* "All Sources" item */}
        <button
          className={`m-src-item ${selectedMarketFilter === "all" ? "selected" : ""}`}
          onClick={() => onSelectFilter("all")}
          title={collapsed ? t("sidebarAllSources") : undefined}
        >
          <div className="m-src-avatar m-src-avatar-all">
            <Icon name="layers" size={16} />
          </div>
          {!collapsed && (
            <>
              <div className="m-src-info">
                <div className="m-src-name">{t("sidebarAllSources")}</div>
                <div className="m-src-meta">
                  {enabledMarkets.length} repos · {totalSkills} skills
                </div>
              </div>
              <span className="m-src-count">{totalSkills}</span>
            </>
          )}
        </button>

        {/* Individual market items */}
        {filteredMarkets.map((m) => {
          const count = skillsByMarket.get(m.id) ?? 0;
          const initial = marketTitle(m).charAt(0).toUpperCase();
          return (
            <button
              key={m.id}
              className={`m-src-item ${String(selectedMarketFilter) === String(m.id) ? "selected" : ""}`}
              onClick={() => onSelectFilter(String(m.id))}
              title={collapsed ? marketTitle(m) : undefined}
            >
              <div className="m-src-avatar">{initial}</div>
              {!collapsed && (
                <>
                  <div className="m-src-info">
                    <div className="m-src-name">{marketTitle(m)}</div>
                    <div className="m-src-meta">
                      {count} skills
                      {m.last_indexed_at && (
                        <> · {m.last_indexed_at}</>
                      )}
                    </div>
                  </div>
                  <span className="m-src-count">{count}</span>
                </>
              )}
            </button>
          );
        })}
      </div>

      {/* Add source button */}
      <div className="m-sidebar-foot">
        <button className="m-add-src" onClick={onAddSource}>
          <Icon name="plus" size={14} />
          {!collapsed && <span>{t("sidebarAddSource")}</span>}
        </button>
      </div>
    </aside>
  );
}
```

### 验证
```bash
npx tsc --noEmit
```
确认 `MarketSidebar` 无类型错误。提交：`feat(market): add MarketSidebar component`

---

## T4 · 重写 SkillMarketPanel 布局（30 分钟）

### 改动文件
`src/components/SkillMarketPanel.tsx`

### 核心改动

#### 4.1 新增 import

在文件顶部 import 区域追加：

```tsx
import MarketSidebar from "./MarketSidebar";
```

#### 4.2 新增 state

在现有 state 声明区域追加：

```tsx
const [drawerOpen, setDrawerOpen] = useState(false);
const [selectedFilterMarkets, setSelectedFilterMarkets] = useState<Set<number>>(new Set());
```

#### 4.3 修改 `visibleSkills` 的过滤逻辑（约第 457 行）

将现有的 `selectedMarketFilter` 单选逻辑改为：

```tsx
const visibleSkills = useMemo(() => {
  const q = searchQuery.trim().toLowerCase();
  return remoteSkills.filter((skill) => {
    // Sidebar selection: single source
    if (selectedMarketFilter !== "all" && String(skill.market_id) !== selectedMarketFilter)
      return false;
    // Multi-select filter chips (only active when sidebar = "all")
    if (selectedMarketFilter === "all" && selectedFilterMarkets.size > 0) {
      if (!selectedFilterMarkets.has(skill.market_id)) return false;
    }
    if (installedOnly && !skill.is_installed) return false;
    if (q && !skill.skill_name.toLowerCase().includes(q) && !(skill.description || "").toLowerCase().includes(q))
      return false;
    return true;
  });
}, [remoteSkills, selectedMarketFilter, selectedFilterMarkets, searchQuery, installedOnly]);
```

#### 4.4 替换 JSX return 的 `<section>` 内容

将整个 `return ( <section ...> ... </section> )` 替换为：

```tsx
return (
  <section className="market-page">
    <div className="market-layout">
      {/* ── Left Sidebar ── */}
      <MarketSidebar
        markets={markets}
        remoteSkills={remoteSkills}
        selectedMarketFilter={selectedMarketFilter}
        onSelectFilter={(f) => {
          setSelectedMarketFilter(f);
          loadRemoteSkills(f === "all" ? undefined : Number(f));
        }}
        onAddSource={() => setShowSourcesModal(true)}
        marketTitle={marketTitle}
        t={t}
      />

      {/* ── Main Content ── */}
      <div className="m-main">
        {/* Top action bar */}
        <div className="m-topbar">
          <div className="m-search-box">
            <Icon name="search" size={14} className="m-search-icon" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t("marketSearchPlaceholder")}
              className="m-search-input"
            />
          </div>
          <button
            className="btn btn-secondary m-actions-btn"
            onClick={() => setDrawerOpen((v) => !v)}
          >
            <Icon name="sliders-horizontal" size={14} />
            {t("actionsToggle")}
            <Icon name={drawerOpen ? "chevron-up" : "chevron-down"} size={12} />
          </button>
          <button
            className="btn btn-primary"
            onClick={checkRemoteUpdates}
            disabled={remoteCheckLoading}
          >
            <Icon name="refresh-cw" size={14} className={remoteCheckLoading ? "sync-spinner" : ""} />
            {remoteCheckLoading ? t("checking") : t("checkRemoteUpdates")}
            {updateCount > 0 && <span className="update-badge">{updateCount}</span>}
          </button>
        </div>

        {/* Collapsible actions drawer */}
        <div className={`m-actions-drawer ${drawerOpen ? "open" : ""}`}>
          <div className="m-actions-inner">
            <button className="btn btn-secondary btn-small" onClick={scanAllRemoteRepositories} disabled={remoteScanLoading}>
              <Icon name="scan" size={14} />
              {remoteScanLoading ? t("scanning") : t("actionsReindexAll")}
            </button>
            <button className="btn btn-secondary btn-small" onClick={requestSyncAllInstalledRemoteSkills} disabled={installLoading}>
              <Icon name="download" size={14} />
              {t("actionsSyncAll")}
            </button>
            <button className="btn btn-secondary btn-small" onClick={() => requestMarkAllRemoteSkillsInstalled(true)} disabled={installLoading}>
              <Icon name="check-circle" size={14} />
              {t("actionsMarkAll")}
            </button>
            <button className="btn btn-secondary btn-small" onClick={() => requestMarkAllRemoteSkillsInstalled(false)} disabled={installLoading}>
              <Icon name="x-circle" size={14} />
              {t("actionsUnmarkAll")}
            </button>
            <button className="btn btn-secondary btn-small" onClick={() => setShowSourcesModal(true)}>
              <Icon name="settings" size={14} />
              {t("actionsManageSources")}
            </button>
          </div>
        </div>

        {/* Filter bar (visible when sidebar = "all") */}
        {selectedMarketFilter === "all" && (
          <div className="m-filter-bar">
            <span className="m-filter-label">
              <Icon name="filter" size={12} />
              {t("filterLabel")}
            </span>
            {markets.filter((m) => m.enabled).map((m) => {
              const isOn = selectedFilterMarkets.has(m.id);
              return (
                <button
                  key={m.id}
                  className={`m-fchip ${isOn ? "on" : ""}`}
                  onClick={() => {
                    setSelectedFilterMarkets((prev) => {
                      const next = new Set(prev);
                      if (next.has(m.id)) next.delete(m.id);
                      else next.add(m.id);
                      return next;
                    });
                  }}
                >
                  {isOn && <Icon name="check" size={12} className="ck" />}
                  {marketTitle(m).split("/")[0]}
                </button>
              );
            })}
            <span className="m-filter-divider" />
            <button
              className={`m-fchip ${installedOnly ? "on" : ""}`}
              onClick={() => setInstalledOnly((v) => !v)}
            >
              {installedOnly && <Icon name="check" size={12} className="ck" />}
              {t("filterInstalledOnly")}
            </button>
          </div>
        )}

        {/* Source header (visible when single source selected) */}
        {selectedMarketFilter !== "all" && (() => {
          const market = markets.find((m) => String(m.id) === selectedMarketFilter);
          if (!market) return null;
          const count = remoteSkills.filter((s) => s.market_id === market.id).length;
          const initial = marketTitle(market).charAt(0).toUpperCase();
          return (
            <div className="m-source-header">
              <div className="m-source-header-avatar">{initial}</div>
              <div className="m-source-header-info">
                <h2>{marketTitle(market)}</h2>
                <p>
                  <Icon name="git-branch" size={12} />
                  {t("sourceHeaderBranch").replace("{0}", market.branch)} · {t("sourceHeaderSkills").replace("{0}", String(count))}
                  {market.last_indexed_at && (
                    <>
                      {" · "}<Icon name="clock" size={12} />
                      {t("sourceHeaderIndexed").replace("{0}", market.last_indexed_at)}
                    </>
                  )}
                </p>
              </div>
              <div className="m-source-header-actions">
                <button className="btn btn-secondary btn-small" onClick={() => syncMarketIndex(market)} disabled={marketLoading || !market.enabled}>
                  <Icon name="refresh-cw" size={14} />
                  {t("sourceHeaderSync")}
                </button>
                <button className="btn btn-secondary btn-small" onClick={() => toggleMarket(market)} disabled={marketLoading}>
                  <Icon name="power" size={14} />
                  {market.enabled ? t("sourceHeaderDisable") : t("sourceHeaderEnable")}
                </button>
                {!BUILTIN_MARKET_IDS.has(String(market.id)) && (
                  <button className="btn btn-danger btn-small" onClick={() => requestDeleteMarket(market)} disabled={marketLoading}>
                    <Icon name="trash" size={14} />
                    {t("sourceHeaderDelete")}
                  </button>
                )}
              </div>
            </div>
          );
        })()}

        {/* Skill grid */}
        <div className="m-grid-wrap">
          {skillLoading ? (
            <div className="skeleton-grid">
              {[1, 2, 3, 4, 5, 6].map((i) => <div key={i} className="skeleton-card" />)}
            </div>
          ) : visibleSkills.length === 0 ? (
            <div className="m-empty-state">
              <Icon name="search" size={48} />
              <p>{searchQuery || installedOnly || selectedFilterMarkets.size > 0
                ? t("emptyNoMatch")
                : t("emptyNoSkills")}</p>
            </div>
          ) : (
            <div className="skill-grid">
              {visibleSkills.map((skill) => {
                const hasUpdate = updateKeys.has(`${skill.market_id}:${skill.skill_name}`);
                const market = markets.find((m) => m.id === skill.market_id);
                return (
                  <div
                    key={skill.id}
                    className={`skill-card ${hasUpdate ? "skill-has-update" : ""}`}
                    role="button"
                    tabIndex={0}
                    onClick={() => setDetailSkill(skill)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        setDetailSkill(skill);
                      }
                    }}
                  >
                    <div className="skill-header">
                      <h3 className="skill-name">{skill.skill_name}</h3>
                      {/* Show source badge in "all" view */}
                      {selectedMarketFilter === "all" && market && (
                        <span className="m-card-src-badge">
                          {marketTitle(market).split("/")[0]}
                        </span>
                      )}
                    </div>
                    <p className="market-card-desc">{skill.description || ""}</p>
                    <div className="market-card-footer" onClick={(e) => e.stopPropagation()}>
                      {!skill.is_installed ? (
                        <button className="btn btn-small btn-primary btn-press" onClick={() => setInstallTarget(skill)} disabled={installLoading}>
                          {t("install")}
                        </button>
                      ) : (
                        <>
                          {hasUpdate && (
                            <button className="btn btn-small btn-primary btn-press" onClick={() => updateOne(skill)} disabled={installLoading}>
                              {t("updateBtn")}
                            </button>
                          )}
                          <span className="market-installed-tag">
                            <Icon name="check" size={12} />
                            {t("installedTag")}
                          </span>
                          <button className="btn btn-small btn-secondary" onClick={() => syncRemoteSkillToTools(skill)} disabled={installLoading}>
                            {t("syncBtn")}
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>

    {/* ── Modals (unchanged) ── */}
    {installTarget && (() => {
      const installMarket = markets.find((m) => m.id === installTarget.market_id);
      return (
        <InstallDialog
          skill={installTarget}
          marketTitle={installMarket ? marketTitle(installMarket) : ""}
          projects={projects}
          tools={tools}
          t={t}
          loading={installLoading}
          onConfirm={handleInstallConfirm}
          onClose={() => setInstallTarget(null)}
        />
      );
    })()}
    {showSourcesModal && (
      <MarketSourcesModal
        t={t} markets={markets} loading={marketLoading} skillCounts={remoteSkills}
        marketErrors={marketSyncErrors}
        onAddToggle={() => setShowAddMarket((v) => !v)} showAdd={showAddMarket}
        marketUrl={marketUrl} setMarketUrl={setMarketUrl}
        marketBranch={marketBranch} setMarketBranch={setMarketBranch}
        marketLayout={marketLayout} setMarketLayout={setMarketLayout}
        onAddByUrl={handleAddMarketByUrl} onToggle={toggleMarket}
        onDeleteRequest={requestDeleteMarket} onSyncIndex={syncMarketIndex}
        onChangeLayout={changeMarketLayout}
        builtinIds={BUILTIN_MARKET_IDS} builtinLabels={BUILTIN_LABELS}
        onClose={() => setShowSourcesModal(false)}
      />
    )}
    {showUpdatesModal && updates && updates.length > 0 && (
      <RemoteUpdatesModal
        t={t} updates={updates} marketTitles={marketTitles}
        loading={installLoading}
        onUpdateOne={(skillName, marketId) => {
          const skill = remoteSkills.find((s) => s.market_id === marketId && s.skill_name === skillName);
          if (skill) updateOne(skill);
        }}
        onClose={() => setShowUpdatesModal(false)}
      />
    )}
    {detailSkill && (() => {
      const detailMarket = markets.find((m) => m.id === detailSkill.market_id);
      return (
        <RemoteSkillDetailModal
          t={t} skill={detailSkill}
          marketTitle={detailMarket ? marketTitle(detailMarket) : ""}
          isInstalled={detailSkill.is_installed}
          installedAt={detailSkill.installed_at}
          addToast={addToast}
          onClose={() => setDetailSkill(null)}
          onInstall={() => { setDetailSkill(null); setInstallTarget(detailSkill); }}
        />
      );
    })()}
  </section>
);
```

### 验证
```bash
npx tsc --noEmit
pnpm test
```
提交：`feat(market): rewrite SkillMarketPanel with sidebar layout`

---

## T5 · CSS 样式（20 分钟）

### 改动文件
`src/App.css`

### 5.1 在 `/* ==================== Market Redesign ==================== */` 注释之后，替换现有的 `.market-toolbar` 到 `.market-chips` 区域（约 2246–2264 行）

将整个 market-toolbar + market-chips 块替换为以下新样式：

```css
/* ==================== Market Sidebar Redesign ==================== */

.market-page {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.market-layout {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* ── Sidebar ── */
.m-sidebar {
  width: 240px;
  min-width: 240px;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  background: var(--bg);
  transition: width 0.2s ease, min-width 0.2s ease;
  overflow: hidden;
}
.m-sidebar-collapsed {
  width: 48px;
  min-width: 48px;
}

.m-sidebar-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 12px 8px;
  flex-shrink: 0;
}
.m-sidebar-title {
  font-size: var(--fs-caption);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  white-space: nowrap;
}
.m-sidebar-collapse-btn {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--text-muted);
  padding: 2px;
  border-radius: var(--radius-sm);
  transition: background 0.12s;
}
.m-sidebar-collapse-btn:hover {
  background: var(--bg-elevated);
  color: var(--text);
}

.m-sidebar-filter {
  padding: 0 10px 8px;
  flex-shrink: 0;
}
.m-sidebar-filter-input {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: var(--fs-meta);
  font-family: var(--font-sans);
  background: var(--input-bg);
  color: var(--text);
}
.m-sidebar-filter-input::placeholder { color: var(--text-muted); }

.m-sidebar-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 6px 6px;
}

.m-src-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  border: none;
  background: none;
  width: 100%;
  text-align: left;
  font-family: var(--font-sans);
  transition: background 0.12s;
}
.m-src-item:hover { background: var(--surface-hover); }
.m-src-item.selected { background: var(--accent-glow); }
.m-sidebar-collapsed .m-src-item {
  justify-content: center;
  padding: 7px 4px;
}

.m-src-avatar {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  background: var(--bg-elevated);
  color: var(--text-secondary);
}
.m-src-avatar-all {
  background: var(--accent-glow);
  color: var(--accent);
}
.m-src-item.selected .m-src-avatar {
  background: var(--accent-glow-strong);
  color: var(--accent);
}

.m-src-info { flex: 1; min-width: 0; }
.m-src-name {
  font-size: var(--fs-body);
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.m-src-meta {
  font-size: var(--fs-caption);
  color: var(--text-muted);
}
.m-src-count {
  font-size: var(--fs-caption);
  font-weight: 500;
  color: var(--text-muted);
  background: var(--bg-elevated);
  padding: 1px 7px;
  border-radius: 999px;
  flex-shrink: 0;
}
.m-src-item.selected .m-src-count {
  background: var(--accent-glow-strong);
  color: var(--accent);
}

/* Collapsed: hide text, show avatar only */
.m-sidebar-collapsed .m-sidebar-title,
.m-sidebar-collapsed .m-sidebar-filter,
.m-sidebar-collapsed .m-src-info,
.m-sidebar-collapsed .m-src-count { display: none; }

.m-sidebar-foot {
  padding: 8px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}
.m-add-src {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: 100%;
  padding: 7px;
  border: 1px dashed var(--border);
  border-radius: var(--radius-sm);
  background: none;
  cursor: pointer;
  font-size: var(--fs-meta);
  color: var(--text-muted);
  font-family: var(--font-sans);
  transition: all 0.12s;
}
.m-add-src:hover {
  border-color: var(--accent-dim);
  color: var(--accent);
  background: var(--accent-glow);
}

/* ── Main Content ── */
.m-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Top bar */
.m-topbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 20px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  background: var(--surface);
}
.m-search-box {
  flex: 1;
  position: relative;
}
.m-search-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-muted);
}
.m-search-input {
  width: 100%;
  padding: 7px 12px 7px 34px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: var(--fs-body);
  font-family: var(--font-sans);
  background: var(--input-bg);
  color: var(--text);
}
.m-search-input::placeholder { color: var(--text-muted); }

.m-actions-btn {
  display: flex;
  align-items: center;
  gap: 6px;
}

/* Actions drawer */
.m-actions-drawer {
  overflow: hidden;
  max-height: 0;
  transition: max-height 0.25s ease;
  border-bottom: 0 solid var(--border);
  flex-shrink: 0;
  background: var(--bg-elevated);
}
.m-actions-drawer.open {
  max-height: 80px;
  border-bottom-width: 1px;
}
.m-actions-inner {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 10px 20px;
}

/* Filter bar */
.m-filter-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 20px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  flex-wrap: wrap;
}
.m-filter-label {
  font-size: var(--fs-caption);
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 4px;
  margin-right: 4px;
}
.m-fchip {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 10px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--card);
  font-size: 11.5px;
  color: var(--text-secondary);
  cursor: pointer;
  font-family: var(--font-sans);
  transition: all 0.12s;
}
.m-fchip:hover { border-color: var(--border-hover); }
.m-fchip.on {
  background: var(--accent-glow);
  border-color: var(--accent-dim);
  color: var(--accent);
}
.m-fchip .ck { display: none; }
.m-fchip.on .ck { display: inline; }
.m-filter-divider {
  width: 1px;
  height: 18px;
  background: var(--border);
  margin: 0 4px;
}

/* Source header */
.m-source-header {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 16px 20px 12px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.m-source-header-avatar {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  font-weight: 700;
  background: var(--accent-glow-strong);
  color: var(--accent);
  flex-shrink: 0;
}
.m-source-header-info { flex: 1; min-width: 0; }
.m-source-header-info h2 {
  font-size: var(--fs-heading);
  font-weight: 600;
  margin: 0;
}
.m-source-header-info p {
  font-size: var(--fs-meta);
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 2px;
}
.m-source-header-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

/* Grid wrapper */
.m-grid-wrap {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
}

/* Card source badge */
.m-card-src-badge {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--bg-elevated);
  color: var(--text-muted);
  white-space: nowrap;
  flex-shrink: 0;
}

/* Empty state */
.m-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 60px 20px;
  color: var(--text-muted);
}
.m-empty-state svg { color: var(--border-hover); }
.m-empty-state p { font-size: var(--fs-body); }

/* skill-header flex row (for name + badge) */
.skill-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 6px;
}
```

### 5.2 删除旧的不再需要的样式

删除以下 CSS 块（已被侧栏布局替代）：
- `.market-toolbar`（约 2246–2256 行）— 替换为 `.m-topbar`
- `.market-chips`（约 2258–2264 行）— 替换为 `.m-filter-bar`
- `.chip` + `.chip-active`（约 2266–2286 行）— 替换为 `.m-fchip`

### 验证
```bash
npx tsc --noEmit
pnpm lint
pnpm test
```

提交：`style(market): add sidebar layout CSS and remove old toolbar/chips`

---

## 完成检查

所有任务完成后：

```bash
# 完整验证
npx tsc --noEmit
pnpm lint
pnpm test
cd src-tauri && cargo check && cd ..

# 视觉检查
pnpm tauri dev
# 验证项：
# 1. 侧栏正常显示所有源，点击切换过滤
# 2. 侧栏折叠/展开动画流畅
# 3. "All Sources" 时显示多选 filter chips
# 4. 单选某源时显示 Source Header + Sync/Disable/Delete
# 5. Actions 折叠/展开动画流畅
# 6. 搜索框实时过滤
# 7. 卡片 source badge 在 "All" 视图显示
# 8. Dark mode 正常（所有颜色走 CSS 变量）
# 9. 空状态正确显示
```

---

## 不在本计划范围内

- MarketSourcesModal 改造（保持现有 modal 不变）
- 后端 / Rust 代码改动
- 数据库 schema 变更
- 本地/全局 tab 的来源 chip（T6 #10 已有独立 spec）
- 排序/视图切换（T5 #9 已有独立 spec）
