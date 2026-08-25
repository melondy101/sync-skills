// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import * as api from "../api";
import type { ScanResult, ToolTemplate } from "../types";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";
import { Icon } from "./Icon";

/**
 * First-run onboarding wizard (3 steps): add discovered tools -> scan -> done.
 * Visibility is controlled by the parent via the localStorage "onboardingDone" flag.
 *
 * Renders as a full-bleed branded splash (ticket #11): centered hero,
 * dot-based step indicator, 2-column tool grid, primary CTA centered.
 */
export function OnboardingWizard({
  t,
  addToast,
  onClose,
}: {
  t: TranslateFn;
  addToast: AddToastFn;
  /** Called on both skip and finish; parent marks onboarding done and reloads data. */
  onClose: () => void;
}) {
  const [step, setStep] = useState(0);
  const [discovered, setDiscovered] = useState<ToolTemplate[]>([]);
  const [loadingDiscovery, setLoadingDiscovery] = useState(true);
  const [adding, setAdding] = useState(false);
  const [addedCount, setAddedCount] = useState<number | null>(null);
  const [scanning, setScanning] = useState(false);
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);

  useEffect(() => {
    api.discoverTools()
      .then(setDiscovered)
      .catch(() => {
        // Silent fail — discovery is a nice-to-have
      })
      .finally(() => setLoadingDiscovery(false));
  }, []);

  async function handleAddDiscovered() {
    setAdding(true);
    let added = 0;
    for (const dt of discovered) {
      try {
        await api.addTool(dt.name, dt.global_path, dt.project_rel_path);
        added++;
      } catch {
        // Skip duplicates silently
      }
    }
    setAdding(false);
    setAddedCount(added);
    if (added > 0) {
      addToast("success", `${t("addedTools")} ${added} ${t("toolUnit")}`);
    }
  }

  async function handleScan() {
    setScanning(true);
    try {
      const result = await api.fullScan();
      setScanResult(result);
      result.errors.forEach((err) => addToast("error", err));
    } catch (e) {
      addToast("error", `${t("scanFailed")}: ${e}`);
    } finally {
      setScanning(false);
    }
  }

  const totalSteps = 3;

  return (
    <div className="modal-overlay wizard-fullscreen">
      <div className="modal wizard-splash" onClick={(e) => e.stopPropagation()}>
        <div className="wizard-hero">
          <Icon name="hexagon" size={56} className="wizard-hero-icon" />
          <h2>Skill Manager</h2>
          <p>{t("wizardTagline")}</p>
        </div>

        {/* Dot-based step indicator */}
        <div className="wizard-dots" aria-label={`Step ${step + 1} of ${totalSteps}`}>
          {Array.from({ length: totalSteps }).map((_, i) => (
            <span
              key={i}
              className={`wizard-dot${i === step ? " wizard-dot-active" : ""}${i < step ? " wizard-dot-done" : ""}`}
            />
          ))}
        </div>

        {step === 0 && (
          <div className="wizard-body">
            <p className="wizard-hint">{t("wizardToolsHint")}</p>
            {loadingDiscovery ? (
              <div className="wizard-placeholder">{t("loading")}</div>
            ) : discovered.length === 0 ? (
              <div className="wizard-placeholder">{t("wizardNoTools")}</div>
            ) : (
              <>
                <div className="wizard-tool-grid">
                  {discovered.map((dt) => (
                    <div key={dt.name} className="wizard-tool-tile">
                      <div className="wizard-tool-tile-avatar">{dt.name.slice(0, 2).toUpperCase()}</div>
                      <div className="wizard-tool-tile-name">{dt.name}</div>
                      <code className="wizard-tool-tile-path" title={dt.global_path}>{dt.global_path}</code>
                    </div>
                  ))}
                </div>
                <div className="wizard-actions">
                  <button
                    className="btn btn-primary btn-large"
                    onClick={handleAddDiscovered}
                    disabled={adding || addedCount !== null}
                  >
                    {adding
                      ? t("adding")
                      : addedCount !== null
                        ? `${t("addedTools")} ${addedCount} ${t("toolUnit")}`
                        : t("addAll")}
                  </button>
                </div>
              </>
            )}
          </div>
        )}

        {step === 1 && (
          <div className="wizard-body">
            <p className="wizard-hint">{t("wizardScanHint")}</p>
            {scanResult ? (
              <div className="scan-summary">
                <span className="scan-summary-item">
                  {t("found")} <strong>{scanResult.skills_found}</strong>
                </span>
                {scanResult.skills_new > 0 && (
                  <span className="scan-summary-item scan-new">{scanResult.skills_new} {t("new")}</span>
                )}
                {scanResult.skills_updated > 0 && (
                  <span className="scan-summary-item scan-updated">{scanResult.skills_updated} {t("updated")}</span>
                )}
              </div>
            ) : (
              <div className="wizard-actions">
                <button className="btn btn-primary btn-large" onClick={handleScan} disabled={scanning}>
                  {scanning ? t("scanning") : t("scanAll")}
                </button>
              </div>
            )}
          </div>
        )}

        {step === 2 && (
          <div className="wizard-body">
            <p className="wizard-hint">{t("wizardDoneHint")}</p>
          </div>
        )}

        <div className="wizard-actions wizard-nav-actions">
          {step < totalSteps - 1 ? (
            <>
              <button className="btn btn-primary btn-large" onClick={() => setStep(step + 1)}>{t("wizardNext")}</button>
              <button className="btn btn-ghost" onClick={onClose}>{t("wizardSkip")}</button>
            </>
          ) : (
            <button className="btn btn-primary btn-large" onClick={onClose}>{t("wizardFinish")}</button>
          )}
        </div>
      </div>
    </div>
  );
}