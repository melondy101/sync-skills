// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useMemo, useState } from "react";
import * as api from "../api";
import InstallDialog from './InstallDialog';
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
      const market = await api.addMarketByUrl(url, branch || undefined);
      addToast("success", `${t("addMarket")}: ${market.owner}/${market.name} (${market.branch})`);
      setMarketUrl("");
      setMarketBranch("");
      setShowAddMarket(false);
      await loadMarkets();
    } catch (e) {
      addToast("error", `${t("addMarket")} failed: ${e}`);
    } finally {
      setMarketLoading(false);
    }
  }

  async function toggleMarket(market: Market) {
    setMarketLoading(true);
    try {
      await api.updateMarket(market.id, market.provider, market.owner, market.name, market.branch, !market.enabled);
      await loadMarkets();
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
      addToast("success", `${t("syncMarketIndex")}: ${result.skills_found}`);
      await Promise.all([loadMarkets(), loadRemoteSkills(market.id)]);
    } catch (e) {
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
    } catch (e) {
      addToast("error", `${t("syncToTools")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  async function syncAllInstalledRemoteSkills() {
    setInstallLoading(true);
    try {
      const marketId = selectedMarketFilter === "all" ? null : Number(selectedMarketFilter);
      const result = await api.syncRemoteInstallationsToTools(installProjectId, marketId, selectedToolPath);
      if (result.errors.length > 0) {
        addToast("error", result.errors.join(", "));
      } else {
        addToast("success", `${t("syncAllActive")}: ${result.synced_to}`);
      }
    } catch (e) {
      addToast("error", `${t("syncAllFailed")}: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  async function markAllRemoteSkillsInstalled(active: boolean) {
    const marketId = selectedMarketFilter === "all" ? null : Number(selectedMarketFilter);
    const result = await api.setAllRemoteSkillsInstalled(installProjectId, marketId, active);
    addToast("success", `${t(active ? "markAllInstalled" : "unmarkAllInstalled")}: ${result.updated}`);
    await loadRemoteSkills(marketId == null ? undefined : Number(marketId));
    onRemoteInstallationsChanged(
      await api.listRemoteInstallations(installProjectId, marketId),
    );
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
      if (selectedMarketFilter !== "all" && String(skill.market_id) !== selectedMarketFilter) return false;
      if (installedOnly && !skill.is_installed) return false;
      if (q && !skill.skill_name.toLowerCase().includes(q) && !(skill.description || "").toLowerCase().includes(q)) return false;
      return true;
    });
  }, [remoteSkills, selectedMarketFilter, searchQuery, installedOnly]);

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
        addToast("success", t("installedToast").replace("{0}", installTarget.skill_name).replace("{1}", toolName));
      }
      setInstallTarget(null);
      await loadRemoteSkills(selectedMarketFilter === "all" ? undefined : Number(selectedMarketFilter));
      onRemoteInstallationsChanged(
        await api.listRemoteInstallations(installProjectId, selectedMarketFilter === "all" ? null : Number(selectedMarketFilter)),
      );
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
    } catch (e) {
      addToast("error", `${t("updateBtn")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  const updateCount = (updates ?? []).length;

  return (
    <section className="section">
      {/* 工具栏 */}
      <div className="market-toolbar">
        <div className="search-box">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t("marketSearchPlaceholder")}
            className="search-input"
            aria-label={t("marketSearchPlaceholder")}
          />
        </div>
        <button className="btn btn-secondary" onClick={() => setShowSourcesModal(true)}>
          {t("manageSources")}
        </button>
        <button className="btn btn-secondary" onClick={checkRemoteUpdates} disabled={remoteCheckLoading}>
          {remoteCheckLoading ? t("checking") : t("checkRemoteUpdates")}
          {updateCount > 0 && <span className="update-badge">{updateCount}</span>}
        </button>
        <details className="menu">
          <summary>{t("batchMenu")}</summary>
          <div className="menu-panel">
            <button className="menu-item" onClick={scanAllRemoteRepositories} disabled={remoteScanLoading}>
              {remoteScanLoading ? t("scanning") : t("reindexAllMarkets")}
            </button>
            <button className="menu-item" onClick={syncAllInstalledRemoteSkills} disabled={installLoading}>
              {t("syncAllActive")}
            </button>
            <button className="menu-item" onClick={() => markAllRemoteSkillsInstalled(true)} disabled={installLoading}>
              {t("markAllInstalled")}
            </button>
            <button className="menu-item" onClick={() => markAllRemoteSkillsInstalled(false)} disabled={installLoading}>
              {t("unmarkAllInstalled")}
            </button>
          </div>
        </details>
      </div>

      {/* 筛选 chips */}
      <div className="market-chips">
        <button
          className={`chip ${selectedMarketFilter === "all" ? "chip-active" : ""}`}
          onClick={() => { setSelectedMarketFilter("all"); loadRemoteSkills(undefined); }}
        >
          {t("filterAll")}
        </button>
        {markets.filter((m) => m.enabled).map((m) => (
          <button
            key={m.id}
            className={`chip ${String(selectedMarketFilter) === String(m.id) ? "chip-active" : ""}`}
            onClick={() => { setSelectedMarketFilter(String(m.id)); loadRemoteSkills(m.id); }}
          >
            {marketTitle(m)}
          </button>
        ))}
        <button
          className={`chip ${installedOnly ? "chip-active" : ""}`}
          onClick={() => setInstalledOnly((v) => !v)}
        >
          {t("filterInstalledOnly")}
        </button>
      </div>

      {/* 技能卡片网格 */}
      {skillLoading ? (
        <div className="skeleton-grid">
          {[1, 2, 3, 4, 5, 6].map((i) => <div key={i} className="skeleton-card" />)}
        </div>
      ) : visibleSkills.length === 0 ? (
        <div className="empty-state">
          <p>{searchQuery || installedOnly ? t("noMatch") : t("noSkillsIndexed")}</p>
        </div>
      ) : (
        <div className="skill-grid">
          {visibleSkills.map((skill) => {
            const hasUpdate = updateKeys.has(`${skill.market_id}:${skill.skill_name}`);
            const market = markets.find((m) => m.id === skill.market_id);
            return (
              <div key={skill.id} className={`skill-card ${hasUpdate ? "skill-has-update" : ""}`}>
                <div className="skill-header">
                  <h3 className="skill-name">{skill.skill_name}</h3>
                </div>
                <p className="market-card-desc">{skill.description || ""}</p>
                <div className="market-card-meta">
                  {market ? marketTitle(market) : ""}
                </div>
                <div className="market-card-footer">
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
                      <span className="market-installed-tag">{t("installedTag")}</span>
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

      {/* 安装对话框 */}
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
        // TODO(任务7)
        <div>MarketSourcesModal placeholder</div>
      )}

      {/* 更新弹窗（任务 7 实现） */}
      {showUpdatesModal && updates && updates.length > 0 && (
        // TODO(任务7)
        <div>RemoteUpdatesModal placeholder</div>
      )}
    </section>
  );
}
