// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import * as api from "../api";
import type { ScanResult, ToolTemplate } from "../types";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";

/**
 * First-run onboarding wizard (3 steps): add discovered tools -> scan -> done.
 * Visibility is controlled by the parent via the localStorage "onboardingDone" flag.
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

  const steps = [t("wizardStepTools"), t("wizardStepScan"), t("wizardStepDone")];

  return (
    <div className="modal-overlay">
      <div className="modal wizard-modal" onClick={(e) => e.stopPropagation()}>
        <h3>{t("wizardTitle")}</h3>
        <p className="wizard-intro">{t("wizardIntro")}</p>

        {/* Step indicator */}
        <div className="wizard-steps">
          {steps.map((label, i) => (
            <span key={i} className={`wizard-step-dot${i === step ? " wizard-step-active" : ""}${i < step ? " wizard-step-done" : ""}`}>
              {label}
            </span>
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
                <ul className="wizard-tool-list">
                  {discovered.map((dt) => (
                    <li key={dt.name} className="wizard-tool-item">
                      <span className="tool-name">{dt.name}</span>
                      <code className="path-text">{dt.global_path}</code>
                    </li>
                  ))}
                </ul>
                <button
                  className="btn btn-primary"
                  onClick={handleAddDiscovered}
                  disabled={adding || addedCount !== null}
                >
                  {adding ? t("adding") : addedCount !== null ? `${t("addedTools")} ${addedCount} ${t("toolUnit")}` : t("addAll")}
                </button>
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
              <button className="btn btn-primary" onClick={handleScan} disabled={scanning}>
                {scanning ? t("scanning") : t("scanAll")}
              </button>
            )}
          </div>
        )}

        {step === 2 && (
          <div className="wizard-body">
            <p className="wizard-hint">{t("wizardDoneHint")}</p>
          </div>
        )}

        <div className="modal-actions">
          {step < 2 ? (
            <>
              <button className="btn btn-primary" onClick={() => setStep(step + 1)}>{t("wizardNext")}</button>
              <button className="btn btn-secondary" onClick={onClose}>{t("wizardSkip")}</button>
            </>
          ) : (
            <button className="btn btn-primary" onClick={onClose}>{t("wizardFinish")}</button>
          )}
        </div>
      </div>
    </div>
  );
}
