// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useEffect, useState } from "react";
import * as api from "../api";
import type { SyncLog } from "../types";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";

export function LogsPanel({
  t,
  addToast,
  onBack,
}: {
  t: TranslateFn;
  addToast: AddToastFn;
  onBack: () => void;
}) {
  const [syncLogs, setSyncLogs] = useState<SyncLog[]>([]);

  const loadSyncLogs = useCallback(async () => {
    try {
      setSyncLogs(await api.getSyncLogs(null, 50));
    } catch (e) {
      addToast("error", `${t("failedLoadLogs")}: ${e}`);
    }
  }, [addToast, t]);

  useEffect(() => {
    loadSyncLogs();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const actionLabels: Record<string, string> = {
    scan: t("actionScan"),
    sync: t("actionSync"),
    remove: t("actionRemove"),
    toggle_on: t("actionEnable"),
    toggle_off: t("actionDisable"),
    add_tool: t("actionAddTool"),
    delete_tool: t("actionDeleteTool"),
    add_project: t("actionAddProject"),
    delete_project: t("actionDeleteProject"),
    edit: t("actionEdit"),
  };

  return (
    <section className="section">
      <div className="panel-header">
        <h2 className="section-title">{t("activityLogs")}</h2>
        <div className="panel-actions">
          <button className="btn btn-small" onClick={loadSyncLogs}>{t("refresh")}</button>
          <button className="btn btn-secondary" onClick={onBack}>{t("back")}</button>
        </div>
      </div>

      {syncLogs.length === 0 ? (
        <div className="empty-state">
          <p>{t("noLogs")}</p>
        </div>
      ) : (
        <div className="log-table-wrapper">
          <table className="log-table">
            <thead>
              <tr>
                <th>{t("time")}</th>
                <th>{t("action")}</th>
                <th>{t("skill")}</th>
                <th>{t("tool")}</th>
                <th>{t("status")}</th>
                <th>{t("detail")}</th>
              </tr>
            </thead>
            <tbody>
              {syncLogs.map((log) => (
                <tr key={log.id} className={`log-row log-${log.status}`}>
                  <td className="log-time">{log.created_at}</td>
                  <td>
                    <span className={`action-badge action-${log.action}`}>
                      {actionLabels[log.action] || log.action}
                    </span>
                    {log.direction && (
                      <span className={`direction-badge dir-${log.direction}`}>
                        {log.direction === "to_ssot" ? "→ SSOT" : "← SSOT"}
                      </span>
                    )}
                  </td>
                  <td>{log.skill_name || "—"}</td>
                  <td>{log.tool_name || (log.action === "delete_tool" || log.action === "add_tool" ? log.detail : "—")}</td>
                  <td>
                    <span className={`status-badge status-${log.status}`}>
                      {log.status}
                    </span>
                  </td>
                  <td className="log-error">{log.detail || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
