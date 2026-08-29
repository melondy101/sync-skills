# 市场远程仓库与本地缓存重构设计

日期：2026-08-29

## 1. 目标

解决市场远程仓库索引中的错误地址、非递归发现和 `HTTP 404 (not found or private)` 问题；为远程仓库增加按 commit 缓存的本地快照；让“市场源管理”弹窗在源较多时可以纵向滚动；并把 Skill Manager 自身的持久数据统一放到 `~/.skill-manager/`，不再写入 `~/.agents/`。

本次改动不改变各 Agent 工具自身的安装目录。`~/.agents/skills/`、`~/.codex/skills/` 等仍然可以作为同步目标，但不再承载 Skill Manager 的数据库、设置、SSOT 或市场缓存。

## 2. 明确不做的事项

- 不自动迁移 `~/.agents/skill-manager.db`、`~/.agents/settings.json` 或 `~/.agents/skill-manager/ssot/`。
- 不读取旧目录作为回退数据源。
- 不自动删除旧目录；旧数据由用户自行保留或清理。
- 不在本次实现 GitHub 私有仓库认证。未配置认证时，私有仓库仍不可访问，但错误信息必须区分仓库访问失败和仓库内文件缺失。
- 不把数据库、设置或 SSOT 放入 `cache/`。缓存是可删除数据，核心数据不是。

## 3. 数据目录

所有持久化路径由一个统一的路径模块生成，其他模块不得自行拼接 `~/.agents` 或 `~/.skill-manager`。

```text
~/.skill-manager/
├── skill-manager.db
├── config/
│   └── settings.json
├── ssot/
│   ├── <skill-name>/
│   └── _p<project-id>/
│       └── <skill-name>/
└── cache/
    └── markets/
        └── <market-id>/
            ├── metadata.json
            └── snapshots/
                └── <commit-sha>/
                    └── repository/
```

路径模块至少提供以下接口：

- `app_data_root()` → `~/.skill-manager/`
- `database_path()` → `~/.skill-manager/skill-manager.db`
- `settings_path()` → `~/.skill-manager/config/settings.json`
- `ssot_root()` → `~/.skill-manager/ssot/`
- `market_cache_root()` → `~/.skill-manager/cache/markets/`
- `market_snapshot_path(market_id, commit_sha)`

运行时临时下载文件可以继续使用操作系统临时目录，但必须在校验成功后原子写入市场缓存，并在失败时清理未完成文件。

## 4. 市场同步与缓存流程

### 4.1 数据流

1. 读取市场的 `owner`、`name` 和 `branch`。
2. 请求远程分支最新 commit SHA。
3. 如果本地存在该 SHA 的完整快照，直接使用缓存。
4. 如果不存在，下载该分支 ZIP 到临时文件。
5. 解压到临时目录并校验仓库根目录。
6. 将完整仓库原子移动到对应 SHA 的快照目录。
7. 在快照中递归扫描 `SKILL.md`，生成索引并写入 SQLite。
8. 更新市场的 `last_commit_sha` 和 `last_indexed_at`。

网络不可用但存在上次成功快照时，市场可以继续展示旧索引，并返回“正在使用缓存”的状态。手动点击“同步索引”始终允许重新请求，不长期缓存 404、403 或超时结果。

### 4.2 递归发现规则

- 从仓库根目录开始递归。
- 跳过隐藏目录和解压产生的无关元数据。
- 当前目录存在 `SKILL.md` 时，将当前目录识别为一个 skill，并停止继续扫描其子目录。
- 当前目录不存在 `SKILL.md` 时，继续扫描子目录。
- 解析 front matter 中的 `name` 和 `description`；缺少 `name` 时使用目录名。
- 内容哈希覆盖 skill 目录内全部非隐藏文件；核心哈希只覆盖 `SKILL.md`。
- 记录 `SKILL.md` 所在目录相对于仓库根目录的路径，例如 `skills/engineering/code-review`。

该规则同时支持根目录单 skill、一级目录集合和任意深度的嵌套集合，不再依赖 `root/subdir` 两种布局猜测。现有 `layout` 字段可以暂时保留以兼容 API，但不再参与发现和安装路径计算，后续再单独删除。

### 4.3 数据模型

`remote_skills` 增加：

```text
repo_path TEXT NOT NULL DEFAULT ''
```

`repo_path` 表示 skill 目录相对于仓库根目录的规范化斜杠路径。根目录 skill 使用空字符串。`remote_url` 从 `repo_path` 构造，只承担展示和跳转功能；安装和更新不得反向解析 `remote_url`。

安装流程从缓存快照中的 `repo_path` 复制整个目录到 SSOT，保留 `references/`、`scripts/`、`assets/` 等附属文件。若远端 SHA 已变化且目标快照不存在，安装前先执行一次市场同步。

### 4.4 缓存生命周期

- 每个市场至少保留当前成功快照和前一个成功快照。
- 只有完整下载、解压和扫描成功的目录才能成为有效快照。
- 失败请求不覆盖已有成功快照和索引。
- 删除市场时删除对应市场缓存；删除单个远程 skill 不删除共享仓库快照。
- 后续可以增加缓存空间展示和手动清理入口，本次只实现内部清理能力。

## 5. 内置市场源

内置源必须使用经过验证的公开仓库：

- `anthropics/skills`
- `obra/superpowers`
- `mattpocock/skills`

当前返回 404 的 `superpowers/skills`、`karpathy/skill` 和 `khazix/skills` 不再作为有效默认源。后两者在找到明确且稳定的替代仓库前不补充猜测地址。

由于明确不做旧数据迁移，新数据库只 seed 新的有效列表；旧数据库不会被新版本读取。内置身份由后端市场数据明确返回或由稳定自然键判断，前端不再用模板字符串 ID 与数值数据库 ID 比较。

## 6. 错误处理

错误信息按失败阶段分类并包含仓库坐标：

- 仓库元数据 404：`仓库 owner/repo 不存在或当前不可访问`
- 分支不存在：`仓库 owner/repo 中不存在分支 branch`
- ZIP 下载失败：显示 HTTP 状态或网络错误
- ZIP 解压失败：显示缓存写入失败，不破坏旧快照
- 扫描完成但没有 `SKILL.md`：显示“仓库中未发现 Skill”
- 单个 `SKILL.md` 解析失败：记录具体 `repo_path`，继续扫描其他 skill

不再对所有 404 统一显示 `not found or private`。没有认证能力时，界面只说明“不可访问”，不暗示应用已经支持私有仓库。

## 7. 源管理弹窗

弹窗采用三段结构：

```text
弹窗容器（受视口高度限制）
├── 标题与关闭按钮
├── 添加源表单
└── 市场源列表（唯一纵向滚动区域）
```

约束：

- 弹窗最大高度为视口高度减去安全边距。
- 标题和添加源表单不随列表滚动。
- 列表使用 `min-height: 0` 和 `overflow-y: auto`。
- 弹窗宽度响应视口，不保留会导致窄窗口横向裁切的固定最小宽度。
- 窄窗口下源信息与操作按钮改为纵向排列。
- 最后一条源的同步、启停和删除操作在滚动后必须可达。

## 8. 测试边界

测试通过公开行为验证，不直接测试私有辅助函数。

### 8.1 Rust 市场操作边界

- 仓库快照包含根目录、一级和多级 `SKILL.md` 时，市场同步返回完整技能集合。
- 父目录命中 `SKILL.md` 后不重复索引其嵌套 skill。
- 相同 commit SHA 再次同步时复用缓存，不重新下载。
- 新 SHA 下载失败时继续保留上一个成功索引。
- 安装远程 skill 时按 `repo_path` 复制完整目录及附属文件。

网络适配器与文件缓存需要可注入测试替身，测试不依赖真实 GitHub 网络。

### 8.2 数据路径边界

- 数据库、设置、SSOT 和市场缓存都解析到 `~/.skill-manager/` 下。
- 生产路径代码不再生成 `~/.agents/skill-manager*` 或 `~/.agents/settings.json`。
- 不存在新数据库时创建全新数据库，不读取或移动旧数据。

### 8.3 前端用户行为边界

- 打开源管理后渲染所有源。
- 市场列表具有独立滚动容器。
- 添加表单与关闭按钮位于滚动容器之外。
- 窄窗口和低高度窗口下最后一条源的操作仍可访问。
- 内置源显示正确名称和徽标，且不出现删除按钮。

## 9. 实施顺序

按垂直切片执行 TDD：

1. 建立统一数据路径模块，并切换数据库、设置和 SSOT。
2. 增加 `repo_path` 数据库字段和模型映射。
3. 引入可测试的仓库快照缓存与 ZIP 下载边界。
4. 通过缓存快照实现递归索引。
5. 让安装、预览和更新统一使用 `repo_path` 与缓存目录。
6. 修正内置市场列表和内置身份。
7. 重构源管理弹窗滚动布局。
8. 按 `CONTRIBUTING.md` 执行提交前全部校验。

每个切片遵循一个失败测试、一个最小实现的红绿循环，不并行堆叠多个未经验证的修改。
