// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { ScanResult } from "../types";
import type { TranslateFn } from "../i18n";

export function ScanResultModal({
  t,
  result,
  onClose,
}: {
  t: TranslateFn;
  result: ScanResult;
  onClose: () => void;
}) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal scan-result-modal" role="dialog" aria-modal="true" aria-label={t("scanResults")} onClick={(e) => e.stopPropagation()}>
        <h3>{t("scanResults")}</h3>
        <div className="scan-summary">
          <span className="scan-summary-item">
            {t("found")} <strong>{result.skills_found}</strong>
          </span>
          {result.skills_new > 0 && (
            <span className="scan-summary-item scan-new">
              {result.skills_new} {t("new")}
            </span>
          )}
          {result.skills_updated > 0 && (
            <span className="scan-summary-item scan-updated">
              {result.skills_updated} {t("updated")}
            </span>
          )}
        </div>
        <div className="scan-detail-table-wrapper">
          <table className="scan-detail-table">
            <thead>
              <tr>
                <th>{t("skill")}</th>
                <th>{t("tool")}</th>
                <th>{t("scope")}</th>
                <th>{t("status")}</th>
              </tr>
            </thead>
            <tbody>
              {result.details.map((d, i) => (
                <tr key={i}>
                  <td className="scan-skill-name" title={d.source_path}>{d.skill_name}</td>
                  <td className="scan-tool-name">{d.tool_name}</td>
                  <td className="scan-scope">{d.scope}</td>
                  <td>
                    <span className={`scan-status scan-status-${d.status}`}>
                      {d.status}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="modal-actions">
          <button className="btn btn-primary" onClick={onClose}>{t("close")}</button>
        </div>
      </div>
    </div>
  );
}
