// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import * as api from "../api";
import type { Tool, ToolTemplate } from "../types";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";
import { Icon } from "./Icon";

function isAbsolutePath(p: string): boolean {
  return (
    p.startsWith("/") ||
    p.startsWith("~") ||
    /^[A-Za-z]:[\\/]/.test(p) ||
    p.startsWith("\\\\")
  );
}

/** Tool paths section: discovery banner, add form, per-tool edit/delete. */
export function ToolsSection({
  t,
  tools,
  addToast,
  onToolsChanged,
}: {
  t: TranslateFn;
  tools: Tool[];
  addToast: AddToastFn;
  onToolsChanged: () => Promise<void>;
}) {
  // Tool editing
  const [editingTool, setEditingTool] = useState<number | null>(null);
  const [editGlobalPath, setEditGlobalPath] = useState("");
  const [editRelPath, setEditRelPath] = useState("");
  const [showAddTool, setShowAddTool] = useState(false);
  const [newToolName, setNewToolName] = useState("");
  const [newToolGlobal, setNewToolGlobal] = useState("");
  const [newToolRel, setNewToolRel] = useState("");
  const [discoveredTools, setDiscoveredTools] = useState<ToolTemplate[]>([]);
  const [templates, setTemplates] = useState<ToolTemplate[]>([]);
  const [addingDiscovered, setAddingDiscovered] = useState(false);
  // Collapsible tool paths section (persisted across sessions)
  const [toolPathsCollapsed, setToolPathsCollapsed] = useState(
    () => localStorage.getItem("toolPathsCollapsed") === "1",
  );

  useEffect(() => {
    // Discovery and templates are nice-to-haves — fail silently
    api.discoverTools().then(setDiscoveredTools).catch(() => {});
    api.listToolTemplates().then(setTemplates).catch(() => {});
  }, []);

  function toggleToolPaths() {
    setToolPathsCollapsed((c) => {
      localStorage.setItem("toolPathsCollapsed", c ? "0" : "1");
      return !c;
    });
  }

  async function handleAddDiscoveredAll() {
    if (discoveredTools.length === 0) return;
    setAddingDiscovered(true);
    let added = 0;
    for (const dt of discoveredTools) {
      try {
        await api.addTool(dt.name, dt.global_path, dt.project_rel_path);
        added++;
      } catch {
        // Skip duplicates silently
      }
    }
    setDiscoveredTools([]);
    setAddingDiscovered(false);
    await onToolsChanged();
    if (added > 0) {
      addToast("success", `${t("addedTools")} ${added} ${t("toolUnit")}`);
    }
  }

  function handleSelectTemplate(e: React.ChangeEvent<HTMLSelectElement>) {
    const idx = parseInt(e.target.value, 10);
    if (isNaN(idx) || idx < 0) {
      setNewToolName("");
      setNewToolGlobal("");
      setNewToolRel("");
      return;
    }
    const tp = templates[idx];
    if (tp) {
      setNewToolName(tp.name);
      setNewToolGlobal(tp.global_path);
      setNewToolRel(tp.project_rel_path);
    }
  }

  function startEdit(tool: Tool) {
    setEditingTool(tool.id);
    setEditGlobalPath(tool.global_path);
    setEditRelPath(tool.project_rel_path);
  }

  async function saveEdit(toolId: number) {
    if (!editGlobalPath.trim()) {
      addToast("error", t("globalPathNotEmpty"));
      return;
    }
    const gPath = editGlobalPath.trim();
    if (!isAbsolutePath(gPath)) {
      addToast("error", t("globalPathMustBeAbsolute"));
      return;
    }
    try {
      await api.updateToolPath(toolId, gPath, editRelPath.trim());
      setEditingTool(null);
      await onToolsChanged();
      addToast("success", t("pathUpdated"));
    } catch (e) {
      addToast("error", `${t("failedUpdate")}: ${e}`);
    }
  }

  async function handleAddTool() {
    if (!newToolName.trim() || !newToolGlobal.trim()) {
      addToast("error", t("nameAndPathRequired"));
      return;
    }
    // Validate global path is absolute
    const gPath = newToolGlobal.trim();
    if (!isAbsolutePath(gPath)) {
      addToast("error", t("globalPathMustBeAbsolute"));
      return;
    }
    // Validate relative path doesn't start with .
    const rPath = newToolRel.trim();
    if (rPath.startsWith("./") || rPath.startsWith("../") || rPath === ".") {
      addToast("error", t("relPathNoRelative"));
      return;
    }
    try {
      await api.addTool(newToolName.trim(), gPath, rPath);
      setShowAddTool(false);
      const addedName = newToolName.trim();
      setNewToolName("");
      setNewToolGlobal("");
      setNewToolRel("");
      await onToolsChanged();
      addToast("success", `${t("toolAdded")} "${addedName}"`);
    } catch (e) {
      addToast("error", `${t("failedAddTool")}: ${e}`);
    }
  }

  async function handleDeleteTool(toolId: number, toolName: string) {
    if (!confirm(`${t("confirmDeleteTool")} "${toolName}"?\n${t("confirmDeleteToolMsg")}`)) return;
    try {
      if (editingTool === toolId) {
        setEditingTool(null);
      }
      await api.deleteTool(toolId, toolName);
      await onToolsChanged();
      addToast("success", `${t("toolDeleted")} "${toolName}"`);
    } catch (e) {
      await onToolsChanged();
      addToast("error", `${t("failedDeleteTool")}: ${e}`);
    }
  }

  return (
    <section className="section">
      <div className="section-header">
        <h2
          className="section-title section-title-toggle"
          onClick={toggleToolPaths}
          title={toolPathsCollapsed ? t("expand") : t("collapse")}
        >
          <span className="collapse-chevron">{toolPathsCollapsed ? "\u25B8" : "\u25BE"}</span>
          {t("toolPaths")}
          {toolPathsCollapsed && <span className="collapsed-count">({tools.length})</span>}
        </h2>
        <button
          className="btn btn-small"
          onClick={() => {
            // Opening the add form should also expand a collapsed section
            if (toolPathsCollapsed && !showAddTool) toggleToolPaths();
            setShowAddTool(!showAddTool);
          }}
        >
          {showAddTool ? t("cancel") : t("addTool")}
        </button>
      </div>

      {!toolPathsCollapsed && (<>
      {discoveredTools.length > 0 && (
        <div className="discovery-banner">
          <span className="discovery-text">
            {t("found")} {discoveredTools.length} {t("toolUnit")}：{" "}
            {discoveredTools.map((dt) => dt.name).join(", ")}
          </span>
          <button
            className="btn btn-primary btn-small"
            onClick={handleAddDiscoveredAll}
            disabled={addingDiscovered}
          >
            {addingDiscovered ? t("adding") : t("addAll")}
          </button>
          <button
            className="btn btn-small"
            onClick={() => setDiscoveredTools([])}
          >
            {t("dismiss")}
          </button>
        </div>
      )}

      {showAddTool && (
        <div className="add-tool-form">
          {templates.length > 0 && (
            <div className="form-row">
              <select
                className="template-select"
                defaultValue=""
                onChange={handleSelectTemplate}
              >
                <option value="" disabled>{t("templatePlaceholder")}</option>
                {templates.map((tp, i) => (
                  <option key={tp.name} value={i}>{tp.name}</option>
                ))}
              </select>
            </div>
          )}
          <div className="form-row">
            <input type="text" value={newToolName} onChange={(e) => setNewToolName(e.target.value)}
              className="edit-input" placeholder={t("toolNamePlaceholder")} />
            <input type="text" value={newToolGlobal} onChange={(e) => setNewToolGlobal(e.target.value)}
              className="edit-input" placeholder={t("globalPathPlaceholder")} />
            <input type="text" value={newToolRel} onChange={(e) => setNewToolRel(e.target.value)}
              className="edit-input" placeholder={t("relPathPlaceholder")} />
            <button className="btn btn-primary btn-small" onClick={handleAddTool}>{t("add")}</button>
          </div>
        </div>
      )}

      <div className="tool-list">
        {tools.map((tool) => (
          <div key={tool.id} className="tool-item">
            <div className="tool-header">
              <span className="tool-name">{tool.name}</span>
              <div className="tool-actions">
                <button className="btn btn-small" onClick={() => startEdit(tool)}>{t("edit")}</button>
                <button
                  className="btn btn-small btn-danger"
                  onClick={() => handleDeleteTool(tool.id, tool.name)}
                  aria-label={t("deleteTool")}
                >
                  <Icon name="x" size={12} />
                </button>
              </div>
            </div>
            {editingTool === tool.id ? (
              <div className="tool-edit">
                <div className="edit-row">
                  <label>{t("globalPath")}</label>
                  <input type="text" value={editGlobalPath} onChange={(e) => setEditGlobalPath(e.target.value)}
                    className="edit-input" />
                </div>
                <div className="edit-row">
                  <label>{t("projectPath")}</label>
                  <input type="text" value={editRelPath} onChange={(e) => setEditRelPath(e.target.value)}
                    className="edit-input" />
                </div>
                <div className="edit-actions">
                  <button onClick={() => saveEdit(tool.id)} className="btn btn-primary btn-small">{t("save")}</button>
                  <button onClick={() => setEditingTool(null)} className="btn btn-secondary btn-small">{t("cancel")}</button>
                </div>
              </div>
            ) : (
              <code className="path-text">{tool.global_path}</code>
            )}
          </div>
        ))}
      </div>
      </>)}
    </section>
  );
}
