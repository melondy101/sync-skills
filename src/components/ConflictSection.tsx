// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useState } from "react";
import * as api from "../api";
import type { ConflictView, SkillDiff } from "../types";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";
import { DiffFilesView, DiffViewControls } from "./DiffView";
import { Icon } from "./Icon";

/** Conflict banner (M5) with per-version resolve buttons and a diff modal. */
export function ConflictSection({
  t,
  conflicts,
  projectId,
  addToast,
  onResolved,
}: {
  t: TranslateFn;
  conflicts: ConflictView[];
  projectId: number;
  addToast: AddToastFn;
  onResolved: () => Promise<void>;
}) {
  const [resolvingConflict, setResolvingConflict] = useState<number | null>(null);
  const [conflictDiff, setConflictDiff] = useState<{ conflictId: number; diff: SkillDiff; toolName: string } | null>(null);
  const [loadingConflictDiff, setLoadingConflictDiff] = useState<number | null>(null);
  const [conflictMaximized, setConflictMaximized] = useState(false);
  const [diffSideBySide, setDiffSideBySide] = useState(true);

  if (conflicts.length === 0) return null;

  async function handleResolveConflict(conflictId: number, keepToolName: string) {
    setResolvingConflict(conflictId);
    try {
      const result = await api.resolveConflict(conflictId, keepToolName, projectId);
      if (result.errors.length > 0) {
        addToast("error", result.errors.join(", "));
      } else {
        addToast("success", t("conflictResolved"));
      }
      await onResolved();
    } catch (e) {
      addToast("error", `${t("failedResolve")}: ${e}`);
    } finally {
      setResolvingConflict(null);
    }
  }

  async function handleViewConflictDiff(conflictId: number, version: { tool_id: number; tool_name: string; source_path: string }) {
    setLoadingConflictDiff(conflictId);
    try {
      const diff = await api.getSkillDiff(conflictId, version.source_path);
      setConflictDiff({ conflictId, diff, toolName: version.tool_name });
    } catch (e) {
      addToast("error", `${t("failedLoadDiff")}: ${e}`);
    } finally {
      setLoadingConflictDiff(null);
    }
  }

  return (
    <>
      <section className="section">
        <div className="conflict-banner">
          <div className="conflict-header">
            <Icon name="alert-triangle" size={16} className="conflict-icon" />
            <span className="conflict-title">{t("conflictsTitle")}</span>
            <span className="badge badge-conflict">{conflicts.length}</span>
          </div>
          <p className="conflict-desc">{t("conflictsDesc")}</p>
          <div className="conflict-list">
            {conflicts.map((c) => (
              <div key={c.id} className="conflict-item">
                <span className="conflict-skill-name">{c.skill_name}</span>
                <div className="conflict-versions">
                  {c.versions.map((v) => (
                    <div key={v.tool_id} className="conflict-version-group">
                      <button
                        className={`btn btn-small ${resolvingConflict === c.id ? "" : "btn-secondary"}`}
                        disabled={resolvingConflict !== null}
                        onClick={() => handleResolveConflict(c.id, v.tool_name)}
                      >
                        {resolvingConflict === c.id ? t("resolving") : `${t("keepVersion")}: ${v.tool_name}`}
                      </button>
                      <button
                        className="btn btn-small"
                        disabled={loadingConflictDiff === c.id}
                        onClick={() => handleViewConflictDiff(c.id, v)}
                      >
                        {loadingConflictDiff === c.id ? t("loading") : t("viewDiff")}
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Conflict Diff Modal */}
      {conflictDiff && (
        <div className="modal-overlay" onClick={() => { setConflictDiff(null); setConflictMaximized(false); }}>
          <div className={`modal updates-modal${conflictMaximized ? " maximized" : ""}`} onClick={(e) => e.stopPropagation()}>
            <div className="diff-header">
              <button className="btn btn-small" onClick={() => setConflictDiff(null)}>&larr; {t("back")}</button>
              <h3 className="diff-title">{t("comparingVersions")}: {conflictDiff.toolName}</h3>
              <DiffViewControls
                t={t}
                sideBySide={diffSideBySide}
                onToggleSideBySide={() => setDiffSideBySide((v) => !v)}
                maximized={conflictMaximized}
                onToggleMaximize={() => setConflictMaximized((v) => !v)}
              />
            </div>
            <div className="diff-meta">
              <span className="diff-path" title={conflictDiff.diff.source_path}>
                source: {conflictDiff.diff.source_path}
              </span>
              <span className="diff-path" title={conflictDiff.diff.ssot_path}>
                ssot: {conflictDiff.diff.ssot_path}
              </span>
            </div>
            <DiffFilesView diff={conflictDiff.diff} sideBySide={diffSideBySide} noChangesLabel={t("noChanges")} />
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={() => { setConflictDiff(null); setConflictMaximized(false); }}>{t("close")}</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
