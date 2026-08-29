// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "../api";
import InstallDialog from './InstallDialog';
import MarketSourcesModal from './MarketSourcesModal';
import { RemoteSkillDetailModal } from './RemoteSkillDetailModal';
import RemoteUpdatesModal from './RemoteUpdatesModal';
import { useConfirm } from './ConfirmProvider';
import { Icon } from "./Icon";
import MarketSidebar from "./MarketSidebar";
import type {
  Market,
  Project,
  RemoteInstallation,
  RemoteSkill,
  RemoteSkillUpdate,
  Tool,
} from "../types";

type Props = {
  projects: Project[];
  projectPaths: Record<number, string>;
  tools: Tool[];
  onRemoteInstallationsChanged: (next: RemoteInstallation[]) => void;
  onSkillsChanged: () => void;
  onMarketsChanged: (markets: Market[]) => void;
  defaultProjectId: number;
  t: (key: string) => string;
  addToast: (type: "success" | "error" | "info", message: string) => void;
};

const BUILTIN_MARKET_IDS = new Set([
  "anthropic-skills",
  "superpowers-skills",
  "mattpocock-skills",
  "andrej-karpathy-skill",
  "khazix-skills",
]);

export default function SkillMarketPanel({
  projects,
  projectPaths,
  tools,
  onRemoteInstallationsChanged,
  onSkillsChanged,
  onMarketsChanged,
  defaultProjectId,
  t,
  addToast,
}: Props) {
  const [markets, setMarkets] = useState<Market[]>([]);
  const [marketTitles, setMarketTitles] = useState<Record<number, string>>({});
  const [marketLoading, setMarketLoading] = useState(false);

  const [showAddMarket, setShowAddMarket] = useState(false);
  const [marketUrl, setMarketUrl] = useState("");
  const [marketBranch, setMarketBranch] = useState("");
  const [marketLayout, setMarketLayout] = useState<"auto" | "root" | "subdir">("auto");

  const [installProjectId, setInstallProjectId] = useState<number>(defaultProjectId || 0);
  const [selectedToolPath, setSelectedToolPath] = useState<string>("");
  const [installLoading, setInstallLoading] = useState(false);

  const [remoteSkills, setRemoteSkills] = useState<RemoteSkill[]>([]);
  const [skillLoading, setSkillLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedMarketFilter, setSelectedMarketFilter] = useState<string>("all");

  const [updates, setUpdates] = useState<RemoteSkillUpdate[] | null>(null);
  const [remoteCheckLoading, setRemoteCheckLoading] = useState(false);
  const [remoteScanLoading, setRemoteScanLoading] = useState(false);
  const [remoteUpdateMode, setRemoteUpdateMode] = useState("installed");

  const [installTarget, setInstallTarget] = useState<RemoteSkill | null>(null);
  const [showSourcesModal, setShowSourcesModal] = useState(false);
  const [showUpdatesModal, setShowUpdatesModal] = useState(false);
  const [installedOnly, setInstalledOnly] = useState(false);
  const [marketSyncErrors, setMarketSyncErrors] = useState<Record<number, string[]>>({});

  // T4 detail Modal: the skill whose card was clicked (null = closed).
  const [detailSkill, setDetailSkill] = useState<RemoteSkill | null>(null);

  // T4/T5: collapsible actions drawer + multi-select market filter
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [selectedFilterMarkets, setSelectedFilterMarkets] = useState<Set<number>>(new Set());

  const { showConfirm, setProgress } = useConfirm();

  const selectedProjectPath = projectPaths[installProjectId] ?? "";

  const resolvedToolPaths = useMemo(() => {
    const options: { toolId: number; name: string; path: string }[] = [];
    for (const tool of tools) {
      const path = installProjectId === 0 ? tool.globalPath : `${selectedProjectPath}/${tool.projectRelPath}`.replace(/\/+/g, "/");
      options.push({ toolId: tool.id, name: tool.name, path });
    }
    return options;
  }, [installProjectId, projectPaths, selectedProjectPath, tools]);

  useEffect(() => {
    if (resolvedToolPaths.length > 0 && !resolvedToolPaths.some((option) => option.path === selectedToolPath)) {
      setSelectedToolPath(resolvedToolPaths[0].path);
    }
  }, [resolvedToolPaths, selectedToolPath]);

  useEffect(() => {
    setInstallProjectId(defaultProjectId || 0);
  }, [defaultProjectId]);

  useEffect(() => {
    loadMarkets();
  }, []);

  useEffect(() => {
    api.listRemoteInstallations(installProjectId, selectedMarketFilter === "all" ? null : Number(selectedMarketFilter)).then(onRemoteInstallationsChanged);
  }, [installProjectId, selectedMarketFilter]);

  // Retained for upcoming modals and existing handlers.
  useEffect(() => {
    void marketTitles;
    void marketLoading;
    void showAddMarket;
    void remoteUpdateMode;
    void setRemoteUpdateMode;
    void BUILTIN_MARKET_IDS;
    void handleAddMarketByUrl;
    void toggleMarket;
    void deleteMarket;
    void syncMarketIndex;
    void setMarketSyncErrors;
    void requestMarkAllRemoteSkillsInstalled;
    void requestDeleteMarket;
    void requestSyncAllInstalledRemoteSkills;
  }, []);

  async function loadMarkets() {
    setMarketLoading(true);
    try {
      const markets = await api.listMarkets();
      setMarkets(markets);
      const titles: Record<number, string> = {};
      for (const market of markets) {
        titles[market.id] = `${market.owner}/${market.name}`;
      }
      setMarketTitles(titles);
      // Bubble the canonical list to the parent so the global/project view
      // can render source-market provenance badges for skills installed from
      // the market. Doing it here (vs. duplicating the loader in App.tsx)
      // keeps the market UI as the single source of truth.
      onMarketsChanged(markets);
    } catch (e) {
      addToast("error", `${t("failedLoadTools")}: ${e}`);
    } finally {
      setMarketLoading(false);
    }
  }

  async function handleAddMarketByUrl() {
    const url = marketUrl.trim();
    const branch = marketBranch.trim();
    if (!url) {
      addToast("error", t("addMarketUrlRequired"));
      return;
    }
    setMarketLoading(true);
    try {
      const market = await api.addMarketByUrl(url, branch || undefined, marketLayout);
      addToast(
        "success",
        `${t("addMarket")}: ${market.owner}/${market.name} · ${t("marketLayout")}: ${
          market.layout === "root" ? t("layoutRoot") : t("layoutSubdir")
        }`,
      );
      setMarketUrl("");
      setMarketBranch("");
      setShowAddMarket(false);
      await loadMarkets();

      // Auto-sync so the user immediately sees whether the repo is reachable
      // and whether it actually contains skills (instead of leaving an empty
      // record and making the user click "Sync Index" again).
      try {
        const result = await api.syncMarketIndex(market.id);
        setMarketSyncErrors((prev) => ({ ...prev, [market.id]: result.errors }));
        if (result.skills_found === 0 && result.errors.length === 0) {
          addToast("info", `${market.owner}/${market.name}: ${t("repoNoSkills")}`);
        } else if (result.errors.length > 0) {
          const first = result.errors[0] ?? "";
          addToast("error", `${market.owner}/${market.name}: ${t("repoSyncFailed").replace("{0}", first)}`);
        } else {
          addToast("success", `${market.owner}/${market.name}: ${result.skills_found} ${result.skills_found === 1 ? "skill" : "skills"}`);
        }
        await Promise.all([loadMarkets(), loadRemoteSkills(market.id)]);
      } catch (e) {
        setMarketSyncErrors((prev) => ({ ...prev, [market.id]: [`${e}`] }));
        addToast("error", `${market.owner}/${market.name}: ${t("repoSyncFailed").replace("{0}", `${e}`)}`);
      }
    } catch (e) {
      addToast("error", `${t("addMarket")} failed: ${e}`);
    } finally {
      setMarketLoading(false);
    }
  }

  async function toggleMarket(market: Market) {
    setMarketLoading(true);
    try {
      await api.updateMarket(market.id, market.provider, market.owner, market.name, market.branch, !market.enabled, market.layout);
      await loadMarkets();
    } catch (e) {
      addToast("error", `${t("editMarket")} failed: ${e}`);
    } finally {
      setMarketLoading(false);
    }
  }

  async function changeMarketLayout(market: Market, layout: "root" | "subdir") {
    if (market.layout === layout) return;
    setMarketLoading(true);
    try {
      await api.updateMarket(market.id, market.provider, market.owner, market.name, market.branch, market.enabled, layout);
      await loadMarkets();
      addToast(
        "success",
        `${market.owner}/${market.name}: ${t("marketLayout")} → ${
          layout === "root" ? t("layoutRoot") : t("layoutSubdir")
        }`,
      );
    } catch (e) {
      addToast("error", `${t("editMarket")} failed: ${e}`);
    } finally {
      setMarketLoading(false);
    }
  }

  async function deleteMarket(market: Market) {
    setMarketLoading(true);
    try {
      await api.deleteMarket(market.id);
      addToast("success", `${t("deleteMarket")}: ${market.id}`);
      setMarketSyncErrors((prev) => {
        if (!(market.id in prev)) return prev;
        const next = { ...prev };
        delete next[market.id];
        return next;
      });
      await loadMarkets();
      if (selectedMarketFilter !== "all" && Number(selectedMarketFilter) === market.id) {
        setSelectedMarketFilter("all");
      }
    } catch (e) {
      addToast("error", `${t("deleteMarket")} failed: ${e}`);
    } finally {
      setMarketLoading(false);
    }
  }

  async function syncMarketIndex(market: Market) {
    setMarketLoading(true);
    try {
      const result = await api.syncMarketIndex(market.id);
      setMarketSyncErrors((prev) => ({ ...prev, [market.id]: result.errors }));
      if (result.skills_found === 0 && result.errors.length === 0) {
        addToast("info", `${market.owner}/${market.name}: ${t("repoNoSkills")}`);
      } else if (result.errors.length > 0) {
        const first = result.errors[0] ?? "";
        addToast("error", `${market.owner}/${market.name}: ${t("repoSyncFailed").replace("{0}", first)}`);
      } else {
        addToast("success", `${t("syncMarketIndex")}: ${result.skills_found}`);
      }
      await Promise.all([loadMarkets(), loadRemoteSkills(market.id)]);
    } catch (e) {
      setMarketSyncErrors((prev) => ({ ...prev, [market.id]: [`${e}`] }));
      addToast("error", `${t("syncMarketIndex")} failed: ${e}`);
    } finally {
      setMarketLoading(false);
    }
  }

  async function loadRemoteSkills(marketId?: number) {
    setSkillLoading(true);
    try {
      setRemoteSkills(await api.listRemoteSkills(marketId ?? null));
    } catch (e) {
      addToast("error", `${t("failedLoadSkills")}: ${e}`);
    } finally {
      setSkillLoading(false);
    }
  }

  async function scanAllRemoteRepositories() {
    setRemoteScanLoading(true);
    try {
      const results = await api.scanAllRemoteRepositories();
      const total = results.reduce((sum, item) => sum + item.skills_found, 0);
      const allErrors = results.reduce<string[]>((acc, item) => acc.concat(item.errors ?? []), []);
      if (allErrors.length > 0) {
        const shown = allErrors.slice(0, 3).join("; ");
        addToast("error", `${t("scanErrors")}: ${shown}${allErrors.length > 3 ? " …" : ""}`);
      }
      addToast("success", `${t("scanComplete")}: ${total}`);
      await loadRemoteSkills(selectedMarketFilter === "all" ? undefined : Number(selectedMarketFilter));
    } catch (e) {
      addToast("error", `${t("scanAllRemote")} failed: ${e}`);
    } finally {
      setRemoteScanLoading(false);
    }
  }

  async function checkRemoteUpdates() {
    setRemoteCheckLoading(true);
    try {
      const commitUpdates: { market_id: number; market_title: string; last_commit_sha: string | null; new_commit_sha: string }[] = await api.checkMarketCommits();
      const changedIds = commitUpdates.map((c) => c.market_id);
      if (changedIds.length === 0) {
        addToast("info", t("noNewCommits"));
      } else {
        addToast("info", t("newCommitsFound").replace("{0}", String(changedIds.length)));
        for (const id of changedIds) {
          await api.syncMarketIndex(id);
        }
        await loadRemoteSkills(selectedMarketFilter === "all" ? undefined : Number(selectedMarketFilter));
      }

      const marketId = selectedMarketFilter === "all" ? null : Number(selectedMarketFilter);
      const result =
        remoteUpdateMode === "installed"
          ? await api.checkRemoteSsoUpdates(marketId, marketId)
          : await api.checkRemoteUpdates(marketId, marketId);
      setUpdates(result);
      if (result.length > 0) {
        setShowUpdatesModal(true);
      }
      if (result.length > 0) {
        addToast("info", t("remoteUpdateFound").replace("{0}", String(result.length)));
      } else {
        addToast("info", t("remoteNoUpdate"));
      }
    } catch (e) {
      addToast("error", `${t("checkRemoteUpdates")} failed: ${e}`);
    } finally {
      setRemoteCheckLoading(false);
    }
  }

  async function syncRemoteSkillToTools(skill: RemoteSkill) {
    setInstallLoading(true);
    try {
      const result = await api.syncRemoteSkillToTools(skill.id, installProjectId, selectedToolPath);
      if (result.errors.length > 0) {
        addToast("error", `${skill.skill_name}: ${result.errors.join(", ")}`);
      } else {
        addToast("success", `${t("syncToTools")}: ${result.synced_to}`);
      }
      onSkillsChanged();
    } catch (e) {
      addToast("error", `${t("syncToTools")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  function requestSyncAllInstalledRemoteSkills() {
    const tool = tools.find((item) => item.globalPath === selectedToolPath);
    const target = tool?.name ?? selectedToolPath;
    showConfirm({
      title: t("confirmSyncAllTitle"),
      message: t("confirmSyncAllMessage").replace("{0}", target),
      confirmText: t("syncAllActive"),
      cancelText: t("cancel"),
      confirmVariant: "primary",
      onConfirm: () => runSyncAllInstalled(target),
    });
  }

  async function runSyncAllInstalled(_target: string) {
    const marketId = selectedMarketFilter === "all" ? null : Number(selectedMarketFilter);
    const unlisten = await listen<{ completed: number; total: number; current?: string }>(
      "market:sync-progress",
      (event) => setProgress(event.payload),
    );
    try {
      await api.syncRemoteInstallationsToTools(installProjectId, marketId, selectedToolPath);
      onSkillsChanged();
    } catch (e) {
      addToast("error", `${t("syncAllActive")} failed: ${e}`);
    } finally {
      setProgress(null);
      unlisten();
    }
  }

  function requestDeleteMarket(market: Market) {
    const skillCount = remoteSkills.filter((s) => s.market_id === market.id).length;
    const detail = skillCount > 0
      ? `${market.owner}/${market.name} · ${market.branch} · ${t("skillsCount").replace("{0}", String(skillCount))}`
      : `${market.owner}/${market.name} · ${market.branch}`;
    showConfirm({
      title: t("confirmDeleteMarketTitle"),
      message: t("confirmDeleteMarketMessage"),
      detail,
      confirmText: t("deleteMarket"),
      cancelText: t("cancel"),
      confirmVariant: "danger",
      onConfirm: () => deleteMarket(market),
    });
  }

  async function markAllRemoteSkillsInstalled(active: boolean) {
    const marketId = selectedMarketFilter === "all" ? null : Number(selectedMarketFilter);
    setInstallLoading(true);
    try {
      const result = await api.setAllRemoteSkillsInstalled(installProjectId, marketId, active);
      addToast("success", `${t(active ? "markAllInstalled" : "unmarkAllInstalled")}: ${result.updated}`);
      await loadRemoteSkills(marketId == null ? undefined : Number(marketId));
      onRemoteInstallationsChanged(
        await api.listRemoteInstallations(installProjectId, marketId),
      );
    } catch (e) {
      addToast("error", `${t(active ? "markAllInstalled" : "unmarkAllInstalled")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  function requestMarkAllRemoteSkillsInstalled(active: boolean) {
    const filteredMarket = markets.find((m) => String(m.id) === selectedMarketFilter);
    const scopeLine = selectedMarketFilter === "all" || !filteredMarket
      ? t("confirmScopeAll")
      : t("confirmScopeMarket").replace("{0}", marketTitle(filteredMarket));
    showConfirm({
      title: t(active ? "confirmMarkAllInstalledTitle" : "confirmUnmarkAllInstalledTitle"),
      message: t(active ? "confirmMarkAllInstalledMessage" : "confirmUnmarkAllInstalledMessage"),
      detail: scopeLine,
      confirmText: t(active ? "markAllInstalled" : "unmarkAllInstalled"),
      cancelText: t("cancel"),
      confirmVariant: "danger",
      onConfirm: () => markAllRemoteSkillsInstalled(active),
    });
  }

  const BUILTIN_LABELS: Record<string, string> = {
    "anthropic-skills": "Anthropic Skills",
    "superpowers-skills": "Superpowers Skills",
    "mattpocock-skills": "Matt Pocock Skills",
    "andrej-karpathy-skill": "Andrej Karpathy Skill",
    "khazix-skills": "Khazix Skills",
  };

  function marketTitle(m: Market): string {
    return BUILTIN_LABELS[String(m.id)] ?? `${m.owner}/${m.name}`;
  }

  const updateKeys = useMemo(
    () => new Set((updates ?? []).map((u) => `${u.market_id}:${u.skill_name}`)),
    [updates],
  );

  const visibleSkills = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return remoteSkills.filter((skill) => {
      // Sidebar selection: single source
      if (selectedMarketFilter !== "all" && String(skill.market_id) !== selectedMarketFilter)
        return false;
      // Multi-select filter chips (only active when sidebar = "all")
      if (selectedMarketFilter === "all" && selectedFilterMarkets.size > 0) {
        if (!selectedFilterMarkets.has(skill.market_id)) return false;
      }
      if (installedOnly && !skill.is_installed) return false;
      if (q && !skill.skill_name.toLowerCase().includes(q) && !(skill.description || "").toLowerCase().includes(q))
        return false;
      return true;
    });
  }, [remoteSkills, selectedMarketFilter, selectedFilterMarkets, searchQuery, installedOnly]);

  async function handleInstallConfirm(projectId: number, toolPath: string, remember: boolean) {
    if (!installTarget) return;
    if (remember) {
      localStorage.setItem("market.install.projectId", String(projectId));
      localStorage.setItem("market.install.toolPath", toolPath);
    }
    const tool = tools.find((item) => item.globalPath === toolPath);
    const actualPath =
      projectId === 0 || !tool
        ? toolPath
        : `${projectPaths[projectId] ?? ""}/${tool.projectRelPath}`.replace(/\/+/g, "/");
    setInstallLoading(true);
    try {
      await api.downloadRemoteSkillToSsot(installTarget.id);
      const result = await api.syncRemoteSkillToTools(installTarget.id, projectId, actualPath);
      if (result.errors.length > 0) {
        addToast("error", `${installTarget.skill_name}: ${result.errors.join(", ")}`);
      } else {
        const toolName = tool?.name ?? toolPath;
        const scope = projectId === 0
          ? t("scopeGlobal")
          : `${t("scopeProject")} · ${projects.find((p) => p.id === projectId)?.name ?? `#${projectId}`}`;
        // Two-line toast: success line, then a hint about where the skill
        // is now visible. Keeps the existing installedToast wording so
        // translations stay in sync.
        addToast(
          "success",
          t("installedToast").replace("{0}", installTarget.skill_name).replace("{1}", toolName),
        );
        addToast(
          "info",
          `${installTarget.skill_name} → ${scope}`,
        );
      }
      setInstallTarget(null);
      await loadRemoteSkills(selectedMarketFilter === "all" ? undefined : Number(selectedMarketFilter));
      onRemoteInstallationsChanged(
        await api.listRemoteInstallations(installProjectId, selectedMarketFilter === "all" ? null : Number(selectedMarketFilter)),
      );
      // Surface the skill in the global/project SkillView (sync_remote_skill_to_tools
      // already upserts the skills row, but App.tsx owns the cached list).
      onSkillsChanged();
    } catch (e) {
      addToast("error", `${t("install")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  async function updateOne(skill: RemoteSkill) {
    const savedProject = Number(localStorage.getItem("market.install.projectId") ?? 0);
    const savedTool = localStorage.getItem("market.install.toolPath") ?? resolvedToolPaths[0]?.path ?? "";
    setInstallLoading(true);
    try {
      await api.downloadRemoteSkillToSsot(skill.id);
      await api.syncRemoteSkillToTools(skill.id, savedProject, savedTool);
      addToast("success", `${t("updateBtn")}: ${skill.skill_name}`);
      await loadRemoteSkills(selectedMarketFilter === "all" ? undefined : Number(selectedMarketFilter));
      onSkillsChanged();
    } catch (e) {
      addToast("error", `${t("updateBtn")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  const updateCount = (updates ?? []).length;

  return (
    <section className="market-page">
      <div className="market-layout">
        {/* ── Left Sidebar ── */}
        <MarketSidebar
          markets={markets}
          remoteSkills={remoteSkills}
          selectedMarketFilter={selectedMarketFilter}
          onSelectFilter={(f) => {
            setSelectedMarketFilter(f);
            loadRemoteSkills(f === "all" ? undefined : Number(f));
          }}
          onAddSource={() => setShowSourcesModal(true)}
          marketTitle={marketTitle}
          t={t}
        />

        {/* ── Main Content ── */}
        <div className="m-main">
          {/* Top action bar */}
          <div className="m-topbar">
            <div className="m-search-box">
              <Icon name="search" size={14} className="m-search-icon" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={t("marketSearchPlaceholder")}
                className="m-search-input"
              />
            </div>
            <button
              className="btn btn-secondary m-actions-btn"
              onClick={() => setDrawerOpen((v) => !v)}
            >
              <Icon name="sliders-horizontal" size={14} />
              {t("actionsToggle")}
              <Icon name={drawerOpen ? "chevron-up" : "chevron-down"} size={12} />
            </button>
            <button
              className="btn btn-primary"
              onClick={checkRemoteUpdates}
              disabled={remoteCheckLoading}
            >
              <Icon name="refresh-cw" size={14} className={remoteCheckLoading ? "sync-spinner" : ""} />
              {remoteCheckLoading ? t("checking") : t("checkRemoteUpdates")}
              {updateCount > 0 && <span className="update-badge">{updateCount}</span>}
            </button>
          </div>

          {/* Collapsible actions drawer */}
          <div className={`m-actions-drawer ${drawerOpen ? "open" : ""}`}>
            <div className="m-actions-inner">
              <button className="btn btn-secondary btn-small" onClick={scanAllRemoteRepositories} disabled={remoteScanLoading}>
                <Icon name="scan" size={14} />
                {remoteScanLoading ? t("scanning") : t("actionsReindexAll")}
              </button>
              <button className="btn btn-secondary btn-small" onClick={requestSyncAllInstalledRemoteSkills} disabled={installLoading}>
                <Icon name="download" size={14} />
                {t("actionsSyncAll")}
              </button>
              <button className="btn btn-secondary btn-small" onClick={() => requestMarkAllRemoteSkillsInstalled(true)} disabled={installLoading}>
                <Icon name="check-circle" size={14} />
                {t("actionsMarkAll")}
              </button>
              <button className="btn btn-secondary btn-small" onClick={() => requestMarkAllRemoteSkillsInstalled(false)} disabled={installLoading}>
                <Icon name="x-circle" size={14} />
                {t("actionsUnmarkAll")}
              </button>
              <button className="btn btn-secondary btn-small" onClick={() => setShowSourcesModal(true)}>
                <Icon name="settings" size={14} />
                {t("actionsManageSources")}
              </button>
            </div>
          </div>

          {/* Filter bar (visible when sidebar = "all") */}
          {selectedMarketFilter === "all" && (
            <div className="m-filter-bar">
              <span className="m-filter-label">
                <Icon name="filter" size={12} />
                {t("filterLabel")}
              </span>
              {markets.filter((m) => m.enabled).map((m) => {
                const isOn = selectedFilterMarkets.has(m.id);
                return (
                  <button
                    key={m.id}
                    className={`m-fchip ${isOn ? "on" : ""}`}
                    onClick={() => {
                      setSelectedFilterMarkets((prev) => {
                        const next = new Set(prev);
                        if (next.has(m.id)) next.delete(m.id);
                        else next.add(m.id);
                        return next;
                      });
                    }}
                  >
                    {isOn && <Icon name="check" size={12} className="ck" />}
                    {marketTitle(m).split("/")[0]}
                  </button>
                );
              })}
              <span className="m-filter-divider" />
              <button
                className={`m-fchip ${installedOnly ? "on" : ""}`}
                onClick={() => setInstalledOnly((v) => !v)}
              >
                {installedOnly && <Icon name="check" size={12} className="ck" />}
                {t("filterInstalledOnly")}
              </button>
            </div>
          )}

          {/* Source header (visible when single source selected) */}
          {selectedMarketFilter !== "all" && (() => {
            const market = markets.find((m) => String(m.id) === selectedMarketFilter);
            if (!market) return null;
            const count = remoteSkills.filter((s) => s.market_id === market.id).length;
            const initial = marketTitle(market).charAt(0).toUpperCase();
            return (
              <div className="m-source-header">
                <div className="m-source-header-avatar">{initial}</div>
                <div className="m-source-header-info">
                  <h2>{marketTitle(market)}</h2>
                  <p>
                    <Icon name="git-branch" size={12} />
                    {t("sourceHeaderBranch").replace("{0}", market.branch)} · {t("sourceHeaderSkills").replace("{0}", String(count))}
                    {market.last_indexed_at && (
                      <>
                        {" · "}<Icon name="clock" size={12} />
                        {t("sourceHeaderIndexed").replace("{0}", market.last_indexed_at)}
                      </>
                    )}
                  </p>
                </div>
                <div className="m-source-header-actions">
                  <button className="btn btn-secondary btn-small" onClick={() => syncMarketIndex(market)} disabled={marketLoading || !market.enabled}>
                    <Icon name="refresh-cw" size={14} />
                    {t("sourceHeaderSync")}
                  </button>
                  <button className="btn btn-secondary btn-small" onClick={() => toggleMarket(market)} disabled={marketLoading}>
                    <Icon name="power" size={14} />
                    {market.enabled ? t("sourceHeaderDisable") : t("sourceHeaderEnable")}
                  </button>
                  {!BUILTIN_MARKET_IDS.has(String(market.id)) && (
                    <button className="btn btn-danger btn-small" onClick={() => requestDeleteMarket(market)} disabled={marketLoading}>
                      <Icon name="trash" size={14} />
                      {t("sourceHeaderDelete")}
                    </button>
                  )}
                </div>
              </div>
            );
          })()}

          {/* Skill grid */}
          <div className="m-grid-wrap">
            {skillLoading ? (
              <div className="skeleton-grid">
                {[1, 2, 3, 4, 5, 6].map((i) => <div key={i} className="skeleton-card" />)}
              </div>
            ) : visibleSkills.length === 0 ? (
              <div className="m-empty-state">
                <Icon name="search" size={48} />
                <p>{searchQuery || installedOnly || selectedFilterMarkets.size > 0
                  ? t("emptyNoMatch")
                  : t("emptyNoSkills")}</p>
              </div>
            ) : (
              <div className="skill-grid">
                {visibleSkills.map((skill) => {
                  const hasUpdate = updateKeys.has(`${skill.market_id}:${skill.skill_name}`);
                  const market = markets.find((m) => m.id === skill.market_id);
                  return (
                    <div
                      key={skill.id}
                      className={`skill-card ${hasUpdate ? "skill-has-update" : ""}`}
                      role="button"
                      tabIndex={0}
                      aria-label={t("openDetailAria").replace("{0}", skill.skill_name)}
                      onClick={() => setDetailSkill(skill)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          setDetailSkill(skill);
                        }
                      }}
                    >
                      <div className="skill-header">
                        <h3 className="skill-name">{skill.skill_name}</h3>
                        {selectedMarketFilter === "all" && market && (
                          <span className="m-card-src-badge">
                            {marketTitle(market).split("/")[0]}
                          </span>
                        )}
                      </div>
                      <p className="market-card-desc">{skill.description || ""}</p>
                      <div className="market-card-footer" onClick={(e) => e.stopPropagation()}>
                        {!skill.is_installed ? (
                          <button className="btn btn-small btn-primary btn-press" onClick={() => setInstallTarget(skill)} disabled={installLoading}>
                            {t("install")}
                          </button>
                        ) : (
                          <>
                            {hasUpdate && (
                              <button className="btn btn-small btn-primary btn-press" onClick={() => updateOne(skill)} disabled={installLoading}>
                                {t("updateBtn")}
                              </button>
                            )}
                            <span className="market-installed-tag">
                              <Icon name="check" size={12} />
                              {t("installedTag")}
                            </span>
                            <button className="btn btn-small btn-secondary" onClick={() => syncRemoteSkillToTools(skill)} disabled={installLoading}>
                              {t("syncBtn")}
                            </button>
                          </>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* ── Modals (unchanged) ── */}
      {installTarget && (() => {
        const installMarket = markets.find((m) => m.id === installTarget.market_id);
        return (
          <InstallDialog
            skill={installTarget}
            marketTitle={installMarket ? marketTitle(installMarket) : ""}
            projects={projects}
            tools={tools}
            t={t}
            loading={installLoading}
            onConfirm={handleInstallConfirm}
            onClose={() => setInstallTarget(null)}
          />
        );
      })()}

      {/* 市场源管理弹窗（任务 7 实现，先占位） */}
      {showSourcesModal && (
        <MarketSourcesModal
          t={t}
          markets={markets}
          loading={marketLoading}
          skillCounts={remoteSkills}
          marketErrors={marketSyncErrors}
          onAddToggle={() => setShowAddMarket((v) => !v)}
          showAdd={showAddMarket}
          marketUrl={marketUrl}
          setMarketUrl={setMarketUrl}
          marketBranch={marketBranch}
          setMarketBranch={setMarketBranch}
          marketLayout={marketLayout}
          setMarketLayout={setMarketLayout}
          onAddByUrl={handleAddMarketByUrl}
          onToggle={toggleMarket}
          onDeleteRequest={requestDeleteMarket}
          onSyncIndex={syncMarketIndex}
          onChangeLayout={changeMarketLayout}
          builtinIds={BUILTIN_MARKET_IDS}
          builtinLabels={BUILTIN_LABELS}
          onClose={() => setShowSourcesModal(false)}
        />
      )}

      {/* 更新弹窗（任务 7 实现） */}
      {showUpdatesModal && updates && updates.length > 0 && (
        <RemoteUpdatesModal
          t={t}
          updates={updates}
          marketTitles={marketTitles}
          loading={installLoading}
          onUpdateOne={(skillName, marketId) => {
            const skill = remoteSkills.find((s) => s.market_id === marketId && s.skill_name === skillName);
            if (skill) updateOne(skill);
          }}
          onClose={() => setShowUpdatesModal(false)}
        />
      )}

      {/* T4: Remote skill detail Modal. Opened by clicking a card. */}
      {detailSkill && (() => {
        const detailMarket = markets.find((m) => m.id === detailSkill.market_id);
        return (
          <RemoteSkillDetailModal
            t={t}
            skill={detailSkill}
            marketTitle={detailMarket ? marketTitle(detailMarket) : ""}
            isInstalled={detailSkill.is_installed}
            installedAt={detailSkill.installed_at}
            addToast={addToast}
            onClose={() => setDetailSkill(null)}
            onInstall={() => {
              setDetailSkill(null);
              setInstallTarget(detailSkill);
            }}
          />
        );
      })()}
    </section>
  );
}
