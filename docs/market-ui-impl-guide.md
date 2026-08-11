# 市场界面改造 · 执行手册（分步实施指南）

- **执行状态**：✅ 全部 8 个任务已落地（2026-08-11），提交在分支 `codex/ui_optimization`：
  - 任务 1（后端 description）：`cf9f296`
  - 任务 2（全局样式）：`53bad22`
  - 任务 3（市场样式）：`11ad7af`
  - 任务 4（i18n）：`3008fda`
  - 任务 5（InstallDialog）：`0b25325`
  - 任务 6（重写 SkillMarketPanel）：`b171d21`
  - 任务 7（弹窗）：`68dcb87`
  - 任务 8（收尾）：`f7e29f0` + `682f9c8`
- 后续在同分支上的演化（不入本手册范围）：`fe72f57` 批量菜单打磨 · `aa6cdf5`/`f3c6dd5` 仓库可达性校验 · `a4a7e09` 加源自动同步 + 每源错误条 · `4c2daa2`/`8c00616` 破坏性动作二次确认 · `8d2f7b9` sync-all 进度 + ConfirmDialog 抽出 context。

- 配套设计文档：`docs/market-ui-redesign.md`（讲"为什么/什么样"）；本手册讲"怎么改"，包含可直接使用的代码。
- 执行方式：**按任务顺序逐个执行，每个任务结束必须跑验证、通过后提交，再做下一个**。不要跨任务混合改动。
- 目标读者假设：对代码库不熟悉的执行者。凡本文给出完整代码处，直接使用；凡给出"保留/删除"清单处，严格按清单操作，不要自由发挥。

---

## 0. 硬性规则（违反任何一条都算失败）

1. 包管理只用 **pnpm**，禁止 npm/yarn。禁止安装任何新依赖（不需要任何新依赖）。
2. 禁止引入 Tailwind、组件库、CSS-in-JS、新字体 CDN。样式全部写进 `src/App.css`，复用现有 CSS 变量。
3. 禁止修改数据库 schema、`src-tauri/src/sync.rs`、`src-tauri/src/diff.rs`、`ssot_path` 相关逻辑。
4. 禁止重命名或删除现有 IPC 命令与 `src/api.ts` 中现有函数签名。
5. 新建的每个源码文件必须带文件头（`pnpm lint` 强制检查）：
   ```
   // Copyright (c) 2026 Skill Manager Contributors
   // SPDX-License-Identifier: AGPL-3.0-only
   ```
6. i18n 改动必须**同时**更新 `src/i18n.ts` 中 zh 和 en 两个对象，key 完全一致。
7. 界面文案禁止出现：SSOT、hash、provider、branch 等技术词（"市场源管理"弹窗内可保留 owner/name 与分支信息）。
8. 每个任务完成后按第 9 节命令验证；全部通过才提交。提交信息格式参照 git log（如 `feat(market): ...`）。

**环境备注**：本机 `cargo check` 偶发 `link.exe error 1224`，这是杀毒软件干扰，不是代码错误——重试一次；仍失败则记录在案继续（Rust 部分交由 CI 验证）。

---

## 任务 0 · 基线确认（5 分钟）

改动前先确认仓库当前是干净的、校验是过的：

```bash
git status                       # 应无未提交改动
npx tsc --noEmit                 # 记录是否通过
pnpm lint
pnpm test
cd src-tauri && cargo check
```

若基线就有失败，记录失败项，之后每个任务只要求"不新增失败"。

---

## 任务 1 · 后端：索引时写入 description（唯一必需的后端改动）

**背景**：市场索引时 description 被硬编码为 `None`，导致卡片没有描述。本地扫描已有的 `parse_front_matter`（`src-tauri/src/scanner.rs:112`，签名 `fn parse_front_matter(content: &str, dir: &Path) -> Result<(String, Option<String>), String>`）可直接复用。

### 1.1 `src-tauri/src/scanner.rs`

把私有函数改为 crate 内可见：

```rust
// 改前（约 112 行）
fn parse_front_matter(content: &str, dir: &Path) -> Result<(String, Option<String>), String> {
// 改后
pub(crate) fn parse_front_matter(content: &str, dir: &Path) -> Result<(String, Option<String>), String> {
```

### 1.2 `src-tauri/src/commands/market.rs` 的 `scan_github_market` 函数

**(a)** 在 `let mut has_skill_md = false;`（约 189 行）下一行加：

```rust
        let mut description: Option<String> = None;
```

**(b)** 在成功写入临时 SKILL.md 之后（约 203 行 `fs::write(skill_dir.join("SKILL.md"), bytes)...?;` 之后）加：

```rust
                            let skill_md_content = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap_or_default();
                            description = crate::scanner::parse_front_matter(&skill_md_content, &skill_dir)
                                .ok()
                                .and_then(|(_, desc)| desc);
```

**(c)** `updates` 元组带上 description（约 222 行）：

```rust
// 改前
        updates.push((skill_name.to_string(), remote_url, ssot_path_str, remote_content_hash, remote_core_hash));
// 改后
        updates.push((skill_name.to_string(), remote_url, ssot_path_str, remote_content_hash, remote_core_hash, description.clone()));
```

**(d)** 解构处（约 229 行）同步改为六元组：

```rust
    for (skill_name, remote_url, ssot_path_str, remote_content_hash, remote_core_hash, description) in updates {
```

**(e)** UPDATE 分支（约 243-246 行）补写 description：

```rust
                conn.execute(
                    "UPDATE remote_skills SET remote_url = ?1, ssot_path = ?2, remote_content_hash = ?3, remote_core_hash = ?4, description = ?5, updated_at = datetime('now') WHERE id = ?6",
                    params![remote_url, ssot_path_str, remote_content_hash, remote_core_hash, description, id],
                ).map_err(|e| format!("Failed to update remote skill: {}", e))?;
```

**(f)** INSERT 分支（约 252-254 行）把 `None::<String>` 换成 `description`：

```rust
                conn.execute(
                    "INSERT INTO remote_skills (id, market_id, skill_name, description, remote_url, ssot_path, remote_content_hash, remote_core_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![id, market.id, skill_name, description, remote_url, ssot_path_str, remote_content_hash, remote_core_hash],
                ).map_err(|e| format!("Failed to insert remote skill: {}", e))?;
```

### 1.3 验证

```bash
cd src-tauri && cargo check && cargo clippy --all-targets -- -D warnings && cargo test
```

通过后提交：`feat(market): index skill description from SKILL.md frontmatter`。

> 注意：已有数据的 description 仍为 NULL，用户下次点「同步索引」后才会填充。这是预期行为。

---

## 任务 2 · 全局样式修正（对比度 / 焦点 / transition）

只改 `src/App.css` 现有 token 与声明，不新增类。

### 2.1 对比度 token（`:root` 亮色块，约 6-58 行）

```css
/* 改前 */
  --accent: #c47f1e;
  --accent-dim: #a56a14;
/* 改后（白字按钮文字达到 ≥4.5:1） */
  --accent: #a56a14;
  --accent-dim: #8f5a0f;

/* 改前 */
  --text-muted: #9999aa;
/* 改后 */
  --text-muted: #6f6f80;
```

### 2.2 暗色 muted（`[data-theme="dark"]` 与 `@media (prefers-color-scheme: dark) [data-theme="system"]` 两处都要改）

```css
/* 改前 */
  --text-muted: #55556a;
/* 改后 */
  --text-muted: #7a7a8e;
```

### 2.3 全局焦点环（加在 `* { box-sizing: border-box; ... }` 规则之后）

```css
:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
```

### 2.4 消灭 `transition: all`

全文搜索 `transition: all`，逐处替换为显式属性。统一用这一条即可（安全覆盖所有现有场景）：

```css
transition: color 0.15s ease, background-color 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease, transform 0.15s ease, opacity 0.15s ease;
```

已知位置：`.tab`（约 227 行）、`.project-btn`（约 271 行）、`.btn`（约 376 行）、`.skill-card`（约 677 行）；以实际搜索结果为准，全部替换。

### 2.5 验证

```bash
npx tsc --noEmit && pnpm lint && pnpm test
```

（CSS 不参与类型检查；如项目有样式相关测试一并通过。）提交：`fix(ui): contrast tokens, focus-visible ring, explicit transitions`。

---

## 任务 3 · 市场新样式（追加到 App.css 末尾）

在 `src/App.css` 文件末尾**追加**以下整块（不改已有规则）：

```css
/* ==================== Market Redesign ==================== */

.market-toolbar {
  display: flex;
  gap: 10px;
  align-items: center;
  flex-wrap: wrap;
  padding: 14px 16px;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  margin-bottom: 16px;
}

.market-chips {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
  margin-bottom: 16px;
}

.chip {
  padding: 5px 14px;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text-secondary);
  font-family: var(--font-sans);
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: color 0.12s ease, background-color 0.12s ease, border-color 0.12s ease;
}

.chip:hover { border-color: var(--border-hover); color: var(--text); }

.chip-active {
  background: var(--accent-glow);
  border-color: var(--accent-dim);
  color: var(--accent);
}

.market-card-desc {
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-secondary);
  margin-bottom: 12px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: 39px;
}

.market-card-meta {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 12px;
}

.market-card-footer {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 8px;
  padding-top: 12px;
  border-top: 1px solid var(--border);
}

.market-installed-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 500;
  color: var(--success);
}

.btn-success-outline {
  background: transparent;
  color: var(--success);
  border: 1px solid var(--success);
}
.btn-success-outline:hover:not(:disabled) { background: var(--success-dim); }

.btn-press:active:not(:disabled) { transform: scale(0.96); }

/* 溢出菜单（原生 details 实现，零 JS） */
.menu { position: relative; }
.menu summary {
  list-style: none;
  cursor: pointer;
  padding: 5px 12px;
  font-size: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface);
  color: var(--text-secondary);
}
.menu summary::-webkit-details-marker { display: none; }
.menu-panel {
  position: absolute;
  right: 0;
  top: calc(100% + 4px);
  min-width: 200px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  padding: 6px;
  z-index: 50;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.menu-item {
  text-align: start;
  padding: 7px 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-family: var(--font-sans);
  font-size: 13px;
  cursor: pointer;
}
.menu-item:hover { background: var(--surface-hover); color: var(--text); }

/* 市场源管理弹窗行 */
.source-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 0;
  border-bottom: 1px solid var(--border);
}
.source-row:last-child { border-bottom: none; }
.source-info { flex: 1; min-width: 0; }
.source-name { font-size: 14px; font-weight: 600; color: var(--text); display: flex; align-items: center; gap: 8px; }
.source-meta { font-size: 12px; color: var(--text-muted); margin-top: 2px; }
.badge-builtin {
  font-size: 11px;
  font-weight: 500;
  padding: 1px 8px;
  border-radius: 999px;
  background: var(--teal-dim);
  color: var(--teal);
}
.modal-wide { max-width: 640px; min-width: 560px; }

/* 更新弹窗行 */
.update-row { padding: 12px 0; border-bottom: 1px solid var(--border); }
.update-row:last-child { border-bottom: none; }
.update-row-head { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
.update-row-name { font-size: 14px; font-weight: 600; color: var(--text); }
.update-row-market { font-size: 12px; color: var(--text-muted); }
.update-row-hash { font-family: var(--font-mono); font-size: 11.5px; color: var(--text-muted); margin-top: 4px; font-variant-numeric: tabular-nums; }
```

提交：`style(market): add redesigned market view styles`。

---

## 任务 4 · i18n 键

在 `src/i18n.ts` 的 zh 对象 `// Skill Market` 注释块内追加（en 对象同位置追加对应英文）：

```ts
    // —— zh 追加 ——
    marketSearchPlaceholder: "搜索技能（名称或描述）…",
    filterAll: "全部",
    filterInstalledOnly: "仅已安装",
    manageSources: "源管理",
    batchMenu: "批量操作",
    reindexAllMarkets: "重新索引全部市场",
    install: "安装",
    installing: "安装中…",
    installDialogTitle: "安装 {0}",
    installToLabel: "安装到",
    scopeGlobal: "全局",
    scopeProject: "项目",
    chooseProject: "选择项目",
    targetToolLabel: "目标工具",
    rememberChoice: "记住我的选择",
    installedToast: "已安装 {0} 到 {1}",
    syncBtn: "同步",
    updateBtn: "更新",
    installedTag: "已安装",
    updatesAvailableTitle: "可更新",
    updateAll: "全部更新",
    localVersion: "本地",
    remoteVersion: "远端",
    noSkillsIndexed: "该市场还没有索引过技能，点「同步索引」拉取。",
    marketSourcesTitle: "市场源管理",
    builtinBadge: "内置",
    skillsCount: "{0} 个技能",
    indexedAt: "索引于 {0}",
    marketUnreachable: "无法访问市场：{0}",
```

```ts
    // —— en 追加 ——
    marketSearchPlaceholder: "Search skills (name or description)…",
    filterAll: "All",
    filterInstalledOnly: "Installed only",
    manageSources: "Sources",
    batchMenu: "Batch",
    reindexAllMarkets: "Re-index all markets",
    install: "Install",
    installing: "Installing…",
    installDialogTitle: "Install {0}",
    installToLabel: "Install to",
    scopeGlobal: "Global",
    scopeProject: "Project",
    chooseProject: "Choose project",
    targetToolLabel: "Target tool",
    rememberChoice: "Remember my choice",
    installedToast: "Installed {0} to {1}",
    syncBtn: "Sync",
    updateBtn: "Update",
    installedTag: "Installed",
    updatesAvailableTitle: "Updates available",
    updateAll: "Update all",
    localVersion: "local",
    remoteVersion: "remote",
    noSkillsIndexed: "No skills indexed yet for this market. Use “Sync Index”.",
    marketSourcesTitle: "Market Sources",
    builtinBadge: "builtin",
    skillsCount: "{0} skills",
    indexedAt: "indexed {0}",
    marketUnreachable: "Market unreachable: {0}",
```

同时**删除**两个孤儿 key（两个语言对象中都删）：`remoteInstallsTab`、`skillMarketTab`。若删除后测试引用报错，说明有组件还在用——先全局搜索确认无引用再删。

验证：`npx tsc --noEmit && pnpm test`。提交：`feat(market): i18n keys for redesigned market UI`。

---

## 任务 5 · 新组件 InstallDialog

新建 `src/components/InstallDialog.tsx`，完整代码如下（文件头两行注释必须保留）：

```tsx
// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef, useState } from "react";
import type { Project, RemoteSkill, Tool } from "../types";

type Props = {
  skill: RemoteSkill;
  marketTitle: string;
  projects: Project[];
  tools: Tool[];
  t: (key: string) => string;
  loading: boolean;
  onConfirm: (projectId: number, toolPath: string, remember: boolean) => void;
  onClose: () => void;
};

const LS_PROJECT = "market.install.projectId";
const LS_TOOL = "market.install.toolPath";

export default function InstallDialog({ skill, marketTitle, projects, tools, t, loading, onConfirm, onClose }: Props) {
  const [projectId, setProjectId] = useState<number>(() => {
    const saved = Number(localStorage.getItem(LS_PROJECT));
    return Number.isFinite(saved) ? saved : 0;
  });
  const [toolPath, setToolPath] = useState<string>(() => localStorage.getItem(LS_TOOL) ?? "");
  const [remember, setRemember] = useState(true);
  const ref = useRef<HTMLDivElement>(null);

  // 项目不存在时回退全局
  useEffect(() => {
    if (projectId !== 0 && !projects.some((p) => p.id === projectId)) setProjectId(0);
  }, [projectId, projects]);

  // 工具默认取第一个
  useEffect(() => {
    if (tools.length > 0 && !tools.some((tool) => tool.globalPath === toolPath)) {
      setToolPath(tools[0].globalPath);
    }
  }, [tools, toolPath]);

  // Esc 关闭 + 打开时聚焦
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("keydown", onKey);
    ref.current?.focus();
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="modal-overlay" onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="modal" role="dialog" aria-modal="true" aria-label={t("installDialogTitle").replace("{0}", skill.skill_name)} ref={ref} tabIndex={-1}>
        <h3>{t("installDialogTitle").replace("{0}", skill.skill_name)}</h3>
        <p className="market-card-meta">{marketTitle}</p>

        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          <div>
            <div className="market-card-meta" style={{ marginBottom: 6 }}>{t("installToLabel")}</div>
            <div style={{ display: "flex", gap: 16, alignItems: "center" }}>
              <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
                <input type="radio" name="install-scope" checked={projectId === 0} onChange={() => setProjectId(0)} />
                {t("scopeGlobal")}
              </label>
              <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
                <input type="radio" name="install-scope" checked={projectId !== 0} onChange={() => setProjectId(projects[0]?.id ?? 0)} />
                {t("scopeProject")}
              </label>
              {projectId !== 0 && (
                <select className="sort-select" value={projectId} onChange={(e) => setProjectId(Number(e.target.value))} aria-label={t("chooseProject")}>
                  {projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
                </select>
              )}
            </div>
          </div>

          <div>
            <div className="market-card-meta" style={{ marginBottom: 6 }}>{t("targetToolLabel")}</div>
            <select className="sort-select" value={toolPath} onChange={(e) => setToolPath(e.target.value)} aria-label={t("targetToolLabel")} style={{ width: "100%" }}>
              {tools.map((tool) => <option key={tool.id} value={tool.globalPath}>{tool.name}</option>)}
            </select>
          </div>

          <label style={{ display: "flex", gap: 6, alignItems: "center", fontSize: 13, color: "var(--text-secondary)" }}>
            <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
            {t("rememberChoice")}
          </label>
        </div>

        <div className="modal-actions">
          <button className="btn btn-secondary" onClick={onClose} disabled={loading}>{t("cancel")}</button>
          <button
            className="btn btn-primary btn-press"
            onClick={() => onConfirm(projectId, toolPath, remember)}
            disabled={loading || tools.length === 0}
          >
            {loading ? t("installing") : t("install")}
          </button>
        </div>
      </div>
    </div>
  );
}

export { LS_PROJECT, LS_TOOL };
```

> 说明：`t("cancel")` 已存在；安装目标用 `tool.globalPath` 作为值（与现有 `selectedToolPath` 语义一致，`syncRemoteSkillToTools` 接收的就是路径字符串）。项目级安装在 P0 用全局路径即可跑通；若 `projectId !== 0`，调用方应按现有 `resolvedToolPaths` 逻辑拼项目路径（见任务 6 的 handleInstallConfirm）。

验证：`npx tsc --noEmit && pnpm lint`。提交：`feat(market): add InstallDialog component`。

---

## 任务 6 · 重写 SkillMarketPanel（核心任务，仔细做）

对 `src/components/SkillMarketPanel.tsx` 做整体重写。Props 签名**保持不变**（App.tsx 不用改）。

### 6.1 保留（原样复制，不要改动逻辑）

- state 保留：`markets`、`marketTitles`、`marketLoading`、`showAddMarket`、`marketUrl`、`marketBranch`、`installProjectId`、`installLoading`、`remoteSkills`、`skillLoading`、`searchQuery`、`selectedMarketFilter`、`updates`、`remoteCheckLoading`、`remoteScanLoading`、`remoteUpdateMode`、`selectedToolPath`、`resolvedToolPaths` 及其自动选择 effect
- state 删除：`marketsCollapsed`（连同 `toggleMarkets` 函数、`installScope`）
- 函数：`loadMarkets`、`handleAddMarketByUrl`、`toggleMarket`、`deleteMarket`、`syncMarketIndex`、`loadRemoteSkills`、`scanAllRemoteRepositories`、`checkRemoteUpdates`（结尾处追加：`if (result.length > 0) setShowUpdatesModal(true);`，并新增对应 state）、`markAllRemoteSkillsInstalled`、`syncRemoteSkillToTools`、`syncAllInstalledRemoteSkills`
- `BUILTIN_MARKET_IDS` 常量
- 派生常量保留：`const selectedProjectPath = projectPaths[installProjectId] ?? "";` 与 `const updateCount = (updates ?? []).length;`（新 JSX 引用了 updateCount）
- 组件 Props 类型定义与 `useEffect(() => loadMarkets(), [])`、`listRemoteInstallations` 的 effect

### 6.2 删除

- 渲染中的 4 个 `<div className="section">` 区块（市场源表格、两条 action-bar、技能表格、更新表格）**全部删除**
- `toggleMarkets`、`marketsCollapsed`、`installScope` 及引用它们的所有 JSX/逻辑（6.1 已列明）
- `selectedToolPath`、`resolvedToolPaths` **保留不删**（卡片上的「同步」按钮仍走 `syncRemoteSkillToTools`，它依赖这两个值）

### 6.3 新增 state

```tsx
  const [installTarget, setInstallTarget] = useState<RemoteSkill | null>(null);
  const [showSourcesModal, setShowSourcesModal] = useState(false);
  const [showUpdatesModal, setShowUpdatesModal] = useState(false);
  const [installedOnly, setInstalledOnly] = useState(false);
```

### 6.4 新增/修改处理函数

```tsx
  const BUILTIN_LABELS: Record<string, string> = {
    "anthropic-skills": "Anthropic Skills",
    "superpowers-skills": "Superpowers Skills",
    "mattpocock-skills": "Matt Pocock Skills",
    "andrej-karpathy-skill": "Andrej Karpathy Skill",
    "khazix-skills": "Khazix Skills",
  };

  function marketTitle(m: Market): string {
    return BUILTIN_LABELS[String(m.id)] ?? `${m.owner}/${m.name}`;
  }

  const updateKeys = useMemo(
    () => new Set((updates ?? []).map((u) => `${u.market_id}:${u.skill_name}`)),
    [updates],
  );

  const visibleSkills = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return remoteSkills.filter((skill) => {
      if (selectedMarketFilter !== "all" && String(skill.market_id) !== selectedMarketFilter) return false;
      if (installedOnly && !skill.is_installed) return false;
      if (q && !skill.skill_name.toLowerCase().includes(q) && !(skill.description || "").toLowerCase().includes(q)) return false;
      return true;
    });
  }, [remoteSkills, selectedMarketFilter, searchQuery, installedOnly]);

  // 安装确认：下载 + 同步串成一步
  async function handleInstallConfirm(projectId: number, toolPath: string, remember: boolean) {
    if (!installTarget) return;
    if (remember) {
      localStorage.setItem("market.install.projectId", String(projectId));
      localStorage.setItem("market.install.toolPath", toolPath);
    }
    // 弹窗以 tool.globalPath 作为工具身份；装到项目时换算成项目路径
    const tool = tools.find((item) => item.globalPath === toolPath);
    const actualPath =
      projectId === 0 || !tool
        ? toolPath
        : `${projectPaths[projectId] ?? ""}/${tool.projectRelPath}`.replace(/\/+/g, "/");
    setInstallLoading(true);
    try {
      await api.downloadRemoteSkillToSsot(installTarget.id);
      const result = await api.syncRemoteSkillToTools(installTarget.id, projectId, actualPath);
      if (result.errors.length > 0) {
        addToast("error", `${installTarget.skill_name}: ${result.errors.join(", ")}`);
      } else {
        const toolName = tool?.name ?? toolPath;
        addToast("success", t("installedToast").replace("{0}", installTarget.skill_name).replace("{1}", toolName));
      }
      setInstallTarget(null);
      await loadRemoteSkills(selectedMarketFilter === "all" ? undefined : Number(selectedMarketFilter));
      onRemoteInstallationsChanged(
        await api.listRemoteInstallations(installProjectId, selectedMarketFilter === "all" ? null : Number(selectedMarketFilter)),
      );
    } catch (e) {
      addToast("error", `${t("install")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  // 单个更新：与安装同链路
  async function updateOne(skill: RemoteSkill) {
    const savedProject = Number(localStorage.getItem("market.install.projectId") ?? 0);
    const savedTool = localStorage.getItem("market.install.toolPath") ?? resolvedToolPaths[0]?.path ?? "";
    setInstallLoading(true);
    try {
      await api.downloadRemoteSkillToSsot(skill.id);
      await api.syncRemoteSkillToTools(skill.id, savedProject, savedTool);
      addToast("success", `${t("updateBtn")}: ${skill.skill_name}`);
      await loadRemoteSkills(selectedMarketFilter === "all" ? undefined : Number(selectedMarketFilter));
    } catch (e) {
      addToast("error", `${t("updateBtn")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }
```

> `checkRemoteUpdates` 修改点：`setUpdates(result)` 之后加 `if (result.length > 0) setShowUpdatesModal(true);`；其余保持原样（包括 commit 检查与 syncMarketIndex 循环）。

### 6.5 新的 return JSX（完整替换）

```tsx
  return (
    <section className="section">
      {/* 工具栏 */}
      <div className="market-toolbar">
        <div className="search-box">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t("marketSearchPlaceholder")}
            className="search-input"
            aria-label={t("marketSearchPlaceholder")}
          />
        </div>
        <button className="btn btn-secondary" onClick={() => setShowSourcesModal(true)}>
          {t("manageSources")}
        </button>
        <button className="btn btn-secondary" onClick={checkRemoteUpdates} disabled={remoteCheckLoading}>
          {remoteCheckLoading ? t("checking") : t("checkRemoteUpdates")}
          {updateCount > 0 && <span className="update-badge">{updateCount}</span>}
        </button>
        <details className="menu">
          <summary>{t("batchMenu")}</summary>
          <div className="menu-panel">
            <button className="menu-item" onClick={scanAllRemoteRepositories} disabled={remoteScanLoading}>
              {remoteScanLoading ? t("scanning") : t("reindexAllMarkets")}
            </button>
            <button className="menu-item" onClick={syncAllInstalledRemoteSkills} disabled={installLoading}>
              {t("syncAllActive")}
            </button>
            <button className="menu-item" onClick={() => markAllRemoteSkillsInstalled(true)} disabled={installLoading}>
              {t("markAllInstalled")}
            </button>
            <button className="menu-item" onClick={() => markAllRemoteSkillsInstalled(false)} disabled={installLoading}>
              {t("unmarkAllInstalled")}
            </button>
          </div>
        </details>
      </div>

      {/* 筛选 chips */}
      <div className="market-chips">
        <button
          className={`chip ${selectedMarketFilter === "all" ? "chip-active" : ""}`}
          onClick={() => { setSelectedMarketFilter("all"); loadRemoteSkills(undefined); }}
        >
          {t("filterAll")}
        </button>
        {markets.filter((m) => m.enabled).map((m) => (
          <button
            key={m.id}
            className={`chip ${String(selectedMarketFilter) === String(m.id) ? "chip-active" : ""}`}
            onClick={() => { setSelectedMarketFilter(String(m.id)); loadRemoteSkills(m.id); }}
          >
            {marketTitle(m)}
          </button>
        ))}
        <button
          className={`chip ${installedOnly ? "chip-active" : ""}`}
          onClick={() => setInstalledOnly((v) => !v)}
        >
          {t("filterInstalledOnly")}
        </button>
      </div>

      {/* 技能卡片网格 */}
      {skillLoading ? (
        <div className="skeleton-grid">
          {[1, 2, 3, 4, 5, 6].map((i) => <div key={i} className="skeleton-card" />)}
        </div>
      ) : visibleSkills.length === 0 ? (
        <div className="empty-state">
          <p>{searchQuery || installedOnly ? t("noMatch") : t("noSkillsIndexed")}</p>
        </div>
      ) : (
        <div className="skill-grid">
          {visibleSkills.map((skill) => {
            const hasUpdate = updateKeys.has(`${skill.market_id}:${skill.skill_name}`);
            const market = markets.find((m) => m.id === skill.market_id);
            return (
              <div key={skill.id} className={`skill-card ${hasUpdate ? "skill-has-update" : ""}`}>
                <div className="skill-header">
                  <h3 className="skill-name">{skill.skill_name}</h3>
                </div>
                <p className="market-card-desc">{skill.description || ""}</p>
                <div className="market-card-meta">
                  {market ? marketTitle(market) : ""}
                </div>
                <div className="market-card-footer">
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
                      <span className="market-installed-tag">{t("installedTag")}</span>
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

      {/* 安装对话框 */}
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

      {/* 市场源管理弹窗（任务 7 实现，先占位） */}
      {showSourcesModal && (
        <MarketSourcesModal
          t={t}
          markets={markets}
          loading={marketLoading}
          skillCounts={remoteSkills}
          onAddToggle={() => setShowAddMarket((v) => !v)}
          showAdd={showAddMarket}
          marketUrl={marketUrl}
          setMarketUrl={setMarketUrl}
          marketBranch={marketBranch}
          setMarketBranch={setMarketBranch}
          onAddByUrl={handleAddMarketByUrl}
          onToggle={toggleMarket}
          onDelete={deleteMarket}
          onSyncIndex={syncMarketIndex}
          builtinIds={BUILTIN_MARKET_IDS}
          builtinLabels={BUILTIN_LABELS}
          onClose={() => setShowSourcesModal(false)}
        />
      )}

      {/* 更新弹窗（任务 7 实现） */}
      {showUpdatesModal && updates && updates.length > 0 && (
        <RemoteUpdatesModal
          t={t}
          updates={updates}
          marketTitles={marketTitles}
          loading={installLoading}
          onUpdateOne={(skillName, marketId) => {
            const skill = remoteSkills.find((s) => s.market_id === marketId && s.skill_name === skillName);
            if (skill) updateOne(skill);
          }}
          onClose={() => setShowUpdatesModal(false)}
        />
      )}
    </section>
  );
```

> 任务 6 阶段若任务 7 的两个弹窗还没写：先把两处弹窗渲染注释掉，并加 `// TODO(任务7)`，保证编译通过；任务 7 完成后放开。

验证：`npx tsc --noEmit && pnpm lint && pnpm test`（`SkillMarketPanel.test.tsx` 如有针对旧表格结构的断言，同步更新为新结构）。提交：`feat(market): card-based browsing view with one-click install`。

---

## 任务 7 · 两个弹窗组件

### 7.1 `src/components/MarketSourcesModal.tsx`

要点（结构遵循 `UpdatesModal.tsx` 的 modal-overlay 模式，加 `modal-wide` 类）：

- 顶部：添加表单——`search-input` 输入仓库链接（value/onChange 用 props 的 marketUrl/setMarketUrl）+ branch select（复用原 add-market-form 的三个 option：自动/main/master）+ `btn-primary` 添加按钮；下方提示文字用 `t("addMarketUrlHint")`
- 列表：每个 market 一行 `.source-row`：
  - 左：`.source-info` → `.source-name`（`builtinLabels[String(m.id)] ?? owner/name` + 内置时加 `<span className="badge-builtin">{t("builtinBadge")}</span>`）+ `.source-meta`（`owner/name · branch · {skillsCount} · {indexedAt}`，技能数按 `skillCounts.filter(s => s.market_id === m.id).length`，索引用 `m.last_indexed_at ?? "-"`）
  - 右：`btn-small btn-secondary` 同步索引（`onSyncIndex(m)`，disabled 当 loading 或 !m.enabled）；`btn-small btn-secondary` 停用/启用（`onToggle(m)`）；非内置时加 `btn-small btn-danger` 删除（`onDelete(m)`）
- 关闭：右上角按钮 + Esc + 点击遮罩，同 InstallDialog 模式；`role="dialog" aria-modal="true"`
- 空列表提示用 `t("marketEmpty")`

### 7.2 `src/components/RemoteUpdatesModal.tsx`

- 标题：`{t("updatesAvailableTitle")}（{updates.length}）`
- 每条 update 一行 `.update-row`：上行 `.update-row-head`（左：`.update-row-name` skill_name + `.update-row-market` marketTitles[market_id]；右：`btn-small btn-primary` 更新 → `onUpdateOne(skill_name, market_id)`）；下行 `.update-row-hash`：`{t("localVersion")} {old_hash.slice(0,6)} → {t("remoteVersion")} {new_hash.slice(0,6)}`
- 底部 `.modal-actions`：`btn-secondary` 关闭
- 同样有 `role="dialog" aria-modal="true"` 与 Esc

两个文件都要 SPDX 文件头。验证同前。提交：`feat(market): sources and updates modals`。

---

## 任务 8 · 清理与收尾

1. 删除 `src/App.css` 中已无人引用的旧类（先全局搜索确认零引用再删）：`.market-select-field`、`.market-select-label`、`.market-select`、`.collapse-chevron`、`.collapsed-count`（后者若其他组件还在用则保留）。
2. 全局搜索 `downloadToSsot` 相关文案 key：`downloadToSsot`、`syncToTools` 在界面按钮上已不再使用，如确认无引用则从 i18n 删除；有引用保留。
3. 按第 9 节跑**全量**验证。

---

## 9. 验证命令（每个任务结束都跑对应子集，收尾跑全量）

```bash
npx tsc --noEmit                                        # 前端类型
pnpm lint                                               # SPDX 头等规范
pnpm test                                               # 前端测试
cd src-tauri && cargo check                             # Rust 编译（任务1后）
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo test
```

手动验证（启动 `pnpm tauri dev`）：

- [ ] 市场 Tab 首屏：搜索框 + chips + 卡片网格；无表格、无 SSOT/hash 字样
- [ ] 点「同步索引」后卡片出现描述文字（任务 1 生效的标志）
- [ ] 搜索能按描述匹配；「仅已安装」chip 生效
- [ ] 点「安装」→ 弹窗 → 确认 → toast 成功 → 卡片变「已安装」
- [ ] 二次安装另一技能：弹窗默认值=上次选择
- [ ] 「检查更新」有结果时自动弹更新窗口，hash 只显示 6 位
- [ ] 「源管理」弹窗：内置市场有友好名与「内置」徽标、无删除按钮
- [ ] Esc 可关闭所有弹窗；Tab 键焦点环可见
- [ ] 切换暗色主题，所有文字可读（无灰字陷进深底）

## 10. 不要做的事（重申）

- 不要给卡片加封面图/图标占位——数据里没有图，宁缺毋滥
- 不要实现相对时间（"2 小时前"）、排序下拉、列表视图切换、技能详情页——这些是 P2，本次不做
- 不要动全局/项目 Tab 的任何渲染逻辑
- 遇到不确定的取舍：停下来问，不要猜
