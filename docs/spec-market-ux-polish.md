# Spec: Market UX Polish + Phase 4 Foundations

> **Status**: Complete（2026-09-18 复核：T1–T6 均已落地）
> **Date**: 2026-08-21
> **Triage label**: `ready-for-agent`（已全部落地，保留原始标记以便回溯）
> **Progress**:
> - #5 T1 Settings 扩展 — ✅ merged（PR #11, `59281be`）
> - #6 T4 Market 详情 Modal — ✅ merged（PR #12, `2ca39bc`）
> - #7 T2 M9 core_hash — ✅ `ops::mod::check_single_skill` 与冲突检测均已改用 `core_hash`（`a0ba215`）
> - #8 T3 M12 文件监听 — ✅ `watcher.rs`（`lib.rs` 启动）→ `skill-file-changed` 事件 → `App.tsx` 按同步模式决定 toast / 自动同步（`a0ba215`）
> - #9 T5 Market 排序/视图/快捷键 — ✅ 排序 + 卡片/列表视图（`a0ba215`）；统一快捷键层与 `?` 速查表（`de8c020`）
> - #10 T6 本地 tab 来源 chip — ✅ `SkillListRow.tsx` 的 `market-provenance` chip + 来源过滤（`a0ba215`）
> **Source decisions**:
> - `doc/market-ux-decisions.md` §五 (Market UX 9 open items)
> - `doc/phase4_design.md` (Phase 4 scope)
> - `doc/phase-3.md` 方向二 / 方向四 / 方向五
> - GitHub issue #4（父）— scope 总览；子 issue #5–#10 — 每条任务的验收标准与边界

## Problem Statement (summary)

Six problems: market cards lack depth, update flow opaque, local tab provenance invisible, operational friction, update detection over-reports, no auto-sync on file change.

## Solution (summary)

- Market detail Modal (file tree + SKILL.md preview + hash comparison) — T4 (#6) ✅
- Card/list view + multi-field sort + grouped batch actions — T5 (#9) ✅
- New "View diff before Market updates" settings toggle — T1 (#5) ✅
- Local tab `Market:` chip + Source filter + remote-source read-only — T6 (#10) ✅
- Unified keyboard-shortcut layer — T5 (#9) ✅
- M9: switch `check_single_skill` to `core_hash` — T2 (#7) ✅
- M12: file watcher (semi-auto toast, full-auto auto-sync) — T3 (#8) ✅
