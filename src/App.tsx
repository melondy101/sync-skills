// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import type {
  Tool, ToolTemplate, Project, SkillView, ScanResult, SyncResult,
  SkillUpdate, SkillDiff, SyncLog, Settings, Toast, InstallationInfo,
  ConflictView, DiffHunk, DiffLine,
} from "./types";

type Lang = "zh" | "en";

const translations: Record<Lang, Record<string, string>> = {
  zh: {
    // Nav
    global: "全局",
    projects: "项目",
    logs: "日志",
    settings: "设置",
    back: "返回",

    // Settings panel
    settingsTitle: "设置",
    appearance: "外观",
    theme: "主题",
    themeLight: "亮色",
    themeDark: "暗色",
    themeSystem: "跟随系统",
    language: "语言",
    langZh: "中文",
    langEn: "英文",
    syncMode: "同步模式",
    syncModeSemi: "半自动（手动同步）",
    syncModeFull: "全自动（检测到变更即同步）",
    syncModeHint: "半自动：扫描检测变更后，由你点击同步。全自动：扫描触发立即同步。",
    preferSymlink: "优先使用符号链接代替复制",
    symlinkHint: "符号链接节省磁盘空间，但 Windows 需要开启开发者模式。失败时自动回退到复制。",
    saveSettings: "保存设置",

    // Logs panel
    activityLogs: "活动日志",
    refresh: "刷新",
    noLogs: "暂无操作记录。",
    time: "时间",
    action: "操作",
    skill: "Skill",
    tool: "工具",
    status: "状态",
    detail: "详情",

    // Log actions
    actionScan: "扫描",
    actionSync: "同步",
    actionEnable: "启用",
    actionDisable: "禁用",
    actionRemove: "移除",
    actionAddTool: "添加工具",
    actionDeleteTool: "删除工具",
    actionAddProject: "添加项目",
    actionDeleteProject: "删除项目",

    // Tool paths
    toolPaths: "工具路径",
    addTool: "+ 添加工具",
    cancel: "取消",
    templatePlaceholder: "选择模板...",
    toolNamePlaceholder: "工具名称",
    globalPathPlaceholder: "全局路径（如 ~/.mytool/skills/）",
    relPathPlaceholder: "项目相对路径（如 .mytool/skills/）",
    add: "添加",
    edit: "编辑",
    save: "保存",
    globalPath: "全局路径：",
    projectPath: "项目路径：",

    // Discovery
    deleteProject: "删除项目",
    addAll: "全部添加",
    adding: "添加中...",
    dismiss: "忽略",

    // Actions
    scanAll: "全部扫描",
    scanning: "扫描中...",
    checkUpdates: "检查更新",
    checking: "检查中...",
    syncAllActive: "同步所有激活",
    searchPlaceholder: "搜索 Skill...",
    allTools: "所有工具",
    sortName: "名称",
    sortUpdated: "更新时间",
    sortCreated: "创建时间",
    ascending: "升序",
    descending: "降序",

    // Skills section
    skills: "Skills",
    updates: "更新",
    noSkillsYet: "尚未发现任何 Skill。",
    configureAndScan: '在上方配置工具路径后点击"全部扫描"来发现 Skill。',
    noMatch: '未找到匹配的 Skill',
    updateAvailable: "有可用更新",
    synced: "已同步",
    syncNow: "立即同步",
    syncingCard: "同步中...",
    checkUpdate: "检查更新",
    checkingUpdate: "检查中...",
    inSync: "已同步",
    changedIn: "变更来自",

    // Project nav
    noProjects: "暂无项目，添加一个开始使用。",
    addProjectBtn: "+ 项目",

    // Modals
    addProjectTitle: "添加项目",
    nameLabel: "名称：",
    pathLabel: "绝对路径：",
    projectNamePlaceholder: "我的项目",
    projectPathPlaceholder: "D:\\my-project",

    // Scan result
    scanResults: "扫描结果",
    found: "发现",
    new: "新增",
    updated: "已更新",
    scope: "范围",
    close: "关闭",

    // Updates modal
    updatesAvailable: "有可用更新",
    viewDiff: "查看差异",
    loading: "加载中...",
    update: "更新",
    skip: "跳过",
    updateToSsot: "更新到 SSOT",
    noChanges: "文件已同步，无实际差异。",
    splitView: "并排",
    unifiedView: "统一",
    maximize: "最大化",
    restore: "还原",
    viewDiffAll: "查看全部差异",
    sourceLabel: "源",

    // Toast messages
    failedLoadTools: "加载工具失败",
    failedLoadSkills: "加载 Skill 失败",
    failedLoadProjects: "加载项目失败",
    failedLoadSettings: "加载设置失败",
    failedLoadLogs: "加载日志失败",
    settingsSaved: "设置已保存",
    failedSaveSettings: "保存设置失败",
    scanComplete: "扫描完成",
    noSkillsFound: "未发现 Skill",
    scanFailed: "扫描失败",
    allUpToDate: "所有 Skill 已是最新",
    checkUpdatesFailed: "检查更新失败",
    failedLoadDiff: "加载差异失败",
    skillUpdated: "Skill 已更新到 SSOT",
    syncFailed: "同步失败",
    toggleFailed: "切换失败",
    syncedToTools: "已同步到",
    tools: "个工具",
    syncComplete: "同步完成",
    syncedCount: "已同步",
    errors: "错误",
    syncAllFailed: "同步全部失败",
    globalPathNotEmpty: "全局路径不能为空",
    globalPathMustBeAbsolute: "全局路径必须是绝对路径（以 /、~/、盘符或 \\\\ 开头）",
    pathUpdated: "路径已更新",
    failedUpdate: "更新失败",
    nameAndPathRequired: "名称和路径为必填",
    relPathNoRelative: "项目相对路径不能以 ./ 或 ../ 开头",
    toolAdded: "已添加工具",
    failedAddTool: "添加工具失败",
    confirmDeleteTool: "删除工具",
    confirmDeleteToolMsg: "这将移除所有相关安装和同步日志。",
    toolDeleted: "已删除工具",
    failedDeleteTool: "删除工具失败",
    nameAndPathRequiredProject: "名称和路径为必填",
    enterAbsolutePath: "请输入绝对路径",
    projectAdded: "已添加项目",
    failedAddProject: "添加项目失败",
    confirmDeleteProject: "删除项目",
    confirmDeleteProjectMsg: "所有相关数据将被移除。",
    projectDeleted: "已删除项目",
    failedDeleteProject: "删除项目失败",
    addedTools: "已添加",
    toolUnit: "个工具",

    // Conflicts (M5)
    conflicts: "冲突",
    conflictsTitle: "Skill 冲突",
    conflictsDesc: "不同工具对同一 Skill 有不同修改。选择保留哪个版本。",
    noConflicts: "暂无冲突。",
    resolveConflict: "裁决冲突",
    keepVersion: "保留此版本",
    resolving: "裁决中...",
    conflictResolved: "冲突已解决",
    failedResolve: "解决冲突失败",
    conflictDetected: "检测到冲突",

    // Timestamps (M7)
    lastSynced: "最后同步",
    ssotUpdated: "SSOT 更新",
    never: "从未",
    syncStatus: "同步状态",
    statusInSync: "已同步",
    statusPending: "待同步",
    statusConflict: "有冲突",

    // Project edit
    editProject: "编辑项目",
    editProjectTitle: "编辑项目",
    projectUpdated: "项目已更新",
    failedUpdateProject: "更新项目失败",

    // Reverse sync & dismiss
    revertToSsot: "用 SSOT 覆盖",
    reverting: "覆盖中...",
    revertedToSsot: "已用 SSOT 版本覆盖",
    failedRevert: "覆盖失败",
    dismissChange: "忽略此更改",
    dismissed: "已忽略",

    // Conflict diff
    comparingVersions: "版本对比",
  },
  en: {
    // Nav
    global: "Global",
    projects: "Projects",
    logs: "Logs",
    settings: "Settings",
    back: "Back",

    // Settings panel
    settingsTitle: "Settings",
    appearance: "Appearance",
    theme: "Theme",
    themeLight: "Light",
    themeDark: "Dark",
    themeSystem: "System",
    language: "Language",
    langZh: "Chinese",
    langEn: "English",
    syncMode: "Sync Mode",
    syncModeSemi: "Semi-Auto (manual sync)",
    syncModeFull: "Full-Auto (sync on change)",
    syncModeHint: "Semi-auto: scan detects changes, you click sync. Full-auto: scan triggers immediate sync.",
    preferSymlink: "Prefer symlinks over copies",
    symlinkHint: "Symlinks save disk space but require developer mode on Windows. Falls back to copy on failure.",
    saveSettings: "Save Settings",

    // Logs panel
    activityLogs: "Activity Logs",
    refresh: "Refresh",
    noLogs: "No operations recorded yet.",
    time: "Time",
    action: "Action",
    skill: "Skill",
    tool: "Tool",
    status: "Status",
    detail: "Detail",

    // Log actions
    actionScan: "Scan",
    actionSync: "Sync",
    actionEnable: "Enable",
    actionDisable: "Disable",
    actionRemove: "Remove",
    actionAddTool: "Add Tool",
    actionDeleteTool: "Delete Tool",
    actionAddProject: "Add Project",
    actionDeleteProject: "Delete Project",

    // Tool paths
    toolPaths: "Tool Paths",
    addTool: "+ Add Tool",
    cancel: "Cancel",
    templatePlaceholder: "Choose a template...",
    toolNamePlaceholder: "Tool name",
    globalPathPlaceholder: "Global path (e.g. ~/.mytool/skills/)",
    relPathPlaceholder: "Project rel path (e.g. .mytool/skills/)",
    add: "Add",
    edit: "Edit",
    save: "Save",
    globalPath: "Global path:",
    projectPath: "Project path:",

    // Discovery
    deleteProject: "Delete project",
    addAll: "Add All",
    adding: "Adding...",
    dismiss: "Dismiss",

    // Actions
    scanAll: "Scan All",
    scanning: "Scanning...",
    checkUpdates: "Check Updates",
    checking: "Checking...",
    syncAllActive: "Sync All Active",
    searchPlaceholder: "Search skills...",
    allTools: "All tools",
    sortName: "Name",
    sortUpdated: "Updated",
    sortCreated: "Created",
    ascending: "Ascending",
    descending: "Descending",

    // Skills section
    skills: "Skills",
    updates: "updates",
    noSkillsYet: "No skills discovered yet.",
    configureAndScan: 'Configure tool paths above and click "Scan All" to discover skills.',
    noMatch: "No matching skills found for",
    updateAvailable: "Update available",
    synced: "synced",
    syncNow: "Sync Now",
    syncingCard: "Syncing...",
    checkUpdate: "Check Update",
    checkingUpdate: "Checking...",
    inSync: "In sync",
    changedIn: "Changed in",

    // Project nav
    noProjects: "No projects yet. Add one to get started.",
    addProjectBtn: "+ Project",

    // Modals
    addProjectTitle: "Add Project",
    nameLabel: "Name:",
    pathLabel: "Absolute path:",
    projectNamePlaceholder: "My Project",
    projectPathPlaceholder: "D:\\my-project",

    // Scan result
    scanResults: "Scan Results",
    found: "Found",
    new: "new",
    updated: "updated",
    scope: "Scope",
    close: "Close",

    // Updates modal
    updatesAvailable: "Updates Available",
    viewDiff: "View Diff",
    loading: "Loading...",
    update: "Update",
    skip: "Skip",
    updateToSsot: "Update to SSOT",
    noChanges: "Files are in sync, no actual differences.",
    splitView: "Split",
    unifiedView: "Unified",
    maximize: "Maximize",
    restore: "Restore",
    viewDiffAll: "View All Diffs",
    sourceLabel: "Source",

    // Toast messages
    failedLoadTools: "Failed to load tools",
    failedLoadSkills: "Failed to load skills",
    failedLoadProjects: "Failed to load projects",
    failedLoadSettings: "Failed to load settings",
    failedLoadLogs: "Failed to load sync logs",
    settingsSaved: "Settings saved",
    failedSaveSettings: "Failed to save settings",
    scanComplete: "Scan complete",
    noSkillsFound: "no skills found",
    scanFailed: "Scan failed",
    allUpToDate: "All skills are up to date",
    checkUpdatesFailed: "Check updates failed",
    failedLoadDiff: "Failed to load diff",
    skillUpdated: "Skill updated to SSOT",
    syncFailed: "Sync failed",
    toggleFailed: "Toggle failed",
    syncedToTools: "Synced",
    tools: "tool(s)",
    syncComplete: "Sync complete",
    syncedCount: "synced",
    errors: "errors",
    syncAllFailed: "Sync all failed",
    globalPathNotEmpty: "Global path cannot be empty",
    globalPathMustBeAbsolute: "Global path must be absolute (start with /, ~/, drive letter, or \\\\)",
    pathUpdated: "Path updated",
    failedUpdate: "Failed to update",
    nameAndPathRequired: "Name and global path are required",
    relPathNoRelative: "Project relative path must not start with ./ or ../",
    toolAdded: "Tool added",
    failedAddTool: "Failed to add tool",
    confirmDeleteTool: "Delete tool",
    confirmDeleteToolMsg: "This will remove all related installations and sync logs.",
    toolDeleted: "Tool deleted",
    failedDeleteTool: "Failed to delete tool",
    nameAndPathRequiredProject: "Name and path are required",
    enterAbsolutePath: "Please enter an absolute path",
    projectAdded: "Project added",
    failedAddProject: "Failed to add project",
    confirmDeleteProject: "Delete project",
    confirmDeleteProjectMsg: "All related data will be removed.",
    projectDeleted: "Project deleted",
    failedDeleteProject: "Failed to delete project",
    addedTools: "Added",
    toolUnit: "tool(s)",

    // Conflicts (M5)
    conflicts: "Conflicts",
    conflictsTitle: "Skill Conflicts",
    conflictsDesc: "Different tools have different versions of the same skill. Choose which to keep.",
    noConflicts: "No conflicts detected.",
    resolveConflict: "Resolve",
    keepVersion: "Keep this version",
    resolving: "Resolving...",
    conflictResolved: "Conflict resolved",
    failedResolve: "Failed to resolve conflict",
    conflictDetected: "Conflict detected",

    // Timestamps (M7)
    lastSynced: "Last synced",
    ssotUpdated: "SSOT updated",
    never: "Never",
    syncStatus: "Sync status",
    statusInSync: "In sync",
    statusPending: "Pending",
    statusConflict: "Conflict",

    // Project edit
    editProject: "Edit project",
    editProjectTitle: "Edit Project",
    projectUpdated: "Project updated",
    failedUpdateProject: "Failed to update project",

    // Reverse sync & dismiss
    revertToSsot: "Overwrite from SSOT",
    reverting: "Overwriting...",
    revertedToSsot: "Overwritten with SSOT version",
    failedRevert: "Overwrite failed",
    dismissChange: "Dismiss this change",
    dismissed: "Dismissed",

    // Conflict diff
    comparingVersions: "Comparing Versions",
  },
};

type Tab = "global" | "projects";
type Panel = "main" | "settings" | "logs";

function App() {
  // Core data
  const [tools, setTools] = useState<Tool[]>([]);
  const [skills, setSkills] = useState<SkillView[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [settings, setSettings] = useState<Settings>({ sync_mode: "semi-auto", prefer_symlink: false, theme: "light", language: "zh" });

  // Translation helper
  const lang = settings.language as Lang;
  const t = (key: string): string => {
    return translations[lang]?.[key] || translations.en[key] || key;
  };

  // Apply theme to document
  useEffect(() => {
    const theme = settings.theme;
    let resolvedTheme = theme;
    if (theme === "system") {
      resolvedTheme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
      const mql = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = (e: MediaQueryListEvent) => {
        document.documentElement.setAttribute("data-theme", e.matches ? "dark" : "light");
        document.body.style.background = getComputedStyle(document.documentElement).getPropertyValue("--bg").trim();
      };
      mql.addEventListener("change", handler);
      return () => mql.removeEventListener("change", handler);
    }
    document.documentElement.setAttribute("data-theme", resolvedTheme);
    document.body.style.background = getComputedStyle(document.documentElement).getPropertyValue("--bg").trim();
  }, [settings.theme]);

  // Auto-save theme and language changes
  useEffect(() => {
    const timer = setTimeout(() => {
      invoke("update_settings", { newSettings: settings }).catch(() => {});
    }, 300);
    return () => clearTimeout(timer);
  }, [settings.theme, settings.language]);

  // UI state
  const [activeTab, setActiveTab] = useState<Tab>("global");
  const [activePanel, setActivePanel] = useState<Panel>("main");
  const [scanning, setScanning] = useState(false);
  const [syncing, setSyncing] = useState<Set<number>>(new Set());
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [checkingSingle, setCheckingSingle] = useState<number | null>(null);
  const [updates, setUpdates] = useState<SkillUpdate[]>([]);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [filterTool, setFilterTool] = useState<number>(-1); // -1 = all tools
  const [sortBy, setSortBy] = useState<"name" | "updated_at" | "created_at">("name");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
  const [selectedProject, setSelectedProject] = useState<number>(0);

  // Tool editing
  const [editingTool, setEditingTool] = useState<number | null>(null);
  const [editGlobalPath, setEditGlobalPath] = useState("");
  const [editRelPath, setEditRelPath] = useState("");
  const [showAddTool, setShowAddTool] = useState(false);
  const [newToolName, setNewToolName] = useState("");
  const [newToolGlobal, setNewToolGlobal] = useState("");
  const [newToolRel, setNewToolRel] = useState("");
  const [discoveredTools, setDiscoveredTools] = useState<ToolTemplate[]>([]);
  const [templates, setTemplates] = useState<ToolTemplate[]>([]);
  const [addingDiscovered, setAddingDiscovered] = useState(false);
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [showUpdatesModal, setShowUpdatesModal] = useState(false);
  const [selectedUpdateDiff, setSelectedUpdateDiff] = useState<{ update: SkillUpdate; diff: SkillDiff } | null>(null);
  const [loadingDiff, setLoadingDiff] = useState<number | null>(null);
  // P2-1/2/3: diff UX state
  const [diffMaximized, setDiffMaximized] = useState(false);
  const [diffSideBySide, setDiffSideBySide] = useState(true);
  const [selectedSkillDiffs, setSelectedSkillDiffs] = useState<{
    skillId: number;
    skillName: string;
    tools: { tool: string | null; toolId: number | null; sourcePath: string; diff: SkillDiff | null; loading: boolean }[];
  } | null>(null);
  const [expandedTools, setExpandedTools] = useState<Set<string>>(new Set());

  // Project editing
  const [showAddProject, setShowAddProject] = useState(false);
  const [newProjectName, setNewProjectName] = useState("");
  const [newProjectPath, setNewProjectPath] = useState("");
  const [editingProject, setEditingProject] = useState<Project | null>(null);
  const [editProjectName, setEditProjectName] = useState("");
  const [editProjectPath, setEditProjectPath] = useState("");

  // Sync logs
  const [syncLogs, setSyncLogs] = useState<SyncLog[]>([]);

  // Conflicts (M5)
  const [conflicts, setConflicts] = useState<ConflictView[]>([]);
  const [resolvingConflict, setResolvingConflict] = useState<number | null>(null);
  const [conflictDiff, setConflictDiff] = useState<{ conflictId: number; diff: SkillDiff; toolName: string } | null>(null);
  const [loadingConflictDiff, setLoadingConflictDiff] = useState<number | null>(null);
  const [conflictMaximized, setConflictMaximized] = useState(false);
  const [reversingSync, setReversingSync] = useState<number | null>(null);

  // Load data on mount
  useEffect(() => {
    loadTools();
    loadSkills();
    loadProjects();
    loadSettings();
    loadDiscovery();
    loadTemplates();
    loadConflicts();
  }, []);

  // Sync selectedProject when tab changes
  useEffect(() => {
    if (activeTab === "global") {
      setSelectedProject((prev) => (prev !== 0 ? 0 : prev));
    } else {
      // Projects tab: auto-select first project if none selected
      setSelectedProject((prev) => {
        if (prev === 0 && projects.length > 0) return projects[0].id;
        if (prev !== 0 && !projects.some((p) => p.id === prev)) {
          // Selected project was deleted, pick first available
          return projects.length > 0 ? projects[0].id : 0;
        }
        return prev;
      });
    }
  }, [activeTab, projects]);

  // Reload skills and conflicts when project changes
  useEffect(() => {
    loadSkills();
    loadConflicts();
  }, [selectedProject]);

  const addToast = useCallback((type: Toast["type"], message: string) => {
    const id = Date.now() + Math.random();
    setToasts((prev) => [...prev, { type, message, id }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 4000);
  }, []);

  // ==================== Data Loaders ====================

  async function loadTools() {
    try {
      setTools(await invoke<Tool[]>("list_tools"));
    } catch (e) {
      addToast("error", `${t("failedLoadTools")}: ${e}`);
    }
  }

  async function loadSkills() {
    try {
      setSkills(await invoke<SkillView[]>("list_skills", { projectId: selectedProject }));
    } catch (e) {
      addToast("error", `${t("failedLoadSkills")}: ${e}`);
    }
  }

  async function loadProjects() {
    try {
      const result = await invoke<Project[]>("list_projects");
      setProjects(result.filter((p) => p.id !== 0)); // Exclude Global from project list
    } catch (e) {
      addToast("error", `${t("failedLoadProjects")}: ${e}`);
    }
  }

  async function loadSettings() {
    try {
      setSettings(await invoke<Settings>("get_settings"));
    } catch (e) {
      addToast("error", `${t("failedLoadSettings")}: ${e}`);
    }
  }

  async function loadSyncLogs() {
    try {
      setSyncLogs(await invoke<SyncLog[]>("get_sync_logs", { skillId: null, limit: 50 }));
    } catch (e) {
      addToast("error", `${t("failedLoadLogs")}: ${e}`);
    }
  }

  async function loadDiscovery() {
    try {
      const found = await invoke<ToolTemplate[]>("discover_tools");
      setDiscoveredTools(found);
    } catch {
      // Silent fail — discovery is a nice-to-have
    }
  }

  async function loadTemplates() {
    try {
      setTemplates(await invoke<ToolTemplate[]>("list_tool_templates"));
    } catch {
      // Silent fail
    }
  }

  async function loadConflicts() {
    try {
      setConflicts(await invoke<ConflictView[]>("list_conflicts", { projectId: selectedProject }));
    } catch (e) {
      addToast("error", `${t("failedResolve")}: ${e}`);
    }
  }

  async function handleResolveConflict(conflictId: number, keepToolName: string) {
    setResolvingConflict(conflictId);
    try {
      const result = await invoke<SyncResult>("resolve_conflict", {
        conflictId,
        keepToolName,
        projectId: selectedProject,
      });
      if (result.errors.length > 0) {
        addToast("error", result.errors.join(", "));
      } else {
        addToast("success", t("conflictResolved"));
      }
      await loadConflicts();
      await loadSkills();
    } catch (e) {
      addToast("error", `${t("failedResolve")}: ${e}`);
    } finally {
      setResolvingConflict(null);
    }
  }

  async function handleAddDiscoveredAll() {
    if (discoveredTools.length === 0) return;
    setAddingDiscovered(true);
    let added = 0;
    for (const t of discoveredTools) {
      try {
        await invoke("add_tool", {
          name: t.name,
          globalPath: t.global_path,
          projectRelPath: t.project_rel_path,
        });
        added++;
      } catch {
        // Skip duplicates silently
      }
    }
    setDiscoveredTools([]);
    setAddingDiscovered(false);
    await loadTools();
    if (added > 0) {
      addToast("success", `${t("addedTools")} ${added} ${t("toolUnit")}`);
    }
  }

  function handleSelectTemplate(e: React.ChangeEvent<HTMLSelectElement>) {
    const idx = parseInt(e.target.value, 10);
    if (isNaN(idx) || idx < 0) {
      setNewToolName("");
      setNewToolGlobal("");
      setNewToolRel("");
      return;
    }
    const t = templates[idx];
    if (t) {
      setNewToolName(t.name);
      setNewToolGlobal(t.global_path);
      setNewToolRel(t.project_rel_path);
    }
  }

  // ==================== Actions ====================

  async function handleScan() {
    setScanning(true);
    try {
      let result: ScanResult;
      if (activeTab === "projects" && selectedProject !== 0) {
        // Project-level scan: scan project-specific paths
        result = await invoke<ScanResult>("scan_scope", {
          toolId: null,
          projectId: selectedProject,
        });
      } else {
        // Global scan
        result = await invoke<ScanResult>("full_scan");
      }
      await loadSkills();

      const parts: string[] = [];
      if (result.skills_found > 0) parts.push(`${t("found")} ${result.skills_found}`);
      if (result.skills_new > 0) parts.push(`${result.skills_new} ${t("new")}`);
      if (result.skills_updated > 0) parts.push(`${result.skills_updated} ${t("updated")}`);

      // Show modal if there are new or updated skills
      if (result.details.length > 0) {
        setScanResult(result);
      } else {
        addToast("success", `${t("scanComplete")}: ${parts.join(", ") || t("noSkillsFound")}`);
      }

      if (result.errors.length > 0) {
        result.errors.forEach((err) => addToast("error", err));
      }

      // Auto-check updates after scan
      if (result.skills_found > 0) {
        await handleCheckUpdates();
      }
    } catch (e) {
      addToast("error", `${t("scanFailed")}: ${e}`);
    } finally {
      setScanning(false);
    }
  }

  async function handleCheckUpdates() {
    setCheckingUpdates(true);
    try {
      const result = await invoke<SkillUpdate[]>("check_updates", { projectId: selectedProject });
      setUpdates(result);
      if (result.length > 0) {
        setShowUpdatesModal(true);
      } else {
        addToast("info", t("allUpToDate"));
      }
    } catch (e) {
      addToast("error", `${t("checkUpdatesFailed")}: ${e}`);
    } finally {
      setCheckingUpdates(false);
    }
  }

  async function handleCheckSingleSkill(skillId: number) {
    setCheckingSingle(skillId);
    try {
      const result = await invoke<SkillUpdate[]>("check_skill_update", { skillId, projectId: selectedProject });
      if (result.length > 0) {
        // Found updates — add all to updates list, show diff for the first one
        setUpdates((prev) => {
          const filtered = prev.filter((u) => u.skill_id !== skillId);
          return [...filtered, ...result];
        });
        const firstUpdate = result[0];
        const diff = await invoke<SkillDiff>("get_skill_diff", { skillId, sourcePath: firstUpdate.source_path });
        setSelectedUpdateDiff({ update: firstUpdate, diff });
        setShowUpdatesModal(true);
      } else {
        addToast("info", `${t("inSync")}`);
      }
    } catch (e) {
      addToast("error", `${t("failedLoadDiff")}: ${e}`);
    } finally {
      setCheckingSingle(null);
    }
  }

  async function handleViewDiff(update: SkillUpdate) {
    setLoadingDiff(update.skill_id);
    try {
      const diff = await invoke<SkillDiff>("get_skill_diff", { skillId: update.skill_id, sourcePath: update.source_path });
      setSelectedUpdateDiff({ update, diff });
    } catch (e) {
      addToast("error", `${t("failedLoadDiff")}: ${e}`);
    } finally {
      setLoadingDiff(null);
    }
  }

  // P2-3: open a skill's diff showing every divergent tool as an expandable block.
  async function handleViewSkillDiffs(skillId: number, skillName: string, skillUpdates: SkillUpdate[]) {
    const tools = skillUpdates.map((u) => ({
      tool: u.changed_tool,
      toolId: u.changed_tool_id,
      sourcePath: u.source_path,
      diff: null as SkillDiff | null,
      loading: true,
    }));
    setSelectedSkillDiffs({ skillId, skillName, tools });
    // Expand every tool block by default so the SSOT (left) view is visible for each.
    setExpandedTools(new Set(tools.map((_, i) => String(i))));
    setDiffMaximized(false);
    setDiffSideBySide(true);

    for (let i = 0; i < skillUpdates.length; i++) {
      const u = skillUpdates[i];
      try {
        const diff = await invoke<SkillDiff>("get_skill_diff", { skillId, sourcePath: u.source_path });
        setSelectedSkillDiffs((prev) =>
          prev ? { ...prev, tools: prev.tools.map((t, idx) => (idx === i ? { ...t, diff, loading: false } : t)) } : prev
        );
      } catch {
        setSelectedSkillDiffs((prev) =>
          prev ? { ...prev, tools: prev.tools.map((t, idx) => (idx === i ? { ...t, loading: false } : t)) } : prev
        );
      }
    }
  }

  function toggleToolBlock(key: string) {
    setExpandedTools((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  async function handleUpdateFromDiff(skillId: number, sourcePath?: string) {
    try {
      const result = await invoke<SyncResult>("sync_skill", {
        skillId,
        projectId: null,
        sourcePath: sourcePath ?? selectedUpdateDiff?.update.source_path ?? null,
      });
      if (result.errors.length > 0) {
        // Keep the update entry so the user can retry after fixing the cause
        addToast("error", `${t("syncFailed")}: ${result.errors.join(", ")}`);
        return;
      }
      addToast("success", t("skillUpdated"));
      closeUpdateEntry(skillId);
      await loadSkills();
    } catch (e) {
      addToast("error", `${t("syncFailed")}: ${e}`);
    }
  }

  function handleSkipUpdate(skillId: number) {
    closeUpdateEntry(skillId);
  }

  // Remove a skill's entries from the updates list, return to the list view,
  // and close the modal entirely once no updates remain.
  function closeUpdateEntry(skillId: number) {
    const remaining = updates.filter((u) => u.skill_id !== skillId);
    setUpdates(remaining);
    setSelectedUpdateDiff(null);
    setSelectedSkillDiffs(null);
    if (remaining.length === 0) {
      setShowUpdatesModal(false);
      setDiffMaximized(false);
    }
  }

  async function handleToggle(skillId: number, toolId: number, active: boolean) {
    try {
      await invoke("toggle_skill", {
        skillId,
        toolId,
        projectId: selectedProject,
        active,
      });
      await loadSkills();

      // Auto-sync when enabling (if full-auto mode)
      if (active && settings.sync_mode === "full-auto") {
        await handleSyncSkill(skillId);
      }
    } catch (e) {
      addToast("error", `${t("toggleFailed")}: ${e}`);
    }
  }

  async function handleSyncSkill(skillId: number) {
    setSyncing((prev) => new Set(prev).add(skillId));
    try {
      const result = await invoke<SyncResult>("sync_skill", { skillId, projectId: selectedProject });
      await loadSkills();
      await handleCheckUpdates();

      if (result.errors.length > 0) {
        addToast("error", `${result.skill_name}: ${result.errors.join(", ")}`);
      } else {
        addToast("success", `${t("syncedToTools")} ${result.skill_name} → ${result.synced_to} ${t("tools")}`);
      }
    } catch (e) {
      addToast("error", `${t("syncFailed")}: ${e}`);
    } finally {
      setSyncing((prev) => {
        const next = new Set(prev);
        next.delete(skillId);
        return next;
      });
    }
  }

  async function handleSyncAll() {
    setScanning(true);
    try {
      const results = await invoke<SyncResult[]>("sync_all_pending");
      await loadSkills();
      await handleCheckUpdates();

      const totalSynced = results.reduce((sum, r) => sum + r.synced_to, 0);
      const totalErrors = results.reduce((sum, r) => sum + r.errors.length, 0);

      addToast(
        totalErrors > 0 ? "error" : "success",
        `${t("syncComplete")}: ${totalSynced} ${t("syncedCount")}, ${totalErrors} ${t("errors")}`,
      );
    } catch (e) {
      addToast("error", `${t("syncAllFailed")}: ${e}`);
    } finally {
      setScanning(false);
    }
  }

  async function handleSaveSettings() {
    try {
      await invoke("update_settings", { newSettings: settings });
      addToast("success", t("settingsSaved"));
    } catch (e) {
      addToast("error", `${t("failedSaveSettings")}: ${e}`);
    }
  }

  // ==================== Tool CRUD ====================

  function startEdit(tool: Tool) {
    setEditingTool(tool.id);
    setEditGlobalPath(tool.global_path);
    setEditRelPath(tool.project_rel_path);
  }

  async function saveEdit(toolId: number) {
    if (!editGlobalPath.trim()) {
      addToast("error", t("globalPathNotEmpty"));
      return;
    }
    const gPath = editGlobalPath.trim();
    if (
      !gPath.startsWith("/") &&
      !gPath.startsWith("~") &&
      !/^[A-Za-z]:[\\/]/.test(gPath) &&
      !gPath.startsWith("\\\\")
    ) {
      addToast("error", t("globalPathMustBeAbsolute"));
      return;
    }
    try {
      await invoke("update_tool_path", {
        toolId,
        globalPath: gPath,
        projectRelPath: editRelPath.trim(),
      });
      setEditingTool(null);
      await loadTools();
      addToast("success", t("pathUpdated"));
    } catch (e) {
      addToast("error", `${t("failedUpdate")}: ${e}`);
    }
  }

  async function handleAddTool() {
    if (!newToolName.trim() || !newToolGlobal.trim()) {
      addToast("error", t("nameAndPathRequired"));
      return;
    }
    // Validate global path is absolute
    const gPath = newToolGlobal.trim();
    if (
      !gPath.startsWith("/") &&
      !gPath.startsWith("~") &&
      !/^[A-Za-z]:[\\/]/.test(gPath) &&
      !gPath.startsWith("\\\\")
    ) {
      addToast("error", t("globalPathMustBeAbsolute"));
      return;
    }
    // Validate relative path doesn't start with .
    const rPath = newToolRel.trim();
    if (rPath.startsWith("./") || rPath.startsWith("../") || rPath === ".") {
      addToast("error", t("relPathNoRelative"));
      return;
    }
    try {
      await invoke("add_tool", {
        name: newToolName.trim(),
        globalPath: gPath,
        projectRelPath: rPath,
      });
      setShowAddTool(false);
      setNewToolName("");
      setNewToolGlobal("");
      setNewToolRel("");
      await loadTools();
      addToast("success", `${t("toolAdded")} "${newToolName.trim()}"`);
    } catch (e) {
      addToast("error", `${t("failedAddTool")}: ${e}`);
    }
  }

  async function handleDeleteTool(toolId: number, toolName: string) {
    if (!confirm(`${t("confirmDeleteTool")} "${toolName}"?\n${t("confirmDeleteToolMsg")}`)) return;
    try {
      // Optimistic update: immediately remove tool from skills and tool list
      setSkills((prev) =>
        prev.map((s) => {
          const filtered = s.installed_tools.filter((inst) => inst.tool_id !== toolId);
          return { ...s, installed_tools: filtered, install_count: filtered.length };
        })
      );
      setTools((prev) => prev.filter((tool) => tool.id !== toolId));
      if (editingTool === toolId) {
        setEditingTool(null);
      }

      await invoke("delete_tool", { toolId, toolName });
      // Reload to get authoritative state from backend
      await Promise.all([loadTools(), loadSkills()]);
      addToast("success", `${t("toolDeleted")} "${toolName}"`);
    } catch (e) {
      // Rollback on failure: reload everything
      await Promise.all([loadTools(), loadSkills()]);
      addToast("error", `${t("failedDeleteTool")}: ${e}`);
    }
  }

  // ==================== Project CRUD ====================

  async function handleAddProject() {
    if (!newProjectName.trim() || !newProjectPath.trim()) {
      addToast("error", t("nameAndPathRequiredProject"));
      return;
    }
    if (newProjectPath.startsWith("./") || newProjectPath.startsWith("../")) {
      addToast("error", t("enterAbsolutePath"));
      return;
    }
    try {
      await invoke("add_project", { name: newProjectName, path: newProjectPath });
      setShowAddProject(false);
      setNewProjectName("");
      setNewProjectPath("");
      await loadProjects();
      addToast("success", `${t("projectAdded")} "${newProjectName}"`);
    } catch (e) {
      addToast("error", `${t("failedAddProject")}: ${e}`);
    }
  }

  async function handleDeleteProject(projectId: number, projectName: string) {
    if (!confirm(`${t("confirmDeleteProject")} "${projectName}"?\n${t("confirmDeleteProjectMsg")}`)) return;
    try {
      await invoke("delete_project", { projectId });
      if (selectedProject === projectId) {
        // Auto-select next project or fall back to global
        const remaining = projects.filter((p) => p.id !== projectId);
        setSelectedProject(remaining.length > 0 ? remaining[0].id : 0);
      }
      await loadProjects();
      addToast("success", `${t("projectDeleted")} "${projectName}"`);
    } catch (e) {
      addToast("error", `${t("failedDeleteProject")}: ${e}`);
    }
  }

  function handleEditProject(project: Project) {
    setEditingProject(project);
    setEditProjectName(project.name);
    setEditProjectPath(project.path);
  }

  async function handleSaveProjectEdit() {
    if (!editingProject) return;
    if (!editProjectName.trim() || !editProjectPath.trim()) {
      addToast("error", t("nameAndPathRequiredProject"));
      return;
    }
    try {
      await invoke("update_project", {
        projectId: editingProject.id,
        name: editProjectName.trim(),
        path: editProjectPath.trim(),
      });
      setEditingProject(null);
      await loadProjects();
      addToast("success", t("projectUpdated"));
    } catch (e) {
      addToast("error", `${t("failedUpdateProject")}: ${e}`);
    }
  }

  async function handleReverseSync(skillId: number, toolId: number) {
    setReversingSync(skillId);
    try {
      const result = await invoke<SyncResult>("reverse_sync_skill", {
        skillId,
        toolId,
        projectId: selectedProject,
      });
      if (result.errors.length > 0) {
        addToast("error", result.errors.join(", "));
      } else {
        addToast("success", t("revertedToSsot"));
      }
      closeUpdateEntry(skillId);
      await loadSkills();
    } catch (e) {
      addToast("error", `${t("failedRevert")}: ${e}`);
    } finally {
      setReversingSync(null);
    }
  }

  async function handleDismissUpdate(update: SkillUpdate) {
    if (!update.changed_tool_id) return;
    try {
      await invoke("dismiss_skill_update", {
        skillId: update.skill_id,
        toolId: update.changed_tool_id,
        currentHash: update.new_hash,
      });
      closeUpdateEntry(update.skill_id);
      addToast("info", t("dismissed"));
    } catch (e) {
      addToast("error", `${t("failedRevert")}: ${e}`);
    }
  }

  async function handleViewConflictDiff(conflictId: number, version: { tool_id: number; tool_name: string; source_path: string }) {
    setLoadingConflictDiff(conflictId);
    try {
      const diff = await invoke<SkillDiff>("get_skill_diff", {
        skillId: conflictId,
        sourcePath: version.source_path,
      });
      setConflictDiff({ conflictId, diff, toolName: version.tool_name });
    } catch (e) {
      addToast("error", `${t("failedLoadDiff")}: ${e}`);
    } finally {
      setLoadingConflictDiff(null);
    }
  }

  // ==================== Render Helpers ====================

  function getInstallStatus(skill: SkillView, toolId: number): InstallationInfo | undefined {
    return skill.installed_tools.find((inst) => inst.tool_id === toolId);
  }

  function hasUpdate(skill: SkillView): boolean {
    return updates.some((u) => u.skill_id === skill.id);
  }

  const filteredSkills = (() => {
    const byTool = filterTool >= 0
      ? skills.filter((s) =>
          s.installed_tools.some((inst) => inst.tool_id === filterTool && inst.status === "active"),
        )
      : skills;
    const filtered = searchQuery
      ? byTool.filter(
          (s) =>
            s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            (s.description && s.description.toLowerCase().includes(searchQuery.toLowerCase())),
        )
      : [...byTool];
    filtered.sort((a, b) => {
      let cmp = 0;
      if (sortBy === "name") {
        cmp = a.name.localeCompare(b.name);
      } else if (sortBy === "updated_at") {
        cmp = a.updated_at.localeCompare(b.updated_at);
      } else {
        cmp = a.created_at.localeCompare(b.created_at);
      }
      return sortDir === "desc" ? -cmp : cmp;
    });
    return filtered;
  })();

  // ==================== Panels ====================

  function renderSettingsPanel() {
    return (
      <section className="section">
        <div className="panel-header">
          <h2 className="section-title">{t("settingsTitle")}</h2>
          <button className="btn btn-secondary" onClick={() => setActivePanel("main")}>
            {t("back")}
          </button>
        </div>

        <div className="settings-group">
          <label className="settings-label">{t("appearance")}</label>
          <div className="settings-row">
            <div className="settings-field">
              <span className="settings-field-label">{t("theme")}</span>
              <select
                className="settings-select"
                value={settings.theme}
                onChange={(e) => setSettings({ ...settings, theme: e.target.value })}
              >
                <option value="light">{t("themeLight")}</option>
                <option value="dark">{t("themeDark")}</option>
                <option value="system">{t("themeSystem")}</option>
              </select>
            </div>
            <div className="settings-field">
              <span className="settings-field-label">{t("language")}</span>
              <select
                className="settings-select"
                value={settings.language}
                onChange={(e) => setSettings({ ...settings, language: e.target.value })}
              >
                <option value="zh">{t("langZh")}</option>
                <option value="en">{t("langEn")}</option>
              </select>
            </div>
          </div>
        </div>

        <div className="settings-group">
          <label className="settings-label">{t("syncMode")}</label>
          <select
            className="settings-select"
            value={settings.sync_mode}
            onChange={(e) => setSettings({ ...settings, sync_mode: e.target.value })}
          >
            <option value="semi-auto">{t("syncModeSemi")}</option>
            <option value="full-auto">{t("syncModeFull")}</option>
          </select>
          <p className="settings-hint">
            {t("syncModeHint")}
          </p>
        </div>

        <div className="settings-group">
          <label className="settings-label">
            <input
              type="checkbox"
              checked={settings.prefer_symlink}
              onChange={(e) => setSettings({ ...settings, prefer_symlink: e.target.checked })}
            />
            {" "}{t("preferSymlink")}
          </label>
          <p className="settings-hint">
            {t("symlinkHint")}
          </p>
        </div>

        <button className="btn btn-primary" onClick={handleSaveSettings}>
          {t("saveSettings")}
        </button>
      </section>
    );
  }

  function renderLogsPanel() {
    const actionLabels: Record<string, string> = {
      scan: t("actionScan"),
      sync: t("actionSync"),
      remove: t("actionRemove"),
      toggle_on: t("actionEnable"),
      toggle_off: t("actionDisable"),
      add_tool: t("actionAddTool"),
      delete_tool: t("actionDeleteTool"),
      add_project: t("actionAddProject"),
      delete_project: t("actionDeleteProject"),
    };

    return (
      <section className="section">
        <div className="panel-header">
          <h2 className="section-title">{t("activityLogs")}</h2>
          <div className="panel-actions">
            <button className="btn btn-small" onClick={loadSyncLogs}>{t("refresh")}</button>
            <button className="btn btn-secondary" onClick={() => setActivePanel("main")}>{t("back")}</button>
          </div>
        </div>

        {syncLogs.length === 0 ? (
          <div className="empty-state">
            <p>{t("noLogs")}</p>
          </div>
        ) : (
          <div className="log-table-wrapper">
            <table className="log-table">
              <thead>
                <tr>
                  <th>{t("time")}</th>
                  <th>{t("action")}</th>
                  <th>{t("skill")}</th>
                  <th>{t("tool")}</th>
                  <th>{t("status")}</th>
                  <th>{t("detail")}</th>
                </tr>
              </thead>
              <tbody>
                {syncLogs.map((log) => (
                  <tr key={log.id} className={`log-row log-${log.status}`}>
                    <td className="log-time">{log.created_at}</td>
                    <td>
                      <span className={`action-badge action-${log.action}`}>
                        {actionLabels[log.action] || log.action}
                      </span>
                      {log.direction && (
                        <span className={`direction-badge dir-${log.direction}`}>
                          {log.direction === "to_ssot" ? "→ SSOT" : "← SSOT"}
                        </span>
                      )}
                    </td>
                    <td>{log.skill_name || "—"}</td>
                    <td>{log.tool_name || (log.action === "delete_tool" || log.action === "add_tool" ? log.detail : "—")}</td>
                    <td>
                      <span className={`status-badge status-${log.status}`}>
                        {log.status}
                      </span>
                    </td>
                    <td className="log-error">{log.detail || "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    );
  }

  // ==================== Main Render ====================

  if (activePanel === "settings") return (
    <main className="container">
      <ToastContainer toasts={toasts} />
      {renderSettingsPanel()}
    </main>
  );

  if (activePanel === "logs") return (
    <main className="container">
      <ToastContainer toasts={toasts} />
      {renderLogsPanel()}
    </main>
  );

  return (
    <main className="container">
      <ToastContainer toasts={toasts} />

      {/* App header */}
      <header className="app-header">
        <div className="brand">
          <span className="brand-icon">⬡</span>
          <span className="brand-name">Skill Manager</span>
        </div>
      </header>

      {/* Top navigation */}
      <nav className="top-nav">
        <div className="tabs">
          <button
            className={`tab ${activeTab === "global" ? "tab-active" : ""}`}
            onClick={() => setActiveTab("global")}
          >
            {t("global")}
          </button>
          <button
            className={`tab ${activeTab === "projects" ? "tab-active" : ""}`}
            onClick={() => setActiveTab("projects")}
          >
            {t("projects")}
          </button>
        </div>
        <div className="nav-actions">
          <button className="btn btn-small btn-ghost" onClick={() => { loadSyncLogs(); setActivePanel("logs"); }}>
            {t("logs")}
          </button>
          <button className="btn btn-small btn-ghost" onClick={() => setActivePanel("settings")}>
            {t("settings")}
          </button>
        </div>
      </nav>

      {/* Project sub-navigation */}
      {activeTab === "projects" && (
        <div className="project-nav">
          {projects.length === 0 && (
            <span className="project-empty-hint">{t("noProjects")}</span>
          )}
          {projects.map((p) => (
            <div key={p.id} className="project-btn-group">
              <button
                className={`project-btn ${selectedProject === p.id ? "project-active" : ""}`}
                onClick={() => setSelectedProject(p.id)}
              >
                {p.name}
              </button>
              <button
                className="project-edit"
                onClick={() => handleEditProject(p)}
                title={t("editProject")}
              >
                ✎
              </button>
              <button
                className="project-delete"
                onClick={() => handleDeleteProject(p.id, p.name)}
                title={t("deleteProject")}
              >
                ×
              </button>
            </div>
          ))}
          <button
            className="btn btn-small btn-primary"
            onClick={() => setShowAddProject(true)}
          >
            {t("addProjectBtn")}
          </button>
        </div>
      )}

      {/* Add project dialog */}
      {showAddProject && (
        <div className="modal-overlay" onClick={() => setShowAddProject(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h3>{t("addProjectTitle")}</h3>
            <div className="form-group">
              <label>{t("nameLabel")}</label>
              <input
                type="text"
                value={newProjectName}
                onChange={(e) => setNewProjectName(e.target.value)}
                className="edit-input"
                placeholder={t("projectNamePlaceholder")}
              />
            </div>
            <div className="form-group">
              <label>{t("pathLabel")}</label>
              <input
                type="text"
                value={newProjectPath}
                onChange={(e) => setNewProjectPath(e.target.value)}
                className="edit-input"
                placeholder={t("projectPathPlaceholder")}
              />
            </div>
            <div className="modal-actions">
              <button className="btn btn-primary" onClick={handleAddProject}>{t("add")}</button>
              <button className="btn btn-secondary" onClick={() => setShowAddProject(false)}>{t("cancel")}</button>
            </div>
          </div>
        </div>
      )}

      {/* Edit project dialog */}
      {editingProject && (
        <div className="modal-overlay" onClick={() => setEditingProject(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h3>{t("editProjectTitle")}</h3>
            <div className="form-group">
              <label>{t("nameLabel")}</label>
              <input
                type="text"
                value={editProjectName}
                onChange={(e) => setEditProjectName(e.target.value)}
                className="edit-input"
              />
            </div>
            <div className="form-group">
              <label>{t("pathLabel")}</label>
              <input
                type="text"
                value={editProjectPath}
                onChange={(e) => setEditProjectPath(e.target.value)}
                className="edit-input"
              />
            </div>
            <div className="modal-actions">
              <button className="btn btn-primary" onClick={handleSaveProjectEdit}>{t("save")}</button>
              <button className="btn btn-secondary" onClick={() => setEditingProject(null)}>{t("cancel")}</button>
            </div>
          </div>
        </div>
      )}

      {/* Tool configuration */}
      <section className="section">
        <div className="section-header">
          <h2 className="section-title">{t("toolPaths")}</h2>
          <button className="btn btn-small" onClick={() => setShowAddTool(!showAddTool)}>
            {showAddTool ? t("cancel") : t("addTool")}
          </button>
        </div>

        {discoveredTools.length > 0 && (
          <div className="discovery-banner">
            <span className="discovery-text">
              {t("found")} {discoveredTools.length} {t("toolUnit")}：{" "}
              {discoveredTools.map((dt) => dt.name).join(", ")}
            </span>
            <button
              className="btn btn-primary btn-small"
              onClick={handleAddDiscoveredAll}
              disabled={addingDiscovered}
            >
              {addingDiscovered ? t("adding") : t("addAll")}
            </button>
            <button
              className="btn btn-small"
              onClick={() => setDiscoveredTools([])}
            >
              {t("dismiss")}
            </button>
          </div>
        )}

        {showAddTool && (
          <div className="add-tool-form">
            {templates.length > 0 && (
              <div className="form-row">
                <select
                  className="template-select"
                  defaultValue=""
                  onChange={handleSelectTemplate}
                >
                  <option value="" disabled>{t("templatePlaceholder")}</option>
                  {templates.map((tp, i) => (
                    <option key={tp.name} value={i}>{tp.name}</option>
                  ))}
                </select>
              </div>
            )}
            <div className="form-row">
              <input type="text" value={newToolName} onChange={(e) => setNewToolName(e.target.value)}
                className="edit-input" placeholder={t("toolNamePlaceholder")} />
              <input type="text" value={newToolGlobal} onChange={(e) => setNewToolGlobal(e.target.value)}
                className="edit-input" placeholder={t("globalPathPlaceholder")} />
              <input type="text" value={newToolRel} onChange={(e) => setNewToolRel(e.target.value)}
                className="edit-input" placeholder={t("relPathPlaceholder")} />
              <button className="btn btn-primary btn-small" onClick={handleAddTool}>{t("add")}</button>
            </div>
          </div>
        )}

        <div className="tool-list">
          {tools.map((tool) => (
            <div key={tool.id} className="tool-item">
              <div className="tool-header">
                <span className="tool-name">{tool.name}</span>
                <div className="tool-actions">
                  <button className="btn btn-small" onClick={() => startEdit(tool)}>{t("edit")}</button>
                  <button className="btn btn-small btn-danger" onClick={() => handleDeleteTool(tool.id, tool.name)}>×</button>
                </div>
              </div>
              {editingTool === tool.id ? (
                <div className="tool-edit">
                  <div className="edit-row">
                    <label>{t("globalPath")}</label>
                    <input type="text" value={editGlobalPath} onChange={(e) => setEditGlobalPath(e.target.value)}
                      className="edit-input" />
                  </div>
                  <div className="edit-row">
                    <label>{t("projectPath")}</label>
                    <input type="text" value={editRelPath} onChange={(e) => setEditRelPath(e.target.value)}
                      className="edit-input" />
                  </div>
                  <div className="edit-actions">
                    <button onClick={() => saveEdit(tool.id)} className="btn btn-primary btn-small">{t("save")}</button>
                    <button onClick={() => setEditingTool(null)} className="btn btn-secondary btn-small">{t("cancel")}</button>
                  </div>
                </div>
              ) : (
                <code className="path-text">{tool.global_path}</code>
              )}
            </div>
          ))}
        </div>
      </section>

      {/* Action bar */}
      <section className="section">
        <div className="action-bar">
          <button className="btn btn-primary" onClick={handleScan} disabled={scanning}>
            {scanning ? t("scanning") : t("scanAll")}
          </button>
          <button className="btn btn-secondary" onClick={handleCheckUpdates} disabled={checkingUpdates}>
            {checkingUpdates ? t("checking") : t("checkUpdates")}
            {updates.length > 0 && <span className="update-badge">{updates.length}</span>}
          </button>
          <button className="btn btn-secondary" onClick={handleSyncAll} disabled={scanning}>
            {t("syncAllActive")}
          </button>
          <div className="search-box">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t("searchPlaceholder")}
              className="search-input"
            />
          </div>
          <select
            className="sort-select"
            value={filterTool}
            onChange={(e) => setFilterTool(parseInt(e.target.value, 10))}
          >
            <option value={-1}>{t("allTools")}</option>
            {tools.map((tool) => (
              <option key={tool.id} value={tool.id}>{tool.name}</option>
            ))}
          </select>
          <select
            className="sort-select"
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as typeof sortBy)}
          >
            <option value="name">{t("sortName")}</option>
            <option value="updated_at">{t("sortUpdated")}</option>
            <option value="created_at">{t("sortCreated")}</option>
          </select>
          <button
            className="btn btn-small sort-dir-btn"
            onClick={() => setSortDir((d) => (d === "asc" ? "desc" : "asc"))}
            title={sortDir === "asc" ? t("ascending") : t("descending")}
          >
            {sortDir === "asc" ? "A\u2192Z" : "Z\u2192A"}
          </button>
        </div>
      </section>

      {/* Conflict Banner (M5) */}
      {conflicts.length > 0 && (
        <section className="section">
          <div className="conflict-banner">
            <div className="conflict-header">
              <span className="conflict-icon">⚠</span>
              <span className="conflict-title">{t("conflictsTitle")}</span>
              <span className="badge badge-conflict">{conflicts.length}</span>
            </div>
            <p className="conflict-desc">{t("conflictsDesc")}</p>
            <div className="conflict-list">
              {conflicts.map((c) => (
                <div key={c.id} className="conflict-item">
                  <span className="conflict-skill-name">{c.skill_name}</span>
                  <div className="conflict-versions">
                    {c.versions.map((v) => (
                      <div key={v.tool_id} className="conflict-version-group">
                        <button
                          className={`btn btn-small ${resolvingConflict === c.id ? "" : "btn-secondary"}`}
                          disabled={resolvingConflict !== null}
                          onClick={() => handleResolveConflict(c.id, v.tool_name)}
                        >
                          {resolvingConflict === c.id ? t("resolving") : `${t("keepVersion")}: ${v.tool_name}`}
                        </button>
                        <button
                          className="btn btn-small"
                          disabled={loadingConflictDiff === c.id}
                          onClick={() => handleViewConflictDiff(c.id, v)}
                        >
                          {loadingConflictDiff === c.id ? t("loading") : t("viewDiff")}
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>
      )}

      {/* Conflict Diff Modal */}
      {conflictDiff && (
        <div className="modal-overlay" onClick={() => { setConflictDiff(null); setConflictMaximized(false); }}>
          <div className={`modal updates-modal${conflictMaximized ? " maximized" : ""}`} onClick={(e) => e.stopPropagation()}>
            <div className="diff-header">
              <button className="btn btn-small" onClick={() => setConflictDiff(null)}>&larr; {t("back")}</button>
              <h3 className="diff-title">{t("comparingVersions")}: {conflictDiff.toolName}</h3>
              <DiffViewControls
                t={t}
                sideBySide={diffSideBySide}
                onToggleSideBySide={() => setDiffSideBySide((v) => !v)}
                maximized={conflictMaximized}
                onToggleMaximize={() => setConflictMaximized((v) => !v)}
              />
            </div>
            <div className="diff-meta">
              <span className="diff-path" title={conflictDiff.diff.source_path}>
                source: {conflictDiff.diff.source_path}
              </span>
              <span className="diff-path" title={conflictDiff.diff.ssot_path}>
                ssot: {conflictDiff.diff.ssot_path}
              </span>
            </div>
            <DiffFilesView diff={conflictDiff.diff} sideBySide={diffSideBySide} noChangesLabel={t("noChanges")} />
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={() => { setConflictDiff(null); setConflictMaximized(false); }}>{t("close")}</button>
            </div>
          </div>
        </div>
      )}

      {/* Skill grid */}
      <section className="section">
        <h2 className="section-title">
          {t("skills")} {filteredSkills.length > 0 && <span className="badge">{filteredSkills.length}</span>}
          {updates.length > 0 && <span className="badge badge-update">{updates.length} {t("updates")}</span>}
        </h2>

        {scanning && skills.length === 0 ? (
          <div className="skeleton-grid">
            {[1, 2, 3].map((i) => (
              <div key={i} className="skeleton-card" />
            ))}
          </div>
        ) : filteredSkills.length === 0 ? (
          <div className="empty-state">
            {searchQuery || filterTool >= 0 ? (
              <p>{t("noMatch")} "{searchQuery}"</p>
            ) : (
              <>
                <p>{t("noSkillsYet")}</p>
                <p>{t("configureAndScan")}</p>
              </>
            )}
          </div>
        ) : (
          <div className="skill-grid">
            {filteredSkills.map((skill) => (
              <div key={skill.id} className={`skill-card ${hasUpdate(skill) ? "skill-has-update" : ""}`}>
                <div className="skill-header">
                  <h3 className="skill-name">{skill.name}</h3>
                  {hasUpdate(skill) && <span className="update-indicator" title={t("updateAvailable")}>●</span>}
                  {syncing.has(skill.id) && <span className="sync-spinner">⟳</span>}
                </div>

                {skill.description && <p className="skill-desc">{skill.description}</p>}

                <code className="skill-path">{skill.source_path}</code>

                {/* Timestamp info (M7) */}
                <div className="skill-timestamps">
                  <span className="timestamp-row">
                    <span className="timestamp-label">{t("ssotUpdated")}:</span>
                    <span className="timestamp-value">{skill.ssot_updated_at || t("never")}</span>
                  </span>
                </div>

                {/* Tool toggles */}
                <div className="skill-tools">
                  {tools.map((tool) => {
                    const inst = getInstallStatus(skill, tool.id);
                    const isActive = inst?.status === "active";
                    return (
                      <div key={tool.id} className="skill-tool-row">
                        <label className="toggle-label">
                          <input
                            type="checkbox"
                            checked={isActive}
                            onChange={(e) => handleToggle(skill.id, tool.id, e.target.checked)}
                          />
                          <span className="toggle-text">{tool.name}</span>
                        </label>
                        {inst?.synced_at && (
                          <span className="sync-time" title={inst.synced_at}>
                            {t("synced")}
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>

                {/* Action buttons */}
                <div className="skill-actions">
                  <button
                    className="btn btn-small"
                    onClick={() => handleCheckSingleSkill(skill.id)}
                    disabled={checkingSingle === skill.id}
                  >
                    {checkingSingle === skill.id ? t("checkingUpdate") : t("checkUpdate")}
                  </button>
                  <button
                    className="btn btn-small btn-primary"
                    onClick={() => handleSyncSkill(skill.id)}
                    disabled={syncing.has(skill.id)}
                  >
                    {syncing.has(skill.id) ? t("syncingCard") : t("syncNow")}
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      {/* Scan Result Modal */}
      {scanResult && (
        <div className="modal-overlay" onClick={() => setScanResult(null)}>
          <div className="modal scan-result-modal" onClick={(e) => e.stopPropagation()}>
            <h3>{t("scanResults")}</h3>
            <div className="scan-summary">
              <span className="scan-summary-item">
                {t("found")} <strong>{scanResult.skills_found}</strong>
              </span>
              {scanResult.skills_new > 0 && (
                <span className="scan-summary-item scan-new">
                  {scanResult.skills_new} {t("new")}
                </span>
              )}
              {scanResult.skills_updated > 0 && (
                <span className="scan-summary-item scan-updated">
                  {scanResult.skills_updated} {t("updated")}
                </span>
              )}
            </div>
            <div className="scan-detail-table-wrapper">
              <table className="scan-detail-table">
                <thead>
                  <tr>
                    <th>{t("skill")}</th>
                    <th>{t("tool")}</th>
                    <th>{t("scope")}</th>
                    <th>{t("status")}</th>
                  </tr>
                </thead>
                <tbody>
                  {scanResult.details.map((d, i) => (
                    <tr key={i}>
                      <td className="scan-skill-name" title={d.source_path}>{d.skill_name}</td>
                      <td className="scan-tool-name">{d.tool_name}</td>
                      <td className="scan-scope">{d.scope}</td>
                      <td>
                        <span className={`scan-status scan-status-${d.status}`}>
                          {d.status}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="modal-actions">
              <button className="btn btn-primary" onClick={() => setScanResult(null)}>{t("close")}</button>
            </div>
          </div>
        </div>
      )}

      {/* Updates Modal */}
      {showUpdatesModal && (
        <div className="modal-overlay" onClick={() => { setShowUpdatesModal(false); setSelectedUpdateDiff(null); setSelectedSkillDiffs(null); setDiffMaximized(false); }}>
          <div className={`modal updates-modal${diffMaximized ? " maximized" : ""}`} onClick={(e) => e.stopPropagation()}>
            {selectedUpdateDiff ? (
              <>
                <div className="diff-header">
                  <button className="btn btn-small" onClick={() => setSelectedUpdateDiff(null)}>&larr; {t("back")}</button>
                  <h3 className="diff-title">{selectedUpdateDiff.update.skill_name}</h3>
                  {selectedUpdateDiff.update.changed_tool && (
                    <span className="diff-tool-badge">{selectedUpdateDiff.update.changed_tool}</span>
                  )}
                  <DiffViewControls
                    t={t}
                    sideBySide={diffSideBySide}
                    onToggleSideBySide={() => setDiffSideBySide((v) => !v)}
                    maximized={diffMaximized}
                    onToggleMaximize={() => setDiffMaximized((v) => !v)}
                  />
                </div>
                <div className="diff-meta">
                  <span className="diff-path" title={selectedUpdateDiff.diff.source_path}>
                    source: {selectedUpdateDiff.diff.source_path}
                  </span>
                  <span className="diff-path" title={selectedUpdateDiff.diff.ssot_path}>
                    ssot: {selectedUpdateDiff.diff.ssot_path}
                  </span>
                </div>
                <DiffFilesView diff={selectedUpdateDiff.diff} sideBySide={diffSideBySide} noChangesLabel={t("noChanges")} />
                <div className="modal-actions">
                  <button
                    className="btn btn-primary"
                    onClick={() => handleUpdateFromDiff(selectedUpdateDiff.update.skill_id, selectedUpdateDiff.update.source_path)}
                  >
                    {t("updateToSsot")}
                  </button>
                  {selectedUpdateDiff.update.changed_tool_id && (
                    <button
                      className="btn btn-secondary"
                      disabled={reversingSync === selectedUpdateDiff.update.skill_id}
                      onClick={() => handleReverseSync(selectedUpdateDiff.update.skill_id, selectedUpdateDiff.update.changed_tool_id!)}
                    >
                      {reversingSync === selectedUpdateDiff.update.skill_id ? t("reverting") : t("revertToSsot")}
                    </button>
                  )}
                  <button
                    className="btn btn-secondary"
                    onClick={() => handleSkipUpdate(selectedUpdateDiff.update.skill_id)}
                  >
                    {t("skip")}
                  </button>
                  {selectedUpdateDiff.update.changed_tool_id && (
                    <button
                      className="btn btn-secondary"
                      onClick={() => handleDismissUpdate(selectedUpdateDiff.update)}
                    >
                      {t("dismissChange")}
                    </button>
                  )}
                </div>
              </>
            ) : selectedSkillDiffs ? (
              <>
                <div className="diff-header">
                  <button className="btn btn-small" onClick={() => setSelectedSkillDiffs(null)}>&larr; {t("back")}</button>
                  <h3 className="diff-title">{selectedSkillDiffs.skillName}</h3>
                  <DiffViewControls
                    t={t}
                    sideBySide={diffSideBySide}
                    onToggleSideBySide={() => setDiffSideBySide((v) => !v)}
                    maximized={diffMaximized}
                    onToggleMaximize={() => setDiffMaximized((v) => !v)}
                  />
                </div>
                <div className="diff-meta">
                  <span className="diff-path" title={selectedSkillDiffs.tools[0]?.diff?.ssot_path}>
                    ssot: {selectedSkillDiffs.tools[0]?.diff?.ssot_path ?? "—"}
                  </span>
                </div>
                {/* P2-3: each divergent tool is an expandable block; left column = SSOT (synced) */}
                <div className="tool-diff-list">
                  {selectedSkillDiffs.tools.map((tool, i) => {
                    const key = String(i);
                    const open = expandedTools.has(key);
                    return (
                      <div key={key} className="tool-diff-block">
                        <div className="tool-diff-block-header" onClick={() => toggleToolBlock(key)}>
                          <span className="tool-diff-caret">{open ? "▾" : "▸"}</span>
                          <span className="update-tool-name">{tool.tool ?? t("sourceLabel")}</span>
                          <code className="update-path" title={tool.sourcePath}>{tool.sourcePath}</code>
                        </div>
                        {open && (
                          <div className="tool-diff-block-body">
                            {tool.loading ? (
                              <div className="diff-loading">{t("loading")}</div>
                            ) : tool.diff ? (
                              <DiffFilesView diff={tool.diff} sideBySide={diffSideBySide} noChangesLabel={t("noChanges")} />
                            ) : (
                              <div className="diff-loading">{t("failedLoadDiff")}</div>
                            )}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
                <div className="modal-actions">
                  <button
                    className="btn btn-small btn-primary"
                    onClick={() => handleUpdateFromDiff(selectedSkillDiffs.skillId, selectedSkillDiffs.tools[0]?.sourcePath)}
                  >
                    {t("updateToSsot")}
                  </button>
                  <button
                    className="btn btn-small"
                    onClick={() => handleSkipUpdate(selectedSkillDiffs.skillId)}
                  >
                    {t("skip")}
                  </button>
                </div>
              </>
            ) : (
              <>
                <h3>{t("updatesAvailable")} ({updates.length})</h3>
                <div className="updates-list">
                  {(() => {
                    // Group updates by skill_id
                    const grouped = new Map<number, SkillUpdate[]>();
                    for (const u of updates) {
                      const list = grouped.get(u.skill_id) || [];
                      list.push(u);
                      grouped.set(u.skill_id, list);
                    }
                    return Array.from(grouped.entries()).map(([skillId, skillUpdates]) => (
                      <div key={skillId} className="update-item-group">
                        <div className="update-item-header">
                          <span className="update-skill-name">{skillUpdates[0].skill_name}</span>
                          <span className="update-count-badge">{skillUpdates.length} {t("toolUnit")}</span>
                          <button
                            className="btn btn-small"
                            onClick={() => handleViewSkillDiffs(skillId, skillUpdates[0].skill_name, skillUpdates)}
                          >
                            {t("viewDiffAll")}
                          </button>
                        </div>
                        {skillUpdates.map((u, idx) => (
                          <div key={idx} className="update-item update-item-tool">
                            <div className="update-item-info">
                              {u.changed_tool && <span className="update-tool-name">{u.changed_tool}</span>}
                              <code className="update-path" title={u.source_path}>{u.source_path}</code>
                            </div>
                            <div className="update-item-actions">
                              <button
                                className="btn btn-small"
                                onClick={() => handleViewDiff(u)}
                                disabled={loadingDiff === u.skill_id}
                              >
                                {loadingDiff === u.skill_id ? t("loading") : t("viewDiff")}
                              </button>
                              {u.changed_tool_id && (
                                <button
                                  className="btn btn-small"
                                  onClick={() => handleDismissUpdate(u)}
                                >
                                  {t("dismissChange")}
                                </button>
                              )}
                            </div>
                          </div>
                        ))}
                        <div className="update-item-actions update-group-actions">
                          <button
                            className="btn btn-small btn-primary"
                            onClick={() => handleUpdateFromDiff(skillId, skillUpdates[0].source_path)}
                          >
                            {t("updateToSsot")}
                          </button>
                          <button
                            className="btn btn-small"
                            onClick={() => handleSkipUpdate(skillId)}
                          >
                            {t("skip")}
                          </button>
                        </div>
                      </div>
                    ));
                  })()}
                </div>
                <div className="modal-actions">
                  <button className="btn btn-secondary" onClick={() => { setShowUpdatesModal(false); setSelectedUpdateDiff(null); setSelectedSkillDiffs(null); setDiffMaximized(false); }}>{t("close")}</button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </main>
  );
}

// P2-1/2/3: shared diff rendering controls (side-by-side toggle + maximize).
function DiffViewControls({
  t,
  sideBySide,
  onToggleSideBySide,
  maximized,
  onToggleMaximize,
}: {
  t: (key: string) => string;
  sideBySide: boolean;
  onToggleSideBySide: () => void;
  maximized: boolean;
  onToggleMaximize: () => void;
}) {
  return (
    <div className="diff-view-controls">
      <div className="diff-view-toggle">
        <button
          className={`btn btn-small${sideBySide ? " active" : ""}`}
          onClick={sideBySide ? undefined : onToggleSideBySide}
        >
          {t("splitView")}
        </button>
        <button
          className={`btn btn-small${!sideBySide ? " active" : ""}`}
          onClick={sideBySide ? onToggleSideBySide : undefined}
        >
          {t("unifiedView")}
        </button>
      </div>
      <button className="btn btn-small" onClick={onToggleMaximize}>
        {maximized ? t("restore") : t("maximize")}
      </button>
    </div>
  );
}

function UnifiedHunk({ hunk }: { hunk: DiffHunk }) {
  return (
    <div className="diff-hunk">
      <div className="diff-hunk-header">
        @@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@
      </div>
      <pre className="diff-lines">
        {hunk.lines.map((line, li) => (
          <div key={li} className={`diff-line diff-line-${line.op === "+" ? "add" : line.op === "-" ? "del" : "ctx"}`}>
            <span className="diff-line-op">{line.op === " " ? " " : line.op}</span>
            <span className="diff-line-content">{line.content}</span>
          </div>
        ))}
      </pre>
    </div>
  );
}

// P2-2: pair deletions/insertions into left (SSOT/old) + right (current/new) columns.
function SideBySideHunk({ hunk }: { hunk: DiffHunk }) {
  const rows: { left?: DiffLine; right?: DiffLine }[] = [];
  let pendL: DiffLine[] = [];
  let pendR: DiffLine[] = [];
  const flush = () => {
    const n = Math.max(pendL.length, pendR.length);
    for (let i = 0; i < n; i++) {
      rows.push({ left: pendL[i], right: pendR[i] });
    }
    pendL = [];
    pendR = [];
  };
  for (const line of hunk.lines) {
    if (line.op === " ") {
      flush();
      rows.push({ left: line, right: line });
    } else if (line.op === "-") {
      pendL.push(line);
    } else if (line.op === "+") {
      pendR.push(line);
    }
  }
  flush();

  const leftClass = (l?: DiffLine) => (l ? (l.op === "-" ? "del" : "ctx") : "empty");
  const rightClass = (r?: DiffLine) => (r ? (r.op === "+" ? "add" : "ctx") : "empty");
  const opSym = (op?: string) => (op === " " ? " " : op ?? " ");

  return (
    <div className="diff-hunk diff-hunk-split">
      <div className="diff-hunk-header">
        @@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@
      </div>
      <div className="diff-split">
        <div className="diff-split-col diff-split-old">
          {rows.map((r, i) => (
            <div key={i} className={`diff-line diff-line-${leftClass(r.left)}`}>
              <span className="diff-line-op">{opSym(r.left?.op)}</span>
              <span className="diff-line-content">{r.left?.content ?? ""}</span>
            </div>
          ))}
        </div>
        <div className="diff-split-col diff-split-new">
          {rows.map((r, i) => (
            <div key={i} className={`diff-line diff-line-${rightClass(r.right)}`}>
              <span className="diff-line-op">{opSym(r.right?.op)}</span>
              <span className="diff-line-content">{r.right?.content ?? ""}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function DiffFilesView({ diff, sideBySide, noChangesLabel }: { diff: SkillDiff; sideBySide: boolean; noChangesLabel: string }) {
  return (
    <div className="diff-files">
      {diff.files.map((file, fi) => (
        <div key={fi} className="diff-file">
          <div className={`diff-file-header diff-file-${file.change}`}>
            <span className="diff-change-badge">{file.change}</span>
            <span className="diff-file-path">{file.path}</span>
          </div>
          {file.hunks.map((hunk, hi) =>
            sideBySide ? <SideBySideHunk key={hi} hunk={hunk} /> : <UnifiedHunk key={hi} hunk={hunk} />
          )}
        </div>
      ))}
      {!diff.has_changes && <div className="diff-no-changes">{noChangesLabel}</div>}
    </div>
  );
}

function ToastContainer({ toasts }: { toasts: Toast[] }) {
  if (toasts.length === 0) return null;
  return (
    <div className="toast-container">
      {toasts.map((toast) => (
        <div key={toast.id} className={`toast toast-${toast.type}`}>
          {toast.message}
        </div>
      ))}
    </div>
  );
}

export default App;
