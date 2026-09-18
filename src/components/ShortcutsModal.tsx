// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useRef } from "react";
import type { TranslateFn } from "../i18n";
import type { Hotkey } from "../hooks/useHotkeys";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { Icon } from "./Icon";

/** Cheat sheet for the app-wide shortcuts, rendered from the live binding list. */
export function ShortcutsModal({
  t,
  hotkeys,
  onClose,
}: {
  t: TranslateFn;
  hotkeys: Hotkey[];
  onClose: () => void;
}) {
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, { onEscape: onClose });

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal shortcuts-modal"
        role="dialog"
        aria-modal="true"
        aria-label={t("shortcutsTitle")}
        onClick={(e) => e.stopPropagation()}
        ref={dialogRef}
        tabIndex={-1}
      >
        <div className="shortcuts-header">
          <h3 style={{ margin: 0 }}>{t("shortcutsTitle")}</h3>
          <button className="btn btn-small btn-ghost" onClick={onClose} aria-label={t("close")}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <dl className="shortcuts-list">
          {hotkeys.map((hotkey) => (
            <div className="shortcuts-row" key={`${hotkey.id}:${hotkey.display}`}>
              <dt>
                <kbd>{hotkey.display}</kbd>
              </dt>
              <dd>{t(hotkey.labelKey)}</dd>
            </div>
          ))}
        </dl>
      </div>
    </div>
  );
}
