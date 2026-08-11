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
    addMarket: "Add Market",
    add: "Add",
    syncMarketIndex: "Sync Index",
    batchMenu: "Batch",
    reindexAllMarkets: "Re-index all markets",
    syncAllActive: "Sync All Active",
    markAllInstalled: "Mark all installed",
    unmarkAllInstalled: "Unmark all installed",
    checkRemoteUpdates: "Check Updates",
    checking: "Checking...",
    scanning: "Scanning...",
    repoNoSkills: "no skills found",
    repoSyncFailed: "Index failed: {0}",
    lastSyncError: "Last sync failed: {0}",
    addMarketUrlPlaceholder: "https://github.com/owner/repo or owner/repo",
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

  it("auto-syncs after adding a market so the user gets immediate feedback", async () => {
    vi.mocked(api.syncMarketIndex).mockResolvedValue({
      market_id: 3,
      skills_found: 4,
      skills_new: 4,
      skills_updated: 0,
      errors: [],
    });

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
    await user.click(screen.getByRole("button", { name: "Sources" }));
    await user.click(screen.getByRole("button", { name: "Add Market" }));

    const urlInput = screen.getByPlaceholderText("https://github.com/owner/repo or owner/repo");
    await user.type(urlInput, "carol/gamma");

    await user.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() => expect(api.addMarketByUrl).toHaveBeenCalledWith("carol/gamma", undefined));
    await waitFor(() => expect(api.syncMarketIndex).toHaveBeenCalledWith(3));
  });

  it("surfaces an empty-repo message when auto-sync finds no skills", async () => {
    vi.mocked(api.syncMarketIndex).mockResolvedValue({
      market_id: 3,
      skills_found: 0,
      skills_new: 0,
      skills_updated: 0,
      errors: [],
    });

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
    await user.click(screen.getByRole("button", { name: "Sources" }));
    await user.click(screen.getByRole("button", { name: "Add Market" }));
    await user.type(screen.getByPlaceholderText("https://github.com/owner/repo or owner/repo"), "carol/empty");
    await user.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() => expect(toast).toHaveBeenCalledWith("info", expect.stringContaining("no skills")));
  });

  it("records per-market sync errors when the index call returns errors", async () => {
    vi.mocked(api.syncMarketIndex).mockResolvedValue({
      market_id: 1,
      skills_found: 2,
      skills_new: 0,
      skills_updated: 2,
      errors: ["missing SKILL.md"],
    });

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
    await user.click(screen.getByRole("button", { name: "Sources" }));
    const syncButtons = screen.getAllByRole("button", { name: "Sync Index" });
    await user.click(syncButtons[0]);

    await waitFor(() => expect(toast).toHaveBeenCalledWith("error", expect.stringContaining("missing SKILL.md")));

    // No assertion on the modal — it would require re-opening the Source modal
    // to read the error indicator. The toast assertion is sufficient to lock in
    // the new contract that errors are surfaced to the user.
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
