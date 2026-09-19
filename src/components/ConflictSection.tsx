// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useRef, useState } from "react";
import * as api from "../api";
import type { ConflictView, SkillDiff, Tool } from "../types";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { DiffFilesView, DiffViewControls } from "./DiffView";
import { useConfirm } from "./ConfirmProvider";
import { Icon } from "./Icon";

type AutoStrategy = "newest" | "preferred-tool";

/** Conflict banner (M5) with per-version resolve buttons and a diff modal. */
export function ConflictSection({
  t,
  conflicts,
  tools,
  projectId,
  addToast,
  onResolved,
}: {
  t: TranslateFn;
  conflicts: ConflictView[];
  /** Configured tools, in the order the "preferred tool" strategy reads. */
  tools: Tool[];
  projectId: number;
  addToast: AddToastFn;
  onResolved: () => Promise<void>;
}) {
  const [resolvingConflict, setResolvingConflict] = useState<number | null>(null);
  const [conflictDiff, setConflictDiff] = useState<{ conflictId: number; diff: SkillDiff; toolName: string } | null>(null);
  const [loadingConflictDiff, setLoadingConflictDiff] = useState<number | null>(null);
  const [conflictMaximized, setConflictMaximized] = useState(false);
  const [diffSideBySide, setDiffSideBySide] = useState(true);
  const [autoStrategy, setAutoStrategy] = useState<AutoStrategy>("newest");
  const [autoRunning, setAutoRunning] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);
  const { showConfirm } = useConfirm();

  function closeConflictDiff() {
    setConflictDiff(null);
    setConflictMaximized(false);
  }

  useFocusTrap(dialogRef, { active: conflictDiff !== null, onEscape: closeConflictDiff });

  if (conflicts.length === 0) return null;

  async function runAutoResolve() {
    setAutoRunning(true);
    try {
      const outcomes = await api.autoResolveConflicts(
        autoStrategy,
        tools.map((tool) => tool.name),
        projectId,
      );
      const resolved = outcomes.filter((o) => o.resolved);
      const skipped = outcomes.filter((o) => !o.resolved);
      if (resolved.length > 0) {
        addToast("success", `${t("autoResolveDone")}: ${resolved.length}`);
      }
      if (skipped.length > 0) {
        addToast("info", `${t("autoResolveSkipped")}: ${skipped.map((o) => o.skill_name).join(", ")}`);
      }
      const failed = resolved.flatMap((o) => o.errors);
      if (failed.length > 0) {
        addToast("error", failed.join(", "));
      }
      if (resolved.length === 0 && skipped.length === outcomes.length) {
        addToast("info", t("autoResolveNothing"));
      }
      await onResolved();
    } catch (e) {
      addToast("error", `${t("failedResolve")}: ${e}`);
    } finally {
      setAutoRunning(false);
    }
  }

  function handleAutoResolve() {
    showConfirm({
      title: t("autoResolveTitle"),
      message:
        autoStrategy === "newest"
          ? t("autoResolveNewestDesc")
          : t("autoResolvePreferredDesc"),
      detail: t("autoResolveDetail"),
      confirmText: t("autoResolveConfirm"),
      cancelText: t("cancel"),
      onConfirm: runAutoResolve,
    });
  }

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
            <div className="conflict-auto">
              <select
                aria-label={t("autoResolveStrategy")}
                value={autoStrategy}
                disabled={autoRunning}
                onChange={(e) => setAutoStrategy(e.target.value as AutoStrategy)}
              >
                <option value="newest">{t("autoResolveNewest")}</option>
                <option value="preferred-tool">{t("autoResolvePreferred")}</option>
              </select>
              <button
                className="btn btn-small btn-secondary"
                disabled={autoRunning}
                onClick={handleAutoResolve}
              >
                {autoRunning ? t("resolving") : t("autoResolve")}
              </button>
            </div>
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
        <div className="modal-overlay" onClick={closeConflictDiff}>
          <div
            className={`modal updates-modal${conflictMaximized ? " maximized" : ""}`}
            role="dialog"
            aria-modal="true"
            aria-label={`${t("comparingVersions")}: ${conflictDiff.toolName}`}
            onClick={(e) => e.stopPropagation()}
            ref={dialogRef}
            tabIndex={-1}
          >
            <div className="diff-header">
              <button className="btn btn-small" onClick={closeConflictDiff}>&larr; {t("back")}</button>
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
              <button className="btn btn-secondary" onClick={closeConflictDiff}>{t("close")}</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
