// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef, type ReactNode } from "react";
import { openDialogCount, useFocusTrap } from "./useFocusTrap";

function Dialog({
  label,
  onEscape,
  active = true,
}: {
  label: string;
  onEscape?: () => void;
  active?: boolean;
  children?: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(ref, { active, onEscape });
  return (
    <div ref={ref} role="dialog" aria-label={label} tabIndex={-1}>
      {label}
    </div>
  );
}

describe("useFocusTrap", () => {
  it("keeps the stack balanced across mount and unmount", () => {
    const { unmount } = render(<Dialog label="one" onEscape={vi.fn()} />);
    expect(openDialogCount()).toBe(1);
    unmount();
    expect(openDialogCount()).toBe(0);
  });

  it("returns focus to the element that opened the dialog", () => {
    const { rerender } = render(<button>Opener</button>);
    const opener = screen.getByRole("button", { name: "Opener" });
    opener.focus();

    rerender(
      <>
        <button>Opener</button>
        <Dialog label="one" onEscape={vi.fn()} />
      </>
    );
    expect(document.activeElement).not.toBe(opener);

    rerender(<button>Opener</button>);
    expect(document.activeElement).toBe(opener);
  });

  it("routes Escape to the topmost dialog only", async () => {
    const user = userEvent.setup();
    const outer = vi.fn();
    const inner = vi.fn();
    const { rerender } = render(
      <>
        <Dialog label="outer" onEscape={outer} />
        <Dialog label="inner" onEscape={inner} />
      </>
    );
    await user.keyboard("{Escape}");
    expect(inner).toHaveBeenCalledTimes(1);
    expect(outer).not.toHaveBeenCalled();

    // With the top dialog gone, the one below it becomes the responder.
    rerender(<Dialog label="outer" onEscape={outer} />);
    await user.keyboard("{Escape}");
    expect(inner).toHaveBeenCalledTimes(1);
    expect(outer).toHaveBeenCalledTimes(1);
  });

  it("installs nothing while inactive", async () => {
    const user = userEvent.setup();
    const onEscape = vi.fn();
    render(<Dialog label="hidden" active={false} onEscape={onEscape} />);
    await user.keyboard("{Escape}");
    expect(openDialogCount()).toBe(0);
    expect(onEscape).not.toHaveBeenCalled();
  });
});
