// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef, useState } from "react";
import * as api from "../api";
import type { RemoteSkill, RemoteSkillDetail } from "../types";
import type { TranslateFn } from "../i18n";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { RemoteSkillFileTree } from "./RemoteSkillFileTree";
import { Icon } from "./Icon";

type Props = {
  t: TranslateFn;
  skill: RemoteSkill;
  marketTitle: string;
  isInstalled: boolean;
  installedAt: string | null;
  /** Emit a transient toast (success / error / info). */
  addToast: (type: "success" | "error" | "info", message: string) => void;
  onClose: () => void;
  /** Trigger the install flow from inside the modal (uses existing button). */
  onInstall: () => void;
};

/**
 * T4 Market: Remote Skill Detail Modal. Renders, in order: full description,
 * file-tree, SKILL.md markdown preview, source market info, local install
 * status, and a hash comparison row (6-char prefix that click-expands to the
 * full 64-char hash with a [Copy] button).
 *
 * Accessibility: `role="dialog"`, `aria-modal="true"`, focus trap, Esc /
 * backdrop close, returns focus to the trigger on close. The hash-row trigger
 * exposes `aria-keyshortcuts` for screen-reader users.
 */
export function RemoteSkillDetailModal({
  t,
  skill,
  marketTitle,
  isInstalled,
  installedAt,
  addToast,
  onClose,
  onInstall,
}: Props) {
  const [detail, setDetail] = useState<RemoteSkillDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [hashExpanded, setHashExpanded] = useState(false);
  const dialogRef = useRef<HTMLDivElement | null>(null);

  useFocusTrap(dialogRef, { active: true, onEscape: onClose });

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setLoadError(null);
    api
      .getRemoteSkillDetail(skill.id)
      .then((d) => {
        if (!cancelled) setDetail(d);
      })
      .catch((e) => {
        if (!cancelled) setLoadError(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [skill.id]);

  // Reset expansion when switching skills so a previous "expanded" state
  // doesn't bleed into the new skill's hash row.
  useEffect(() => {
    setHashExpanded(false);
  }, [skill.id]);

  function handleBackdropClick(e: React.MouseEvent<HTMLDivElement>) {
    if (e.target === e.currentTarget) onClose();
  }

  async function copyHash(value: string) {
    try {
      await navigator.clipboard.writeText(value);
      addToast("success", t("copiedHash"));
    } catch {
      addToast("error", t("copyFailed"));
    }
  }

  const remotePrefix = skill.remote_content_hash.slice(0, 6);
  const remoteFull = skill.remote_content_hash;
  const localFull = detail?.local_content_hash ?? null;
  const localPrefix = localFull ? localFull.slice(0, 6) : null;

  return (
    <div className="modal-overlay" onClick={handleBackdropClick}>
      <div
        ref={dialogRef}
        className="modal remote-skill-detail-modal modal-wide"
        role="dialog"
        aria-modal="true"
        aria-label={t("remoteSkillDetailTitle")}
        aria-keyshortcuts="Escape"
        tabIndex={-1}
      >
        <div className="remote-skill-detail-header">
          <div>
            <h3 className="remote-skill-detail-title">{skill.skill_name}</h3>
            <div className="remote-skill-detail-subtitle">
              {marketTitle}
            </div>
          </div>
          <button
            className="btn btn-small btn-ghost"
            onClick={onClose}
            aria-label={t("close")}
          >
            <Icon name="x" size={14} />
          </button>
        </div>

        {loading ? (
          <div className="remote-skill-detail-placeholder">{t("loading")}</div>
        ) : loadError ? (
          <div className="remote-skill-detail-placeholder error">
            {t("failedLoadSkillDetail")}: {loadError}
          </div>
        ) : detail ? (
          <div className="remote-skill-detail-body">
            <Section title={t("description")}>
              {detail.description ? (
                <p className="remote-skill-detail-description">{detail.description}</p>
              ) : (
                <p className="remote-skill-detail-muted">—</p>
              )}
            </Section>

            <Section title={t("files")}>
              {detail.ssot_missing ? (
                <p className="remote-skill-detail-muted">{t("notInstalledHint")}</p>
              ) : (
                <RemoteSkillFileTree files={detail.files} label={t("fileTreeLabel")} />
              )}
            </Section>

            <Section title={t("skillMdLabel")}>
              {detail.skill_md_content == null ? (
                <p className="remote-skill-detail-muted">{t("notInstalledHint")}</p>
              ) : (
                <pre className="remote-skill-md-preview">{detail.skill_md_content}</pre>
              )}
            </Section>

            <Section title={t("sourceMarket")}>
              <dl className="remote-skill-detail-meta">
                <Meta label={t("remoteUrl")} value={detail.remote_url} mono />
                <Meta label={t("ssotPath")} value={detail.ssot_path} mono />
              </dl>
            </Section>

            <Section title={t("installStatus")}>
              {isInstalled ? (
                <p className="remote-skill-detail-status">
                  {t("installedTag")}
                  {installedAt ? ` · ${installedAt}` : ""}
                </p>
              ) : (
                <p className="remote-skill-detail-muted">{t("remoteNotInstalled")}</p>
              )}
            </Section>

            <Section title={t("hashComparison")}>
              <div className="remote-skill-hash-row">
                <span className="remote-skill-hash-label">{t("localVersion")}</span>
                {localPrefix ? (
                  <button
                    type="button"
                    className="remote-skill-hash-value"
                    onClick={() => setHashExpanded((v) => !v)}
                    onKeyDown={(e) => {
                      if (e.key === " " || e.key === "Enter") {
                        e.preventDefault();
                        setHashExpanded((v) => !v);
                      }
                    }}
                    aria-expanded={hashExpanded}
                    aria-keyshortcuts="Enter Space"
                    aria-label={t("hashClickToExpand")}
                  >
                    {localPrefix}
                  </button>
                ) : (
                  <span className="remote-skill-hash-value muted">{t("hashNoLocal")}</span>
                )}
                <span className="remote-skill-hash-arrow">→</span>
                <span className="remote-skill-hash-label">{t("remoteVersion")}</span>
                <span className="remote-skill-hash-value">{remotePrefix}</span>
                <button
                  className="btn btn-tiny btn-secondary"
                  onClick={() => copyHash(remoteFull)}
                  aria-label={t("copyHash")}
                >
                  {t("copyBtn")}
                </button>
              </div>
              {hashExpanded && localFull && (
                <div className="remote-skill-hash-expanded">
                  <code className="skill-path">{localFull}</code>
                  <button
                    className="btn btn-tiny btn-secondary"
                    onClick={() => copyHash(localFull)}
                    aria-label={t("copyHash")}
                  >
                    {t("copyBtn")}
                  </button>
                </div>
              )}
            </Section>
          </div>
        ) : null}

        <div className="modal-actions">
          {!isInstalled && (
            <button className="btn btn-primary" onClick={onInstall} disabled={loading}>
              {t("install")}
            </button>
          )}
          <button className="btn btn-secondary" onClick={onClose}>
            {t("close")}
          </button>
        </div>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="remote-skill-detail-section">
      <h4 className="remote-skill-detail-section-title">{title}</h4>
      <div className="remote-skill-detail-section-body">{children}</div>
    </section>
  );
}

function Meta({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="remote-skill-detail-meta-row">
      <dt className="remote-skill-detail-meta-label">{label}</dt>
      <dd className={`remote-skill-detail-meta-value${mono ? " mono" : ""}`}>{value}</dd>
    </div>
  );
}