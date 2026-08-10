// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { RemoteSkillUpdate } from "../types";

type Props = {
  t: (key: string) => string;
  updates: RemoteSkillUpdate[];
  marketTitles: Record<number, string>;
  loading: boolean;
  onUpdateOne: (skillName: string, marketId: number) => void;
  onClose: () => void;
};

export default function RemoteUpdatesModal({ t, updates, marketTitles, loading, onUpdateOne, onClose }: Props) {
  return (
    <div className="modal-overlay" onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="modal" role="dialog" aria-modal="true" aria-label={t("updatesAvailableTitle")}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0 }}>
            {t("updatesAvailableTitle")}（{updates.length}）
          </h3>
          <button className="btn btn-small btn-ghost" onClick={onClose} aria-label={t("close")}>
            ✕
          </button>
        </div>

        <div>
          {updates.map((update) => (
            <div key={update.id} className="update-row">
              <div className="update-row-head">
                <div>
                  <div className="update-row-name">{update.skill_name}</div>
                  <div className="update-row-market">{marketTitles[update.market_id] ?? update.market_id}</div>
                </div>
                <button
                  className="btn btn-small btn-primary"
                  onClick={() => onUpdateOne(update.skill_name, update.market_id)}
                  disabled={loading}
                >
                  {t("updateBtn")}
                </button>
              </div>
              <div className="update-row-hash">
                {t("localVersion")} {update.old_hash.slice(0, 6)} → {t("remoteVersion")} {update.new_hash.slice(0, 6)}
              </div>
            </div>
          ))}
        </div>

        <div className="modal-actions">
          <button className="btn btn-secondary" onClick={onClose}>{t("close")}</button>
        </div>
      </div>
    </div>
  );
}
