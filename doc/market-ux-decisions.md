# Market 用户体验待确定事项决策记录

> 9 项待评审开放项（编号 8 / 14 / 22 / 33 / 43 / 56 / 81 / 87 / 98）已全部决策完成。

---

## 五、9 个开放项 — 决策完成（2026-08-21）

以下事项已经过逐项讨论并锁定决策，落地为 GitHub issues #5–#10（父 issue #4）：

| # | 开放项 | 决策 | 落地 ticket |
|---|--------|------|------------|
| 8 | 文件结构预览/技能详情层 | **标准版**：SKILL.md 预览 + 文件树（references/scripts/assets） | #6 (T4) |
| 14 | `[查看差异]` 功能 | 复用 `DiffView`；新增 Settings 开关"View diff before Market updates"（默认 ON），关闭后只显示 hash + commit | #9 (T5) |
| 22 | hash 对比展示形式 | 6 位 mono 缩写 + 点击展开完整 64 位 + `[复制]` 按钮 | #6 (T4) |
| 33 | 远程安装技能在本地 Tab 中的可操作范围 | 本地 Tab 显示 `Market: owner/repo` chip + Source 筛选；远程 Skill 在本地 Tab 中**不可编辑/更新**，保留 toggle / 反向同步 / 删除 | #10 (T6) |
| 43 | 批量操作按钮的禁用逻辑 | 按 📋 元操作 / ⚙️ 结果操作分组；按钮始终可点，零目标时 toast 提示（不引入 disabled 态） | #9 (T5) |
| 56 | 排序功能 | 多字段排序（name / updated_at / installed / market_id），asc/desc，复用本地 Tab chip 样式 | #9 (T5) |
| 81 | 卡片/列表视图切换 | 卡片/列表可切换，与本地 Tab 一致 | #9 (T5) |
| 87 | 远程技能详情面板 | 点击卡片展开详情 Modal（含 description / file tree / SKILL.md 预览 / hash / 本地安装状态 / 来源市场） | #6 (T4) |
| 98 | 键盘快捷键 | 全部支持 `Ctrl+K` / `/` / 箭头键 / `Enter` / `U` / `I` / `Esc`；本地 Tab 同步 | #9 (T5) |

完整 spec 见 [`docs/spec-market-ux-polish.md`](../docs/spec-market-ux-polish.md)。
