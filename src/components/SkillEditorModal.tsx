// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import * as api from "../api";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";

/**
 * Built-in SKILL.md editor. Loads the SSOT copy on mount; saving writes the
 * file back and re-syncs the skill to all active tools (save_skill_md).
 */
export function SkillEditorModal({
  t,
  skillId,
  skillName,
  projectId,
  addToast,
  onSaved,
  onClose,
}: {
  t: TranslateFn;
  skillId: number;
  skillName: string;
  projectId: number;
  addToast: AddToastFn;
  onSaved: () => Promise<void>;
  onClose: () => void;
}) {
  const [filePath, setFilePath] = useState("");
  const [content, setContent] = useState("");
  const [originalContent, setOriginalContent] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const dirty = content !== originalContent;

  useEffect(() => {
    api.readSkillMd(skillId, projectId)
      .then((file) => {
        setFilePath(file.path);
        setContent(file.content);
        setOriginalContent(file.content);
      })
      .catch((e) => setLoadError(String(e)))
      .finally(() => setLoading(false));
  }, [skillId, projectId]);

  function handleClose() {
    if (dirty && !confirm(t("editorUnsaved"))) return;
    onClose();
  }

  async function handleSave() {
    setSaving(true);
    try {
      const result = await api.saveSkillMd(skillId, projectId, content);
      if (result.errors.length > 0) {
        addToast("error", `${t("editorSaveFailed")}: ${result.errors.join(", ")}`);
        // File was written even if some tool syncs failed — keep editor state consistent
        setOriginalContent(content);
        return;
      }
      addToast("success", t("editorSaved"));
      setOriginalContent(content);
      await onSaved();
      onClose();
    } catch (e) {
      addToast("error", `${t("editorSaveFailed")}: ${e}`);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="modal-overlay" onClick={handleClose}>
      <div className="modal editor-modal" onClick={(e) => e.stopPropagation()}>
        <div className="editor-header">
          <h3>{t("editorTitle")} — {skillName}</h3>
          {dirty && <span className="editor-dirty-dot" title={t("editorUnsaved")}>●</span>}
        </div>
        {filePath && <code className="skill-path editor-path" title={filePath}>{filePath}</code>}

        {loading ? (
          <div className="editor-placeholder">{t("loading")}</div>
        ) : loadError ? (
          <div className="editor-placeholder editor-error">{t("editorLoadFailed")}: {loadError}</div>
        ) : (
          <textarea
            className="editor-textarea"
            value={content}
            onChange={(e) => setContent(e.target.value)}
            spellCheck={false}
          />
        )}

        <div className="modal-actions">
          <button
            className="btn btn-primary"
            onClick={handleSave}
            disabled={loading || loadError !== null || saving || !dirty}
          >
            {saving ? t("editorSaving") : t("editorSave")}
          </button>
          <button className="btn btn-secondary" onClick={handleClose}>{t("close")}</button>
        </div>
      </div>
    </div>
  );
}
