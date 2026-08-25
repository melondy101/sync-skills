// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { SkillView, Tool, InstallationInfo, Market } from "../types";
import type { TranslateFn } from "../i18n";
import { Icon } from "./Icon";

interface SkillListRowProps {
  skill: SkillView;
  tools: Tool[];
  t: TranslateFn;
  hasUpdate: boolean;
  syncing: boolean;
  checkingSingle: boolean;
  sourceMarket?: Market;
  getInstallStatus: (skill: SkillView, toolId: number) => InstallationInfo | undefined;
  onToggle: (skillId: number, toolId: number, active: boolean) => void;
  onSync: () => void;
  onCheckUpdate: () => void;
  onHealthCheck: () => void;
  onEdit: () => void;
  onOpenMarket: () => void;
}

export function SkillListRow({
  skill,
  tools,
  t,
  hasUpdate,
  syncing,
  checkingSingle,
  sourceMarket,
  getInstallStatus,
  onToggle,
  onSync,
  onCheckUpdate,
  onHealthCheck,
  onEdit,
  onOpenMarket,
}: SkillListRowProps) {
  return (
    <>
      <tr className={`skill-list-main ${hasUpdate ? "skill-has-update" : ""}`}>
        <td className="col-name">
          <span className="list-skill-name">{skill.name}</span>
          {hasUpdate && <Icon name="circle" size={8} className="update-indicator" />}
          {syncing && <Icon name="refresh-cw" size={14} className="sync-spinner" />}
          {skill.description && <span className="list-skill-desc">{skill.description}</span>}
          {sourceMarket && (
            <button
              type="button"
              className="market-provenance market-provenance-inline"
              onClick={onOpenMarket}
              title={t("openInMarket")}
            >
              {t("fromMarket").replace("{0}", `${sourceMarket.owner}/${sourceMarket.name}`)}
            </button>
          )}
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
            <button
              className="btn btn-small"
              onClick={onCheckUpdate}
              disabled={checkingSingle}
            >
              {checkingSingle ? t("checkingUpdate") : t("checkUpdate")}
            </button>
            <button className="btn btn-small" onClick={onHealthCheck}>
              {t("healthCheck")}
            </button>
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
