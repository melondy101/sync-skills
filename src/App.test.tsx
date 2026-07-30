// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Orchestration tests for App: data loading, project scoping, scan flow,
// per-skill sync and client-side filtering. The whole api layer is mocked
// (components never call `invoke` directly, per CONTRIBUTING.md).

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import App from "./App";
import * as api from "./api";
import type { Project, Settings, SkillView, Tool } from "./types";

vi.mock("./api");

const mockTool: Tool = {
  id: 1,
  name: "Claude",
  global_path: "~/.claude/skills/",
  project_rel_path: ".claude/skills/",
  created_at: "2026-07-01T00:00:00Z",
  updated_at: "2026-07-01T00:00:00Z",
};

function makeSkill(id: number, name: string): SkillView {
  return {
    id,
    name,
    description: `${name} description`,
    source_path: `~/.agents/skill-manager/ssot/${name}/SKILL.md`,
    content_hash: `hash-${id}`,
    core_hash: `core-${id}`,
    project_id: 0,
    ssot_updated_at: "2026-07-20T00:00:00Z",
    created_at: "2026-07-01T00:00:00Z",
    updated_at: "2026-07-20T00:00:00Z",
    installed_tools: [
      {
        tool_id: 1,
        tool_name: "Claude",
        status: "active",
        synced_at: "2026-07-20T00:00:00Z",
        installation_synced_at: "2026-07-20T00:00:00Z",
      },
    ],
    install_count: 1,
    has_update: false,
  };
}

const mockSkills = [makeSkill(1, "alpha-skill"), makeSkill(2, "beta-skill")];

const mockProject: Project = {
  id: 7,
  name: "Demo Project",
  path: "/demo",
  created_at: "2026-07-01T00:00:00Z",
};

const mockSettings: Settings = {
  sync_mode: "semi-auto",
  prefer_symlink: false,
  theme: "light",
  language: "en",
  close_action: "exit",
};

beforeEach(() => {
  vi.clearAllMocks();
  // Skip the first-run onboarding wizard.
  localStorage.setItem("onboardingDone", "1");

  vi.mocked(api.listTools).mockResolvedValue([mockTool]);
  vi.mocked(api.listSkills).mockResolvedValue(mockSkills);
  vi.mocked(api.listProjects).mockResolvedValue([mockProject]);
  vi.mocked(api.getSettings).mockResolvedValue(mockSettings);
  vi.mocked(api.listConflicts).mockResolvedValue([]);
  vi.mocked(api.updateSettings).mockResolvedValue(undefined);
  vi.mocked(api.checkUpdates).mockResolvedValue([]);
  // ToolsSection loads these on mount (both fail-silent nice-to-haves).
  vi.mocked(api.discoverTools).mockResolvedValue([]);
  vi.mocked(api.listToolTemplates).mockResolvedValue([]);
});

async function renderApp() {
  render(<App />);
  // Wait until mount loaders resolved and cards rendered in English.
  await screen.findByText("alpha-skill");
  await screen.findByRole("button", { name: "Scan All" });
}

describe("App orchestration", () => {
  it("loads shared data on mount and renders skill cards for the global scope", async () => {
    await renderApp();

    expect(api.listTools).toHaveBeenCalled();
    expect(api.listProjects).toHaveBeenCalled();
    expect(api.getSettings).toHaveBeenCalled();
    // Global tab is the default scope (project id 0).
    expect(api.listSkills).toHaveBeenCalledWith(0);
    expect(api.listConflicts).toHaveBeenCalledWith(0);
    expect(screen.getByText("beta-skill")).toBeInTheDocument();
  });

  it("auto-selects the first project when switching to the Projects tab and reloads its skills", async () => {
    await renderApp();
    vi.mocked(api.listSkills).mockClear();
    vi.mocked(api.listConflicts).mockClear();

    fireEvent.click(screen.getByRole("button", { name: "Projects" }));

    await waitFor(() => {
      expect(api.listSkills).toHaveBeenCalledWith(mockProject.id);
      expect(api.listConflicts).toHaveBeenCalledWith(mockProject.id);
    });
  });

  it("runs a full scan from the global tab and auto-checks updates when skills were found", async () => {
    vi.mocked(api.fullScan).mockResolvedValue({
      skills_found: 2,
      skills_new: 0,
      skills_updated: 0,
      errors: [],
      details: [],
    });
    await renderApp();

    fireEvent.click(screen.getByRole("button", { name: "Scan All" }));

    // No new/updated details -> toast instead of the scan result modal,
    // then the automatic update check reports everything is current.
    await screen.findByText(/All skills are up to date/);
    expect(api.fullScan).toHaveBeenCalledTimes(1);
    expect(api.scanScope).not.toHaveBeenCalled();
    expect(api.checkUpdates).toHaveBeenCalledWith(0);
  });

  it("syncs a single skill through the api layer and refreshes skills and updates", async () => {
    vi.mocked(api.syncSkill).mockResolvedValue({
      skill_id: 1,
      skill_name: "alpha-skill",
      synced_to: 1,
      errors: [],
    });
    await renderApp();
    vi.mocked(api.listSkills).mockClear();

    fireEvent.click(screen.getAllByRole("button", { name: "Sync Now" })[0]);

    await waitFor(() => {
      expect(api.syncSkill).toHaveBeenCalledWith(1, 0);
      // Successful sync reloads the skill list and re-checks updates.
      expect(api.listSkills).toHaveBeenCalledWith(0);
      expect(api.checkUpdates).toHaveBeenCalledWith(0);
    });
  });

  it("filters rendered skill cards by the search query without hitting the backend", async () => {
    await renderApp();
    vi.mocked(api.listSkills).mockClear();

    fireEvent.change(screen.getByPlaceholderText("Search skills..."), {
      target: { value: "beta" },
    });

    expect(screen.getByText("beta-skill")).toBeInTheDocument();
    expect(screen.queryByText("alpha-skill")).not.toBeInTheDocument();
    expect(api.listSkills).not.toHaveBeenCalled();
  });
});
