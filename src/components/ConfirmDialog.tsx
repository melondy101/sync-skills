// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef } from "react";

export type ConfirmVariant = "primary" | "danger";

export type SyncProgress = {
  completed: number;
  total: number;
  current?: string;
};

type Props = {
  title: string;
  message: string;
  detail?: string;
  confirmText: string;
  cancelText: string;
  confirmVariant?: ConfirmVariant;
  /** When set, the dialog swaps the action buttons for a progress bar. */
  progress?: SyncProgress | null;
  loading: boolean;
  onConfirm: () => void;
  onCancel: () => void;
};

/**
 * Lightweight confirmation dialog. Use before destructive or hard-to-undo
 * actions (e.g. "mark all installed" across every market). Keeps the same
 * modal-overlay/modal shell as the other dialogs so focus rings, Esc-to-close,
 * and overlay-click-to-close all behave consistently.
 *
 * When `progress` is set, the dialog switches from a confirm-button layout to
 * a progress bar (cancel stays available but the action button is disabled).
 */
export default function ConfirmDialog({
  title,
  message,
  detail,
  confirmText,
  cancelText,
  confirmVariant = "primary",
  progress,
  loading,
  onConfirm,
  onCancel,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const isProgress = progress != null && progress.total > 0;
  const percent = isProgress ? Math.min(100, Math.round((progress!.completed / progress!.total) * 100)) : 0;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    ref.current?.focus();
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  return (
    <div className="modal-overlay" onClick={(e) => { if (e.target === e.currentTarget) onCancel(); }}>
      <div
        className="modal"
        role="alertdialog"
        aria-modal="true"
        aria-label={title}
        ref={ref}
        tabIndex={-1}
      >
        <h3>{title}</h3>
        <p style={{ margin: "0 0 8px", color: "var(--text-secondary)" }}>{message}</p>
        {detail && (
          <p className="market-card-meta" style={{ marginBottom: isProgress ? 12 : 16 }}>
            {detail}
          </p>
        )}
        {isProgress && (
          <div className="confirm-progress" aria-live="polite">
            <div className="progress-bar" role="progressbar" aria-valuenow={progress!.completed} aria-valuemin={0} aria-valuemax={progress!.total}>
              <div className="progress-fill" style={{ width: `${percent}%` }} />
            </div>
            <div className="progress-label">
              <span>{progress!.completed} / {progress!.total}</span>
              {progress!.current && <span className="progress-current"> · {progress!.current}</span>}
            </div>
          </div>
        )}
        <div className="modal-actions">
          <button className="btn btn-secondary" onClick={onCancel} disabled={loading || isProgress}>
            {cancelText}
          </button>
          <button
            className={confirmVariant === "danger" ? "btn btn-danger-filled" : "btn btn-primary"}
            onClick={onConfirm}
            disabled={loading || isProgress}
          >
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}
