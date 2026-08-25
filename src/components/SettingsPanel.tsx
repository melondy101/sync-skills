// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import * as api from "../api";
import type { AppUpdateInfo, Settings } from "../types";
import type { TranslateFn } from "../i18n";
import { Icon } from "./Icon";
import { ToggleSwitch } from "./ToggleSwitch";

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

type Section = "appearance" | "sync" | "proxy" | "update";

const SECTIONS: ReadonlyArray<{ key: Section; labelKey: string }> = [
  { key: "appearance", labelKey: "appearance" },
  { key: "sync", labelKey: "syncMode" },
  { key: "proxy", labelKey: "proxySection" },
  { key: "update", labelKey: "appUpdateSection" },
];

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
  const [section, setSection] = useState<Section>("appearance");

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

      <div className="settings-layout">
        <nav className="settings-nav" aria-label={t("settingsTitle")}>
          {SECTIONS.map((s) => (
            <button
              key={s.key}
              type="button"
              className={`settings-nav-item${section === s.key ? " settings-nav-item-active" : ""}`}
              onClick={() => setSection(s.key)}
              aria-current={section === s.key ? "page" : undefined}
            >
              {t(s.labelKey)}
            </button>
          ))}
        </nav>

        <div className="settings-content">
          {section === "appearance" && (
            <>
              <div className="settings-group">
                <label className="settings-label">{t("theme")}</label>
                <div className="theme-cards">
                  {(["light", "dark", "system"] as const).map((theme) => (
                    <button
                      key={theme}
                      type="button"
                      className={`theme-card${settings.theme === theme ? " theme-card-active" : ""}`}
                      onClick={() => onChange({ ...settings, theme })}
                      aria-pressed={settings.theme === theme}
                    >
                      <div className={`theme-preview theme-preview-${theme}`} />
                      <div className="theme-card-label">
                        {t(`theme${theme.charAt(0).toUpperCase()}${theme.slice(1)}`)}
                      </div>
                    </button>
                  ))}
                </div>
              </div>

              <div className="settings-group">
                <label className="settings-label">{t("language")}</label>
                <div className="theme-cards">
                  {(["zh", "en"] as const).map((lang) => (
                    <button
                      key={lang}
                      type="button"
                      className={`theme-card${settings.language === lang ? " theme-card-active" : ""}`}
                      onClick={() => onChange({ ...settings, language: lang })}
                      aria-pressed={settings.language === lang}
                    >
                      <div className="theme-card-label">{t(`lang${lang.toUpperCase()}`)}</div>
                    </button>
                  ))}
                </div>
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
                <p className="settings-hint">{t("closeActionHint")}</p>
              </div>
            </>
          )}

          {section === "sync" && (
            <>
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
                <p className="settings-hint">{t("syncModeHint")}</p>
              </div>

              <div className="settings-group">
                <ToggleSwitch
                  checked={settings.auto_sync_on_file_change}
                  onChange={(next) => onChange({ ...settings, auto_sync_on_file_change: next })}
                  label={t("autoSyncOnFileChange")}
                />
                <p className="settings-hint">{t("autoSyncOnFileChangeHint")}</p>
              </div>

              <div className="settings-group">
                <ToggleSwitch
                  checked={settings.view_market_diff_before_update}
                  onChange={(next) => onChange({ ...settings, view_market_diff_before_update: next })}
                  label={t("viewMarketDiffBeforeUpdate")}
                />
                <p className="settings-hint">{t("viewMarketDiffBeforeUpdateHint")}</p>
              </div>

              <div className="settings-group">
                <ToggleSwitch
                  checked={settings.prefer_symlink}
                  onChange={(next) => onChange({ ...settings, prefer_symlink: next })}
                  label={t("preferSymlink")}
                />
                <p className="settings-hint">{t("symlinkHint")}</p>
              </div>
            </>
          )}

          {section === "proxy" && (
            <>
              <div className="settings-group">
                <ToggleSwitch
                  checked={settings.use_system_proxy}
                  onChange={(next) => onChange({ ...settings, use_system_proxy: next })}
                  label={t("systemProxy")}
                />
              </div>
              <div className="settings-group">
                <ToggleSwitch
                  checked={settings.use_proxy}
                  disabled={settings.use_system_proxy}
                  onChange={(next) => onChange({ ...settings, use_proxy: next })}
                  label={t("useProxy")}
                />
              </div>
              <div className="settings-group">
                <label className="settings-label">{t("proxyUrl")}</label>
                <input
                  className="settings-input"
                  type="text"
                  value={settings.proxy_url ?? ""}
                  onChange={(e) => onChange({ ...settings, proxy_url: e.target.value || null })}
                  placeholder={t("proxyUrlPlaceholder")}
                  disabled={settings.use_system_proxy || !settings.use_proxy}
                />
              </div>
              <p className="settings-hint">{t("proxyHint")}</p>
            </>
          )}

          {section === "update" && (
            <>
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
                  <p className="settings-hint update-latest"><Icon name="check" size={14} /> {t("upToDate")} (v{updateCheck.latest})</p>
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
                    <span className="settings-hint update-latest"><Icon name="check" size={14} /> {t("downloadComplete")}</span>
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
            </>
          )}
        </div>
      </div>

      <div className="settings-footer">
        <button className="btn btn-primary" onClick={onSave}>
          {t("saveSettings")}
        </button>
      </div>
    </section>
  );
}