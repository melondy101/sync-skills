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
    marketTab: "Skills",
    addMarket: "Add Market",
    cancel: "Cancel",
    allMarkets: "All markets",
    filterByMarket: "Filter by market",
    installed: "Installed",
    all: "All",
    scanAllRemote: "Scan All",
    checkRemoteUpdates: "Check Updates",
    remoteNoUpdate: "No remote updates",
    downloadToSsot: "Download to SSOT",
    syncToTools: "Sync to tools",
    markAllInstalled: "Mark all installed",
    unmarkAllInstalled: "Unmark all installed",
    searchPlaceholder: "Search",
    skills: "Skills",
    addMarketUrlPlaceholder: "https://github.com/owner/repo",
    addMarketUrlRequired: "Please enter a market URL",
    addMarketUrlHint: "Only the repo link is needed",
    defaultBranchAuto: "Auto main/master",
    noNewCommits: "No new commits",
    newCommitsFound: "{0} new commit(s)",
    checking: "Checking...",
    scanning: "Scanning...",
  };
  return map[key] ?? key;
}

describe("SkillMarketPanel", () => {
  it("shows the market list and an add-market button that reveals a URL field", async () => {
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
    expect(screen.getByRole("heading", { name: "Market", level: 2 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Add Market" })).toBeInTheDocument();
    expect(screen.queryByPlaceholderText("https://github.com/owner/repo")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Add Market" }));
    expect(screen.getByPlaceholderText("https://github.com/owner/repo")).toBeInTheDocument();
    expect(screen.getByLabelText("Auto main/master")).toBeInTheDocument();
  });

  it("adds a market by URL via the backend", async () => {
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
    await user.click(screen.getByRole("button", { name: "Add Market" }));
    await user.type(screen.getByPlaceholderText("https://github.com/owner/repo"), "github.com/carol/gamma");
    await user.selectOptions(screen.getByLabelText("Auto main/master"), "master");
    await user.click(screen.getByRole("button", { name: "Add Market" }));

    await waitFor(() => expect(api.addMarketByUrl).toHaveBeenCalledWith("github.com/carol/gamma", "master"));
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

  it("filters skills by the selected market", async () => {
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
    await waitFor(() => expect(screen.getByLabelText("Skills").children).toHaveLength(3));

    await user.selectOptions(screen.getByLabelText("Skills"), "1");

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
    await user.click(screen.getByRole("button", { name: "Mark all installed" }));
    await waitFor(() => expect(api.setAllRemoteSkillsInstalled).toHaveBeenCalledWith(0, null, true));

    await user.click(screen.getByRole("button", { name: "Unmark all installed" }));
    await waitFor(() => expect(api.setAllRemoteSkillsInstalled).toHaveBeenCalledWith(0, null, false));
  });
});
