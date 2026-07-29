// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Typed wrappers around Tauri commands. Keep every backend call in one place
// so components never import `invoke` directly.

import { invoke } from "@tauri-apps/api/core";
import type {
  Tool, ToolTemplate, Project, SkillView, ScanResult, SyncResult,
  SkillUpdate, SkillDiff, SyncLog, Settings, ConflictView,
  SkillFile, SkillLint, AppUpdateInfo,
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
