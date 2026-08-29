// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useMemo, useState } from "react";
import { Icon } from "./Icon";
import type { Market, RemoteSkill } from "../types";

type Props = {
  markets: Market[];
  remoteSkills: RemoteSkill[];
  selectedMarketFilter: string;
  onSelectFilter: (filter: string) => void;
  onAddSource: () => void;
  marketTitle: (m: Market) => string;
  t: (key: string) => string;
};

export default function MarketSidebar({
  markets,
  remoteSkills,
  selectedMarketFilter,
  onSelectFilter,
  onAddSource,
  marketTitle,
  t,
}: Props) {
  const [collapsed, setCollapsed] = useState(false);
  const [filterQuery, setFilterQuery] = useState("");

  const skillsByMarket = useMemo(() => {
    const counts = new Map<number, number>();
    for (const skill of remoteSkills) {
      counts.set(skill.market_id, (counts.get(skill.market_id) ?? 0) + 1);
    }
    return counts;
  }, [remoteSkills]);

  const totalSkills = remoteSkills.length;
  const enabledMarkets = markets.filter((m) => m.enabled);

  const filteredMarkets = useMemo(() => {
    const q = filterQuery.trim().toLowerCase();
    if (!q) return enabledMarkets;
    return enabledMarkets.filter((m) =>
      marketTitle(m).toLowerCase().includes(q),
    );
  }, [enabledMarkets, filterQuery, marketTitle]);

  return (
    <aside className={`m-sidebar ${collapsed ? "m-sidebar-collapsed" : ""}`}>
      <div className="m-sidebar-head">
        {!collapsed && (
          <h3 className="m-sidebar-title">{t("sidebarSources")}</h3>
        )}
        <button
          className="m-sidebar-collapse-btn"
          onClick={() => setCollapsed((v) => !v)}
          title={collapsed ? t("sidebarExpand") : t("sidebarCollapse")}
          aria-label={collapsed ? t("sidebarExpand") : t("sidebarCollapse")}
        >
          <Icon
            name={collapsed ? "panel-left-open" : "panel-left-close"}
            size={16}
          />
        </button>
      </div>

      {!collapsed && (
        <div className="m-sidebar-filter">
          <input
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder={t("sidebarFilterPlaceholder")}
            className="m-sidebar-filter-input"
          />
        </div>
      )}

      <div className="m-sidebar-list">
        <button
          className={`m-src-item ${selectedMarketFilter === "all" ? "selected" : ""}`}
          onClick={() => onSelectFilter("all")}
          title={collapsed ? t("sidebarAllSources") : undefined}
        >
          <div className="m-src-avatar m-src-avatar-all">
            <Icon name="layers" size={16} />
          </div>
          {!collapsed && (
            <>
              <div className="m-src-info">
                <div className="m-src-name">{t("sidebarAllSources")}</div>
                <div className="m-src-meta">
                  {enabledMarkets.length} repos · {totalSkills} skills
                </div>
              </div>
              <span className="m-src-count">{totalSkills}</span>
            </>
          )}
        </button>

        {filteredMarkets.map((m) => {
          const count = skillsByMarket.get(m.id) ?? 0;
          const initial = marketTitle(m).charAt(0).toUpperCase();
          return (
            <button
              key={m.id}
              className={`m-src-item ${String(selectedMarketFilter) === String(m.id) ? "selected" : ""}`}
              onClick={() => onSelectFilter(String(m.id))}
              title={collapsed ? marketTitle(m) : undefined}
            >
              <div className="m-src-avatar">{initial}</div>
              {!collapsed && (
                <>
                  <div className="m-src-info">
                    <div className="m-src-name">{marketTitle(m)}</div>
                    <div className="m-src-meta">
                      {count} skills
                      {m.last_indexed_at && (
                        <> · {m.last_indexed_at}</>
                      )}
                    </div>
                  </div>
                  <span className="m-src-count">{count}</span>
                </>
              )}
            </button>
          );
        })}
      </div>

      <div className="m-sidebar-foot">
        <button className="m-add-src" onClick={onAddSource}>
          <Icon name="plus" size={14} />
          {!collapsed && <span>{t("sidebarAddSource")}</span>}
        </button>
      </div>
    </aside>
  );
}
