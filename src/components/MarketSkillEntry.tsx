// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { memo, type KeyboardEvent, type ReactNode } from "react";
import { Icon } from "./Icon";
import type { RemoteSkill } from "../types";

export type MarketSkillProps = {
  skill: RemoteSkill;
  /** Pending update for this skill's `market_id:skill_name` pair. */
  hasUpdate: boolean;
  /** Source badge text; empty string hides the badge (a single market is selected). */
  sourceBadge: string;
  /** A market mutation is in flight, so every action button is disabled. */
  busy: boolean;
  t: (key: string) => string;
  onOpen: (skill: RemoteSkill) => void;
  onInstall: (skill: RemoteSkill) => void;
  onUpdate: (skill: RemoteSkill) => void;
  onSync: (skill: RemoteSkill) => void;
};

// The row/card is a focusable div rather than a button because it nests buttons,
// so it needs the explicit Enter/Space activation a <button> would give for free.
function activate(
  event: KeyboardEvent,
  skill: RemoteSkill,
  onOpen: (skill: RemoteSkill) => void,
) {
  if (event.key !== "Enter" && event.key !== " ") return;
  event.preventDefault();
  onOpen(skill);
}

function SourceBadge({ text }: { text: string }) {
  if (!text) return null;
  return <span className="m-card-src-badge">{text}</span>;
}

function InstalledTag({ t }: { t: (key: string) => string }) {
  return (
    <span className="market-installed-tag">
      <Icon name="check" size={12} />
      {t("installedTag")}
    </span>
  );
}

/**
 * The three state-dependent actions. `between` lets the grid tile keep its
 * historical ordering — update, installed tag, sync — without duplicating this.
 */
function Actions({
  skill,
  hasUpdate,
  busy,
  t,
  onInstall,
  onUpdate,
  onSync,
  between,
}: MarketSkillProps & { between?: ReactNode }) {
  if (!skill.is_installed) {
    return (
      <button
        className="btn btn-small btn-primary btn-press"
        onClick={() => onInstall(skill)}
        disabled={busy}
      >
        {t("install")}
      </button>
    );
  }
  return (
    <>
      {hasUpdate && (
        <button
          className="btn btn-small btn-primary btn-press"
          onClick={() => onUpdate(skill)}
          disabled={busy}
        >
          {t("updateBtn")}
        </button>
      )}
      {between}
      <button className="btn btn-small btn-secondary" onClick={() => onSync(skill)} disabled={busy}>
        {t("syncBtn")}
      </button>
    </>
  );
}

/** Grid tile. Actions stop propagation so they don't also open the detail modal. */
export const MarketSkillCard = memo(function MarketSkillCard(props: MarketSkillProps) {
  const { skill, hasUpdate, sourceBadge, t, onOpen } = props;
  return (
    <div
      className={`skill-card ${hasUpdate ? "skill-has-update" : ""}`}
      role="button"
      tabIndex={0}
      aria-label={t("openDetailAria").replace("{0}", skill.skill_name)}
      onClick={() => onOpen(skill)}
      onKeyDown={(e) => activate(e, skill, onOpen)}
    >
      <div className="skill-header">
        <h3 className="skill-name">{skill.skill_name}</h3>
        <SourceBadge text={sourceBadge} />
      </div>
      <p className="market-card-desc">{skill.description || ""}</p>
      <div className="market-card-footer" onClick={(e) => e.stopPropagation()}>
        <Actions {...props} between={skill.is_installed ? <InstalledTag t={t} /> : null} />
      </div>
    </div>
  );
});

/** Dense table row, the alternative to {@link MarketSkillCard}. */
export const MarketSkillRow = memo(function MarketSkillRow(props: MarketSkillProps) {
  const { skill, hasUpdate, sourceBadge, t, onOpen } = props;
  return (
    <div
      className={`skill-list-row ${hasUpdate ? "skill-has-update" : ""}`}
      role="button"
      tabIndex={0}
      aria-label={t("openDetailAria").replace("{0}", skill.skill_name)}
      onClick={() => onOpen(skill)}
      onKeyDown={(e) => activate(e, skill, onOpen)}
    >
      <div className="skill-list-cell skill-list-name">
        <h3 className="skill-name">{skill.skill_name}</h3>
        <SourceBadge text={sourceBadge} />
      </div>
      <p className="skill-list-cell skill-list-desc">{skill.description || ""}</p>
      <div className="skill-list-cell skill-list-status">
        {skill.is_installed ? (
          <InstalledTag t={t} />
        ) : (
          <span className="market-not-installed-tag">{t("notInstalled")}</span>
        )}
      </div>
      <div className="skill-list-cell skill-list-actions" onClick={(e) => e.stopPropagation()}>
        <Actions {...props} />
      </div>
    </div>
  );
});
