// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef, type RefObject } from "react";

type Options = {
  /** When `false` the hook still listens for Esc but does not trap focus. */
  active?: boolean;
  /** Called when the user presses Escape while focus is inside the trap. */
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

/**
 * Lightweight focus trap for modal dialogs.
 *
 * - On mount: focuses the first focusable child, falling back to the
 *   container itself when no children are focusable.
 * - On Tab / Shift+Tab at the edges: cycles focus inside the container.
 * - On Escape: invokes `onEscape`.
 *
 * Kept small and dependency-free so existing modals can adopt it without
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

    const focusables = () =>
      Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
        (el) => !el.hasAttribute("disabled") && el.offsetParent !== null,
      );

    // Move initial focus inside the dialog.
    const initial = focusables()[0] ?? container;
    initial.focus();

    const containerEl: HTMLElement = container;

    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.stopPropagation();
        onEscapeRef.current?.();
        return;
      }
      if (e.key !== "Tab") return;
      const items = focusables();
      if (items.length === 0) {
        e.preventDefault();
        containerEl.focus();
        return;
      }
      const first = items[0];
      const last = items[items.length - 1];
      const current = document.activeElement as HTMLElement | null;
      if (e.shiftKey && current === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && current === last) {
        e.preventDefault();
        first.focus();
      }
    }

    container.addEventListener("keydown", onKeyDown);
    return () => {
      container.removeEventListener("keydown", onKeyDown);
      // Return focus to whatever opened the modal.
      if (previous && typeof previous.focus === "function") {
        previous.focus();
      }
    };
  }, [active, containerRef]);
}