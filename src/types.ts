// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

export interface Tool {
  id: number;
  name: string;
  global_path: string;
  project_rel_path: string;
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
