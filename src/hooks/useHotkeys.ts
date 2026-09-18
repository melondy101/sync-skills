// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef } from "react";
import { openDialogCount } from "./useFocusTrap";

export type Hotkey = {
  id: string;
  /** Values compared against `event.key` lowercased: "/", "k", "arrowdown". */
  keys: string[];
  /** Requires Ctrl (Windows/Linux) or Cmd (macOS). */
  withMeta?: boolean;
  /** Fire even while the caret sits in a text field. Off by default. */
  allowInInput?: boolean;
  /** Cheat-sheet text, e.g. "Ctrl/⌘ K". */
  display: string;
  /** i18n key for the cheat-sheet description. */
  labelKey: string;
};

const EDITABLE = new Set(["input", "textarea", "select"]);

function isEditable(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) return false;
  return EDITABLE.has(target.tagName.toLowerCase()) || target.isContentEditable;
}

/**
 * App-wide keyboard shortcuts.
 *
 * Deliberately narrow: an open dialog owns the keyboard (see `openDialogCount`),
 * and plain keys are ignored while the caret is in a text field so typing "/"
 * into a search box stays a search string. Callers pass the same list they hand
 * to the cheat sheet, so the help panel cannot drift from what is bound — keep
 * that list referentially stable (a module constant).
 */
export function useHotkeys(
  hotkeys: Hotkey[],
  onTrigger: (id: string, event: KeyboardEvent) => void,
) {
  const handlersRef = useRef(onTrigger);
  useEffect(() => {
    handlersRef.current = onTrigger;
  }, [onTrigger]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (openDialogCount() > 0) return;
      const inField = isEditable(event.target);
      const pressed = event.key.toLowerCase();
      const meta = event.ctrlKey || event.metaKey;
      for (const hotkey of hotkeys) {
        if (inField && !hotkey.allowInInput && !hotkey.withMeta) continue;
        if (!hotkey.keys.includes(pressed)) continue;
        if (!!hotkey.withMeta !== meta || event.altKey) continue;
        event.preventDefault();
        handlersRef.current(hotkey.id, event);
        return;
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [hotkeys]);
}
