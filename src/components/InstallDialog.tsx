// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef, useState } from "react";
import type { Project, RemoteSkill, Tool } from "../types";

type Props = {
  skill: RemoteSkill;
  marketTitle: string;
  projects: Project[];
  tools: Tool[];
  t: (key: string) => string;
  loading: boolean;
  onConfirm: (projectId: number, toolPath: string, remember: boolean) => void;
  onClose: () => void;
};

const LS_PROJECT = "market.install.projectId";
const LS_TOOL = "market.install.toolPath";

export default function InstallDialog({ skill, marketTitle, projects, tools, t, loading, onConfirm, onClose }: Props) {
  const [projectId, setProjectId] = useState<number>(() => {
    const saved = Number(localStorage.getItem(LS_PROJECT));
    return Number.isFinite(saved) ? saved : 0;
  });
  const [toolPath, setToolPath] = useState<string>(() => localStorage.getItem(LS_TOOL) ?? "");
  const [remember, setRemember] = useState(true);
  const ref = useRef<HTMLDivElement>(null);

  // 项目不存在时回退全局
  useEffect(() => {
    if (projectId !== 0 && !projects.some((p) => p.id === projectId)) setProjectId(0);
  }, [projectId, projects]);

  // 工具默认取第一个
  useEffect(() => {
    if (tools.length > 0 && !tools.some((tool) => tool.globalPath === toolPath)) {
      setToolPath(tools[0].globalPath);
    }
  }, [tools, toolPath]);

  // Esc 关闭 + 打开时聚焦
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("keydown", onKey);
    ref.current?.focus();
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="modal-overlay" onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="modal" role="dialog" aria-modal="true" aria-label={t("installDialogTitle").replace("{0}", skill.skill_name)} ref={ref} tabIndex={-1}>
        <h3>{t("installDialogTitle").replace("{0}", skill.skill_name)}</h3>
        <p className="market-card-meta">{marketTitle}</p>

        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          <div>
            <div className="market-card-meta" style={{ marginBottom: 6 }}>{t("installToLabel")}</div>
            <div style={{ display: "flex", gap: 16, alignItems: "center" }}>
              <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
                <input type="radio" name="install-scope" checked={projectId === 0} onChange={() => setProjectId(0)} />
                {t("scopeGlobal")}
              </label>
              <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
                <input type="radio" name="install-scope" checked={projectId !== 0} onChange={() => setProjectId(projects[0]?.id ?? 0)} />
                {t("scopeProject")}
              </label>
              {projectId !== 0 && (
                <select className="sort-select" value={projectId} onChange={(e) => setProjectId(Number(e.target.value))} aria-label={t("chooseProject")}>
                  {projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
                </select>
              )}
            </div>
          </div>

          <div>
            <div className="market-card-meta" style={{ marginBottom: 6 }}>{t("targetToolLabel")}</div>
            <select className="sort-select" value={toolPath} onChange={(e) => setToolPath(e.target.value)} aria-label={t("targetToolLabel")} style={{ width: "100%" }}>
              {tools.map((tool) => <option key={tool.id} value={tool.globalPath}>{tool.name}</option>)}
            </select>
          </div>

          <label style={{ display: "flex", gap: 6, alignItems: "center", fontSize: 13, color: "var(--text-secondary)" }}>
            <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
            {t("rememberChoice")}
          </label>
        </div>

        <div className="modal-actions">
          <button className="btn btn-secondary" onClick={onClose} disabled={loading}>{t("cancel")}</button>
          <button
            className="btn btn-primary btn-press"
            onClick={() => onConfirm(projectId, toolPath, remember)}
            disabled={loading || tools.length === 0}
          >
            {loading ? t("installing") : t("install")}
          </button>
        </div>
      </div>
    </div>
  );
}

export { LS_PROJECT, LS_TOOL };
