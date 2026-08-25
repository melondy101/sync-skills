// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useMemo } from "react";
import type { Market, RemoteSkill } from "../types";
import { Icon } from "./Icon";

type Props = {
  t: (key: string) => string;
  markets: Market[];
  loading: boolean;
  skillCounts: RemoteSkill[];
  marketErrors: Record<number, string[]>;
  onAddToggle: () => void;
  showAdd: boolean;
  marketUrl: string;
  setMarketUrl: (value: string) => void;
  marketBranch: string;
  setMarketBranch: (value: string) => void;
  marketLayout: "auto" | "root" | "subdir";
  setMarketLayout: (value: "auto" | "root" | "subdir") => void;
  onAddByUrl: () => void;
  onToggle: (market: Market) => void;
  onDeleteRequest: (market: Market) => void;
  onSyncIndex: (market: Market) => void;
  onChangeLayout: (market: Market, layout: "root" | "subdir") => void;
  builtinIds: Set<string>;
  builtinLabels: Record<string, string>;
  onClose: () => void;
};

type BranchOption =
  | { value: ""; label: string }
  | { value: "main"; label: string }
  | { value: "master"; label: string };

type LayoutOption = {
  value: "auto" | "root" | "subdir";
  label: string;
};

const BRANCH_OPTIONS: BranchOption[] = [
  { value: "", label: "defaultBranchAuto" },
  { value: "main", label: "main" },
  { value: "master", label: "master" },
];

const LAYOUT_OPTIONS: LayoutOption[] = [
  { value: "auto", label: "layoutAuto" },
  { value: "subdir", label: "layoutSubdir" },
  { value: "root", label: "layoutRoot" },
];

export default function MarketSourcesModal({
  t,
  markets,
  loading,
  skillCounts,
  marketErrors,
  onAddToggle,
  showAdd,
  marketUrl,
  setMarketUrl,
  marketBranch,
  setMarketBranch,
  marketLayout,
  setMarketLayout,
  onAddByUrl,
  onToggle,
  onDeleteRequest,
  onSyncIndex,
  onChangeLayout,
  builtinIds,
  builtinLabels,
  onClose,
}: Props) {
  const skillsByMarket = useMemo(() => {
    const counts = new Map<number, number>();
    for (const skill of skillCounts) {
      counts.set(skill.market_id, (counts.get(skill.market_id) ?? 0) + 1);
    }
    return counts;
  }, [skillCounts]);

  return (
    <div className="modal-overlay" onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div className="modal modal-wide" role="dialog" aria-modal="true" aria-label={t("marketSourcesTitle")}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
          <h3 style={{ margin: 0 }}>{t("marketSourcesTitle")}</h3>
          <button className="btn btn-small btn-ghost" onClick={onClose} aria-label={t("close")}>
            <Icon name="x" size={14} />
          </button>
        </div>

        <div style={{ marginBottom: 16 }}>
          <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
            <input
              className="search-input"
              value={marketUrl}
              onChange={(e) => setMarketUrl(e.target.value)}
              placeholder={t("addMarketUrlPlaceholder")}
              style={{ flex: "1 1 220px" }}
            />
            <select
              className="sort-select"
              value={marketBranch}
              onChange={(e) => setMarketBranch(e.target.value)}
              aria-label={t("defaultBranchAuto")}
            >
              {BRANCH_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {t(option.label)}
                </option>
              ))}
            </select>
            <select
              className="sort-select"
              value={marketLayout}
              onChange={(e) => setMarketLayout(e.target.value as "auto" | "root" | "subdir")}
              aria-label={t("marketLayout")}
              title={t("marketLayout")}
            >
              {LAYOUT_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {t("marketLayout")}: {t(option.label)}
                </option>
              ))}
            </select>
            <button className="btn btn-primary" onClick={showAdd ? onAddByUrl : onAddToggle} disabled={loading || (showAdd && !marketUrl.trim())}>
              {showAdd ? t("add") : t("addMarket")}
            </button>
          </div>
          {showAdd && (
            <div className="market-card-meta" style={{ marginTop: 8 }}>
              {t("addMarketUrlHint")}
            </div>
          )}
        </div>

        {markets.length === 0 ? (
          <div className="empty-state">
            <p>{t("marketEmpty")}</p>
          </div>
        ) : (
          <div>
            {markets.map((market) => {
              const isBuiltin = builtinIds.has(String(market.id));
              const skillCount = skillsByMarket.get(market.id) ?? 0;
              const title = builtinLabels[String(market.id)] ?? `${market.owner}/${market.name}`;
              const errors = marketErrors[market.id];
              const firstError = errors && errors.length > 0 ? errors[0] : null;
              return (
                <div key={market.id} className="source-row">
                  <div className="source-info">
                    <div className="source-name">
                      <span>{title}</span>
                      {isBuiltin && <span className="badge-builtin">{t("builtinBadge")}</span>}
                    </div>
                    <div className="source-meta">
                      {market.owner}/{market.name} · {market.branch} · {t("skillsCount").replace("{0}", String(skillCount))} · {t("marketLayout")}: {market.layout === "root" ? t("layoutRoot") : t("layoutSubdir")} · {t("indexedAt").replace("{0}", market.last_indexed_at ?? "-")}
                    </div>
                    {firstError && (
                      <div className="source-error" role="alert" title={errors.length > 1 ? errors.join("\n") : firstError}>
                        {t("lastSyncError").replace("{0}", firstError)}
                        {errors && errors.length > 1 && <span className="source-error-count"> (+{errors.length - 1})</span>}
                      </div>
                    )}
                  </div>
                  <div className="source-actions" style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
                    <select
                      className="sort-select"
                      value={market.layout === "root" ? "root" : "subdir"}
                      onChange={(e) => onChangeLayout(market, e.target.value as "root" | "subdir")}
                      aria-label={t("marketLayout")}
                      title={t("marketLayout")}
                      disabled={loading}
                    >
                      <option value="subdir">{t("layoutSubdir")}</option>
                      <option value="root">{t("layoutRoot")}</option>
                    </select>
                    <button className="btn btn-small btn-secondary" onClick={() => onSyncIndex(market)} disabled={loading || !market.enabled}>
                      {t("syncMarketIndex")}
                    </button>
                    <button className="btn btn-small btn-secondary" onClick={() => onToggle(market)} disabled={loading}>
                      {market.enabled ? t("disable") : t("enable")}
                    </button>
                    {!isBuiltin && (
                      <button className="btn btn-small btn-danger" onClick={() => onDeleteRequest(market)} disabled={loading}>
                        {t("deleteMarket")}
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
