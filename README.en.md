<p align="center">
  <a href="README.md">中文</a> | <strong>English</strong> | <a href="README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.18-blue?style=flat-square" alt="version">
  <img src="https://img.shields.io/badge/Tauri-v2-orange?style=flat-square&logo=tauri" alt="tauri">
  <img src="https://img.shields.io/badge/Rust-2021-brown?style=flat-square&logo=rust" alt="rust">
  <img src="https://img.shields.io/badge/React-19-blue?style=flat-square&logo=react" alt="react">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="license">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey?style=flat-square" alt="platform">
</p>

<h1 align="center">⬡ Skill Manager</h1>

<p align="center">
  <strong>Cross-tool Skill synchronization for AI coding assistants</strong><br>
  Edit once, sync everywhere.
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#installation">Installation</a> •
  <a href="#development">Development</a> •
  <a href="#roadmap">Roadmap</a>
</p>

---

## Why Skill Manager

AI coding assistants (Claude Code, Cursor, Windsurf, Cline, etc.) all use `SKILL.md` files to define reusable knowledge and workflows. When you use multiple tools, skill files scatter across different directories — manual syncing is tedious and error-prone.

Skill Manager provides a desktop GUI to manage all your skills in one place and automatically sync them across every configured AI tool.

## Features

- **Tool Management** — Register AI coding tool paths with auto-discovery for 13+ known tools
- **Skill Scanning** — Recursively scan directories to discover all `SKILL.md`-based skill directories
- **SSOT Sync** — Hub-and-spoke model centered on `~/.agents/skill-manager/ssot/`, distributing to all tools
- **Reverse Sync** — Push SSOT content to a specific tool directory, overwriting local changes
- **Conflict Management** — Detect version conflicts between tools with diff preview and resolution
- **Change Dismissal** — Persistently ignore specific tool changes until content changes again
- **Project-level Management** — Configure independent skill sets per project, with edit support
- **Diff Detection** — Built-in LCS diff view (side-by-side / unified) showing precise file-level changes
- **Theme Switching** — Light / Dark / Follow System
- **i18n** — 中文 / English / 日本語
- **Activity Logs** — Complete audit trail of all operations

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Claude Code │     │   Cursor    │     │  Windsurf   │
│  .claude/    │     │  .cursor/   │     │  .windsurf/ │
└──────┬───────┘     └──────┬──────┘     └──────┬──────┘
       │                    │                    │
       │     Skill Manager (Tauri Desktop)      │
       │         ┌──────────────────┐           │
       └────────►│  ~/.agents/      │◄──────────┘
                 │  skill-manager/  │
                 │  ssot/ (SSOT Hub)│
                 └────────┬─────────┘
                          │
                 ┌────────┴─────────┐
                 │  skill-manager.db │
                 │  (SQLite)        │
                 └──────────────────┘
```

**Stack:** Tauri v2 (Rust + React 19 + TypeScript + SQLite)

## Installation

### Build from Source

**Prerequisites:**

- [Rust](https://www.rust-lang.org/tools/install) (2021 edition)
- [Node.js](https://nodejs.org/) >= 22 and [pnpm](https://pnpm.io/) >= 11 (**use pnpm, not npm / yarn**)
- [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)

**Build:**

```bash
git clone https://github.com/huang-yi-dae/sync-skills.git
cd sync-skills
pnpm install
pnpm tauri build
```

Build artifacts (installers) are in `src-tauri/target/release/bundle/`.

### Development Mode

```bash
pnpm install
pnpm tauri dev
```

### Release

Installers for all platforms are built automatically via GitHub Actions — no local packaging needed:

1. Make sure `main` is up to date;
2. Push a version tag:

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. CI (`.github/workflows/release.yml`) builds `.msi` / `.nsis.exe` (Windows),
   `.dmg` / `.app` (universal, macOS) and `.deb` / `.AppImage` (Linux) on three runners,
   and collects them into a **GitHub Release draft**;
4. Publish the draft from the Releases page.

> Installers are currently unsigned, so Windows / macOS will show SmartScreen / Gatekeeper warnings.

## Development

```
sync-skills/
├── src/                    # Frontend (React + TypeScript)
│   ├── App.tsx             # Main component
│   ├── App.css             # Styles (CSS variable theme system)
│   ├── types.ts            # TypeScript type definitions
│   └── main.tsx            # Entry point
├── src-tauri/src/          # Rust backend (lib.rs / commands/ / ops.rs / sync.rs / db.rs / scanner.rs / diff.rs / lock.rs / lint.rs, etc.)
├── doc/                    # Planning docs (PRD, design, phase plan, DDL)
├── docs/                   # Issue tracker & resolution notes (e.g. testing-issues-triage.md)
└── .github/workflows/      # CI: verify.yml quality gate + release.yml multi-platform installers
```

### Verification

The full pre-commit checklist (type check / lint / unit tests, 6 commands across frontend + Rust) lives in [CONTRIBUTING.md](CONTRIBUTING.md); CI (`.github/workflows/verify.yml`) enforces the same set of checks on every push / PR.

## Roadmap

| Version | Status | Scope |
|---------|--------|-------|
| v0.1.0 | ✅ Done | Core: tool management, skill scanning, SSOT sync, project management |
| v0.2.0 | ✅ Done | Auto-discovery, sorting/filtering, diff detection, diff view |
| v0.3.0 | ✅ Done | Theme switching, i18n, hash stability fixes |
| v0.4.0 | ✅ Done | Name-as-identity, conflict detection/resolution, timestamps, project edit, reverse sync, change dismissal |
| v0.5.0 | Planned | LockManager integration, core_hash change detection, file watcher |
| v0.6.0 | In progress | In-app one-click update download & install, UX improvements (onboarding polish, keyboard shortcuts, batch operations, list performance) — batch operations (sync-all / mark-all in Market) landed on branch `codex/ui_optimization` |
| v0.7.0 | Proposed | MCP integration: MCP server config management & cross-tool sync, expose an MCP interface for agents |

## License

[AGPL-3.0](LICENSE)
