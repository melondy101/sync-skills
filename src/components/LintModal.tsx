// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import * as api from "../api";
import type { SkillLint } from "../types";
import type { TranslateFn } from "../i18n";
import { Icon } from "./Icon";

/**
 * Skill health check modal. Without `skill` it lints every skill in the
 * project scope; with `skill` it checks just that one (per-card entry).
 */
export function LintModal({
  t,
  projectId,
  skill,
  addToast,
  onEdit,
  onFixed,
  onClose,
}: {
  t: TranslateFn;
  projectId: number;
  /** When set, lint only this skill instead of the whole scope. */
  skill?: { id: number; name: string } | null;
  addToast: (type: "success" | "error" | "info", message: string) => void;
  /** Open the built-in editor for a skill (rendered above this modal). */
  onEdit: (skillId: number, skillName: string) => void;
  /** Notify the parent that a fix wrote and synced files (reload skills). */
  onFixed: () => void;
  onClose: () => void;
}) {
  const [results, setResults] = useState<SkillLint[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fixing, setFixing] = useState<number | null>(null);

  useEffect(() => {
    const load = skill
      ? api.lintSkill(skill.id, projectId).then((r) => [r])
      : api.lintSkills(projectId);
    load.then(setResults).catch((e) => setError(String(e)));
    // Depend on the id, not the object: parent passes an inline literal whose
    // identity changes on every render, which would re-trigger the lint.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, skill?.id]);

  // Localize an issue code via the lint_* translation keys, appending the
  // optional parameter (e.g. expected name, max length) when present.
  function issueText(code: string, param?: string | null): string {
    const msg = t(`lint_${code}`);
    return param ? `${msg} (${param})` : msg;
  }

  // Run the backend auto-fix and swap in the fresh lint result for that skill.
  async function handleFix(r: SkillLint) {
    setFixing(r.skill_id);
    try {
      const fresh = await api.fixSkill(r.skill_id, projectId);
      setResults((prev) =>
        prev ? prev.map((x) => (x.skill_id === r.skill_id ? fresh : x)) : prev
      );
      if (fresh.issues.length < r.issues.length) {
        addToast("success", `${r.skill_name}: ${t("lintFixDone")}`);
        onFixed();
      } else {
        addToast("info", `${r.skill_name}: ${t("lintFixNone")}`);
      }
    } catch (e) {
      addToast("error", `${t("lintFixFailed")}: ${e}`);
    } finally {
      setFixing(null);
    }
  }

  const withIssues = results?.filter((r) => r.issues.length > 0) ?? [];

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal lint-modal" onClick={(e) => e.stopPropagation()}>
        <h3>{skill ? `${t("lintTitle")} — ${skill.name}` : t("lintTitle")}</h3>

        {error ? (
          <div className="lint-status lint-error">{t("lintFailed")}: {error}</div>
        ) : results === null ? (
          <div className="lint-status">{t("lintRunning")}</div>
        ) : withIssues.length === 0 ? (
          <div className="lint-status lint-ok"><Icon name="check" size={14} /> {t("lintAllGood")}</div>
        ) : (
          <div className="lint-list">
            {withIssues.map((r) => (
              <div key={r.skill_id} className="lint-item">
                <div className="lint-item-header">
                  <span className="lint-skill-name">{r.skill_name}</span>
                  <span className="lint-count-badge">{r.issues.length} {t("lintIssues")}</span>
                  <div className="lint-item-actions">
                    {r.issues.some((i) => i.fixable) && (
                      <button
                        className="btn btn-small btn-primary"
                        disabled={fixing !== null}
                        onClick={() => handleFix(r)}
                      >
                        {fixing === r.skill_id ? t("lintFixing") : t("lintFix")}
                      </button>
                    )}
                    <button
                      className="btn btn-small"
                      onClick={() => onEdit(r.skill_id, r.skill_name)}
                    >
                      {t("editSkill")}
                    </button>
                  </div>
                </div>
                <ul className="lint-issue-list">
                  {r.issues.map((issue, i) => (
                    <li key={i} className={`lint-issue lint-issue-${issue.severity}`}>
                      <span className={`lint-sev lint-sev-${issue.severity}`}>
                        {issue.severity === "error" ? t("sevError") : t("sevWarning")}
                      </span>
                      <span className="lint-issue-msg">{issueText(issue.code, issue.param)}</span>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        )}

        <div className="modal-actions">
          <button className="btn btn-primary" onClick={onClose}>{t("close")}</button>
        </div>
      </div>
    </div>
  );
}
