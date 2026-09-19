<p align="center">
  <a href="README.md">中文</a> | <strong>English</strong> | <a href="README.ja.md">日本語</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.2.1-blue?style=flat-square" alt="version">
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
- **SSOT Sync** — Hub-and-spoke model centered on `~/.skill-manager/ssot/`, distributing to all tools
- **Reverse Sync** — Push SSOT content to a specific tool directory, overwriting local changes
- **Conflict Management** — Detect version conflicts between tools, with diff preview, manual resolution and automatic adjudication (newest edit / preferred tool; ambiguous cases are left to you)
- **Change Dismissal** — Persistently ignore specific tool changes until content changes again
- **Project-level Management** — Configure independent skill sets per project, with edit support
- **Diff Detection** — Built-in LCS diff view (side-by-side / unified) showing precise file-level changes
- **Skill Market** — Browse, search, and one-click install skills from GitHub / GitLab (including self-hosted instances) / Bitbucket / Azure DevOps repos with built-in market source management (private Azure DevOps orgs need the `AZURE_DEVOPS_PAT` environment variable)
- **File Watching** — Monitors the SSOT tree and auto-syncs on disk changes (full-auto) or notifies (semi-auto)
- **App Self-update** — One-click detection, download, and install of new versions
- **Onboarding & Health Check** — First-run wizard, built-in Lint checks with auto-fix, and an in-app SKILL.md editor
- **Keyboard Shortcuts** — Global search / navigation / sync / source management, with a `?` cheat sheet
- **MCP Server** — Read-only stdio server (`skill-manager-mcp`, sharing the GUI's index); the Settings panel registers / unregisters `skill-manager` in Claude Code, Claude Desktop, Cursor, Qoder, Gemini CLI, Windsurf and Codex
- **Theme & i18n** — Light / Dark / Follow System, 中文 / English / 日本語
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
       └────────►│ ~/.skill-manager/│◄──────────┘
                 │ ssot/ (SSOT Hub) │
                 └────────┬─────────┘
                          │
                 ┌────────┴─────────┐
                 │  skill-manager.db │
                 │  (SQLite)        │
                 └──────────────────┘
```

**Stack:** Tauri v2 (Rust + React 19 + TypeScript + SQLite)

The main UI has three tabs: **Global** (global skill management), **Projects** (project-level skill management), and **Market** (remote skill marketplace).
**Settings** and **Logs** panels are available from the top-right. The app supports minimize-to-tray.

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
pnpm stage:sidecar   # required on a fresh clone, see below
pnpm tauri dev
```

`bundle.externalBin` in `tauri.conf.json` makes Tauri's build script check for `src-tauri/bin/skill-manager-mcp-<target-triple>[.exe]` on **every** cargo invocation, so a fresh clone has to run `pnpm stage:sidecar` first — nothing is built yet at that point, so the script writes a placeholder to break the cycle. Run the same command **again** after the first build to swap the placeholder for the real server binary. Skipping the step fails `pnpm tauri dev` with `resource path ... doesn't exist`.

### Release

Installers for all platforms are built automatically via GitHub Actions — no local packaging needed:

1. Make sure `main` is up to date;
2. Push a version tag:

   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

3. CI (`.github/workflows/release.yml`) builds `.msi` / `.nsis.exe` (Windows),
   `.dmg` / `.app` (universal, macOS) and `.deb` / `.AppImage` (Linux) on three runners,
   and collects them into a **GitHub Release draft**;
4. Publish the draft from the Releases page.

> Installers are currently unsigned, so Windows / macOS will show SmartScreen / Gatekeeper warnings.

**Note**: The version badge tracks the latest published tag; unreleased commits on `main` are listed in the Unreleased section of [docs/CHANGELOG.md](docs/CHANGELOG.md).

## Development

```
sync-skills/
├── src/                    # Frontend (React + TypeScript)
│   ├── App.tsx             # Main component & routing
│   ├── App.css             # Styles (CSS variable theme system)
│   ├── api.ts              # Tauri IPC wrappers
│   ├── i18n/               # Localization strings
│   ├── types.ts            # TypeScript type definitions
│   ├── main.tsx            # Entry point
│   ├── components/         # UI components
│   └── hooks/              # Custom hooks
├── src-tauri/src/          # Rust backend
│   ├── lib.rs              # Entry point & command registration
│   ├── commands/           # Tauri command layer
│   ├── ops/                # Domain logic
│   ├── sync.rs             # File sync
│   ├── scanner.rs          # Directory scanning
│   ├── diff.rs             # LCS diff algorithm
│   ├── db.rs               # SQLite data access
│   ├── lock.rs             # LockManager
│   ├── lint.rs             # SKILL.md health checks
│   ├── market.rs           # Remote marketplace
│   ├── mcp.rs              # Outbound MCP stdio server (read-only tools)
│   ├── watcher.rs          # Skill directory file watcher
│   └── ...
├── doc/                    # Planning docs (PRD, design, phase plan)
├── docs/                   # Issue triage, UI reviews, market redesign docs
└── .github/workflows/      # CI: verify.yml + release.yml
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
| v0.5.0 | ✅ Done | LockManager integration, core_hash change detection |
| v0.6.0 | ✅ Done | In-app update, onboarding wizard, health check / Lint, built-in editor, Market batch operations, file-watcher auto-sync, GitLab market sources, dialog-wide focus trap & a11y, global keyboard shortcuts with a `?` cheat sheet, Market list render performance, PRD §14 path rules enforced in one place with inline form feedback, corrupt-index self-healing, workspace discovery and one-click import, scheduled update checks (off by default), full-auto watcher now really syncs |
| v0.7.0 | ✅ Done | MCP integration: `skill-manager-mcp`, a read-only stdio server (9 tools over the same SQLite index the GUI writes), plus a Settings ▸ MCP panel that idempotently registers `skill-manager` in the JSON configs of Claude Code / Claude Desktop / Cursor / Qoder / Gemini CLI / Windsurf and in Codex's TOML (comments preserved through `toml_edit`), with the binary shipped inside the installer (`bundle.externalBin` + `pnpm stage:sidecar`); market sources completed with Bitbucket Cloud, self-hosted GitLab and Azure DevOps; automatic conflict adjudication (newest edit / preferred tool) |

> The versions above are roadmap stage labels and are independent of the published package version (latest tag: `v0.3.0`).

## License

[AGPL-3.0](LICENSE)
