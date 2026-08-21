# Spec: Market UX Polish + Phase 4 Foundations

> **Status**: Ready for implementation
> **Date**: 2026-XX-XX
> **Triage label**: `ready-for-agent`
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
