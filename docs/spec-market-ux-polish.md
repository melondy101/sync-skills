# Spec: Market UX Polish + Phase 4 Foundations

> **Status**: In progress（2026-08-21）
> **Date**: 2026-08-21
> **Triage label**: `ready-for-agent`（T1 / T4 已落地，剩余 T2 / T3 / T5 / T6 仍 ready-for-agent）
> **Progress**:
> - #5 T1 Settings 扩展 — ✅ merged（PR #11, `59281be`）
> - #6 T4 Market 详情 Modal — ✅ merged（PR #12, `2ca39bc`）
> - #7 T2 M9 core_hash — 🟡 未开工（依赖 #5，已 unblocked）
> - #8 T3 M12 文件监听 — 🟡 未开工（依赖 #5，已 unblocked）
> - #9 T5 Market 排序/视图/快捷键 — 🟡 未开工（依赖 #5 + #6，已 unblocked）
> - #10 T6 本地 tab 来源 chip — 🟡 未开工（依赖 #6，已 unblocked）
> **Source decisions**:
> - `doc/market-ux-decisions.md` §五 (Market UX 9 open items)
> - `doc/phase4_design.md` (Phase 4 scope)
> - `doc/phase-3.md` 方向二 / 方向四 / 方向五

See full content in conversation; minimal version here for commit.

## Problem Statement (summary)

Six problems: market cards lack depth, update flow opaque, local tab provenance invisible, operational friction, update detection over-reports, no auto-sync on file change.

## Solution (summary)

- Market detail Modal (file tree + SKILL.md preview + hash comparison)
- Card/list view + multi-field sort + grouped batch actions
- New "View diff before Market updates" settings toggle
- Local tab `Market:` chip + Source filter + remote-source read-only
- Unified keyboard-shortcut layer
- M9: switch `check_single_skill` to `core_hash`
- M12: file watcher (semi-auto toast, full-auto auto-sync)

Full spec content with user stories, implementation decisions, testing decisions, and out-of-scope lives in the conversation history.
