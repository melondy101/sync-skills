# 贡献指南（Contributing Guide）

感谢你对 Skill Manager 的关注！本文档说明参与贡献的流程与规范。

## 开发环境

- Node.js ≥ 22，pnpm ≥ 11（**请使用 pnpm，不要使用 npm / yarn**）
- Rust stable 工具链（通过 [rustup](https://rustup.rs/) 安装）
- Tauri v2 的平台依赖：参见 [Tauri 官方文档](https://v2.tauri.app/start/prerequisites/)

```bash
pnpm install        # 安装依赖
pnpm tauri dev      # 启动开发环境
```

提交前请确保以下校验全部通过：

```bash
npx tsc --noEmit                 # 前端类型检查
pnpm lint                        # 前端规范检查（SPDX 头、禁止直接 invoke 等）
pnpm test                        # 前端单元测试
cd src-tauri && cargo check      # Rust 编译检查
cd src-tauri && cargo clippy --all-targets -- -D warnings   # Rust 规范检查
cd src-tauri && cargo test       # Rust 单元测试
```

## 代码规范

### 通用

- 每个源码文件头部保留版权与 SPDX 声明（由 `pnpm lint` 强制检查）：
  ```
  // Copyright (c) 2026 Skill Manager Contributors
  // SPDX-License-Identifier: AGPL-3.0-only
  ```
- 注释与代码风格与所在文件保持一致，不引入与周边不符的新风格。
- 不提交 `src-tauri/target/`、`node_modules/`、`dist/`（已在 .gitignore 中）。

### 前端（TypeScript + React 19）

- 所有后端调用统一走 [src/api.ts](src/api.ts) 的封装，组件内**禁止**直接 `invoke`（由 `pnpm lint` 强制检查，规则见 [eslint.config.js](eslint.config.js)）。
- 类型定义集中在 [src/types.ts](src/types.ts)，与 Rust 侧结构体字段保持 snake_case 对齐。
- 用户可见文案一律通过 [src/i18n.ts](src/i18n.ts) 的 key 引用，新增文案需同时提供 zh / en 两种语言。
- 组件放在 `src/components/`，复用逻辑放在 `src/hooks/`；组件只关注展示与交互，业务规则放在后端。

### 后端（Rust + Tauri v2）

- 遵循四层结构：`lib.rs` 只做注册；`commands/` 是按资源域划分的薄命令层；领域逻辑写在 `ops/`（子模块拆分）+ `sync.rs` 等核心模块；数据访问收敛在 `db.rs`。
- Rust 代码须通过 `cargo clippy --all-targets -- -D warnings`（CI 中强制）。
- 新增 Tauri command 时：放入 `commands/` 对应域文件（或新建域文件并在 `commands/mod.rs` 注册），并加入 `lib.rs` 的 `invoke_handler` 列表。
- 涉及同一 skill 的同步 / 检测操作必须经过 `LockManager` 串行化。
- 错误统一以 `Result<T, String>` 返回给前端，不要 panic；字符串处理注意按字符而非字节索引，避免多字节字符 panic。
- 修改 SSOT 路径逻辑时注意「名字即身份」与路径域隔离规则（详见 [AGENTS.md](AGENTS.md)）。

## Git 工作流

### 分支命名

| 前缀 | 用途 |
|------|------|
| `feat/xxx` | 新功能 |
| `fix/xxx` | Bug 修复 |
| `docs/xxx` | 文档变更 |
| `refactor/xxx` | 重构（不改变行为） |
| `chore/xxx` | 构建 / CI / 依赖等杂项 |

### Commit 规范

采用 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)：

```
<type>(<scope>): <subject>

[可选 body：说明动机与实现要点]
```

- **type**：`feat` / `fix` / `docs` / `refactor` / `test` / `chore` / `perf` / `ci`
- **scope**（可选）：受影响的域，如 `sync`、`scan`、`settings`、`ui`、`updater`
- **subject**：一句话说清做了什么，使用祈使句，不加句号，建议不超过 72 字符
- 一个 commit 只做一件事；不要把无关改动混在同一个 commit 里

示例：

```
feat(updater): download release asset locally with progress events
fix(sync): avoid same-name overwrite across project domains
docs: add MCP integration direction to roadmap
```

### Pull Request 规范

PR 描述请包含以下部分：

```markdown
## 背景 / 动机
为什么需要这个改动（关联 issue 用 `Closes #123`）。

## 变更内容
- 按条列出主要改动点，标注涉及的模块 / 文件。

## 测试
- 如何验证的：`tsc --noEmit`、`cargo check`、`cargo test` 结果；
- 涉及 UI 的改动附截图或录屏；
- 涉及同步 / 扫描逻辑的改动说明手动验证场景。

## 影响范围与风险
数据库 schema、SSOT 路径、跨平台行为等敏感面是否受影响。
```

其他要求：

- PR 标题遵循与 commit 相同的 Conventional Commits 格式。
- 保持 PR 聚焦、体量小；大改动请先开 issue 讨论方案。
- CI 通过后才会被 review；review 意见解决后再合并。
- 不要在本地手动 `tauri build` 产物并提交；发布统一由推送 `v*` 标签触发 CI 完成。

## 问题反馈

- Bug / 功能建议：提交 GitHub Issue，附复现步骤、期望行为、系统与版本信息。
- 已知问题与排查记录见 [docs/testing-issues-triage.md](docs/testing-issues-triage.md)。

## 许可证

本项目采用 AGPL-3.0-only 许可证。提交贡献即表示你同意你的代码以相同许可证发布。
