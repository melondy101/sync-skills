// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Typed wrappers around Tauri commands. Keep every backend call in one place
// so components never import `invoke` directly.

import { invoke } from "@tauri-apps/api/core";
import type {
  Tool, ToolTemplate, Project, SkillView, ScanResult, SyncResult,
  SkillUpdate, SkillDiff, SyncLog, Settings, ConflictView,
  SkillFile, SkillLint, AppUpdateInfo,
  Market, MarketTemplate, RemoteSkill, RemoteSkillDetail, RemoteSkillUpdate, MarketSyncResult,
  RemoteInstallation, MarketCommitUpdate, RemoteSkillInstalledResult,
} from "./types";

// ==================== Tools ====================

export const listTools = () => invoke<Tool[]>("list_tools");

export const addTool = (name: string, globalPath: string, projectRelPath: string) =>
  invoke<Tool>("add_tool", { name, globalPath, projectRelPath });

export const updateToolPath = (toolId: number, globalPath: string, projectRelPath: string) =>
  invoke("update_tool_path", { toolId, globalPath, projectRelPath });

export const deleteTool = (toolId: number, toolName: string) =>
  invoke("delete_tool", { toolId, toolName });

export const listToolTemplates = () => invoke<ToolTemplate[]>("list_tool_templates");

export const discoverTools = () => invoke<ToolTemplate[]>("discover_tools");

// ==================== Skills ====================

export const listSkills = (projectId: number) =>
  invoke<SkillView[]>("list_skills", { projectId });

export const readSkillMd = (skillId: number, projectId: number) =>
  invoke<SkillFile>("read_skill_md", { skillId, projectId });

export const saveSkillMd = (skillId: number, projectId: number, content: string) =>
  invoke<SyncResult>("save_skill_md", { skillId, projectId, content });

export const lintSkills = (projectId: number) =>
  invoke<SkillLint[]>("lint_skills", { projectId });

export const lintSkill = (skillId: number, projectId: number) =>
  invoke<SkillLint>("lint_skill", { skillId, projectId });

export const fixSkill = (skillId: number, projectId: number) =>
  invoke<SkillLint>("fix_skill", { skillId, projectId });

// ==================== Scan ====================

export const fullScan = () => invoke<ScanResult>("full_scan");

export const scanScope = (toolId: number | null, projectId: number) =>
  invoke<ScanResult>("scan_scope", { toolId, projectId });

// ==================== Sync ====================

export const toggleSkill = (skillId: number, toolId: number, projectId: number, active: boolean) =>
  invoke("toggle_skill", { skillId, toolId, projectId, active });

export const syncSkill = (skillId: number, projectId: number, sourcePath?: string | null) =>
  invoke<SyncResult>("sync_skill", { skillId, projectId, sourcePath: sourcePath ?? null });

export const syncAllPending = () => invoke<SyncResult[]>("sync_all_pending");

export const checkUpdates = (projectId: number) =>
  invoke<SkillUpdate[]>("check_updates", { projectId });

export const checkSkillUpdate = (skillId: number, projectId: number) =>
  invoke<SkillUpdate[]>("check_skill_update", { skillId, projectId });

export const getSkillDiff = (skillId: number, sourcePath?: string | null) =>
  invoke<SkillDiff>("get_skill_diff", { skillId, sourcePath: sourcePath ?? null });

export const reverseSyncSkill = (skillId: number, toolId: number, projectId: number) =>
  invoke<SyncResult>("reverse_sync_skill", { skillId, toolId, projectId });

export const dismissSkillUpdate = (skillId: number, toolId: number, currentHash: string) =>
  invoke("dismiss_skill_update", { skillId, toolId, currentHash });

// ==================== Settings ====================

export const getSettings = () => invoke<Settings>("get_settings");

export const updateSettings = (newSettings: Settings) =>
  invoke("update_settings", { newSettings });

// ==================== App self-update ====================

export const checkAppUpdate = () => invoke<AppUpdateInfo>("check_app_update");

export const downloadAppUpdate = (url: string, fileName: string) =>
  invoke<string>("download_app_update", { url, fileName });

export const installAppUpdate = (installerPath: string) =>
  invoke("install_app_update", { installerPath });

// ==================== Projects ====================

export const listProjects = () => invoke<Project[]>("list_projects");

export const addProject = (name: string, path: string) =>
  invoke<Project>("add_project", { name, path });

export const deleteProject = (projectId: number) =>
  invoke("delete_project", { projectId });

export const updateProject = (projectId: number, name: string, path: string) =>
  invoke("update_project", { projectId, name, path });

// ==================== Logs ====================

export const getSyncLogs = (skillId: number | null, limit: number) =>
  invoke<SyncLog[]>("get_sync_logs", { skillId, limit });

// ==================== Conflicts ====================

export const listConflicts = (projectId: number) =>
  invoke<ConflictView[]>("list_conflicts", { projectId });

export const resolveConflict = (conflictId: number, keepToolName: string, projectId: number) =>
  invoke<SyncResult>("resolve_conflict", { conflictId, keepToolName, projectId });

// ==================== Skill Market ====================

export const listMarkets = () => invoke<Market[]>("list_markets");

export const listMarketTemplates = () => invoke<MarketTemplate[]>("list_market_templates");

export const addMarket = (provider: string, owner: string, name: string, branch: string) =>
  invoke<Market>("add_market", { provider, owner, name, branch });

export const addMarketByUrl = (url: string, branch?: string, layout?: "auto" | "root" | "subdir") =>
  invoke<Market>("add_market_by_url", { url, branch: branch ?? null, layout: layout ?? "auto" });

export const checkMarketCommits = () =>
  invoke<MarketCommitUpdate[]>("check_market_commits");

export const updateMarket = (id: number, provider: string, owner: string, name: string, branch: string, enabled: boolean, layout?: string | null) =>
  invoke<Market>("update_market", { id, provider, owner, name, branch, enabled, layout: layout ?? null });

export const deleteMarket = (id: number) =>
  invoke("delete_market", { id });

export const syncMarketIndex = (marketId: number) =>
  invoke<MarketSyncResult>("sync_market_index", { marketId });

export const syncAllMarketIndices = () =>
  invoke<MarketSyncResult[]>("sync_all_market_indices");

export const listRemoteSkills = (marketId?: number | null) =>
  invoke<RemoteSkill[]>("list_remote_skills", { marketId: marketId ?? null });

export const downloadRemoteSkill = (remoteSkillId: number) =>
  invoke<SyncResult>("download_remote_skill", { remoteSkillId });

export const listRemoteInstallations = (projectId: number, marketId: number | null) =>
  invoke<RemoteInstallation[]>("list_remote_installations", { projectId, marketId });

export const scanAllRemoteRepositories = () =>
  invoke<MarketSyncResult[]>("scan_all_remote_repositories");

export const downloadRemoteSkillToSsot = (remoteSkillId: number) =>
  invoke<SyncResult>("download_remote_skill_to_ssot", { remoteSkillId });

export const syncRemoteSkillToTools = (remoteSkillId: number, projectId: number, toolPath: string) =>
  invoke<SyncResult>("sync_remote_skill_to_tools", { remoteSkillId, projectId, toolPath });

export const syncRemoteInstallationsToTools = (projectId: number, marketId: number | null, toolPath: string) =>
  invoke<SyncResult>("sync_remote_installations_to_tools", { projectId, marketId, toolPath });

export const toggleRemoteInstallation = (remoteSkillId: number, projectId: number, scope: string, active: boolean) =>
  invoke<RemoteInstallation>("toggle_remote_installation", { remoteSkillId, projectId, scope, active });

export const syncRemoteInstallations = (projectId: number, marketId: number | null) =>
  invoke<SyncResult>("sync_remote_installations", { projectId, marketId });

export const checkRemoteUpdates = (marketId?: number | null, marketFilter?: number | null) =>
  invoke<RemoteSkillUpdate[]>("check_remote_updates", {
    marketId: marketId ?? null,
    marketFilter: marketFilter ?? null,
  });

export const checkRemoteSsoUpdates = (marketId?: number | null, marketFilter?: number | null) =>
  invoke<RemoteSkillUpdate[]>("check_remote_ssot_updates", {
    marketId: marketId ?? null,
    marketFilter: marketFilter ?? null,
  });

export const setRemoteSkillInstalled = (remoteSkillId: number, active: boolean) =>
  invoke("toggle_remote_installation", { remoteSkillId, projectId: 0, scope: "global", active });

export const setAllRemoteSkillsInstalled = (projectId: number, marketId: number | null, active: boolean) =>
  invoke<RemoteSkillInstalledResult>("set_all_remote_skills_installed", { projectId, marketId, active });

// ==================== Remote Skill Detail (T4) ====================

export const getRemoteSkillDetail = (remoteSkillId: number) =>
  invoke<RemoteSkillDetail>("get_remote_skill_detail", { remoteSkillId });


