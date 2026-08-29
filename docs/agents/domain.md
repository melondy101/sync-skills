# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Layout

**Single-context repo.** Root-level `CONTEXT.md` and `docs/adr/` cover the whole codebase.

The repo is a Tauri v2 desktop app (Rust + React 19 + TypeScript + SQLite) — Rust core in `src-tauri/`, frontend in `src/`. Both share the same domain vocabulary; do **not** split into per-context docs unless a real boundary emerges (e.g. a separately-packaged SDK).

## Before exploring, read these

- **`CONTEXT.md`** at the repo root, or
- **`CONTEXT-MAP.md`** at the repo root if it exists: it points at one `CONTEXT.md` per context. Read each one relevant to the topic.
- **`docs/adr/`**: read ADRs that touch the area you're about to work in.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

Existing domain docs this repo already keeps (do not duplicate; cross-link from `CONTEXT.md` when it's created):

- `AGENTS.md` — domain rules (SSOT model, diff algorithm, release workflow)
- `doc/` — PRD, design discussions, phase-3 / phase-4 / market-ux decision logs
- `docs/spec-market-ux-polish.md` — active spec for the current effort
- `docs/HANDOFF.md` — onboarding and current state
- `docs/testing-issues-triage.md` — issue/triage history

## File structure

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-ssot-name-as-identity.md
│   └── 0002-diff-lcs-threshold.md
└── src/
    └── src-tauri/
```

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

Key vocabulary already locked in `AGENTS.md` — preserve it:

- **SSOT** — single source of truth; `name` is identity, multiple installs allowed.
- **ssot_path** — Rust helper that returns the SSOT directory for a given skill name and project id.
- **diff (LCS)** — diff algorithm with a 5000-line threshold for switching to LCS-based output.

If a concept you need isn't in the glossary yet, that's a signal: either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders), but worth reopening because…_
