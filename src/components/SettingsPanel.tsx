// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Settings } from "../types";
import type { TranslateFn } from "../i18n";

/** Compare semver-ish strings numerically; returns >0 if a is newer than b. */
function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map((n) => parseInt(n, 10) || 0);
  const pb = b.split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
}

export function SettingsPanel({
  t,
  settings,
  onChange,
  onSave,
  onBack,
}: {
  t: TranslateFn;
  settings: Settings;
  onChange: (settings: Settings) => void;
  onSave: () => void;
  onBack: () => void;
}) {
  // App self-update check (local to this panel)
  const [appVersion, setAppVersion] = useState("");
  const [updateCheck, setUpdateCheck] = useState<{
    status: "idle" | "checking" | "latest" | "outdated" | "none" | "error";
    latest?: string;
    url?: string;
    error?: string;
  }>({ status: "idle" });

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => {});
  }, []);

  async function handleCheckAppUpdate() {
    setUpdateCheck({ status: "checking" });
    try {
      const resp = await fetch(
        "https://api.github.com/repos/huang-yi-dae/sync-skills/releases/latest",
        { headers: { Accept: "application/vnd.github+json" } },
      );
      if (resp.status === 404) {
        // Repo has no published releases yet
        setUpdateCheck({ status: "none" });
        return;
      }
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const rel = await resp.json();
      const latest = String(rel.tag_name || "").replace(/^v/, "");
      const url = rel.html_url || "https://github.com/huang-yi-dae/sync-skills/releases";
      if (latest && compareVersions(latest, appVersion) > 0) {
        setUpdateCheck({ status: "outdated", latest, url });
      } else {
        setUpdateCheck({ status: "latest", latest: latest || appVersion });
      }
    } catch (e) {
      setUpdateCheck({ status: "error", error: String(e) });
    }
  }

  return (
    <section className="section">
      <div className="panel-header">
        <h2 className="section-title">{t("settingsTitle")}</h2>
        <button className="btn btn-secondary" onClick={onBack}>
          {t("back")}
        </button>
      </div>

      <div className="settings-group">
        <label className="settings-label">{t("appearance")}</label>
        <div className="settings-row">
          <div className="settings-field">
            <span className="settings-field-label">{t("theme")}</span>
            <select
              className="settings-select"
              value={settings.theme}
              onChange={(e) => onChange({ ...settings, theme: e.target.value })}
            >
              <option value="light">{t("themeLight")}</option>
              <option value="dark">{t("themeDark")}</option>
              <option value="system">{t("themeSystem")}</option>
            </select>
          </div>
          <div className="settings-field">
            <span className="settings-field-label">{t("language")}</span>
            <select
              className="settings-select"
              value={settings.language}
              onChange={(e) => onChange({ ...settings, language: e.target.value })}
            >
              <option value="zh">{t("langZh")}</option>
              <option value="en">{t("langEn")}</option>
            </select>
          </div>
        </div>
      </div>

      <div className="settings-group">
        <label className="settings-label">{t("syncMode")}</label>
        <select
          className="settings-select"
          value={settings.sync_mode}
          onChange={(e) => onChange({ ...settings, sync_mode: e.target.value })}
        >
          <option value="semi-auto">{t("syncModeSemi")}</option>
          <option value="full-auto">{t("syncModeFull")}</option>
        </select>
        <p className="settings-hint">
          {t("syncModeHint")}
        </p>
      </div>

      <div className="settings-group">
        <label className="settings-label">
          <input
            type="checkbox"
            checked={settings.prefer_symlink}
            onChange={(e) => onChange({ ...settings, prefer_symlink: e.target.checked })}
          />
          {" "}{t("preferSymlink")}
        </label>
        <p className="settings-hint">
          {t("symlinkHint")}
        </p>
      </div>

      <div className="settings-group">
        <label className="settings-label">{t("appUpdateSection")}</label>
        <div className="app-update-row">
          <span className="app-version">{t("currentVersion")}: v{appVersion || "?"}</span>
          <button
            className="btn btn-small"
            onClick={handleCheckAppUpdate}
            disabled={updateCheck.status === "checking"}
          >
            {updateCheck.status === "checking" ? t("checkingAppUpdate") : t("checkAppUpdate")}
          </button>
        </div>
        {updateCheck.status === "latest" && (
          <p className="settings-hint update-latest">✓ {t("upToDate")} (v{updateCheck.latest})</p>
        )}
        {updateCheck.status === "none" && (
          <p className="settings-hint">{t("noReleases")}</p>
        )}
        {updateCheck.status === "outdated" && (
          <div className="app-update-row">
            <span className="settings-hint update-available">
              {t("newVersionFound")}: v{updateCheck.latest}
            </span>
            <button
              className="btn btn-primary btn-small"
              onClick={() => updateCheck.url && openUrl(updateCheck.url)}
            >
              {t("viewRelease")}
            </button>
          </div>
        )}
        {updateCheck.status === "error" && (
          <p className="settings-hint update-error">{t("updateCheckFailed")}: {updateCheck.error}</p>
        )}
      </div>

      <button className="btn btn-primary" onClick={onSave}>
        {t("saveSettings")}
      </button>
    </section>
  );
}
