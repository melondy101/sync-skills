// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef, useState } from "react";
import * as api from "../api";
import type { Project, WorkspaceCandidate } from "../types";
import type { TranslateFn } from "../i18n";
import { localizeApiError } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { Icon } from "./Icon";

/** Project sub-navigation with add/edit/delete dialogs. */
export function ProjectNav({
  t,
  projects,
  selectedProject,
  onSelect,
  addToast,
  onProjectsChanged,
}: {
  t: TranslateFn;
  projects: Project[];
  selectedProject: number;
  onSelect: (projectId: number) => void;
  addToast: AddToastFn;
  onProjectsChanged: () => Promise<void>;
}) {
  const [showAddProject, setShowAddProject] = useState(false);
  const [newProjectName, setNewProjectName] = useState("");
  const [newProjectPath, setNewProjectPath] = useState("");
  const [addPathHint, setAddPathHint] = useState("");
  const [editingProject, setEditingProject] = useState<Project | null>(null);
  const [editProjectName, setEditProjectName] = useState("");
  const [editProjectPath, setEditProjectPath] = useState("");
  const [editPathHint, setEditPathHint] = useState("");
  const [wsCandidates, setWsCandidates] = useState<WorkspaceCandidate[]>([]);
  const [addingWorkspaces, setAddingWorkspaces] = useState(false);
  const addDialogRef = useRef<HTMLDivElement>(null);
  const editDialogRef = useRef<HTMLDivElement>(null);

  useFocusTrap(addDialogRef, {
    active: showAddProject,
    onEscape: () => setShowAddProject(false),
  });
  useFocusTrap(editDialogRef, {
    active: editingProject !== null,
    onEscape: () => setEditingProject(null),
  });

  async function handleAddProject() {
    if (!newProjectName.trim() || !newProjectPath.trim()) {
      addToast("error", t("nameAndPathRequiredProject"));
      return;
    }
    try {
      await api.addProject(newProjectName, newProjectPath);
      setShowAddProject(false);
      const addedName = newProjectName;
      setNewProjectName("");
      setNewProjectPath("");
      await onProjectsChanged();
      addToast("success", `${t("projectAdded")} "${addedName}"`);
    } catch (e) {
      addToast("error", `${t("failedAddProject")}: ${localizeApiError(t, String(e))}`);
    }
  }

  // The backend owns the path rules (`src-tauri/src/paths.rs`); the form only asks
  // whether what was typed points at a directory, so a typo shows up before submit.
  async function reportPath(raw: string, setHint: (hint: string) => void) {
    if (!raw.trim()) {
      setHint("");
      return;
    }
    try {
      const check = await api.checkPath(raw);
      setHint(check.is_dir ? "" : `${t("pathErrorNotFound")}: ${check.expanded}`);
    } catch (e) {
      setHint(localizeApiError(t, String(e)));
    }
  }

  // PRD §11 自动检测工作区目录: the tools already remember which folders were
  // opened, so the panel only proposes them — one import per click, and a
  // failure on one candidate must not stop the rest.
  useEffect(() => {
    let active = true;
    api
      .discoverWorkspaces()
      .then((found) => {
        if (active) setWsCandidates(found);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, []);

  async function handleAddWorkspace(candidate: WorkspaceCandidate) {
    try {
      await api.addProject(candidate.name, candidate.path);
      setWsCandidates((prev) => prev.filter((c) => c.path !== candidate.path));
      await onProjectsChanged();
      addToast("success", `${t("projectAdded")} "${candidate.name}"`);
    } catch (e) {
      addToast("error", `${t("failedAddProject")}: ${localizeApiError(t, String(e))}`);
    }
  }

  async function handleAddAllWorkspaces() {
    setAddingWorkspaces(true);
    try {
      for (const candidate of [...wsCandidates]) {
        await handleAddWorkspace(candidate);
      }
    } finally {
      setAddingWorkspaces(false);
    }
  }

  async function handleDeleteProject(projectId: number, projectName: string) {
    if (!confirm(`${t("confirmDeleteProject")} "${projectName}"?\n${t("confirmDeleteProjectMsg")}`)) return;
    try {
      await api.deleteProject(projectId);
      if (selectedProject === projectId) {
        // Auto-select next project or fall back to global
        const remaining = projects.filter((p) => p.id !== projectId);
        onSelect(remaining.length > 0 ? remaining[0].id : 0);
      }
      await onProjectsChanged();
      addToast("success", `${t("projectDeleted")} "${projectName}"`);
    } catch (e) {
      addToast("error", `${t("failedDeleteProject")}: ${e}`);
    }
  }

  function handleEditProject(project: Project) {
    setEditingProject(project);
    setEditProjectName(project.name);
    setEditProjectPath(project.path);
  }

  async function handleSaveProjectEdit() {
    if (!editingProject) return;
    if (!editProjectName.trim() || !editProjectPath.trim()) {
      addToast("error", t("nameAndPathRequiredProject"));
      return;
    }
    try {
      await api.updateProject(editingProject.id, editProjectName.trim(), editProjectPath.trim());
      setEditingProject(null);
      await onProjectsChanged();
      addToast("success", t("projectUpdated"));
    } catch (e) {
      addToast("error", `${t("failedUpdateProject")}: ${localizeApiError(t, String(e))}`);
    }
  }

  return (
    <>
      <div className="project-nav">
        {projects.length === 0 && (
          <span className="project-empty-hint">{t("noProjects")}</span>
        )}
        {projects.map((p) => (
          <div key={p.id} className="project-btn-group">
            <button
              className={`project-btn ${selectedProject === p.id ? "project-active" : ""}`}
              onClick={() => onSelect(p.id)}
            >
              {p.name}
            </button>
            <button
              className="project-edit"
              onClick={() => handleEditProject(p)}
              title={t("editProject")}
              aria-label={t("editProject")}
            >
              <Icon name="pencil" size={14} />
            </button>
            <button
              className="project-delete"
              onClick={() => handleDeleteProject(p.id, p.name)}
              title={t("deleteProject")}
              aria-label={t("deleteProject")}
            >
              <Icon name="x" size={14} />
            </button>
          </div>
        ))}
        {wsCandidates.length > 0 && (
          <div className="workspace-candidates" role="region" aria-label={t("foundWorkspaces")}>
            <p className="workspace-candidates-title">
              {t("foundWorkspaces")} ({wsCandidates.length})
            </p>
            {wsCandidates.map((candidate) => (
              <div className="workspace-candidate" key={`${candidate.source}:${candidate.path}`}>
                <span className="workspace-candidate-name" title={candidate.path}>
                  {candidate.name}
                  <span className="workspace-candidate-source">
                    {candidate.source === "cursor" ? t("wsSourceCursor") : t("wsSourceClaude")}
                  </span>
                </span>
                <button
                  className="btn btn-small btn-primary"
                  disabled={addingWorkspaces}
                  onClick={() => void handleAddWorkspace(candidate)}
                >
                  {t("add")}
                </button>
              </div>
            ))}
            <div className="workspace-candidates-actions">
              <button
                className="btn btn-small btn-secondary"
                disabled={addingWorkspaces}
                onClick={() => void handleAddAllWorkspaces()}
              >
                {t("addAll")}
              </button>
              <button
                className="btn btn-small btn-secondary"
                onClick={() => setWsCandidates([])}
              >
                {t("dismiss")}
              </button>
            </div>
          </div>
        )}

        <button
          className="btn btn-small btn-primary"
          onClick={() => setShowAddProject(true)}
        >
          {t("addProjectBtn")}
        </button>
      </div>

      {/* Add project dialog */}
      {showAddProject && (
        <div className="modal-overlay" onClick={() => setShowAddProject(false)}>
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-label={t("addProjectTitle")}
            onClick={(e) => e.stopPropagation()}
            ref={addDialogRef}
            tabIndex={-1}
          >
            <h3>{t("addProjectTitle")}</h3>
            <div className="form-group">
              <label>{t("nameLabel")}</label>
              <input
                type="text"
                value={newProjectName}
                onChange={(e) => setNewProjectName(e.target.value)}
                className="edit-input"
                placeholder={t("projectNamePlaceholder")}
              />
            </div>
            <div className="form-group">
              <label>{t("pathLabel")}</label>
              <input
                type="text"
                value={newProjectPath}
                onChange={(e) => setNewProjectPath(e.target.value)}
                onBlur={() => void reportPath(newProjectPath, setAddPathHint)}
                className="edit-input"
                placeholder={t("projectPathPlaceholder")}
              />
              {addPathHint && <p className="settings-hint update-error">{addPathHint}</p>}
            </div>
            <div className="modal-actions">
              <button className="btn btn-primary" onClick={handleAddProject}>{t("add")}</button>
              <button className="btn btn-secondary" onClick={() => setShowAddProject(false)}>{t("cancel")}</button>
            </div>
          </div>
        </div>
      )}

      {/* Edit project dialog */}
      {editingProject && (
        <div className="modal-overlay" onClick={() => setEditingProject(null)}>
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-label={t("editProjectTitle")}
            onClick={(e) => e.stopPropagation()}
            ref={editDialogRef}
            tabIndex={-1}
          >
            <h3>{t("editProjectTitle")}</h3>
            <div className="form-group">
              <label>{t("nameLabel")}</label>
              <input
                type="text"
                value={editProjectName}
                onChange={(e) => setEditProjectName(e.target.value)}
                className="edit-input"
              />
            </div>
            <div className="form-group">
              <label>{t("pathLabel")}</label>
              <input
                type="text"
                value={editProjectPath}
                onChange={(e) => setEditProjectPath(e.target.value)}
                onBlur={() => void reportPath(editProjectPath, setEditPathHint)}
                className="edit-input"
              />
              {editPathHint && <p className="settings-hint update-error">{editPathHint}</p>}
            </div>
            <div className="modal-actions">
              <button className="btn btn-primary" onClick={handleSaveProjectEdit}>{t("save")}</button>
              <button className="btn btn-secondary" onClick={() => setEditingProject(null)}>{t("cancel")}</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
