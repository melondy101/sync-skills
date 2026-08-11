// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef } from "react";

export type ConfirmVariant = "primary" | "danger";

type Props = {
  title: string;
  message: string;
  detail?: string;
  confirmText: string;
  cancelText: string;
  confirmVariant?: ConfirmVariant;
  loading: boolean;
  onConfirm: () => void;
  onCancel: () => void;
};

/**
 * Lightweight confirmation dialog. Use before destructive or hard-to-undo
 * actions (e.g. "mark all installed" across every market). Keeps the same
 * modal-overlay/modal shell as the other dialogs so focus rings, Esc-to-close,
 * and overlay-click-to-close all behave consistently.
 */
export default function ConfirmDialog({
  title,
  message,
  detail,
  confirmText,
  cancelText,
  confirmVariant = "primary",
  loading,
  onConfirm,
  onCancel,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);

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
          <p className="market-card-meta" style={{ marginBottom: 16 }}>
            {detail}
          </p>
        )}
        <div className="modal-actions">
          <button className="btn btn-secondary" onClick={onCancel} disabled={loading}>
            {cancelText}
          </button>
          <button
            className={confirmVariant === "danger" ? "btn btn-danger-filled" : "btn btn-primary"}
            onClick={onConfirm}
            disabled={loading}
          >
            {confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}
