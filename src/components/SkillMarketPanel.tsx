// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useMemo, useState } from "react";
import * as api from "../api";
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

  const [marketsCollapsed, setMarketsCollapsed] = useState(
    () => localStorage.getItem("marketsCollapsed") === "1",
  );
  const [showAddMarket, setShowAddMarket] = useState(false);
  const [marketUrl, setMarketUrl] = useState("");
  const [marketBranch, setMarketBranch] = useState("");

  const [installProjectId, setInstallProjectId] = useState<number>(defaultProjectId || 0);
  const [installScope, setInstallScope] = useState<"global" | "project">("global");
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
  }, [installProjectId, installScope, selectedMarketFilter]);

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

  function toggleMarkets() {
    setMarketsCollapsed((c) => {
      localStorage.setItem("marketsCollapsed", c ? "0" : "1");
      return !c;
    });
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

  async function installRemoteSkill(skill: RemoteSkill) {
    setInstallLoading(true);
    try {
      await api.downloadRemoteSkillToSsot(skill.id);
      addToast("success", `${t("downloadToSsot")}: ${skill.skill_name}`);
      await Promise.all([loadRemoteSkills(skill.market_id)]);
      onRemoteInstallationsChanged(
        await api.listRemoteInstallations(installProjectId, selectedMarketFilter === "all" ? null : Number(selectedMarketFilter)),
      );
    } catch (e) {
      addToast("error", `${t("downloadToSsot")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
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

  const filteredRemoteSkills = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return remoteSkills.filter((skill) => {
      if (selectedMarketFilter !== "all" && String(skill.market_id) !== selectedMarketFilter) return false;
      if (q && !skill.skill_name.toLowerCase().includes(q) && !(skill.remote_url || "").toLowerCase().includes(q)) return false;
      return true;
    });
  }, [remoteSkills, selectedMarketFilter, searchQuery]);

  const updateCount = (updates ?? []).length;

  return (
    <section className="section">
      <h2 className="section-title">{t("market")}</h2>

      <div className="section">
        <div className="section-header">
          <h3
            className="section-title section-title-toggle"
            onClick={toggleMarkets}
            title={marketsCollapsed ? t("expand") : t("collapse")}
          >
            <span className="collapse-chevron">{marketsCollapsed ? "▸" : "▾"}</span>
            {t("marketTab")}
            {!marketsCollapsed && <span className="collapsed-count">({markets.length})</span>}
          </h3>
          <button
            className="btn btn-small"
            onClick={() => setShowAddMarket((prev) => !prev)}
          >
            {showAddMarket ? t("cancel") : t("addMarket")}
          </button>
        </div>

        {!marketsCollapsed && (
          <>
            {showAddMarket && (
              <div className="add-market-form" style={{ marginBottom: 10 }}>
                <div className="form-row">
                  <input
                    className="search-input"
                    value={marketUrl}
                    onChange={(e) => setMarketUrl(e.target.value)}
                    placeholder={t("addMarketUrlPlaceholder")}
                  />
                  <select
                    className="sort-select"
                    value={marketBranch}
                    onChange={(e) => setMarketBranch(e.target.value)}
                    aria-label={t("defaultBranchAuto")}
                  >
                    <option value="">{t("defaultBranchAuto")}</option>
                    <option value="main">main</option>
                    <option value="master">master</option>
                  </select>
                  <button className="btn btn-primary" onClick={handleAddMarketByUrl} disabled={marketLoading}>
                    {t("addMarket")}
                  </button>
                </div>
                <div style={{ color: "var(--text-muted)", fontSize: 12, marginTop: 6 }}>
                  {t("addMarketUrlHint")}
                </div>
              </div>
            )}

            <div className="market-select-field">
              <label className="market-select-label" htmlFor="market-filter">{t("marketTab")}</label>
              <select
                className="sort-select market-select"
                id="market-filter"
                aria-label={t("marketTab")}
                value={selectedMarketFilter}
                onChange={(e) => {
                  const next = e.target.value;
                  setSelectedMarketFilter(next);
                  const marketId = next === "all" ? undefined : Number(next);
                  loadRemoteSkills(marketId);
                }}
              >
                <option value="all">{t("allMarkets")}</option>
                {markets.map((market) => {
                  const title = marketTitles[market.id] ?? `${market.owner}/${market.name}`;
                  const suffix = market.branch ? ` (${market.branch})` : "";
                  return (
                    <option key={market.id} value={market.id}>
                      {title}{suffix}
                      {BUILTIN_MARKET_IDS.has(String(market.id)) ? " [builtin]" : ""}
                    </option>
                  );
                })}
              </select>
              {selectedMarketFilter !== "all" && (() => {
                const market = markets.find((item) => item.id === Number(selectedMarketFilter));
                if (!market || BUILTIN_MARKET_IDS.has(String(market.id))) return null;
                return (
                  <button
                    className="btn btn-small btn-danger"
                    onClick={() => deleteMarket(market)}
                    disabled={marketLoading}
                  >
                    {t("deleteMarket")}
                  </button>
                );
              })()}
            </div>

            <div className="skill-list">
              <table className="skill-table">
                <thead>
                  <tr>
                    <th>provider</th>
                    <th>owner</th>
                    <th>name</th>
                    <th>branch</th>
                    <th>enabled</th>
                    <th>{t("lastIndexedAt")}</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {markets.length === 0 ? (
                    <tr>
                      <td colSpan={7} className="empty-state">{t("marketEmpty")}</td>
                    </tr>
                  ) : (
                    markets.map((m) => {
                      return (
                        <tr key={m.id}>
                          <td>{m.provider}</td>
                          <td>{m.owner}</td>
                          <td>{m.name}</td>
                          <td>{m.branch}</td>
                          <td>{m.enabled ? t("enable") : t("disable")}</td>
                          <td>{m.last_indexed_at ?? "-"}</td>
                          <td>
                            <button className="btn btn-small btn-secondary" onClick={() => toggleMarket(m)} disabled={marketLoading}>
                              {m.enabled ? t("disable") : t("enable")}
                            </button>
                            <button className="btn btn-small btn-secondary" onClick={() => syncMarketIndex(m)} disabled={marketLoading || !m.enabled}>
                              {t("syncMarketIndex")}
                            </button>
                          </td>
                        </tr>
                      );
                    })
                  )}
                </tbody>
              </table>
            </div>
          </>
        )}
      </div>

      <div className="section">
        <div className="action-bar">
          <div className="search-box">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t("searchPlaceholder")}
              className="search-input"
            />
          </div>
          <button className="btn btn-secondary" onClick={checkRemoteUpdates} disabled={remoteCheckLoading}>
            {remoteCheckLoading ? t("checking") : t("checkRemoteUpdates")}
          </button>
          <button className="btn btn-secondary" onClick={scanAllRemoteRepositories} disabled={remoteScanLoading}>
            {remoteScanLoading ? t("scanning") : t("scanAllRemote")}
          </button>
          <select
            className="sort-select"
            value={remoteUpdateMode}
            onChange={(e) => setRemoteUpdateMode(e.target.value)}
          >
            <option value="installed">{t("installed")}</option>
            <option value="all">{t("allMarkets")}</option>
          </select>
        </div>

        <div className="action-bar" style={{ marginTop: 10 }}>
          <select
            className="sort-select"
            value={installProjectId}
            onChange={(e) => setInstallProjectId(Number(e.target.value))}
          >
            <option value={0}>{t("global")}</option>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>{project.name}</option>
            ))}
            <option value="-1">{t("projects")}</option>
          </select>
          <select
            className="sort-select"
            value={installScope}
            onChange={(e) => setInstallScope(e.target.value === "global" ? "global" : "project")}
          >
            <option value="global">{t("global")}</option>
            <option value="project">{t("projects")}</option>
          </select>
          <select
            className="sort-select"
            value={selectedToolPath}
            onChange={(e) => setSelectedToolPath(e.target.value)}
            disabled={resolvedToolPaths.length === 0}
          >
            {resolvedToolPaths.length === 0 ? (
              <option value="">{t("noSkillsYet")}</option>
            ) : (
              resolvedToolPaths.map((option) => (
                <option key={option.toolId} value={option.path}>{option.name}</option>
              ))
            )}
          </select>
          <button className="btn btn-primary" onClick={syncAllInstalledRemoteSkills} disabled={installLoading}>
            {t("syncAllActive")}
          </button>
          <button className="btn btn-secondary" onClick={() => markAllRemoteSkillsInstalled(true)} disabled={installLoading || marketLoading}>
            {t("markAllInstalled")}
          </button>
          <button className="btn btn-secondary" onClick={() => markAllRemoteSkillsInstalled(false)} disabled={installLoading || marketLoading}>
            {t("unmarkAllInstalled")}
          </button>
        </div>
      </div>

      <div className="section">
        <h3 className="section-title">
          {t("skills")} {filteredRemoteSkills.length > 0 && <span className="badge">{filteredRemoteSkills.length}</span>}
          {updateCount > 0 && <span className="badge badge-update">{updateCount} {t("updates")}</span>}
        </h3>

        {skillLoading ? (
          <div className="empty-state">{t("scanning")}</div>
        ) : filteredRemoteSkills.length === 0 ? (
          <div className="empty-state">{t("noMatch")}</div>
        ) : (
          <div className="skill-list">
            <table className="skill-table">
              <thead>
                <tr>
                  <th>{t("skills")}</th>
                  <th>{t("remoteUrl")}</th>
                  <th>{t("ssotPath")}</th>
                  <th>{t("remoteInstalled")}</th>
                  <th>{t("lastIndexedAt")}</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {filteredRemoteSkills.map((skill) => (
                  <tr key={skill.id}>
                    <td>{skill.skill_name}</td>
                    <td>{skill.remote_url}</td>
                    <td>{skill.ssot_path}</td>
                    <td>{skill.is_installed ? t("remoteInstalled") : t("remoteNotInstalled")}</td>
                    <td>{skill.installed_at ?? "-"}</td>
                    <td>
                      {!skill.is_installed && (
                        <button className="btn btn-small btn-secondary" onClick={() => installRemoteSkill(skill)} disabled={installLoading}>
                          {t("downloadToSsot")}
                        </button>
                      )}
                      <button className="btn btn-small btn-secondary" onClick={() => syncRemoteSkillToTools(skill)} disabled={installLoading || !skill.is_installed}>
                        {skill.is_installed ? t("syncToTools") : t("downloadToSsot")}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="section">
        <h3 className="section-title">
          {t("checkRemoteUpdates")} {updateCount > 0 && <span className="badge badge-update">{updateCount}</span>}
        </h3>
        {(!updates || updates.length === 0) ? (
          <div className="empty-state">{t("remoteNoUpdate")}</div>
        ) : (
          <div className="skill-list">
            <table className="skill-table">
              <thead>
                <tr>
                  <th>market</th>
                  <th>{t("skills")}</th>
                  <th>{t("remoteUrl")}</th>
                  <th>old</th>
                  <th>new</th>
                </tr>
              </thead>
              <tbody>
                {updates.map((update) => (
                  <tr key={update.id}>
                    <td>{marketTitles[update.market_id] ?? update.market_id}</td>
                    <td>{update.skill_name}</td>
                    <td>{update.remote_url}</td>
                    <td>{update.old_hash.slice(0, 12)}</td>
                    <td>{update.new_hash.slice(0, 12)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </section>
  );
}
