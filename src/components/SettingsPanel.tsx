// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import * as api from "../api";
import type { AppUpdateInfo, Settings } from "../types";
import type { TranslateFn } from "../i18n";

type UpdateCheckState =
  | { status: "idle" }
  | { status: "checking" }
  | { status: "latest"; latest: string }
  | { status: "none" }
  | { status: "outdated"; info: AppUpdateInfo }
  | { status: "downloading"; info: AppUpdateInfo; percent: number }
  | { status: "downloaded"; info: AppUpdateInfo; path: string }
  | { status: "installing" }
  | { status: "error"; error: string };

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
  const [updateCheck, setUpdateCheck] = useState<UpdateCheckState>({ status: "idle" });

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => {});
  }, []);

  async function handleCheckAppUpdate() {
    setUpdateCheck({ status: "checking" });
    try {
      const info = await api.checkAppUpdate();
      if (info.no_releases) {
        setUpdateCheck({ status: "none" });
      } else if (info.update_available) {
        setUpdateCheck({ status: "outdated", info });
      } else {
        setUpdateCheck({ status: "latest", latest: info.latest_version });
      }
    } catch (e) {
      const raw = String(e);
      // Backend sentinel for "could not reach GitHub at all"
      setUpdateCheck({
        status: "error",
        error: raw.includes("NETWORK_ERROR") ? t("updateNetworkError") : raw,
      });
    }
  }

  async function handleDownloadUpdate(info: AppUpdateInfo) {
    // No installer asset for this platform: fall back to the release page
    if (!info.asset_url || !info.asset_name) {
      openUrl(info.release_url);
      return;
    }
    setUpdateCheck({ status: "downloading", info, percent: 0 });
    const unlisten = await listen<{ downloaded: number; total: number }>(
      "app-update-progress",
      (e) => {
        const { downloaded, total } = e.payload;
        const size = total > 0 ? total : info.asset_size ?? 0;
        const percent = size > 0 ? Math.min(100, Math.round((downloaded / size) * 100)) : 0;
        setUpdateCheck({ status: "downloading", info, percent });
      },
    );
    try {
      const path = await api.downloadAppUpdate(info.asset_url, info.asset_name);
      setUpdateCheck({ status: "downloaded", info, path });
    } catch (e) {
      setUpdateCheck({ status: "error", error: String(e) });
    } finally {
      unlisten();
    }
  }

  async function handleInstallUpdate(path: string) {
    setUpdateCheck({ status: "installing" });
    try {
      await api.installAppUpdate(path);
      // App exits here on success
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
            checked={settings.auto_sync_on_file_change}
            onChange={(e) => onChange({ ...settings, auto_sync_on_file_change: e.target.checked })}
          />
          {" "}{t("autoSyncOnFileChange")}
        </label>
        <p className="settings-hint">
          {t("autoSyncOnFileChangeHint")}
        </p>
      </div>

      <div className="settings-group">
        <label className="settings-label">
          <input
            type="checkbox"
            checked={settings.view_market_diff_before_update}
            onChange={(e) => onChange({ ...settings, view_market_diff_before_update: e.target.checked })}
          />
          {" "}{t("viewMarketDiffBeforeUpdate")}
        </label>
        <p className="settings-hint">
          {t("viewMarketDiffBeforeUpdateHint")}
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
        <label className="settings-label">{t("closeAction")}</label>
        <select
          className="settings-select"
          value={settings.close_action}
          onChange={(e) => onChange({ ...settings, close_action: e.target.value })}
        >
          <option value="exit">{t("closeActionExit")}</option>
          <option value="tray">{t("closeActionTray")}</option>
          <option value="minimize">{t("closeActionMinimize")}</option>
        </select>
        <p className="settings-hint">
          {t("closeActionHint")}
        </p>
      </div>

      <div className="settings-group">
        <label className="settings-label">{t("proxySection")}</label>
        <label className="settings-label">
          <input
            type="checkbox"
            checked={settings.use_system_proxy}
            onChange={(e) => onChange({ ...settings, use_system_proxy: e.target.checked })}
          />
          {" "}{t("systemProxy")}
        </label>
        <label className="settings-label">
          <input
            type="checkbox"
            checked={settings.use_proxy}
            disabled={settings.use_system_proxy}
            onChange={(e) => onChange({ ...settings, use_proxy: e.target.checked })}
          />
          {" "}{t("useProxy")}
        </label>
        <div className="settings-row">
          <div className="settings-field">
            <span className="settings-field-label">{t("proxyUrl")}</span>
            <input
              className="settings-input"
              type="text"
              value={settings.proxy_url ?? ""}
              onChange={(e) => onChange({ ...settings, proxy_url: e.target.value || null })}
              placeholder={t("proxyUrlPlaceholder")}
              disabled={settings.use_system_proxy || !settings.use_proxy}
            />
          </div>
        </div>
        <p className="settings-hint">{t("proxyHint")}</p>
      </div>
      <div className="settings-group">
        <label className="settings-label">{t("appUpdateSection")}</label>
        <div className="app-update-row">
          <span className="app-version">{t("currentVersion")}: v{appVersion || "?"}</span>
          <button
            className="btn btn-small"
            onClick={handleCheckAppUpdate}
            disabled={["checking", "downloading", "installing"].includes(updateCheck.status)}
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
              {t("newVersionFound")}: v{updateCheck.info.latest_version}
            </span>
            <button
              className="btn btn-primary btn-small"
              onClick={() => handleDownloadUpdate(updateCheck.info)}
            >
              {updateCheck.info.asset_url ? t("downloadUpdate") : t("viewRelease")}
            </button>
          </div>
        )}
        {updateCheck.status === "downloading" && (
          <div className="app-update-row">
            <div className="update-progress-track">
              <div
                className="update-progress-fill"
                style={{ width: `${updateCheck.percent}%` }}
              />
            </div>
            <span className="settings-hint">
              {t("downloadingUpdate")}... {updateCheck.percent}%
            </span>
          </div>
        )}
        {updateCheck.status === "downloaded" && (
          <div className="app-update-row">
            <span className="settings-hint update-latest">✓ {t("downloadComplete")}</span>
            <button
              className="btn btn-primary btn-small"
              onClick={() => handleInstallUpdate(updateCheck.path)}
            >
              {t("installNow")}
            </button>
            <span className="settings-hint">{t("installHint")}</span>
          </div>
        )}
        {updateCheck.status === "installing" && (
          <p className="settings-hint">{t("installingUpdate")}</p>
        )}
        {updateCheck.status === "error" && (
          <div>
            <p className="settings-hint update-error">{t("updateCheckFailed")}: {updateCheck.error}</p>
            <p className="settings-hint">{t("updateProxyHint")}</p>
          </div>
        )}
      </div>

      <button className="btn btn-primary" onClick={onSave}>
        {t("saveSettings")}
      </button>
    </section>
  );
}
