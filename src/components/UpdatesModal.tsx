// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useState } from "react";
import * as api from "../api";
import type { SkillDiff, SkillUpdate } from "../types";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";
import { DiffFilesView, DiffViewControls } from "./DiffView";

export interface UpdateDiffEntry {
  update: SkillUpdate;
  diff: SkillDiff;
}

/** Updates modal: grouped update list, single-tool diff view and all-tools diff view. */
export function UpdatesModal({
  t,
  projectId,
  updates,
  initialDiff,
  addToast,
  onUpdatesChanged,
  onSynced,
  onClose,
}: {
  t: TranslateFn;
  projectId: number;
  updates: SkillUpdate[];
  /** Pre-loaded diff when opened from a single-skill check. */
  initialDiff?: UpdateDiffEntry | null;
  addToast: AddToastFn;
  onUpdatesChanged: (remaining: SkillUpdate[]) => void;
  onSynced: () => Promise<void>;
  onClose: () => void;
}) {
  const [selectedUpdateDiff, setSelectedUpdateDiff] = useState<UpdateDiffEntry | null>(initialDiff ?? null);
  const [loadingDiff, setLoadingDiff] = useState<number | null>(null);
  // P2-1/2/3: diff UX state
  const [diffMaximized, setDiffMaximized] = useState(false);
  const [diffSideBySide, setDiffSideBySide] = useState(true);
  const [selectedSkillDiffs, setSelectedSkillDiffs] = useState<{
    skillId: number;
    skillName: string;
    tools: { tool: string | null; toolId: number | null; sourcePath: string; diff: SkillDiff | null; loading: boolean }[];
  } | null>(null);
  const [expandedTools, setExpandedTools] = useState<Set<string>>(new Set());
  const [reversingSync, setReversingSync] = useState<number | null>(null);

  // Remove a skill's entries from the updates list, return to the list view,
  // and close the modal entirely once no updates remain.
  function closeUpdateEntry(skillId: number) {
    const remaining = updates.filter((u) => u.skill_id !== skillId);
    onUpdatesChanged(remaining);
    setSelectedUpdateDiff(null);
    setSelectedSkillDiffs(null);
    if (remaining.length === 0) {
      setDiffMaximized(false);
      onClose();
    }
  }

  async function handleViewDiff(update: SkillUpdate) {
    setLoadingDiff(update.skill_id);
    try {
      const diff = await api.getSkillDiff(update.skill_id, update.source_path);
      setSelectedUpdateDiff({ update, diff });
    } catch (e) {
      addToast("error", `${t("failedLoadDiff")}: ${e}`);
    } finally {
      setLoadingDiff(null);
    }
  }

  // P2-3: open a skill's diff showing every divergent tool as an expandable block.
  async function handleViewSkillDiffs(skillId: number, skillName: string, skillUpdates: SkillUpdate[]) {
    const tools = skillUpdates.map((u) => ({
      tool: u.changed_tool,
      toolId: u.changed_tool_id,
      sourcePath: u.source_path,
      diff: null as SkillDiff | null,
      loading: true,
    }));
    setSelectedSkillDiffs({ skillId, skillName, tools });
    // Expand every tool block by default so the SSOT (left) view is visible for each.
    setExpandedTools(new Set(tools.map((_, i) => String(i))));
    setDiffMaximized(false);
    setDiffSideBySide(true);

    for (let i = 0; i < skillUpdates.length; i++) {
      const u = skillUpdates[i];
      try {
        const diff = await api.getSkillDiff(skillId, u.source_path);
        setSelectedSkillDiffs((prev) =>
          prev ? { ...prev, tools: prev.tools.map((tl, idx) => (idx === i ? { ...tl, diff, loading: false } : tl)) } : prev
        );
      } catch {
        setSelectedSkillDiffs((prev) =>
          prev ? { ...prev, tools: prev.tools.map((tl, idx) => (idx === i ? { ...tl, loading: false } : tl)) } : prev
        );
      }
    }
  }

  function toggleToolBlock(key: string) {
    setExpandedTools((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  async function handleUpdateFromDiff(skillId: number, sourcePath?: string) {
    try {
      // Must match the scope used by check_updates — a project skill's SSOT
      // lives under _p<project_id>/, so syncing with the wrong scope writes
      // to the global SSOT and the update never clears.
      const result = await api.syncSkill(skillId, projectId, sourcePath ?? selectedUpdateDiff?.update.source_path ?? null);
      if (result.errors.length > 0) {
        // Keep the update entry so the user can retry after fixing the cause
        addToast("error", `${t("syncFailed")}: ${result.errors.join(", ")}`);
        return;
      }
      addToast("success", t("skillUpdated"));
      closeUpdateEntry(skillId);
      await onSynced();
    } catch (e) {
      addToast("error", `${t("syncFailed")}: ${e}`);
    }
  }

  function handleSkipUpdate(skillId: number) {
    closeUpdateEntry(skillId);
  }

  async function handleReverseSync(skillId: number, toolId: number) {
    setReversingSync(skillId);
    try {
      const result = await api.reverseSyncSkill(skillId, toolId, projectId);
      if (result.errors.length > 0) {
        addToast("error", result.errors.join(", "));
      } else {
        addToast("success", t("revertedToSsot"));
      }
      closeUpdateEntry(skillId);
      await onSynced();
    } catch (e) {
      addToast("error", `${t("failedRevert")}: ${e}`);
    } finally {
      setReversingSync(null);
    }
  }

  async function handleDismissUpdate(update: SkillUpdate) {
    if (!update.changed_tool_id) return;
    try {
      await api.dismissSkillUpdate(update.skill_id, update.changed_tool_id, update.new_hash);
      closeUpdateEntry(update.skill_id);
      addToast("info", t("dismissed"));
    } catch (e) {
      addToast("error", `${t("failedRevert")}: ${e}`);
    }
  }

  function handleOverlayClose() {
    setSelectedUpdateDiff(null);
    setSelectedSkillDiffs(null);
    setDiffMaximized(false);
    onClose();
  }

  return (
    <div className="modal-overlay" onClick={handleOverlayClose}>
      <div className={`modal updates-modal${diffMaximized ? " maximized" : ""}`} onClick={(e) => e.stopPropagation()}>
        {selectedUpdateDiff ? (
          <>
            <div className="diff-header">
              <button className="btn btn-small" onClick={() => setSelectedUpdateDiff(null)}>&larr; {t("back")}</button>
              <h3 className="diff-title">{selectedUpdateDiff.update.skill_name}</h3>
              {selectedUpdateDiff.update.changed_tool && (
                <span className="diff-tool-badge">{selectedUpdateDiff.update.changed_tool}</span>
              )}
              <DiffViewControls
                t={t}
                sideBySide={diffSideBySide}
                onToggleSideBySide={() => setDiffSideBySide((v) => !v)}
                maximized={diffMaximized}
                onToggleMaximize={() => setDiffMaximized((v) => !v)}
              />
            </div>
            <div className="diff-meta">
              <span className="diff-path" title={selectedUpdateDiff.diff.source_path}>
                source: {selectedUpdateDiff.diff.source_path}
              </span>
              <span className="diff-path" title={selectedUpdateDiff.diff.ssot_path}>
                ssot: {selectedUpdateDiff.diff.ssot_path}
              </span>
            </div>
            <DiffFilesView diff={selectedUpdateDiff.diff} sideBySide={diffSideBySide} noChangesLabel={t("noChanges")} />
            <div className="modal-actions">
              <button
                className="btn btn-primary"
                onClick={() => handleUpdateFromDiff(selectedUpdateDiff.update.skill_id, selectedUpdateDiff.update.source_path)}
              >
                {t("updateToSsot")}
              </button>
              {selectedUpdateDiff.update.changed_tool_id && (
                <button
                  className="btn btn-secondary"
                  disabled={reversingSync === selectedUpdateDiff.update.skill_id}
                  onClick={() => handleReverseSync(selectedUpdateDiff.update.skill_id, selectedUpdateDiff.update.changed_tool_id!)}
                >
                  {reversingSync === selectedUpdateDiff.update.skill_id ? t("reverting") : t("revertToSsot")}
                </button>
              )}
              <button
                className="btn btn-secondary"
                onClick={() => handleSkipUpdate(selectedUpdateDiff.update.skill_id)}
              >
                {t("skip")}
              </button>
              {selectedUpdateDiff.update.changed_tool_id && (
                <button
                  className="btn btn-secondary"
                  onClick={() => handleDismissUpdate(selectedUpdateDiff.update)}
                >
                  {t("dismissChange")}
                </button>
              )}
            </div>
          </>
        ) : selectedSkillDiffs ? (
          <>
            <div className="diff-header">
              <button className="btn btn-small" onClick={() => setSelectedSkillDiffs(null)}>&larr; {t("back")}</button>
              <h3 className="diff-title">{selectedSkillDiffs.skillName}</h3>
              <DiffViewControls
                t={t}
                sideBySide={diffSideBySide}
                onToggleSideBySide={() => setDiffSideBySide((v) => !v)}
                maximized={diffMaximized}
                onToggleMaximize={() => setDiffMaximized((v) => !v)}
              />
            </div>
            <div className="diff-meta">
              <span className="diff-path" title={selectedSkillDiffs.tools[0]?.diff?.ssot_path}>
                ssot: {selectedSkillDiffs.tools[0]?.diff?.ssot_path ?? "—"}
              </span>
            </div>
            {/* P2-3: each divergent tool is an expandable block; left column = SSOT (synced) */}
            <div className="tool-diff-list">
              {selectedSkillDiffs.tools.map((tool, i) => {
                const key = String(i);
                const open = expandedTools.has(key);
                return (
                  <div key={key} className="tool-diff-block">
                    <div className="tool-diff-block-header" onClick={() => toggleToolBlock(key)}>
                      <span className="tool-diff-caret">{open ? "▾" : "▸"}</span>
                      <span className="update-tool-name">{tool.tool ?? t("sourceLabel")}</span>
                      <code className="update-path" title={tool.sourcePath}>{tool.sourcePath}</code>
                    </div>
                    {open && (
                      <div className="tool-diff-block-body">
                        {tool.loading ? (
                          <div className="diff-loading">{t("loading")}</div>
                        ) : tool.diff ? (
                          <DiffFilesView diff={tool.diff} sideBySide={diffSideBySide} noChangesLabel={t("noChanges")} />
                        ) : (
                          <div className="diff-loading">{t("failedLoadDiff")}</div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
            <div className="modal-actions">
              <button
                className="btn btn-small btn-primary"
                onClick={() => handleUpdateFromDiff(selectedSkillDiffs.skillId, selectedSkillDiffs.tools[0]?.sourcePath)}
              >
                {t("updateToSsot")}
              </button>
              <button
                className="btn btn-small"
                onClick={() => handleSkipUpdate(selectedSkillDiffs.skillId)}
              >
                {t("skip")}
              </button>
            </div>
          </>
        ) : (
          <>
            <h3>{t("updatesAvailable")} ({updates.length})</h3>
            <div className="updates-list">
              {(() => {
                // Group updates by skill_id
                const grouped = new Map<number, SkillUpdate[]>();
                for (const u of updates) {
                  const list = grouped.get(u.skill_id) || [];
                  list.push(u);
                  grouped.set(u.skill_id, list);
                }
                return Array.from(grouped.entries()).map(([skillId, skillUpdates]) => (
                  <div key={skillId} className="update-item-group">
                    <div className="update-item-header">
                      <span className="update-skill-name">{skillUpdates[0].skill_name}</span>
                      <span className="update-count-badge">{skillUpdates.length} {t("toolUnit")}</span>
                      <button
                        className="btn btn-small"
                        onClick={() => handleViewSkillDiffs(skillId, skillUpdates[0].skill_name, skillUpdates)}
                      >
                        {t("viewDiffAll")}
                      </button>
                    </div>
                    {skillUpdates.map((u, idx) => (
                      <div key={idx} className="update-item update-item-tool">
                        <div className="update-item-info">
                          {u.changed_tool && <span className="update-tool-name">{u.changed_tool}</span>}
                          <code className="update-path" title={u.source_path}>{u.source_path}</code>
                        </div>
                        <div className="update-item-actions">
                          <button
                            className="btn btn-small"
                            onClick={() => handleViewDiff(u)}
                            disabled={loadingDiff === u.skill_id}
                          >
                            {loadingDiff === u.skill_id ? t("loading") : t("viewDiff")}
                          </button>
                          {u.changed_tool_id && (
                            <button
                              className="btn btn-small"
                              onClick={() => handleDismissUpdate(u)}
                            >
                              {t("dismissChange")}
                            </button>
                          )}
                        </div>
                      </div>
                    ))}
                    <div className="update-item-actions update-group-actions">
                      <button
                        className="btn btn-small btn-primary"
                        onClick={() => handleUpdateFromDiff(skillId, skillUpdates[0].source_path)}
                      >
                        {t("updateToSsot")}
                      </button>
                      <button
                        className="btn btn-small"
                        onClick={() => handleSkipUpdate(skillId)}
                      >
                        {t("skip")}
                      </button>
                    </div>
                  </div>
                ));
              })()}
            </div>
            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={handleOverlayClose}>{t("close")}</button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
