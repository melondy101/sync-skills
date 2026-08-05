// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { SkillView, Tool, InstallationInfo, RemoteInstallation } from "../types";
import type { TranslateFn } from "../i18n";

interface SkillListRowProps {
  skill: SkillView;
  tools: Tool[];
  t: TranslateFn;
  hasUpdate: boolean;
  syncing: boolean;
  checkingSingle: boolean;
  getInstallStatus: (skill: SkillView, toolId: number) => InstallationInfo | undefined;
  remoteInstallations: RemoteInstallation[];
  selectedProjectId: number;
  onToggle: (skillId: number, toolId: number, active: boolean) => void;
  onSync: () => void;
  onCheckUpdate: () => void;
  onHealthCheck: () => void;
  onEdit: () => void;
}

function isRemoteSkill(skill: SkillView, remoteInstallations: RemoteInstallation[], selectedProjectId: number): boolean {
  return remoteInstallations.some((inst) => inst.projectId === selectedProjectId && inst.skillName === skill.name);
}

export function SkillListRow({
  skill, tools, t, hasUpdate, syncing, checkingSingle,
  getInstallStatus, remoteInstallations, selectedProjectId,
  onToggle, onSync, onCheckUpdate, onHealthCheck, onEdit,
}: SkillListRowProps) {
  const remote = isRemoteSkill(skill, remoteInstallations, selectedProjectId);
  return (
    <>
      <tr className={`skill-list-main ${hasUpdate ? "skill-has-update" : ""}`}>
        <td className="col-name">
          <span className="list-skill-name">{skill.name}</span>
          {remote && <span className="badge badge-update">remote</span>}
          {hasUpdate && <span className="update-indicator" title={t("updateAvailable")}>●</span>}
          {syncing && <span className="sync-spinner">⟳</span>}
          {skill.description && <span className="list-skill-desc">{skill.description}</span>}
        </td>
        {tools.map((tool) => {
          const inst = getInstallStatus(skill, tool.id);
          const isActive = inst?.status === "active";
          return (
            <td key={tool.id} className="col-tool">
              <label className="toggle-label list-toggle">
                <input
                  type="checkbox"
                  checked={isActive}
                  onChange={(e) => onToggle(skill.id, tool.id, e.target.checked)}
                />
                {inst?.synced_at && <span className="sync-dot" title={inst.synced_at} />}
              </label>
            </td>
          );
        })}
        <td className="col-actions">
          <button className="btn btn-small" onClick={onEdit} title={t("editSkill")}>
            {t("editSkill")}
          </button>
        </td>
      </tr>
      <tr className="skill-list-actions">
        <td colSpan={tools.length + 2}>
          <div className="list-action-row">
            {!remote && (
              <button
                className="btn btn-small"
                onClick={onCheckUpdate}
                disabled={checkingSingle}
              >
                {checkingSingle ? t("checkingUpdate") : t("checkUpdate")}
              </button>
            )}
            {!remote && (
              <button className="btn btn-small" onClick={onHealthCheck}>
                {t("healthCheck")}
              </button>
            )}
            <button
              className="btn btn-small btn-primary"
              onClick={onSync}
              disabled={syncing}
            >
              {syncing ? t("syncingCard") : t("syncNow")}
            </button>
          </div>
        </td>
      </tr>
    </>
  );
}
