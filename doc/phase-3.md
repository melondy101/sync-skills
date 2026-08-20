# 第三阶段：演进记录与待办

> **基于**: M4-M7 完成 (commit 9568c20)
> **日期**: 2026-07-14
> **状态**: 持续迭代

---

## Bug #1：工具目录修改后同步异常（P0）— ✅ 已修复

**修复日期**: 2026-07-14

**修复内容**:
- `get_skill_diff` 接受可选 `source_path` 参数，前端传入实际变更的工具目录路径
- `do_sync_skill` 接受 `source_override` 参数，同步后从 SSOT 刷新 hash 并将 `source_path` 更新为 SSOT
- `resolve_conflict` 同样改为从 SSOT 刷新 hash + 更新 source_path
- 新增 DB 方法 `update_skill_source_path`

---

## ✅ 已完成功能

### M10：冲突 Diff 预览

冲突横幅每个版本旁新增"查看差异"按钮，复用 `get_skill_diff`（传入 `source_path`）+ 弹窗渲染 diff。用户裁决前可看到具体差异。

### 项目编辑

新增 `update_project` IPC 命令 + DB 方法 + 前端编辑弹窗（✎ 按钮），支持修改项目名称和路径。

### 反向同步（SSOT → 工具）

新增 `reverse_sync_skill` IPC 命令，将 SSOT 内容推送到指定工具目录，覆盖工具本地更改。更新弹窗 diff 视图中有"用 SSOT 覆盖"按钮。

### 忽略更改

新增 `dismissed_updates` 表 + `dismiss_skill_update` IPC 命令。`check_single_skill` 检查 dismiss 状态——如果 dismissed_hash 与当前 hash 一致则跳过。工具目录再次变更（新 hash）后会重新提示。

---

## 当前状态总结

M4-M7 建立了新的架构基础：名字即身份、双层时间戳、冲突检测与裁决 UI。LockManager 已接入并投入使用，Bug #1 与 M10 已修复。剩余待办：

| 组件 | 状态 |
|------|------|
| LockManager | ✅ 已接入（`do_sync_skill` / `check_single_skill` / `reverse_sync_skill` / `sync_remote_skill_to_tools`） |
| 时间戳字段 | 已存储和展示，未驱动"是否需要同步"的判断 |
| check_updates | 仍用 `content_hash`（全目录）；`core_hash` 已实现并用于冲突检测，但未用于更新判断 |
| 文件监听 | 完全缺失，full-auto 模式仍需手动扫描 |
| ~~工具→中心同步~~ | ~~检测可用，但 diff 展示和同步后状态清除有 bug~~ ✅ Bug #1 已修复 |
| 项目编辑 | ✅ 已完成 |
| 反向同步 | ✅ 已完成 |
| 忽略更改 | ✅ 已完成 |
| 市场 UI 改造 | ✅ 已完成（分支 `codex/ui_optimization`） |

---

## 方向一：接入 LockManager（M8）

**目标**：让 sync 和 scan 互斥，防止并发操作同一 Skill 导致文件冲突。

**状态**：✅ 已完成

**实现**：
1. `run()` 中实例化 `LockManager`，通过 `.manage()` 注册为 Tauri State
2. `do_sync_skill()` / `reverse_sync_skill()` / `check_single_skill()` / `sync_remote_skill_to_tools()` 均使用 `acquire_blocking` / `try_acquire_blocking`
3. `scan_tool_paths()` 不直接加锁（只读扫描，不修改文件）

**验证**：同时触发两个 sync 同一 Skill 的操作，第二个会等待第一个完成。

---

## 方向二：core_hash 变更检测（M9）

**目标**：与设计文档对齐——变更检测只看 SKILL.md，不看附属文件。

**状态**：部分完成

**实现**：
- `compute_core_hash()` 已实现（仅对 `SKILL.md` 计算 SHA-256）
- 冲突检测已使用 `core_hash` 比较（不同工具的 `core_hash` 不同才标记冲突）
- `check_single_skill()` **仍使用 `content_hash`**（全目录递归），附属文件变更仍会触发更新提示
- DB 中 `content_hash` 保留（用于 diff 展示）

**遗留**：`check_single_skill` 的更新判断尚未切换到 `core_hash`，这是 M9 剩余的工作。

---

## 方向三：冲突 Diff 预览（M10）

**目标**：用户在裁决冲突前，能看到各版本的差异。

**状态**：✅ 已完成

**实现**：
- 冲突横幅每个版本旁新增"查看差异"按钮，复用 `get_skill_diff`（传入 `source_path`）+ 弹窗渲染 diff
- 用户裁决前可看到具体差异内容

---

## 方向四：时间戳驱动同步判断（M11）

**目标**：用时间戳辅助判断"是否需要同步"。

**状态**：部分完成

**实现**：
- `ssot_updated_at` 和 `installation_synced_at` / `synced_at` 已存储并在 Skill 卡片底部展示
- 同步按钮仍始终可用，未按时间戳自动高亮/变灰
- hash 比较仍是最终验证手段

**遗留**：时间戳尚未驱动"是否需要同步"的 UI 状态。

---

## 方向五：文件监听自动同步（M12）

**目标**：full-auto 模式下，文件变更自动触发同步，无需手动扫描。

**状态**：计划中

**做法**：
1. 添加 `notify` crate 到 Cargo.toml
2. 新建 `watcher.rs`，实现文件监听器：
   - 监听所有已注册工具的全局路径和项目路径
   - 只关注 `SKILL.md` 文件的创建/修改事件
   - 事件去抖（debounce 500ms）
3. 检测到变更后：
   - 半自动模式：弹出 toast "检测到 X 个 Skill 变更"，用户点击后扫描
   - 全自动模式：自动执行 scan + sync
4. 监听器随 app 启动/停止，路径变更时重新注册

**风险**：
- Windows 上 `notify` 使用 ReadDirectoryChangesW，大量目录可能有性能问题
- 需要处理文件被其他进程锁定的情况
- 去抖时间需要调优（太短会重复触发，太大会感觉迟钝）

---

## 方向六：local.md 去留决策

**现状**：SSOT 目录下创建 `local.md` 标记文件，`collect_files` 跳过它。

**分析**：
- 在 name-as-identity 模型下，`local.md` 的原始用途（区分双向同步 vs 只读 Skill）已被 `skill_installations` 表替代
- 但 `local.md` 有一个副作用：标记"这个目录由 Skill Manager 管理"，防止用户误删
- `hash.rs` 和 `diff.rs` 的 `collect_files` 都跳过 `local.md`，如果去掉需要清理多处代码

**结论**：保留 `local.md`，语义为"此目录由 Skill Manager 管理"。

---

## 推荐优先级

| 优先级 | 方向 | 工作量 | 价值 |
|--------|------|--------|------|
| ~~P0~~ | ~~Bug #1 工具→中心同步异常~~ | ~~0.5d~~ | ✅ 已修复 |
| ~~P0~~ | ~~M8 LockManager 接入~~ | ~~0.5d~~ | ✅ 已完成 |
| P0 | M9 core_hash 用于更新检测 | 0.5d | 与设计文档对齐，减少误报 |
| ~~P1~~ | ~~M10 冲突 Diff 预览~~ | ~~1-2d~~ | ✅ 已完成 |
| P1 | M11 时间戳驱动同步 UI | 0.5d | 状态可视化 |
| P2 | M12 文件监听 | 2-3d | 全自动体验，但复杂度高 |
| — | local.md 去留 | 0.1d | 已决定保留 |

建议优先完成 M9（core_hash 用于更新检测），其次 M11（时间戳驱动 UI）。

---

## 测试指南

### 环境确认

你的机器上已有：
- Rust 1.94.0 ✓
- pnpm 11.5.0 ✓
- Windows 10/11 ✓

### 启动开发模式

```bash
cd D:\Develop\sync-skills
pnpm tauri dev
```

这会同时启动 Vite 前端 (localhost:1420) 和 Rust 后端，弹出一个桌面窗口。

### 数据库迁移测试

M4 的迁移逻辑会在 app 启动时自动执行。如果你之前运行过旧版本：

1. **备份**：`copy %USERPROFILE%\.agents\skill-manager.db %USERPROFILE%\.agents\skill-manager.db.bak`
2. 启动 app → 迁移自动执行
3. 验证：打开日志面板，看是否有迁移错误

如果想从零开始测试：
1. 删除旧 DB：`del %USERPROFILE%\.agents\skill-manager.db`
2. 启动 app → 全新 schema 创建

### 测试清单

**Bug #1 工具→中心同步**：
- [ ] 在工具目录（如 sbuild）中修改 SKILL.md 内容
- [ ] 点击"检查更新"→ 应显示"有更新"
- [ ] 点击"View Diff"→ 应显示实际变更内容（当前为空）
- [ ] 点击"Sync"→ 同步完成后，"有更新"标记应消失（当前不消失）

**M4 身份模型**：
- [ ] 在 Claude Code 和 Codex CLI 目录下各放一个同名 Skill（如 `plan`）
- [ ] 扫描后应只显示一个 Skill 卡片，但安装到两个工具
- [ ] 切换工具的 checkbox 应独立控制

**M5 冲突检测**：
- [ ] 让两个工具的 `plan/SKILL.md` 内容不同
- [ ] 扫描后应出现冲突横幅（红色 banner）
- [ ] 点击 "保留此版本: Claude Code" → 另一个工具的内容应被覆盖
- [ ] 冲突横幅消失

**M7 时间戳**：
- [ ] Skill 卡片上应显示 "SSOT 更新: 2026-07-13 ..."
- [ ] 同步后时间戳应更新

### 已知限制

- ~~工具→中心同步的 diff 和状态清除有 bug（Bug #1）~~ ✅ 已修复
- LockManager 未接入，同时操作同一 Skill 不会等待（M8 解决）~~ → ✅ 已完成
- check_updates 仍用 content_hash，附属文件变更也会触发更新提示（M9 部分完成：core_hash 已用于冲突检测，但更新检测仍用 content_hash）
- ~~冲突裁决没有 diff 预览，是盲选（M10 解决）~~ ✅ 已完成
- 文件监听缺失，full-auto 模式仍需手动扫描（M12 计划中）
- 时间戳已展示但未驱动同步按钮状态（M11 部分完成）
