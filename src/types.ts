// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

export interface Tool {
  id: number;
  name: string;
  global_path: string;
  project_rel_path: string;
  globalPath: string;
  projectRelPath: string;
  created_at: string;
  updated_at: string;
}

export interface ToolTemplate {
  name: string;
  global_path: string;
  project_rel_path: string;
}

export interface Project {
  id: number;
  name: string;
  path: string;
  created_at: string;
}

export interface InstallationInfo {
  tool_id: number;
  tool_name: string;
  status: string;
  synced_at: string | null;
  installation_synced_at: string | null;
}

export interface SkillView {
  id: number;
  name: string;
  description: string | null;
  source_path: string;
  content_hash: string;
  core_hash: string;
  project_id: number;
  ssot_updated_at: string | null;
  source_market_id: number | null;
  created_at: string;
  updated_at: string;
  installed_tools: InstallationInfo[];
  install_count: number;
  has_update: boolean;
}

export interface ScanDetail {
  skill_name: string;
  tool_name: string;
  scope: string;
  status: string;
  source_path: string;
}

export interface ScanResult {
  skills_found: number;
  skills_new: number;
  skills_updated: number;
  errors: string[];
  details: ScanDetail[];
}

export interface SyncResult {
  skill_id: number;
  skill_name: string;
  synced_to: number;
  errors: string[];
}

export interface SkillUpdate {
  skill_id: number;
  skill_name: string;
  source_path: string;
  old_hash: string;
  new_hash: string;
  changed_tool: string | null;
  changed_tool_id: number | null;
}

export interface AppUpdateInfo {
  latest_version: string;
  release_url: string;
  update_available: boolean;
  no_releases: boolean;
  asset_name: string | null;
  asset_url: string | null;
  asset_size: number | null;
}

export interface SyncLog {
  id: number;
  skill_id: number | null;
  skill_name: string | null;
  tool_id: number | null;
  tool_name: string | null;
  project_id: number;
  action: string;
  direction: string | null;
  status: string;
  detail: string | null;
  created_at: string;
}

export interface Settings {
  sync_mode: string;
  prefer_symlink: boolean;
  theme: string;
  language: string;
  close_action: string;
  use_system_proxy: boolean;
  use_proxy: boolean;
  proxy_url: string | null;
  /**
   * Phase 4 / issue #5 (T1 — Settings extension).
   * When true (default), the Market update modal shows a diff view before
   * installing. When false, only the commit message + hash are shown.
   */
  view_market_diff_before_update: boolean;
  /**
   * Phase 4 / issue #5 (T1 — Settings extension).
   * When true, the file-system watcher (Phase 4 / M12) auto-syncs on SKILL.md
   * change. Mirrors `sync_mode == "full-auto"`; the backend exposes a
   * `effective_auto_sync_on_file_change` helper that ORs both. The frontend
   * mirrors this rule so the UI stays in sync without a round-trip.
   */
  auto_sync_on_file_change: boolean;
}

export interface DiffLine {
  op: string;
  content: string;
}

export interface DiffHunk {
  old_start: number;
  old_count: number;
  new_start: number;
  new_count: number;
  lines: DiffLine[];
}

export interface FileDiff {
  path: string;
  change: "added" | "deleted" | "modified";
  hunks: DiffHunk[];
}

export interface SkillDiff {
  skill_name: string;
  source_path: string;
  ssot_path: string;
  files: FileDiff[];
  has_changes: boolean;
}

export interface RemoteInstallation {
  id: number;
  remoteSkillId: number;
  skillName: string;
  marketId: number;
  marketTitle: string;
  projectId: number;
  scope: string;
  installedAt: string | null;
}

export interface ToolPathOption {
  toolId: number;
  name: string;
  path: string;
}

// M5: Conflict types
export interface ConflictVersion {
  tool_id: number;
  tool_name: string;
  core_hash: string;
  source_path: string;
}

export interface ConflictView {
  id: number;
  skill_id: number;
  skill_name: string;
  detected_at: string;
  versions: ConflictVersion[];
}

export interface Toast {
  type: "success" | "error" | "info";
  message: string;
  id: number;
}

// Built-in SKILL.md editor
export interface SkillFile {
  path: string;
  content: string;
}

// Skill health check (Lint)
export interface LintIssue {
  code: string;
  severity: "error" | "warning";
  param?: string | null;
  fixable: boolean;
}

export interface SkillLint {
  skill_id: number;
  skill_name: string;
  issues: LintIssue[];
}

// ==================== Skill Market (Remote Repositories) ====================

export interface MarketTemplate {
  id: string;
  label: string;
  description: string;
  provider: string;
  owner: string;
  name: string;
  branch: string;
  kind: string;
  root_skill: boolean;
}


export interface Market {
  id: number;
  provider: string;
  owner: string;
  name: string;
  branch: string;
  enabled: boolean;
  last_indexed_at: string | null;
  last_checked_at: string | null;
  last_commit_sha: string | null;
  /**
   * Repository layout: "subdir" (each subdirectory is a skill, default)
   * or "root" (the whole repo is one skill). Added in 0.1.18; older
   * market rows return undefined at runtime and default to "subdir".
   */
  layout?: "subdir" | "root";
  created_at: string;
  updated_at: string;
}

export interface RemoteSkill {
  id: number;
  market_id: number;
  skill_name: string;
  description: string | null;
  remote_url: string;
  ssot_path: string;
  remote_content_hash: string;
  remote_core_hash: string;
  is_installed: boolean;
  installed_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface RemoteSkillUpdate {
  id: number;
  market_id: number;
  skill_name: string;
  remote_url: string;
  ssot_path: string;
  old_hash: string;
  new_hash: string;
  has_changes: boolean;
}

/**
 * T4 detail-Modal payload. Mirrors `RemoteSkillDetail` in
 * `src-tauri/src/models.rs`. Reads SSOT-on-disk contents only; the user must
 * install the skill first to see SKILL.md content and the file tree.
 */
export interface RemoteSkillDetail {
  remote_skill_id: number;
  skill_name: string;
  description: string | null;
  remote_url: string;
  ssot_path: string;
  remote_content_hash: string;
  remote_core_hash: string;
  /** SHA-256 of the SSOT directory contents. `null` when SSOT is absent. */
  local_content_hash: string | null;
  /** SHA-256 of the SSOT SKILL.md. `null` when SSOT is absent. */
  local_core_hash: string | null;
  /** `null` when the SSOT copy is absent (skill is not installed yet). */
  skill_md_content: string | null;
  /** Relative paths under SSOT, excluding SKILL.md/local.md and dotfiles. */
  files: string[];
  /** `true` when the SSOT directory does not exist on disk. */
  ssot_missing: boolean;
}

export interface MarketSyncResult {
  market_id: number;
  skills_found: number;
  skills_new: number;
  skills_updated: number;
  errors: string[];
}

export interface RemoteSkillInstalledResult {
  updated: number;
}

export interface MarketCommitUpdate {
  market_id: number;
  market_title: string;
  last_commit_sha: string | null;
  new_commit_sha: string;
}

