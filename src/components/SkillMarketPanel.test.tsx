// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import SkillMarketPanel from "./SkillMarketPanel";
import { ConfirmProvider } from "./ConfirmProvider";
import * as api from "../api";

vi.mock("../api");

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const listenCallbacks: Record<string, (event: { payload: unknown }) => void> = {};
const listenMock = vi.mocked((await import("@tauri-apps/api/event")).listen);
listenMock.mockImplementation(((event: string, callback: (event: { payload: unknown }) => void) => {
  listenCallbacks[event] = callback;
  return Promise.resolve(() => {
    delete listenCallbacks[event];
  });
}) as never);

function renderWithProviders(ui: React.ReactElement) {
  return render(<ConfirmProvider>{ui}</ConfirmProvider>);
}

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
  vi.mocked(api.deleteMarket).mockResolvedValue(undefined);
  vi.mocked(api.syncRemoteInstallationsToTools).mockResolvedValue({ skill_id: 0, skill_name: "", synced_to: 0, errors: [] });
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
    cancel: "cancel",
    deleteMarket: "Delete Market",
    confirmMarkAllInstalledTitle: "Mark all as installed?",
    confirmMarkAllInstalledMessage: "Every skill in the current market filter will be flagged as installed.",
    confirmUnmarkAllInstalledTitle: "Unmark all as installed?",
    confirmUnmarkAllInstalledMessage: "Every skill in the current market filter will be flagged as not installed.",
    confirmScopeAll: "Scope: all markets",
    confirmScopeMarket: "Scope: {0} only",
    confirmDeleteMarketTitle: "Delete this market?",
    confirmDeleteMarketMessage: "All skills indexed from this market will be removed.",
    confirmSyncAllTitle: "Sync all installed skills?",
    confirmSyncAllMessage: "Every installed skill will be copied or symlinked to {0}.",
    skillsCount: "{0} skills",
  };
  return map[key] ?? key;
}

describe("SkillMarketPanel", () => {
  it("shows the redesigned market toolbar, chips, and empty state", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
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

    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
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

    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
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

    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
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
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
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
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[{ id: 1, name: "tool", global_path: "/tmp", project_rel_path: "", globalPath: "/tmp", projectRelPath: "", created_at: "", updated_at: "" }]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
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
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByText("Batch"));
    await user.click(screen.getByRole("button", { name: "Mark all installed" }));
    // New: confirmation dialog must appear before the destructive action runs.
    const dialog1 = await screen.findByRole("alertdialog");
    await user.click(within(dialog1).getByRole("button", { name: "Mark all installed" }));
    await waitFor(() => expect(api.setAllRemoteSkillsInstalled).toHaveBeenCalledWith(0, null, true));

    await user.click(screen.getByText("Batch"));
    await user.click(screen.getByRole("button", { name: "Unmark all installed" }));
    const dialog2 = await screen.findByRole("alertdialog");
    await user.click(within(dialog2).getByRole("button", { name: "Unmark all installed" }));
    await waitFor(() => expect(api.setAllRemoteSkillsInstalled).toHaveBeenCalledWith(0, null, false));
  });

  it("cancels the destructive batch action when the user dismisses the confirm dialog", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByText("Batch"));
    await user.click(screen.getByRole("button", { name: "Mark all installed" }));

    // Dialog opens. Cancel and assert the API was never invoked.
    await screen.findByRole("alertdialog");
    await user.click(screen.getByRole("button", { name: "cancel" }));

    expect(api.setAllRemoteSkillsInstalled).not.toHaveBeenCalled();
  });

  it("confirms before deleting a market from the Sources modal", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Sources" }));

    // Two non-builtin markets are rendered → two delete buttons. Click the first.
    const deleteButtons = screen.getAllByRole("button", { name: "Delete Market" });
    await user.click(deleteButtons[0]);

    // Dialog appears; cancel path keeps the API silent.
    const dialog = await screen.findByRole("alertdialog");
    await user.click(within(dialog).getByRole("button", { name: "cancel" }));
    expect(api.deleteMarket).not.toHaveBeenCalled();

    // Re-open and confirm → API gets called.
    const deleteButtons2 = screen.getAllByRole("button", { name: "Delete Market" });
    await user.click(deleteButtons2[0]);
    const dialog2 = await screen.findByRole("alertdialog");
    await user.click(within(dialog2).getByRole("button", { name: "Delete Market" }));
    await waitFor(() => expect(api.deleteMarket).toHaveBeenCalledWith(1));
  });

  it("confirms before running Sync All Installed", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[{ id: 1, name: "tool", global_path: "/tmp", project_rel_path: "", globalPath: "/tmp", projectRelPath: "", created_at: "", updated_at: "" }]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByText("Batch"));
    await user.click(screen.getByRole("button", { name: "Sync All Active" }));

    // Cancel first → API silent.
    const dialog1 = await screen.findByRole("alertdialog");
    await user.click(within(dialog1).getByRole("button", { name: "cancel" }));
    expect(api.syncRemoteInstallationsToTools).not.toHaveBeenCalled();

    // Re-open and confirm.
    await user.click(screen.getByText("Batch"));
    await user.click(screen.getByRole("button", { name: "Sync All Active" }));
    const dialog2 = await screen.findByRole("alertdialog");
    await user.click(within(dialog2).getByRole("button", { name: "Sync All Active" }));
    await waitFor(() => expect(api.syncRemoteInstallationsToTools).toHaveBeenCalled());
  });

  it("shows progress bar updates during sync-all via market:sync-progress events", async () => {
    let resolveSync: () => void = () => {};
    vi.mocked(api.syncRemoteInstallationsToTools).mockImplementation(
      () => new Promise<{ skill_id: number; skill_name: string; synced_to: number; errors: string[] }>((resolve) => {
        resolveSync = () => resolve({ skill_id: 0, skill_name: "", synced_to: 3, errors: [] });
      }),
    );

    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[{ id: 1, name: "tool", global_path: "/tmp", project_rel_path: "", globalPath: "/tmp", projectRelPath: "", created_at: "", updated_at: "" }]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByText("Batch"));
    await user.click(screen.getByRole("button", { name: "Sync All Active" }));
    const dialog = await screen.findByRole("alertdialog");
    await user.click(within(dialog).getByRole("button", { name: "Sync All Active" }));

    // Wait for the listener to be registered, then push a progress event.
    await waitFor(() => expect(listenCallbacks["market:sync-progress"]).toBeDefined());
    listenCallbacks["market:sync-progress"]?.({ payload: { completed: 1, total: 3, current: "alpha" } });
    await waitFor(() => {
      const dialog = screen.getByRole("alertdialog");
      expect(within(dialog).getByText("1 / 3")).toBeInTheDocument();
      // The progress label contains 'alpha' but the chip 'alice/alpha-market'
      // also matches. Scope to the progress-current span.
      const progressCurrent = dialog.querySelector(".progress-current");
      expect(progressCurrent).toBeTruthy();
      expect(progressCurrent?.textContent).toContain("alpha");
    });

    // Complete the sync → dialog closes.
    resolveSync();
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
  });
});

