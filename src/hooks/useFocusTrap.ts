// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef, type RefObject } from "react";

type Options = {
  /**
   * When `false` nothing is installed — no focus grab, no Tab cycle, no Esc.
   * Needed by dialogs that are conditionally rendered inside a persistent
   * parent, where the effect cannot re-run just because the dialog appeared.
   */
  active?: boolean;
  /** Called when the user presses Escape while this dialog is on top. */
  onEscape?: () => void;
};

const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

type Entry = {
  container: HTMLElement;
  onEscape?: () => void;
};

// Dialogs stack (health check -> editor, market card -> detail -> install).
// A single window listener plus a LIFO stack is what makes "only the topmost
// dialog answers Escape" expressible: independent window listeners all fire for
// the same keypress, and a listener scoped to the container goes deaf as soon
// as focus lands outside it — which happens whenever a dialog swaps views.
const stack: Entry[] = [];

const focusables = (container: HTMLElement) =>
  Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (el) => !el.hasAttribute("disabled") && el.offsetParent !== null,
  );

function onKeyDown(e: KeyboardEvent) {
  const top = stack[stack.length - 1];
  if (!top) return;

  if (e.key === "Escape") {
    e.stopPropagation();
    top.onEscape?.();
    return;
  }
  if (e.key !== "Tab") return;

  const items = focusables(top.container);
  if (items.length === 0) {
    e.preventDefault();
    top.container.focus();
    return;
  }
  const first = items[0];
  const last = items[items.length - 1];
  const current = document.activeElement as HTMLElement | null;
  if (!top.container.contains(current)) {
    e.preventDefault();
    (e.shiftKey ? last : first).focus();
  } else if (e.shiftKey && current === first) {
    e.preventDefault();
    last.focus();
  } else if (!e.shiftKey && current === last) {
    e.preventDefault();
    first.focus();
  }
}

function push(entry: Entry) {
  if (stack.length === 0) window.addEventListener("keydown", onKeyDown);
  stack.push(entry);
}

function pop(entry: Entry) {
  const index = stack.lastIndexOf(entry);
  if (index >= 0) stack.splice(index, 1);
  if (stack.length === 0) window.removeEventListener("keydown", onKeyDown);
}

/** How many dialogs are currently trapping. Global shortcuts must stand down
 *  while this is non-zero so keystrokes do not leak behind an open dialog. */
export function openDialogCount() {
  return stack.length;
}

/**
 * Lightweight focus trap for modal dialogs.
 *
 * - On mount: focuses the first focusable child, falling back to the
 *   container itself when no children are focusable.
 * - On Tab / Shift+Tab: keeps focus inside the topmost open dialog, pulling it
 *   back in when the browser dropped it to `body`.
 * - On Escape: invokes `onEscape` for the topmost open dialog only.
 * - On unmount: returns focus to whatever opened the dialog.
 *
 * Kept small and dependency-free so existing dialogs can adopt it without
 * pulling in a focus-management library.
 */
export function useFocusTrap<T extends HTMLElement>(
  containerRef: RefObject<T | null>,
  { active = true, onEscape }: Options = {},
) {
  const onEscapeRef = useRef(onEscape);
  useEffect(() => {
    onEscapeRef.current = onEscape;
  }, [onEscape]);

  useEffect(() => {
    if (!active) return;
    const container = containerRef.current;
    if (!container) return;

    const previous = document.activeElement as HTMLElement | null;
    const entry: Entry = { container, onEscape: () => onEscapeRef.current?.() };
    push(entry);

    const initial = focusables(container)[0];
    (initial ?? container).focus();

    return () => {
      pop(entry);
      if (previous && typeof previous.focus === "function") {
        previous.focus();
      }
    };
  }, [active, containerRef]);
}
