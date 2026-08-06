// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useMemo, useState } from "react";
import * as api from "../api";
import type {
  Market,
  MarketTemplate,
  Project,
  RemoteInstallation,
  RemoteSkill,
  RemoteSkillUpdate,
  SkillDiff,
  Tool,
} from "../types";
import { DiffFilesView } from "./DiffView";

type ToolPathOption = {
  toolId: number;
  name: string;
  path: string;
};

type RemoteScanResult = {
  market_id: number;
  skills_found: number;
  skills_new: number;
  skills_updated: number;
  errors: string[];
};

type MarketTab = "markets" | "installs" | "skills";

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
  const [activeTab, setActiveTab] = useState<MarketTab>("markets");
  const [owner, setOwner] = useState("");
  const [repo, setRepo] = useState("");
  const [branch, setBranch] = useState("main");
  const [marketTemplateId, setMarketTemplateId] = useState("anthropic-skills");
  const [markets, setMarkets] = useState<Market[]>([]);
  const [templates, setTemplates] = useState<MarketTemplate[]>([]);
  const [marketTitles, setMarketTitles] = useState<Record<number, string>>({});
  const [templateDescription, setTemplateDescription] = useState("");
  const [marketLoading, setMarketLoading] = useState(false);

  const [installProjectId, setInstallProjectId] = useState<number>(defaultProjectId || 0);
  const [installScope, setInstallScope] = useState<"global" | "project">("global");
  const [installations, setInstallations] = useState<RemoteInstallation[]>([]);
  const [installLoading, setInstallLoading] = useState(false);

  const [remoteSkills, setRemoteSkills] = useState<RemoteSkill[]>([]);
  const [updates, setUpdates] = useState<RemoteSkillUpdate[]>([]);
  const [selectedDiff, setSelectedDiff] = useState<{ update: RemoteSkillUpdate; diff: SkillDiff } | null>(null);
  const [skillLoading, setSkillLoading] = useState(false);
  const [remoteCheckLoading, setRemoteCheckLoading] = useState(false);
  const [diffLoading, setDiffLoading] = useState(false);
  const [selectedMarketId, setSelectedMarketId] = useState<number | "all">("all");
  const [selectedToolPath, setSelectedToolPath] = useState<string>("");
  const [remoteScanResult, setRemoteScanResult] = useState<RemoteScanResult[] | null>(null);
  const [remoteScanLoading, setRemoteScanLoading] = useState(false);

  const selectedProjectPath = projectPaths[installProjectId] ?? "";

  const resolvedToolPaths = useMemo<ToolPathOption[]>(() => {
    const options: ToolPathOption[] = [];
    for (const tool of tools) {
      const path = installProjectId === 0 ? tool.globalPath : `${selectedProjectPath}/${tool.projectRelPath}`.replace(/\/+/g, "/");
      options.push({ toolId: tool.id, name: tool.name, path });
    }
    return options;
  }, [installProjectId, projectPaths, selectedProjectPath, tools]);

  const activeInstallations = useMemo(
    () => installations.filter((item) => item.projectId === installProjectId),
    [installProjectId, installations],
  );

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
    api.listMarketTemplates().then(setTemplates).catch(() => {});

    const saved = localStorage.getItem("skillMarketSelectedTemplate");
    if (saved && BUILTIN_MARKET_IDS.has(saved)) {
      setMarketTemplateId(saved);
    }
  }, []);

  useEffect(() => {
    localStorage.setItem("skillMarketSelectedTemplate", marketTemplateId);
    const template = templates.find((item) => item.id === marketTemplateId);
    if (template) {
      setOwner(template.owner);
      setRepo(template.name);
      setBranch(template.branch);
      setTemplateDescription(template.description);
    }
  }, [marketTemplateId, templates]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const markets = await api.listMarkets();
        const titles: Record<number, string> = {};
        for (const market of markets) {
          titles[market.id] = `${market.owner}/${market.name}`;
        }
        if (!cancelled) setMarketTitles(titles);
      } catch {
        // best-effort display only
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (activeTab === "installs") {
      loadInstallations();
    }
  }, [activeTab, installProjectId, installScope]);

  useEffect(() => {
    if (activeTab === "skills") {
      const marketId = selectedMarketId === "all" ? undefined : Number(selectedMarketId);
      loadRemoteSkills(marketId);
    }
  }, [activeTab, selectedMarketId]);

  async function loadMarkets() {
    setMarketLoading(true);
    try {
      setMarkets(await api.listMarkets());
    } catch (e) {
      addToast("error", `${t("failedLoadTools")}: ${e}`);
    } finally {
      setMarketLoading(false);
    }
  }

  async function scanAllRemoteRepositories() {
    setRemoteScanLoading(true);
    setRemoteScanResult(null);
    try {
      const results = await api.scanAllRemoteRepositories();
      const total = results.reduce((sum, item) => sum + item.skills_found, 0);
      addToast("success", `${t("scanComplete")}: ${total}`);
      setRemoteScanResult(results);
      await loadRemoteSkills(selectedMarketId === "all" ? undefined : Number(selectedMarketId));
    } catch (e) {
      addToast("error", `${t("scanAllRemote")} failed: ${e}`);
    } finally {
      setRemoteScanLoading(false);
    }
  }

  async function addMarket() {
    const resolvedOwner = owner.trim();
    const resolvedRepo = repo.trim();
    const resolvedBranch = branch.trim() || "main";
    if (!resolvedOwner || !resolvedRepo) {
      addToast("error", `${t("addMarket")}: owner/repo required`);
      return;
    }
    setMarketLoading(true);
    try {
      const market = await api.addMarket("github", resolvedOwner, resolvedRepo, resolvedBranch);
      addToast("success", `${t("addMarket")}: ${market.id}`);
      setOwner("");
      setRepo("");
      setBranch("main");
      setMarketTemplateId("");
      setTemplateDescription("");
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
      if (selectedMarketId === market.id) {
        setSelectedMarketId("all");
      }
      await loadRemoteSkills(selectedMarketId === "all" ? undefined : Number(selectedMarketId));
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

  async function syncAllMarketIndices() {
    setMarketLoading(true);
    try {
      const results = await api.syncAllMarketIndices();
      const total = results.reduce((sum, item) => sum + item.skills_found, 0);
      addToast("success", `${t("syncAllMarketIndices")}: ${total}`);
      await loadMarkets();
      await loadRemoteSkills(selectedMarketId === "all" ? undefined : Number(selectedMarketId));
    } catch (e) {
      addToast("error", `${t("syncAllMarketIndices")} failed: ${e}`);
    } finally {
      setMarketLoading(false);
    }
  }

  async function loadInstallations() {
    setInstallLoading(true);
    try {
      setInstallations(
        await api.listRemoteInstallations(
          installProjectId,
          selectedMarketId === "all" ? null : Number(selectedMarketId),
        ),
      );
    } catch (e) {
      addToast("error", `${t("failedLoadSkills")}: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  async function installRemoteSkill(skill: RemoteSkill) {
    setInstallLoading(true);
    try {
      await api.downloadRemoteSkillToSsot(skill.id);
      addToast("success", `${t("downloadToSsot")}: ${skill.skill_name}`);
      await Promise.all([loadRemoteSkills(skill.market_id), loadInstallations()]);
      onRemoteInstallationsChanged(
        await api.listRemoteInstallations(
          installProjectId,
          selectedMarketId === "all" ? null : Number(selectedMarketId),
        ),
      );
    } catch (e) {
      addToast("error", `${t("downloadToSsot")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  async function uninstallRemoteSkill(skill: RemoteSkill) {
    setInstallLoading(true);
    try {
      const current = installations.find((item) => item.remoteSkillId === skill.id && item.projectId === installProjectId && item.scope === installScope);
      if (!current) {
        addToast("error", t("toggleFailed"));
        return;
      }
      await api.toggleRemoteInstallation(skill.id, installProjectId, installScope, false);
      addToast("success", `${t("actionRemove")}: ${skill.skill_name}`);
      await Promise.all([loadRemoteSkills(skill.market_id), loadInstallations()]);
      onRemoteInstallationsChanged(
        await api.listRemoteInstallations(
          installProjectId,
          selectedMarketId === "all" ? null : Number(selectedMarketId),
        ),
      );
    } catch (e) {
      addToast("error", `${t("toggleFailed")}: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  async function syncInstalledRemoteSkill(skill: RemoteSkill) {
    setInstallLoading(true);
    try {
      const result = await api.syncRemoteSkillToTools(skill.id, installProjectId, selectedToolPath);
      if (result.errors.length > 0) {
        addToast("error", `${skill.skill_name}: ${result.errors.join(", ")}`);
      } else {
        addToast("success", `${t("syncToTools")}: ${result.synced_to}`);
      }
      await loadInstallations();
    } catch (e) {
      addToast("error", `${t("syncToTools")} failed: ${e}`);
    } finally {
      setInstallLoading(false);
    }
  }

  async function syncAllInstalledRemoteSkills() {
    setInstallLoading(true);
    try {
      const result = await api.syncRemoteInstallationsToTools(installProjectId, selectedMarketId === "all" ? null : Number(selectedMarketId), selectedToolPath);
      if (result.errors.length > 0) {
        addToast("error", result.errors.join(", "));
      } else {
        addToast("success", `${t("syncAllActive")}: ${result.synced_to}`);
      }
      await loadInstallations();
    } catch (e) {
      addToast("error", `${t("syncAllFailed")}: ${e}`);
    } finally {
      setInstallLoading(false);
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

  async function downloadRemoteSkill(skill: RemoteSkill) {
    setSkillLoading(true);
    try {
      await api.downloadRemoteSkillToSsot(skill.id);
      addToast("success", `${t("downloadToSsot")}: ${skill.skill_name}`);
      await loadRemoteSkills(skill.market_id);
      await checkRemoteUpdates(skill.market_id);
    } catch (e) {
      addToast("error", `${t("downloadToSsot")} failed: ${e}`);
    } finally {
      setSkillLoading(false);
    }
  }

  async function syncRemoteSkillToTools(skill: RemoteSkill) {
    setSkillLoading(true);
    try {
      const result = await api.syncRemoteSkillToTools(skill.id, installProjectId, selectedToolPath);
      if (result.errors.length > 0) {
        addToast("error", `${skill.skill_name}: ${result.errors.join(", ")}`);
      } else {
        addToast("success", `${t("syncToTools")}: ${result.synced_to}`);
      }
      await loadRemoteSkills(skill.market_id);
    } catch (e) {
      addToast("error", `${t("syncToTools")} failed: ${e}`);
    } finally {
      setSkillLoading(false);
    }
  }

  async function checkRemoteUpdates(marketId?: number) {
    setRemoteCheckLoading(true);
    try {
      const result = await api.checkRemoteUpdates(marketId ?? null);
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

  async function viewRemoteDiff(update: RemoteSkillUpdate) {
    setDiffLoading(true);
    try {
      const diff = await api.getRemoteSkillDiff(update.id);
      setSelectedDiff({ update, diff });
    } catch (e) {
      addToast("error", `${t("failedLoadDiff")}: ${e}`);
    } finally {
      setDiffLoading(false);
    }
  }

  function closeDiff() {
    setSelectedDiff(null);
  }

  const selectedTemplate = templates.find((item) => item.id === marketTemplateId) ?? null;
  const isBuiltinMarket = selectedTemplate != null && BUILTIN_MARKET_IDS.has(selectedTemplate.id);

  const filteredRemoteSkills = useMemo(
    () =>
      remoteSkills.filter((skill) => {
        if (selectedMarketId === "all") return true;
        return skill.market_id === Number(selectedMarketId);
      }),
    [remoteSkills, selectedMarketId],
  );

  const filteredUpdates = useMemo(
    () =>
      updates.filter((update) => {
        if (selectedMarketId === "all") return true;
        return update.market_id === Number(selectedMarketId);
      }),
    [selectedMarketId, updates],
  );

  const groupedUpdates = useMemo(() => {
    const groups = new Map<number, RemoteSkillUpdate[]>();
    for (const update of filteredUpdates) {
      const list = groups.get(update.market_id) || [];
      list.push(update);
      groups.set(update.market_id, list);
    }
    return groups;
  }, [filteredUpdates]);

  const updateCount = filteredUpdates.length;

  const remoteScanSummary = useMemo(() => {
    if (!remoteScanResult) return null;
    const total = remoteScanResult.reduce((sum, item) => sum + item.skills_found, 0);
    const newSkills = remoteScanResult.reduce((sum, item) => sum + item.skills_new, 0);
    const updated = remoteScanResult.reduce((sum, item) => sum + item.skills_updated, 0);
    const errors = remoteScanResult.flatMap((item) => item.errors);
    return { total, newSkills, updated, errors };
  }, [remoteScanResult]);
  return (
    <section className="section">
      <h2 className="section-title">{t("market")}</h2>

      <nav className="top-nav">
        <div className="tabs">
          <button className={`tab ${activeTab === "markets" ? "tab-active" : ""}`} onClick={() => setActiveTab("markets")}>
            {t("marketTab")}
          </button>
          <button className={`tab ${activeTab === "installs" ? "tab-active" : ""}`} onClick={() => setActiveTab("installs")}>
            {t("remoteInstallsTab")}
          </button>
          <button className={`tab ${activeTab === "skills" ? "tab-active" : ""}`} onClick={() => setActiveTab("skills")}>
            {t("skillsTab")}
          </button>
        </div>
      </nav>

      {activeTab === "markets" && (
        <>
          <div className="add-market-form">
            <div className="form-row">
              <select
                className="template-select"
                value={marketTemplateId}
                onChange={(e) => setMarketTemplateId(e.target.value)}
              >
                <option value="">{t("templatePlaceholder")}</option>
                {templates.map((template) => (
                  <option key={template.id} value={template.id}>
                    {template.label} · {template.kind === "standard" ? t("kindStandard") : template.kind === "nonstandard" ? t("kindNonstandard") : template.kind}
                  </option>
                ))}
              </select>
            </div>
            {!!templateDescription && (
              <p style={{ color: "var(--text-muted)", fontSize: 13, margin: "8px 0" }}>{templateDescription}</p>
            )}
            {isBuiltinMarket && (
              <p style={{ color: "var(--text-muted)", fontSize: 12, margin: "4px 0 0" }}>{t("builtinMarketNotice")}</p>
            )}
            <div className="form-row">
              <input className="search-input" value={owner} onChange={(e) => setOwner(e.target.value)} placeholder="owner" />
              <input className="search-input" value={repo} onChange={(e) => setRepo(e.target.value)} placeholder="repo" />
              <input className="search-input" value={branch} onChange={(e) => setBranch(e.target.value)} placeholder={t("branch")} />
            </div>
            <div className="form-row" style={{ marginTop: 10 }}>
              <button className="btn btn-secondary" onClick={loadMarkets} disabled={marketLoading}>{t("refresh")}</button>
              <button className="btn btn-primary" onClick={addMarket} disabled={marketLoading}>{t("addMarket")}</button>
              <button className="btn btn-secondary" onClick={syncAllMarketIndices} disabled={marketLoading}>{t("syncAllMarketIndices")}</button>
              {isBuiltinMarket && (
                <span style={{ color: "var(--text-muted)", fontSize: 12, alignSelf: "center" }}>
                  {t("builtinMarketNotice")}
                </span>
              )}
            </div>
          </div>

          <div className="section">
            {markets.length === 0 ? (
              <div className="empty-state">{t("noLogs")}</div>
            ) : (
              <div className="skill-list">
                <table className="skill-table">
                  <thead>
                    <tr>
                      <th>provider</th>
                      <th>owner</th>
                      <th>name</th>
                      <th>branch</th>
                      <th>kind</th>
                      <th>enabled</th>
                      <th>{t("lastIndexedAt")}</th>
                      <th>{t("lastCheckedAt")}</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {markets.map((m) => {
                      const template = templates.find((item) => item.owner === m.owner && item.name === m.name && item.branch === m.branch);
                      const kind = template?.kind ?? "custom";
                      const builtin = template != null && BUILTIN_MARKET_IDS.has(template.id);
                      return (
                        <tr key={m.id}>
                          <td>{m.provider}</td>
                          <td>{m.owner}</td>
                          <td>{m.name}</td>
                          <td>{m.branch}</td>
                          <td>{kind}</td>
                          <td>{m.enabled ? "true" : "false"}</td>
                          <td>{m.last_indexed_at ?? "-"}</td>
                          <td>{m.last_checked_at ?? "-"}</td>
                          <td>
                            <button className="btn btn-small btn-secondary" onClick={() => toggleMarket(m)} disabled={marketLoading}>
                              {m.enabled ? "disable" : "enable"}
                            </button>
                            <button className="btn btn-small btn-secondary" onClick={() => deleteMarket(m)} disabled={marketLoading || builtin}>
                              {builtin ? t("actionRemove") : t("deleteMarket")}
                            </button>
                            <button className="btn btn-small btn-secondary" onClick={() => syncMarketIndex(m)} disabled={marketLoading || !m.enabled}>
                              {t("syncMarketIndex")}
                            </button>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </>
      )}

      {activeTab === "installs" && (
        <>
          <div className="action-bar">
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
            <button className="btn btn-secondary" onClick={loadInstallations} disabled={installLoading}>
              {t("refresh")}
            </button>
            <button className="btn btn-primary" onClick={syncAllInstalledRemoteSkills} disabled={installLoading}>
              {t("syncAllActive")}
            </button>
            <select
              className="sort-select"
              value={selectedMarketId}
              onChange={(e) => {
                const value = e.target.value;
                const next = value === "all" ? "all" : Number(value);
                setSelectedMarketId(next);
                if (next === "all") {
                  loadInstallations();
                }
              }}
            >
              <option value="all">{t("allMarkets")}</option>
              {markets.map((market) => (
                <option key={market.id} value={market.id}>{market.owner}/{market.name}</option>
              ))}
            </select>
            <span className="sort-select" style={{ color: "var(--text-muted)" }}>{t("filterByMarket")}</span>
          </div>

          <div className="section">
            {remoteScanSummary && (
              <div style={{ color: "var(--text-muted)", fontSize: 12, marginBottom: 8 }}>
                {t("scanComplete")}: {remoteScanSummary.total} / {t("new")}: {remoteScanSummary.newSkills} / {t("updated")}: {remoteScanSummary.updated}
                {remoteScanSummary.errors.length > 0 && (
                  <span style={{ marginLeft: 8, color: "var(--error)" }}>
                    {remoteScanSummary.errors.length} errors
                  </span>
                )}
              </div>
            )}
            <h3 className="section-title">
              {installScope === "global" ? t("global") : t("projects")} {activeInstallations.length > 0 && <span className="badge">{activeInstallations.length}</span>}
            </h3>
            {activeInstallations.length === 0 ? (
              <div className="empty-state">{t("noSkillsYet")}</div>
            ) : (
              <div className="skill-list">
                <table className="skill-table">
                  <thead>
                    <tr>
                      <th>{t("skills")}</th>
                      <th>{t("remoteUrl")}</th>
                      <th>{t("ssotPath")}</th>
                      <th>{t("remoteInstalled")}</th>
                      <th>{t("lastSynced")}</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {activeInstallations.map((item) => {
                      const remote = remoteSkills.find((skill) => skill.id === item.remoteSkillId);
                      if (!remote) return null;
                      return (
                        <tr key={item.id}>
                          <td>
                            <div>{item.skillName}</div>
                            <div style={{ color: "var(--text-muted)", fontSize: 12 }}>{item.marketTitle}</div>
                          </td>
                          <td>{remote?.remote_url ?? "-"}</td>
                          <td>{remote?.ssot_path ?? "-"}</td>
                          <td>
                            <label className="toggle-label list-toggle">
                              <input type="checkbox" checked readOnly />
                              {item.installedAt && <span className="sync-dot" title={item.installedAt} />}
                            </label>
                          </td>
                          <td>{item.installedAt ? new Date(item.installedAt).toLocaleString() : "-"}</td>
                          <td>
                            <button className="btn btn-small btn-secondary" onClick={() => syncInstalledRemoteSkill(remote)} disabled={installLoading}>
                              {t("syncNow")}
                            </button>
                            <button className="btn btn-small btn-secondary" onClick={() => uninstallRemoteSkill(remote)} disabled={installLoading}>
                              {t("actionRemove")}
                            </button>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </>
      )}
      {activeTab === "skills" && (
        <>
          <div className="action-bar">
            <select
              className="sort-select"
              value={selectedMarketId}
              onChange={(e) => {
                const value = e.target.value;
                const next = value === "all" ? "all" : Number(value);
                setSelectedMarketId(next);
                loadRemoteSkills(next === "all" ? undefined : Number(next));
              }}
            >
              <option value="all">{t("allMarkets")}</option>
              {markets.map((market) => (
                <option key={market.id} value={market.id}>{market.owner}/{market.name}</option>
              ))}
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
            <button className="btn btn-secondary" onClick={() => loadRemoteSkills(selectedMarketId === "all" ? undefined : Number(selectedMarketId))} disabled={skillLoading}>
              {t("refresh")}
            </button>
            <button className="btn btn-secondary" onClick={() => checkRemoteUpdates(selectedMarketId === "all" ? undefined : Number(selectedMarketId))} disabled={remoteCheckLoading}>
              {t("checkRemoteUpdates")}
            </button>
            <button className="btn btn-secondary" onClick={scanAllRemoteRepositories} disabled={marketLoading || remoteScanLoading}>
              {remoteScanLoading ? t("scanning") : t("scanAllRemote")}
            </button>
          </div>

          <div className="section">
            {filteredRemoteSkills.length === 0 ? (
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
                          <button className="btn btn-small btn-secondary" onClick={() => syncRemoteSkillToTools(skill)} disabled={skillLoading || !skill.is_installed}>
                            {t("syncToTools")}
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
            {updates.length === 0 ? (
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
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {Array.from(groupedUpdates.entries()).map(([marketId, group]) => (
                      <>
                        {group.map((update) => (
                          <tr key={update.id}>
                            <td>{marketTitles[marketId] ?? marketId}</td>
                            <td>{update.skill_name}</td>
                            <td>{update.remote_url}</td>
                            <td>{update.old_hash.slice(0, 12)}</td>
                            <td>{update.new_hash.slice(0, 12)}</td>
                            <td>
                              <button className="btn btn-small btn-secondary" onClick={() => viewRemoteDiff(update)} disabled={diffLoading}>
                                {t("checkUpdates")}
                              </button>
                              <button className="btn btn-small btn-secondary" onClick={() => downloadRemoteSkill(remoteSkills.find((s) => s.id === update.id) || filteredRemoteSkills[0])} disabled={skillLoading}>
                                {t("downloadToSsot")}
                              </button>
                            </td>
                          </tr>
                        ))}
                      </>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </>
      )}
      {selectedDiff && (
        <div className="modal-overlay" onClick={closeDiff}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="diff-header">
              <button className="btn btn-small" onClick={closeDiff}>&larr; {t("back")}</button>
              <h3 className="diff-title">{selectedDiff.update.skill_name}</h3>
            </div>
            <div className="diff-meta">
              <span className="diff-path" title={selectedDiff.diff.source_path}>{t("sourceLabel")}: {selectedDiff.diff.source_path}</span>
              <span className="diff-path" title={selectedDiff.diff.ssot_path}>remote SSOT: {selectedDiff.diff.ssot_path}</span>
            </div>
            {diffLoading ? (
              <div className="empty-state">{t("scanning")}</div>
            ) : (
              <DiffFilesView diff={selectedDiff.diff} sideBySide noChangesLabel={t("noChanges")} />
            )}
            <div className="modal-actions">
              <button className="btn btn-primary" onClick={() => downloadRemoteSkill(remoteSkills.find((s) => s.id === selectedDiff.update.id) || filteredRemoteSkills[0])} disabled={skillLoading}>
                {t("downloadToSsot")}
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
