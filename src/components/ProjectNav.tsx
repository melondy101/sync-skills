// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useState } from "react";
import * as api from "../api";
import type { Project } from "../types";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";

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
  const [editingProject, setEditingProject] = useState<Project | null>(null);
  const [editProjectName, setEditProjectName] = useState("");
  const [editProjectPath, setEditProjectPath] = useState("");

  async function handleAddProject() {
    if (!newProjectName.trim() || !newProjectPath.trim()) {
      addToast("error", t("nameAndPathRequiredProject"));
      return;
    }
    if (newProjectPath.startsWith("./") || newProjectPath.startsWith("../")) {
      addToast("error", t("enterAbsolutePath"));
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
      addToast("error", `${t("failedAddProject")}: ${e}`);
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
      addToast("error", `${t("failedUpdateProject")}: ${e}`);
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
            >
              ✎
            </button>
            <button
              className="project-delete"
              onClick={() => handleDeleteProject(p.id, p.name)}
              title={t("deleteProject")}
            >
              ×
            </button>
          </div>
        ))}
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
          <div className="modal" onClick={(e) => e.stopPropagation()}>
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
                className="edit-input"
                placeholder={t("projectPathPlaceholder")}
              />
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
          <div className="modal" onClick={(e) => e.stopPropagation()}>
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
                className="edit-input"
              />
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
