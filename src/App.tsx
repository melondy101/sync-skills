// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

// App is the orchestration layer: it owns cross-component data (tools, skills,
// projects, settings, updates, conflicts) and top-level UI state, and wires the
// feature components together. All rendering details live in src/components/.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import "./ui-enhancements.css";
import * as api from "./api";
import type {
  Tool, Project, SkillView, ScanResult, SkillUpdate,
  Settings, InstallationInfo, ConflictView, RemoteInstallation, Market,
} from "./types";
import { makeT, type Lang } from "./i18n";
import { useToasts } from "./hooks/useToasts";
import { useTheme } from "./hooks/useTheme";
import { ToastContainer } from "./components/ToastContainer";
import { SettingsPanel } from "./components/SettingsPanel";
import { LogsPanel } from "./components/LogsPanel";
import { ToolsSection } from "./components/ToolsSection";
import { ProjectNav } from "./components/ProjectNav";
import { ConflictSection } from "./components/ConflictSection";
import { ScanResultModal } from "./components/ScanResultModal";
import { UpdatesModal, type UpdateDiffEntry } from "./components/UpdatesModal";
import { OnboardingWizard } from "./components/OnboardingWizard";
import { LintModal } from "./components/LintModal";
import { SkillEditorModal } from "./components/SkillEditorModal";
import SkillMarketPanel from "./components/SkillMarketPanel";
import { ConfirmProvider } from "./components/ConfirmProvider";

import { SkillListRow } from "./components/SkillListRow";
import { Icon } from "./components/Icon";
import { SkillAvatar } from "./components/SkillAvatar";

type Tab = "global" | "projects" | "market";
type Panel = "main" | "settings" | "logs";

// Mirrors `Settings::default()` in src-tauri/src/settings.rs. Used as the
// initial state so `settings` is never null (it is read during the first
// render) and as the fallback when the backend loader fails.
const DEFAULT_SETTINGS: Settings = {
  sync_mode: "semi-auto",
  prefer_symlink: false,
  theme: "light",
  language: "zh",
  close_action: "exit",
  use_system_proxy: true,
  use_proxy: false,
  proxy_url: null,
  view_market_diff_before_update: true,
  auto_sync_on_file_change: false,
};

// Watcher events arrive per file; one rename or a checkout touches many skills
// at once, so the auto-sync pass waits for the burst to settle before scanning.
const AUTO_SYNC_DEBOUNCE_MS = 1500;

function App() {
  // ==================== Shared data ====================
  const [tools, setTools] = useState<Tool[]>([]);
  const [skills, setSkills] = useState<SkillView[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);

  // Settings update with coupling: when the user changes `sync_mode`, mirror
  // it onto `auto_sync_on_file_change` so the two UI controls stay in sync
  // per the Phase 4 / T1 spec ("Mirrors `sync_mode == "full-auto"`"). The
  // reverse direction (toggling the checkbox directly) leaves `sync_mode`
  // alone so the two remain independently editable.
  // The canonical OR that downstream code (file watcher, Phase 4 / M12) reads
  // lives in `src-tauri/src/settings.rs::Settings::effective_auto_sync_on_file_change`.
  const handleSettingsChange = useCallback((next: Settings) => {
    setSettings((prev) => {
      if (prev.sync_mode !== next.sync_mode) {
        return {
          ...next,
          auto_sync_on_file_change: next.sync_mode === "full-auto",
        };
      }
      return next;
    });
  }, []);

  // `t` is threaded into most of the tree; a fresh closure per render would make
  // every memoized child re-render even when the language is unchanged.
  const t = useMemo(() => makeT(settings.language as Lang), [settings.language]);
  useTheme(settings.theme);
  const { toasts, addToast } = useToasts();

  // Auto-save theme and language changes without an explicit save button.
  useEffect(() => {
    const timer = setTimeout(() => {
      api
        .updateSettings(settings)
        .catch(() => {});
    }, 300);
    return () => clearTimeout(timer);
  }, [settings]);

  // T3 / M12: the backend watcher reports on-disk changes. Auto-sync runs on the
  // same OR the backend's `effective_auto_sync_on_file_change` computes; with
  // neither enabled the user only gets a notice.
  //
  // A refresh scan is not enough — it marks updates but leaves SSOT and the tool
  // directories out of step — so the pending batch is synced after the rescan.
  // Events burst (one rename touches several skills), hence the coalescing timer,
  // and the sync pass is read through a ref so the subscription never calls a
  // stale closure of `handleSyncAll`.
  const syncAllRef = useRef(handleSyncAll);
  useEffect(() => {
    syncAllRef.current = handleSyncAll;
  });

  const autoSyncTimer = useRef<number | null>(null);
  useEffect(() => {
    const autoSync = settings.sync_mode === "full-auto" || settings.auto_sync_on_file_change;
    const unlisten = listen<{ skill_name: string; path: string }>("skill-file-changed", (event) => {
      if (!autoSync) {
        addToast("info", `"${event.payload.skill_name}" changed on disk — check for updates`);
        return;
      }
      if (autoSyncTimer.current !== null) window.clearTimeout(autoSyncTimer.current);
      autoSyncTimer.current = window.setTimeout(() => {
        autoSyncTimer.current = null;
        void (async () => {
          try {
            await api.fullScan();
          } catch {
            // A failed rescan must not swallow the sync of what already changed.
          }
          await syncAllRef.current();
        })();
      }, AUTO_SYNC_DEBOUNCE_MS);
    });
    return () => {
      if (autoSyncTimer.current !== null) window.clearTimeout(autoSyncTimer.current);
      unlisten.then((fn) => fn());
    };
  }, [settings.sync_mode, settings.auto_sync_on_file_change, addToast]);

  // ==================== UI state ====================
  const [activeTab, setActiveTab] = useState<Tab>("global");
  const [activePanel, setActivePanel] = useState<Panel>("main");
  const [selectedProject, setSelectedProject] = useState<number>(0);
  const [scanning, setScanning] = useState(false);
  const [syncing, setSyncing] = useState<Set<number>>(new Set());
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [checkingSingle, setCheckingSingle] = useState<number | null>(null);
  const [updates, setUpdates] = useState<SkillUpdate[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [filterMarket, setFilterMarket] = useState<number>(-1); // -1 = all markets / local only
  const [sortBy, setSortBy] = useState<"name" | "updated_at" | "created_at">("name");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("asc");
  const [viewMode, setViewMode] = useState<"card" | "list">("card");
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [showUpdatesModal, setShowUpdatesModal] = useState(false);
  // Pre-loaded diff for the single-skill check flow (passed to UpdatesModal)
  const [updatesInitialDiff, setUpdatesInitialDiff] = useState<UpdateDiffEntry | null>(null);
  const [conflicts, setConflicts] = useState<ConflictView[]>([]);
  const [remoteInstallations, setRemoteInstallations] = useState<RemoteInstallation[]>([]);
  const [, setRemoteInstallationsLoading] = useState(false);
  // Markets are owned by SkillMarketPanel (so it can drive auto-sync, layout
  // editing, etc.), but the parent needs a copy to render a provenance badge
  // on each skill card. We get it through `onMarketsChanged` rather than
  // duplicating the load, so there's a single source of truth.
  const [marketsById, setMarketsById] = useState<Record<number, Market>>({});

  const projectPaths = useMemo(() => {
    const map: Record<number, string> = { 0: "" };
    for (const project of projects) {
      map[project.id] = project.path;
    }
    return map;
  }, [projects]);

  // New features: onboarding wizard, health check, built-in editor
  const [showWizard, setShowWizard] = useState(() => localStorage.getItem("onboardingDone") !== "1");
  // Health check scope: "all" = whole project, SkillView = single card, null = closed
  const [lintTarget, setLintTarget] = useState<"all" | SkillView | null>(null);
  // Editor target: opened from a skill card or from the lint modal's Edit action
  const [editingSkill, setEditingSkill] = useState<{ id: number; name: string } | null>(null);
  // Bumped to force the lint modal to re-run after an edit is saved
  const [lintRunId, setLintRunId] = useState(0);

  // ==================== Data loaders ====================

  async function loadTools() {
    try {
      setTools(await api.listTools());
    } catch (e) {
      addToast("error", `${t("failedLoadTools")}: ${e}`);
    }
  }

  async function loadSkills() {
    try {
      setSkills(await api.listSkills(selectedProject));
    } catch (e) {
      addToast("error", `${t("failedLoadSkills")}: ${e}`);
    }
  }

  async function loadProjects() {
    try {
      const result = await api.listProjects();
      setProjects(result.filter((p) => p.id !== 0)); // Exclude Global from project list
    } catch (e) {
      addToast("error", `${t("failedLoadProjects")}: ${e}`);
    }
  }

  async function loadSettings() {
    try {
      setSettings(await api.getSettings());
    } catch (e) {
      // Fall back to hardcoded defaults when backend loader fails
      setSettings(DEFAULT_SETTINGS);
      addToast("error", `${t("failedLoadSettings")}: ${e}`);
    }
  }

  async function loadConflicts() {
    try {
      setConflicts(await api.listConflicts(selectedProject));
    } catch (e) {
      addToast("error", `${t("failedResolve")}: ${e}`);
    }
  }
  async function loadRemoteInstallations() {
    setRemoteInstallationsLoading(true);
    try {
      const marketId = filterMarket >= 0 ? filterMarket : null;
      setRemoteInstallations(await api.listRemoteInstallations(selectedProject, marketId));
    } catch (e) {
      addToast("error", `${t("failedLoadSkills")}: ${e}`);
    } finally {
      setRemoteInstallationsLoading(false);
    }
  }

  // Load data on mount
  useEffect(() => {
    loadTools();
    loadSkills();
    loadProjects();
    loadSettings();
    loadConflicts();
    loadRemoteInstallations();
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

  useEffect(() => {
    loadRemoteInstallations();
  }, [selectedProject, filterMarket]);

  const installProjectId = useMemo(() => (activeTab === "market" ? selectedProject : 0), [activeTab, selectedProject]);

  // ==================== Actions ====================

  async function handleScan() {
    setScanning(true);
    try {
      let result: ScanResult;
      if (activeTab === "projects" && selectedProject !== 0) {
        // Project-level scan: scan project-specific paths
        result = await api.scanScope(null, selectedProject);
      } else {
        // Global scan
        result = await api.fullScan();
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
      const result = await api.checkUpdates(selectedProject);
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
      const result = await api.checkSkillUpdate(skillId, selectedProject);
      if (result.length > 0) {
        // Found updates — add all to updates list, show diff for the first one
        setUpdates((prev) => {
          const filtered = prev.filter((u) => u.skill_id !== skillId);
          return [...filtered, ...result];
        });
        const firstUpdate = result[0];
        const diff = await api.getSkillDiff(skillId, firstUpdate.source_path);
        setUpdatesInitialDiff({ update: firstUpdate, diff });
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

  async function handleToggle(skillId: number, toolId: number, active: boolean) {
    try {
      await api.toggleSkill(skillId, toolId, selectedProject, active);
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
      const result = await api.syncSkill(skillId, selectedProject);
      await loadSkills();
      await handleCheckUpdates();

      if (result.errors.length > 0) {
        addToast("error", `${result.skill_name}: ${result.errors.join(", ")}`);
      } else {
        addToast("success", `${t("syncedToTools")} ${result.skill_name} -> ${result.synced_to} ${t("tools")}`);
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
      const results = await api.syncAllPending();
      await loadSkills();
      await handleCheckUpdates();

      const totalSynced = results.reduce((sum, r) => sum + r.synced_to, 0);
      const totalErrors = results.reduce((sum, r) => sum + r.errors.length, 0);

      addToast(
        totalErrors > 0 ? "error" : "success",
        `${t("syncComplete")}: ${totalSynced} ${t("syncedCount")}, ${totalErrors} ${t("errors")}`
      );
    } catch (e) {
      addToast("error", `${t("syncAllFailed")}: ${e}`);
    } finally {
      setScanning(false);
    }
  }

  async function handleSaveSettings() {
    try {
      await api.updateSettings(settings);
      addToast("success", t("settingsSaved"));
    } catch (e) {
      addToast("error", `${t("failedSaveSettings")}: ${e}`);
    }
  }

  function handleWizardClose() {
    localStorage.setItem("onboardingDone", "1");
    setShowWizard(false);
    // Pick up whatever the wizard added/scanned
    loadTools();
    loadSkills();
    loadConflicts();
  }

  // ==================== Render helpers ====================

  function getInstallStatus(skill: SkillView, toolId: number): InstallationInfo | undefined {
    return skill.installed_tools.find((inst) => inst.tool_id === toolId);
  }

  function hasUpdate(skill: SkillView): boolean {
    return updates.some((u) => u.skill_id === skill.id);
  }

  const filteredSkills = (() => {
    const byMarket =
      filterMarket >= 0
        ? skills.filter((s) =>
            (remoteInstallations ?? []).some(
              (inst) => inst.projectId === selectedProject && inst.marketId === filterMarket && inst.skillName === s.name,
            ),
          )
        : skills;
    const filtered = searchQuery
      ? byMarket.filter(
          (s) =>
            s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            (s.description && s.description.toLowerCase().includes(searchQuery.toLowerCase())),
        )
      : [...byMarket];
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

  // ==================== Main Render ====================

  if (activePanel === "settings") return (
    <main className="container">
      <ToastContainer toasts={toasts} />
      <SettingsPanel
        t={t}
        settings={settings}
        onChange={handleSettingsChange}
        onSave={handleSaveSettings}
        onBack={() => setActivePanel("main")}
      />
    </main>
  );

  if (activePanel === "logs") return (
    <main className="container">
      <ToastContainer toasts={toasts} />
      <LogsPanel t={t} addToast={addToast} onBack={() => setActivePanel("main")} />
    </main>
  );

  // Show loading spinner until settings are loaded from the backend
  if (!settings) {
    return (
      <main className="container">
        <div className="loading-screen">
          <Icon name="hexagon" size={48} />
          <p>Loading…</p>
        </div>
      </main>
    );
  }

  return (
    <main className="container">
      <ConfirmProvider>
      <ToastContainer toasts={toasts} />

      {/* App header */}
      <header className="app-header">
        <div className="brand">
          <span className="brand-icon"><Icon name="hexagon" size={22} /></span>
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
          <button
            className={`tab ${activeTab === "market" ? "tab-active" : ""}`}
            onClick={() => setActiveTab("market")}
          >
            {t("market")}
          </button>
        </div>
        <div className="nav-actions">
          <button className="btn btn-small btn-ghost" onClick={() => setActivePanel("logs")}>
            {t("logs")}
          </button>
          <button className="btn btn-small btn-ghost" onClick={() => setActivePanel("settings")}>
            {t("settings")}
          </button>
        </div>
      </nav>

      {/* Project sub-navigation */}
      {activeTab === "projects" && (
        <ProjectNav
          t={t}
          projects={projects}
          selectedProject={selectedProject}
          onSelect={setSelectedProject}
          addToast={addToast}
          onProjectsChanged={loadProjects}
        />
      )}

      {/* Tool configuration */}
      {activeTab !== "market" && (
        <ToolsSection
          t={t}
          tools={tools}
          addToast={addToast}
          onToolsChanged={async () => {
            await Promise.all([loadTools(), loadSkills()]);
          }}
        />
      )}

      {/* Action bar */}
      {activeTab !== "market" && (
      <section className="section">
        <div className="action-bar">
          <div className="action-group action-group-primary">
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
          </div>

          <div className="search-box">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t("searchPlaceholder")}
              className="search-input"
            />
          </div>

          <div className="action-group action-group-view">
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
            <select
              className="market-filter-select"
              value={filterMarket}
              onChange={(e) => setFilterMarket(Number(e.target.value))}
              title={t("filterByMarket")}
            >
              <option value={-1}>{t("allMarkets")}</option>
              {Object.values(marketsById).map((m) => (
                <option key={m.id} value={m.id}>
                  {m.owner}/{m.name}
                </option>
              ))}
            </select>
            <button
              className="btn btn-small view-toggle-btn"
              onClick={() => setViewMode((v) => (v === "card" ? "list" : "card"))}
              title={viewMode === "card" ? t("viewList") : t("viewCard")}
            >
              {viewMode === "card" ? <Icon name="list" size={16} /> : <Icon name="grid-3x3" size={16} />}
            </button>
            <details className="overflow-menu">
              <summary className="btn btn-small btn-ghost overflow-menu-trigger" aria-label={t("moreActions")}>
                <Icon name="more-horizontal" size={16} />
              </summary>
              <div className="overflow-menu-items" role="menu">
                <button type="button" role="menuitem" onClick={() => { setLintTarget("all"); (document.activeElement as HTMLElement | null)?.blur?.(); }}>
                  {t("healthCheck")}
                </button>
                <button type="button" role="menuitem" onClick={() => { setActivePanel("logs"); (document.activeElement as HTMLElement | null)?.blur?.(); }}>
                  {t("logs")}
                </button>
                <button type="button" role="menuitem" onClick={() => { setActivePanel("settings"); (document.activeElement as HTMLElement | null)?.blur?.(); }}>
                  {t("settings")}
                </button>
              </div>
            </details>
          </div>
        </div>
      </section>
      )}

      {/* Conflict banner + diff modal (M5) */}
      {activeTab !== "market" && (
      <ConflictSection
        t={t}
        conflicts={conflicts}
        projectId={selectedProject}
        addToast={addToast}
        onResolved={async () => {
          await Promise.all([loadConflicts(), loadSkills()]);
        }}
      />
      )}

      {/* Skill grid */}
      {activeTab !== "market" && (
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
            {searchQuery || filterMarket >= 0 ? (
              <p>{t("noMatch")} "{searchQuery}"</p>
            ) : (
              <>
                <p>{t("noSkillsYet")}</p>
                <p>{t("configureAndScan")}</p>
              </>
            )}
          </div>
        ) : viewMode === "list" ? (
          <div className="skill-list">
            <table className="skill-table">
              <thead>
                <tr>
                  <th className="col-name">{t("skills")}</th>
                  {tools.map((tool) => (
                    <th key={tool.id} className="col-tool">{tool.name}</th>
                  ))}
                  <th className="col-actions"></th>
                </tr>
              </thead>
              <tbody>
                {filteredSkills.map((skill) => (
                  <SkillListRow
                    key={skill.id}
                    skill={skill}
                    tools={tools}
                    t={t}
                    hasUpdate={hasUpdate(skill)}
                    syncing={syncing.has(skill.id)}
                    checkingSingle={checkingSingle === skill.id}
                    sourceMarket={skill.source_market_id != null ? marketsById[skill.source_market_id] : undefined}
                    getInstallStatus={getInstallStatus}
                    onToggle={handleToggle}
                    onSync={() => handleSyncSkill(skill.id)}
                    onCheckUpdate={() => handleCheckSingleSkill(skill.id)}
                    onHealthCheck={() => setLintTarget(skill)}
                    onEdit={() => setEditingSkill(skill)}
                    onOpenMarket={() => setActiveTab("market")}
                  />
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="skill-grid">
            {filteredSkills.map((skill) => (
              <div key={skill.id} className={`skill-card ${hasUpdate(skill) ? "skill-has-update" : ""}`}>
                <div className="skill-header">
                  <SkillAvatar name={skill.name} size={36} className="skill-card-avatar" />
                  <div className="skill-header-text">
                    <div className="skill-name-row">
                      <h3 className="skill-name">{skill.name}</h3>
                      {hasUpdate(skill) && <Icon name="circle" size={8} className="update-indicator" />}
                      {syncing.has(skill.id) && <Icon name="refresh-cw" size={14} className="sync-spinner" />}
                    </div>
                    {skill.description && <p className="skill-desc">{skill.description}</p>}
                  </div>
                </div>

                {skill.source_market_id != null && marketsById[skill.source_market_id] && (
                  <button
                    type="button"
                    className="market-provenance"
                    onClick={() => setActiveTab("market")}
                    title={t("openInMarket")}
                  >
                    {t("fromMarket").replace("{0}", `${marketsById[skill.source_market_id].owner}/${marketsById[skill.source_market_id].name}`)}
                  </button>
                )}

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
                  {(() => {
                    const allSynced = tools.length > 0 && tools.every((tool) => {
                      const inst = getInstallStatus(skill, tool.id);
                      return inst?.synced_at != null;
                    });
                    return (
                      <>
                        {allSynced && (
                          <span
                            className="synced-dot"
                            title={t("allSynced")}
                            aria-label={t("allSynced")}
                          />
                        )}
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
                      </>
                    );
                  })()}
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
                    className="btn btn-small"
                    onClick={() => setLintTarget(skill)}
                  >
                    {t("healthCheck")}
                  </button>
                  <button
                    className="btn btn-small"
                    onClick={() => setEditingSkill(skill)}
                  >
                    {t("editSkill")}
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
      )}

      {/* Scan Result Modal */}
      {scanResult && (
        <ScanResultModal t={t} result={scanResult} onClose={() => setScanResult(null)} />
      )}

      {/* Updates Modal */}
      {showUpdatesModal && (
        <UpdatesModal
          t={t}
          projectId={selectedProject}
          updates={updates}
          initialDiff={updatesInitialDiff}
          addToast={addToast}
          onUpdatesChanged={setUpdates}
          onSynced={loadSkills}
          onClose={() => {
            setShowUpdatesModal(false);
            setUpdatesInitialDiff(null);
          }}
        />
      )}

      {/* Onboarding wizard (first run) */}
      {showWizard && (
        <OnboardingWizard t={t} addToast={addToast} onClose={handleWizardClose} />
      )}

      {/* Skill health check (whole scope or single card) */}
      {lintTarget && (
        <LintModal
          key={lintRunId}
          t={t}
          projectId={selectedProject}
          skill={lintTarget === "all" ? null : { id: lintTarget.id, name: lintTarget.name }}
          addToast={addToast}
          onEdit={(id, name) => setEditingSkill({ id, name })}
          onFixed={loadSkills}
          onClose={() => setLintTarget(null)}
        />
      )}

      {/* Built-in SKILL.md editor */}
      {editingSkill && (
        <SkillEditorModal
          t={t}
          skillId={editingSkill.id}
          skillName={editingSkill.name}
          projectId={selectedProject}
          addToast={addToast}
          onSaved={async () => {
            await loadSkills();
            // Re-run the health check underneath so its results reflect the edit.
            if (lintTarget) setLintRunId((n) => n + 1);
          }}
          onClose={() => setEditingSkill(null)}
        />
      )}

      {/* Skill market */}
      {activeTab === "market" && (
        <SkillMarketPanel
          t={t}
          addToast={addToast}
          projects={projects}
          projectPaths={projectPaths}
          tools={tools}
          onRemoteInstallationsChanged={setRemoteInstallations}
          onSkillsChanged={() => { void loadSkills(); }}
          onMarketsChanged={(markets) => setMarketsById(Object.fromEntries(markets.map((m) => [m.id, m])))}
          defaultProjectId={installProjectId}
        />
      )}
      </ConfirmProvider>
    </main>
  );
}

export default App;




