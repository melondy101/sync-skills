// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as api from "../api";
import type { McpSuggestedEntry, McpTargetStatus } from "../types";
import { McpServersSection } from "./McpServersSection";

const t = (key: string) => key;
const addToast = vi.fn();

function target(overrides: Partial<McpTargetStatus> = {}): McpTargetStatus {
  return {
    tool: "Cursor",
    path: "C:/Users/me/.cursor/mcp.json",
    registered: true,
    file_exists: true,
    installed: false,
    matches: false,
    error: null,
    ...overrides,
  };
}

const suggested: McpSuggestedEntry = {
  command: "C:/Apps/skill-manager-mcp.exe",
  args: [],
  command_exists: true,
};

describe("McpServersSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(api, "mcpSuggestedEntry").mockResolvedValue(suggested);
    vi.spyOn(api, "mcpStatus").mockResolvedValue([
      target(),
      target({ tool: "Qoder", installed: true, matches: true }),
      target({ tool: "Gemini CLI", registered: false, file_exists: false, error: null }),
    ]);
    vi.spyOn(api, "mcpInstall").mockResolvedValue([
      target({ installed: true, matches: true }),
      target({ tool: "Qoder", installed: true, matches: true }),
    ]);
    vi.spyOn(api, "mcpUninstall").mockResolvedValue([target()]);
  });

  it("reads every tool's registration state before offering to write", async () => {
    render(<McpServersSection t={t} addToast={addToast} />);

    await waitFor(() => expect(api.mcpStatus).toHaveBeenCalledWith(null));
    expect(screen.getByText("Cursor")).toBeInTheDocument();
    expect(screen.getByText("mcpStateMissing")).toBeInTheDocument();
    expect(screen.getByText("mcpStateCurrent")).toBeInTheDocument();
    expect(screen.getByText("mcpStateUnregistered")).toBeInTheDocument();
  });

  it("registers the server and repaints from what the backend actually wrote", async () => {
    const user = userEvent.setup();
    render(<McpServersSection t={t} addToast={addToast} />);
    await waitFor(() => expect(screen.getByText("Cursor")).toBeInTheDocument());

    await user.click(screen.getByRole("button", { name: "mcpInstall" }));

    await waitFor(() => expect(api.mcpInstall).toHaveBeenCalledWith(null));
    expect(addToast).toHaveBeenCalledWith("success", "mcpInstalled");
    expect(api.mcpStatus).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("mcpStateUnregistered")).not.toBeInTheDocument();
  });

  it("removes only our entry and reports it back", async () => {
    const user = userEvent.setup();
    render(<McpServersSection t={t} addToast={addToast} />);
    await waitFor(() => expect(screen.getByText("mcpStateCurrent")).toBeInTheDocument());

    await user.click(screen.getByRole("button", { name: "mcpRemove" }));

    await waitFor(() => expect(api.mcpUninstall).toHaveBeenCalled());
    expect(addToast).toHaveBeenCalledWith("info", "mcpRemoved");
  });

  it("says so when the bundled server binary is missing", async () => {
    vi.mocked(api.mcpSuggestedEntry).mockResolvedValue({
      ...suggested,
      command_exists: false,
    });

    render(<McpServersSection t={t} addToast={addToast} />);

    await waitFor(() => expect(screen.getByText(/mcpBinaryMissing/)).toBeInTheDocument());
  });

  it("will not write when no tool has a config directory to hold it", async () => {
    vi.mocked(api.mcpStatus).mockResolvedValue([target({ registered: false })]);

    render(<McpServersSection t={t} addToast={addToast} />);
    await waitFor(() => expect(screen.getByText("Cursor")).toBeInTheDocument());

    expect(screen.getByRole("button", { name: "mcpInstall" })).toBeDisabled();
    expect(screen.getByText("mcpNoWritableTargets")).toBeInTheDocument();
  });
});
