# Local Sync Loop — Design

> 本地闭环同步设计：覆盖 SSOT ↔ 工具目录的自动同步 + 远程仓库的拉取与更新检测。
> 与现有 Market（远端 pull）并轨，但**不**包含 Local → Remote push。

## 1. 目标与非目标

**目标**

- 本机 SSOT 修改后，自动反映到所有已安装工具目录（Claude / Cursor / Codex …）。
- 远端 Market 的 skill 索引在合适的时机自动刷新，远端 skill 的版本变更可被及时发现。
- 失败有可见路径，不静默吞错。

**非目标（P0 范围内不做）**

- 本机 SSOT → 远端 push / publish。后续如果需要再做。
- Skill 编辑器内的实时协同 / 多端同步。
- 跨平台打包差异处理（沿用现有 Tauri 行为）。

## 2. 范围与现状基线

**已实现（沿用，不重写）**

- 远端 Market：`src-tauri/src/commands/market.rs`，GitHub repo × N，按 `raw.githubusercontent.com` 拉 SKILL.md。
- 版本判断：`src-tauri/src/hash.rs` SHA-256 content hash + commit SHA 早退。
- SSOT 路径：`src-tauri/src/sync.rs::ssot_path(name, project_id)`，全局 `~/.agents/skill-manager/ssot/<name>/`。
- DB：`remote_skills` 表已含 `remote_content_hash` / `remote_core_hash` / `ssot_path` / `remote_url` / `is_installed`。

**未实现（本文档覆盖）**

- 本地文件监听器（watcher）— `settings.rs` 已有 `auto_sync_on_file_change` 字段占位，未接线。
- 远端轮询 — 现状仅手动按钮 + 应用启动单次 load。
- 失败 UX（同步事件的可见性）— 现状无。
- 同步审计表 — 现状无。

## 3. 架构

```
┌─────────────────┐    file events    ┌──────────────┐
│  fs (SSOT +     │ ────────────────▶ │   watcher    │
│  tool dirs)     │                   │  (notify)    │
└─────────────────┘                   └──────┬───────┘
                                             │ debounce 1s
                                             ▼
                                      ┌──────────────┐
                                      │   debouncer  │
                                      │  + batcher   │
                                      └──────┬───────┘
                                             │
                                             ▼
                                      ┌──────────────┐
                                      │  relinker    │
                                      │  (per skill) │
                                      │  hash guard  │
                                      └──────┬───────┘
                                             │ fail?
                                ┌────────────┴────────────┐
                                ▼                         ▼
                       ┌──────────────┐          ┌──────────────┐
                       │ sync_events  │          │  Blocking    │
                       │ (DB + log)   │          │  Modal (UI)  │
                       └──────────────┘          └──────────────┘

┌─────────────────┐  start / focus    ┌──────────────┐
│  GitHub APIs    │ ◀─────────────── │  remote      │
│  (commits +     │                   │  puller      │
│   contents)     │  manual button   │              │
└─────────────────┘ ◀─────────────── └──────────────┘
```

两条独立管道：**本地闭环**（watcher → debouncer → relinker）和**远端拉取**（puller → DB）。两者通过 `sync_events` 表共享可观测性，不互相阻塞。

## 4. 本地闭环：Watcher

### 4.1 监听范围

两个根路径，分流处理：

| 根 | 路径 | 触发后行为 |
|---|---|---|
| SSOT 根 | `~/.agents/skill-manager/ssot/<name>/` | 自动 hash 比对 + 重链 |
| 工具目录 | 各 AI 工具的 `skills/` 目录（按已注册工具枚举） | 检测到外部修改 → **冲突告警**（R4.6） |

### 4.2 事件过滤（三道防线）

事件 → 队列前先过滤：

1. **临时文件**：`*.swp`、`*~`、`.DS_Store`、`._*`、`*.tmp`。
2. **隐藏文件 / `.git/` 内部**：`notify::RecursiveMode::NonRecursive` + 自定义 `is_ignored(path)`。
3. **metadata-only 变更**：`notify::EventKind::Modify(ModifyKind::Metadata(_))` 丢弃，只接 `ModifyKind::Data(_)`。

三层全过才进队列。任何一条命中 = 静默丢弃，不写日志。

### 4.3 Debounce 与批次

- **Debounce 1s**：窗口内同类事件折叠为单次触发。
- **3 次指数退避（200ms / 1s / 5s）**：处理失败后重试。
- **批次语义**：1s 窗口结束时**聚合**所有事件，按 skill 分组，每个 skill 取最新事件（不是全部事件 — 内容已变则只关心最终态）。

### 4.4 Relink 守门

Debouncer flush 时，对每个受影响的 skill：

```text
1. compute_content_hash(ssot_dir)
2. compare with previous hash stored in memory (last-cycle cache)
3. if equal → skip relink (内容未变)
4. if different → tokio::spawn relink_to_all_tools(skill_name)
```

**Hash 比对守门**的意义：编辑器保存但内容回滚、IDE 触发的虚假事件、watcher 自身的多次触发 — 全部在 hash 这一层被吸收，不产生无意义的 link IO。

### 4.5 并发模型

- Relink 用 `tokio::spawn` + `Semaphore::new(4)`（最多 4 个并发重链任务）。
- 失败按 skill 聚合，不阻断其他 skill。
- 整体超时 30s（极端情况下 abort）。

### 4.6 工具目录告警（conflict UX）

用户在工具目录里手改文件 → watcher 检测到 → 弹**非阻塞提示**（区别于 R5 失败 Modal）：

- 文案："检测到外部修改：`<skill>` 在 `<tool>` 目录被改动"。
- 操作：\[采纳到 SSOT\] \[忽略\] \[查看差异\]。

这条是 Q1-B 的"留口子"实现，不在 P0 必做范围，可作为后续 ticket。

## 5. 远端拉取：Puller

### 5.1 触发时机

| 触发点 | 行为 |
|---|---|
| 应用启动 | 拉全量索引（首次开应用，DB 可能全过期） |
| 窗口聚焦 | 仅 `check_market_commits`（轻量，commit SHA 早退） |
| 用户手动 | 同窗口聚焦 |

**明确不做**：定时器轮询。桌面应用定时器是反模式。

### 5.2 启动全量

按市场枚举，每个市场：

```text
1. GET /repos/{owner}/{repo}/commits/{branch} → 更新 last_commit_sha
2. 如果 last_commit_sha 变了 → 触发全量目录扫描
3. 全量扫描：GET /repos/{owner}/{repo}/contents/ → 探测每个子目录的 SKILL.md
4. 探测：HEAD raw.githubusercontent.com/.../SKILL.md → 取 content hash
5. upsert remote_skills 行
```

启动全量是阻塞的（用户预期"打开应用后看到完整状态"），但**单市场超时 10s**，超时即放弃标记 stale。

### 5.3 聚焦轻量

只走步骤 1（commit SHA 早退）。命中再升级到步骤 2-5。`last_commit_sha` 未变 → 一秒内返回，0 副作用。

### 5.4 Stale UX

每个市场维护 `last_index_sync_at`：

- 失败时**不弹任何东西**（静默落表 + 日志）。
- `now - last_index_sync_at > 24h` → 弹一次 OS notification："`<market>` 索引已过期 \<时长\>，点击刷新"。
- 弹过一次后 24h 内不再弹（限流）。

## 6. 失败 UX

### 6.1 聚合

1s 窗口内的失败事件按 skill 分组，**每个 debounce 周期最多触发一次 Modal**。

未聚合的失败不弹 Modal（避免 1 秒 5 个 skill 失败刷屏）。

### 6.2 强阻塞 Modal

Tauri 应用内 Modal，强阻塞：

- ESC 无效、点击外部无效、必须显式关闭。
- 默认视图：失败批次的 skill 列表 + 每个 skill 一行原因摘要。
- 失败信息必须包括 skill 名称 + 失败原因（IO / 权限 / 路径不存在 / hash 校验失败 等）。
- 关闭 = 用户 ack，下一批次才允许再弹。

### 6.3 详情展开（同一 Modal 内）

Modal 内嵌可展开详情区：

- 完整错误堆栈。
- `sync_events` 最近 10 条相关记录。
- 失败时间戳 + 重试次数。

点击 `[查看详情]` → 同 Modal 内容区就地展开。**不跳转**（避免关 Modal → 跳详情页的两次操作）。

### 6.4 同步落库

每次失败：

- `sync_events` 表写一行：`event_type = 'watcher_relink_failed'`。
- 日志文件（Rust `tracing`）写完整堆栈。
- 用户点 `[关闭]` → Modal 关闭，事件已 ack。

### 6.5 模态生命周期

- Modal 在打开期间，新失败事件 → 静默落表 + 日志，**不追加到 Modal**。
- 用户关闭后下一周期才能弹新 Modal。
- 目的：用户 ack 一批 = 信任锚点；未 ack 时持续弹 = 反向闭环。

## 7. 数据模型

### 7.1 新增表：`sync_events`

```sql
CREATE TABLE sync_events (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type    TEXT NOT NULL,    -- enum: see below
    market_id     INTEGER,          -- nullable, FK markets(id)
    skill_name    TEXT,             -- nullable
    status        TEXT NOT NULL,    -- 'ok' | 'failed' | 'pending'
    error_kind    TEXT,             -- 'io' | 'permission' | 'not_found' | 'hash_mismatch' | 'other'
    error_message TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_sync_events_created_at ON sync_events(created_at DESC);
CREATE INDEX idx_sync_events_skill_name ON sync_events(skill_name);
```

`event_type` enum：

- `watcher_fired`：watcher debounce flush
- `ssot_relinked`：skill 重链成功
- `watcher_relink_failed`：skill 重链失败
- `remote_pull_ok`：远端索引拉取成功
- `remote_pull_failed`：远端拉取失败
- `stale_threshold_hit`：超过 24h stale 阈值

### 7.2 扩展 `markets` 表

```sql
ALTER TABLE markets ADD COLUMN last_index_sync_at TEXT;
```

仅作为 stale 阈值判断用，单一字段，nullable（旧市场迁移时为 NULL）。

### 7.3 `settings.auto_sync_on_file_change`

`settings.rs` 已有字段，启用：

- `effective_auto_sync_on_file_change()` 已实现 `auto_sync_on_file_change || sync_mode == "full-auto"`。
- 加 watcher 模块读取该值，false 时不启动 watcher。

## 8. 资源管理

### 8.1 Watcher 生命周期

- 启动：`tauri::Builder::setup` 中按 `settings.auto_sync_on_file_change` 决定是否 spawn watcher task。
- 运行中：`tokio_util::CancellationToken` 控制。
- 关闭：用户在设置页关掉 watcher → cancel token → watcher task 收到信号 → flush 当前批次（200ms 宽限期）→ 退出。
- 应用退出：`Drop` 实现 + cancel token，进程 exit 前干净退出。

### 8.2 错误隔离

所有 watcher / puller 任务**吞错 + 写日志**，**不允许 panic 冒泡到 UI 层**。`tokio::spawn` 的 task 全部 `.expect` → 实际用 `match` 捕获。

## 9. 错误处理总表

| 失败场景 | 处理 |
|---|---|
| Watcher 启动失败（无权限、notify error） | 表 + 日志 + 设置页"watcher 未启动"提示，**不弹 Modal** |
| 单 skill 重链失败（IO / 权限） | 聚合进 Modal，**不阻断其他 skill** |
| 远端 commit SHA 拉取失败（GitHub 限流、网络） | 表 + 日志，**静默** |
| 远端索引扫描失败 | 表 + 日志 + 该市场 stale 时长累加 |
| 单工具 link 失败（symlink 死链等） | Modal 中按工具聚合，UI 文案区分工具维度 |
| 数据库写入失败（`sync_events` insert） | 日志记录，**不阻断** sync 主流程 |

## 10. 测试策略

### 10.1 单元测试（Rust）

- `hash.rs`：content hash 稳定（同内容不同路径 = 同 hash）、嵌套目录排序稳定、空目录处理。
- debouncer：1s 窗口聚合、3 次退避、批次 flush 时机。
- 失败聚合：N 个 skill 失败 → 单次 Modal 事件。

### 10.2 集成测试

- 模拟 watcher 事件序列（手工 emit `notify::Event`），验证 hash 守门不触发 relink。
- 模拟 commit SHA 变化 → 触发全量索引。
- 模拟 GitHub 429 → 验证不弹 Modal、不重试到爆。

### 10.3 手动验证清单

- [ ] 启动应用：触发全量拉取，DB 有数据。
- [ ] 编辑 SSOT 文件 → 1s 内工具目录的对应文件被重链。
- [ ] 编辑器保存但内容回滚 → 不触发任何 IO。
- [ ] 制造磁盘满 → 失败 Modal 弹出，详情可展开。
- [ ] 应用启动后断网 → 不弹任何 Modal，stale 计时开始。
- [ ] 24h 后启动 → 弹 stale OS notification 一次。
- [ ] 关闭 watcher 设置 → 资源 task 干净退出。

## 11. 实施顺序（建议）

按依赖关系排序，每项可独立成 ticket：

1. **DB 迁移**：`sync_events` 表 + `markets.last_index_sync_at`。
2. **Watcher 核心**：notify crate + debouncer + 三道过滤。
3. **Relink 守门**：hash 守门 + 并发重链。
4. **失败 Modal**：Tauri Modal + 详情展开。
5. **远端轮询**：commit SHA 早退 + 启动全量。
6. **Stale UX**：24h 阈值 + OS notification。
7. **设置开关接线**：`settings.auto_sync_on_file_change` 真接通 watcher。

每项 ticket 单独跑测试、独立 review。

## 12. 决策记录

- **不做 Local → Remote push**：用户明确表达。
- **不做定时器轮询**：桌面应用反模式，聚焦拉 + 启动拉足够。
- **失败走强阻塞 Modal 而非 OS notification**：用户偏好显式 ack，不静默吞错。
- **Modal 内嵌详情而非跳转**：避免关 Modal → 跳详情页的两次操作。
- **Watcher 监听 SSOT + 工具目录两个根**：工具目录外部修改必须有可见路径，但不自动反向同步。

## 13. 开放问题（后续 ticket 处理）

- 工具目录告警（4.6）是否进 P0？默认不在。
- `sync_events` 表的清理策略（保留多久？10 万条上限？）。
- 跨平台 path 处理（Windows 长路径、macOS symlink）。
- Multiple watchers per skill（如同 skill 在多个工具目录同时被改）的合并语义。
