// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef } from "react";
import { useFocusTrap } from "./useFocusTrap";
import { useHotkeys, type Hotkey } from "./useHotkeys";

const HOTKEYS: Hotkey[] = [
  { id: "search", keys: ["k"], withMeta: true, display: "Ctrl/⌘ K", labelKey: "shortcutFocusSearch" },
  { id: "search-plain", keys: ["/"], display: "/", labelKey: "shortcutFocusSearch" },
  { id: "force", keys: ["f"], withMeta: true, allowInInput: true, display: "Ctrl/⌘ F", labelKey: "shortcutFocusSearch" },
  { id: "help", keys: ["?"], display: "?", labelKey: "shortcutShowHelp" },
];

function Harness({ onTrigger, dialogOpen = false }: { onTrigger: (id: string) => void; dialogOpen?: boolean }) {
  useHotkeys(HOTKEYS, onTrigger);
  return (
    <>
      <input aria-label="Search" />
      <Dialog label="Blocking" active={dialogOpen} />
    </>
  );
}

function Dialog({ label, active = false }: { label: string; active?: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(ref, { active, onEscape: undefined });
  return <div ref={ref} role="dialog" aria-label={label} tabIndex={-1} />;
}

describe("useHotkeys", () => {
  it("triggers only the binding whose key matches", async () => {
    const user = userEvent.setup();
    const onTrigger = vi.fn();
    render(<Harness onTrigger={onTrigger} />);

    await user.keyboard("?");
    expect(onTrigger).toHaveBeenCalledWith("help", expect.anything());

    onTrigger.mockClear();
    await user.keyboard("x");
    expect(onTrigger).not.toHaveBeenCalled();
  });

  it("requires the modifier for modifier bindings", async () => {
    const user = userEvent.setup();
    const onTrigger = vi.fn();
    render(<Harness onTrigger={onTrigger} />);

    await user.keyboard("k");
    expect(onTrigger).not.toHaveBeenCalled();

    await user.keyboard("{Control>}{k}{/Control}");
    expect(onTrigger).toHaveBeenCalledWith("search", expect.anything());
  });

  it("stands down while the caret is in a text field, unless opted in", async () => {
    const user = userEvent.setup();
    const onTrigger = vi.fn();
    render(<Harness onTrigger={onTrigger} />);
    screen.getByRole("textbox", { name: "Search" }).focus();

    await user.keyboard("/");
    expect(onTrigger).not.toHaveBeenCalled();

    await user.keyboard("{Control>}{f}{/Control}");
    expect(onTrigger).toHaveBeenCalledWith("force", expect.anything());
  });

  it("yields the keyboard to an open dialog", async () => {
    const user = userEvent.setup();
    const onTrigger = vi.fn();
    const { rerender } = render(<Harness onTrigger={onTrigger} />);

    rerender(<Harness onTrigger={onTrigger} dialogOpen />);
    await user.keyboard("?");
    expect(onTrigger).not.toHaveBeenCalled();
  });
});
