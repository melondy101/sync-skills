// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import SkillMarketPanel from "./SkillMarketPanel";
import * as api from "../api";

vi.mock("../api");

const markets = [
  { id: 1, provider: "github", owner: "alice", name: "alpha-market", branch: "main", enabled: true, last_indexed_at: null, last_checked_at: null, last_commit_sha: null, created_at: "", updated_at: "" },
  { id: 2, provider: "github", owner: "bob", name: "beta-market", branch: "main", enabled: true, last_indexed_at: null, last_checked_at: null, last_commit_sha: null, created_at: "", updated_at: "" },
];

const toast = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listMarkets).mockResolvedValue(markets);
  vi.mocked(api.listRemoteSkills).mockResolvedValue([]);
  vi.mocked(api.listRemoteInstallations).mockResolvedValue([]);
  vi.mocked(api.scanAllRemoteRepositories).mockResolvedValue([]);
  vi.mocked(api.checkMarketCommits).mockResolvedValue([]);
  vi.mocked(api.checkRemoteUpdates).mockResolvedValue([]);
  vi.mocked(api.checkRemoteSsoUpdates).mockResolvedValue([]);
  vi.mocked(api.setAllRemoteSkillsInstalled).mockResolvedValue({ updated: 2 });
  vi.mocked(api.addMarketByUrl).mockResolvedValue({ id: 3, provider: "github", owner: "carol", name: "gamma", branch: "main", enabled: true, last_indexed_at: null, last_checked_at: null, last_commit_sha: null, created_at: "", updated_at: "" });
  vi.mocked(api.syncMarketIndex).mockResolvedValue({ market_id: 1, skills_found: 0, skills_new: 0, skills_updated: 0, errors: [] });
});

function makeT(key: string) {
  const map: Record<string, string> = {
    market: "Market",
    marketSearchPlaceholder: "Search skills (name or description)…",
    filterAll: "All",
    filterInstalledOnly: "Installed only",
    manageSources: "Sources",
    batchMenu: "Batch",
    reindexAllMarkets: "Re-index all markets",
    syncAllActive: "Sync All Active",
    markAllInstalled: "Mark all installed",
    unmarkAllInstalled: "Unmark all installed",
    checkRemoteUpdates: "Check Updates",
    checking: "Checking...",
    scanning: "Scanning...",
    noSkillsIndexed: "No skills indexed yet for this market. Use “Sync Index”.",
    noMatch: "No matching skills found",
  };
  return map[key] ?? key;
}

describe("SkillMarketPanel", () => {
  it("shows the redesigned market toolbar, chips, and empty state", async () => {
    render(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    expect(screen.getByPlaceholderText("Search skills (name or description)…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sources" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Check Updates" })).toBeInTheDocument();
    expect(screen.getByText("Batch")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "All" })).toBeInTheDocument();

    await waitFor(() => expect(screen.getByText("alice/alpha-market")).toBeInTheDocument());
    await waitFor(() => expect(screen.getByText("bob/beta-market")).toBeInTheDocument());

    expect(screen.getByRole("button", { name: "Installed only" })).toBeInTheDocument();
    expect(screen.getByText("No skills indexed yet for this market. Use “Sync Index”.")).toBeInTheDocument();
  });

  it("check updates does a lightweight commit check first", async () => {
    render(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Check Updates" }));

    await waitFor(() => {
      expect(api.checkMarketCommits).toHaveBeenCalled();
      expect(api.checkRemoteSsoUpdates).toHaveBeenCalled();
      expect(api.checkRemoteUpdates).not.toHaveBeenCalled();
    });
  });

  it("filters skills by the selected market chip", async () => {
    render(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[{ id: 1, name: "tool", global_path: "/tmp", project_rel_path: "", globalPath: "/tmp", projectRelPath: "", created_at: "", updated_at: "" }]}
        onRemoteInstallationsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await waitFor(() => expect(screen.getByText("alice/alpha-market")).toBeInTheDocument());

    await user.click(screen.getByText("alice/alpha-market"));

    await waitFor(() => expect(api.listRemoteSkills).toHaveBeenCalledWith(1));
  });

  it("batch marks / unmarks all remote skills installed", async () => {
    render(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByText("Batch"));
    await user.click(screen.getByRole("button", { name: "Mark all installed" }));
    await waitFor(() => expect(api.setAllRemoteSkillsInstalled).toHaveBeenCalledWith(0, null, true));

    await user.click(screen.getByText("Batch"));
    await user.click(screen.getByRole("button", { name: "Unmark all installed" }));
    await waitFor(() => expect(api.setAllRemoteSkillsInstalled).toHaveBeenCalledWith(0, null, false));
  });
});
