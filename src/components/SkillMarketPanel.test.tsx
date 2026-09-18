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

const remoteSkills = [
  {
    id: 10,
    market_id: 1,
    skill_name: "demo-skill",
    description: "Demonstrates the T4 detail Modal.",
    remote_url: "https://github.com/alice/alpha-market/tree/main/demo-skill",
    ssot_path: "/tmp/ssot/demo-skill",
    remote_content_hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    remote_core_hash: "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
    is_installed: true,
    installed_at: "2026-08-20 12:00:00",
    created_at: "2026-08-20 11:00:00",
    updated_at: "2026-08-20 12:00:00",
  },
];

const detailPayload = {
  remote_skill_id: 10,
  skill_name: "demo-skill",
  description: "Demonstrates the T4 detail Modal.",
  remote_url: "https://github.com/alice/alpha-market/tree/main/demo-skill",
  ssot_path: "/tmp/ssot/demo-skill",
  remote_content_hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  remote_core_hash: "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
  local_content_hash: "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
  local_core_hash: "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100",
  skill_md_content: "---\nname: demo-skill\ndescription: demo\n---\nbody",
  files: ["references/spec.md", "scripts/build.sh"],
  ssot_missing: false,
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listMarkets).mockResolvedValue(markets);
  vi.mocked(api.listRemoteSkills).mockResolvedValue(remoteSkills);
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
  vi.mocked(api.getRemoteSkillDetail).mockResolvedValue(detailPayload);
});

function makeT(key: string) {
  const map: Record<string, string> = {
    shortcutsTitle: "Keyboard shortcuts",
    shortcutFocusSearch: "Search skills",
    shortcutMoveSelection: "Move between skills",
    shortcutOpenDetail: "Open the focused skill",
    shortcutSyncAllInstalled: "Sync all installed skills",
    shortcutOpenSources: "Manage market sources",
    shortcutClearSearch: "Clear search",
    shortcutShowHelp: "Show this panel",
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
    noSkillsIndexed: 'No skills indexed yet for this market. Use "Sync Index".',
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
    remoteSkillDetailTitle: "Skill detail",
    openDetailAria: "Open detail for {0}",
    copyHash: "Copy full hash",
    copyBtn: "Copy",
    description: "Description",
    files: "Files",
    sourceMarket: "Source market",
    installStatus: "Install status",
    hashComparison: "Version comparison",
    skillMdLabel: "SKILL.md",
    fileTreeLabel: "Skill files",
    installedTag: "Installed",
    hashClickToExpand: "Click to expand the full hash",
    // T4 sidebar redesign
    sidebarSources: "Sources",
    sidebarAllSources: "All Sources",
    sidebarFilterPlaceholder: "Filter sources…",
    sidebarAddSource: "Add Source",
    sidebarCollapse: "Collapse sidebar",
    sidebarExpand: "Expand sidebar",
    actionsToggle: "Actions",
    actionsReindexAll: "Reindex All",
    actionsSyncAll: "Sync All Installed",
    actionsMarkAll: "Mark All Installed",
    actionsUnmarkAll: "Unmark All Installed",
    actionsManageSources: "Manage Sources",
    filterLabel: "Filter:",
    sourceHeaderSync: "Sync Index",
    sourceHeaderDisable: "Disable",
    sourceHeaderEnable: "Enable",
    sourceHeaderDelete: "Delete",
    sourceHeaderBranch: "branch: {0}",
    sourceHeaderIndexed: "indexed {0}",
    sourceHeaderSkills: "{0} skills",
    emptyNoSkills: "No skills indexed for this source",
    emptyNoMatch: "No skills match your filters",
    cardSourceBadge: "{0}",
  };
  return map[key] ?? key;
}

describe("SkillMarketPanel", () => {
  it("shows the redesigned sidebar, topbar, and empty state", async () => {
    vi.mocked(api.listRemoteSkills).mockResolvedValue([]);

    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    expect(screen.getByPlaceholderText("Search skills (name or description)…")).toBeInTheDocument();
    expect(screen.getByText("All Sources")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Check Updates" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Actions" })).toBeInTheDocument();

    await waitFor(() => expect(screen.getByText("alice/alpha-market")).toBeInTheDocument());
    await waitFor(() => expect(screen.getByText("bob/beta-market")).toBeInTheDocument());

    expect(screen.getByRole("button", { name: "Installed only" })).toBeInTheDocument();
    expect(screen.getByText("No skills indexed for this source")).toBeInTheDocument();
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
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByText("Add Source"));
    await user.click(screen.getByRole("button", { name: "Add Market" }));

    const urlInput = screen.getByPlaceholderText("https://github.com/owner/repo or owner/repo");
    await user.type(urlInput, "carol/gamma");

    await user.click(screen.getByRole("button", { name: "Add" }));

    await waitFor(() => expect(api.addMarketByUrl).toHaveBeenCalledWith("carol/gamma", undefined, "auto"));
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
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByText("Add Source"));
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
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByText("Manage Sources"));
    const syncButtons = screen.getAllByRole("button", { name: "Sync Index" });
    await user.click(syncButtons[0]);

    await waitFor(() => expect(toast).toHaveBeenCalledWith("error", expect.stringContaining("missing SKILL.md")));
  });

  it("check updates does a lightweight commit check first", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        onMarketsChanged={vi.fn()}
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

  it("filters skills by the selected market in the sidebar", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[{ id: 1, name: "tool", global_path: "/tmp", project_rel_path: "", globalPath: "/tmp", projectRelPath: "", created_at: "", updated_at: "" }]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        onMarketsChanged={vi.fn()}
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

  it("batch marks / unmarks all remote skills installed via actions drawer", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Actions" }));
    const markAllBtn = await screen.findByRole("button", { name: "Mark All Installed" });
    await user.click(markAllBtn);
    const dialog1 = await screen.findByRole("alertdialog");
    await user.click(within(dialog1).getByRole("button", { name: "Mark all installed" }));
    await waitFor(() => expect(api.setAllRemoteSkillsInstalled).toHaveBeenCalledWith(0, null, true));

    await user.click(screen.getByRole("button", { name: "Actions" }));
    const unmarkAllBtn = await screen.findByRole("button", { name: "Unmark All Installed" });
    await user.click(unmarkAllBtn);
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
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Actions" }));
    await user.click(screen.getByRole("button", { name: "Mark All Installed" }));

    await screen.findByRole("alertdialog");
    await user.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "cancel" }));

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
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Manage Sources" }));

    const deleteButtons = screen.getAllByRole("button", { name: "Delete Market" });
    await user.click(deleteButtons[0]);

    const dialog = await screen.findByRole("alertdialog");
    await user.click(within(dialog).getByRole("button", { name: "cancel" }));
    expect(api.deleteMarket).not.toHaveBeenCalled();

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
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Actions" }));
    const syncAllBtn = await screen.findByRole("button", { name: "Sync All Installed" });
    await user.click(syncAllBtn);

    const dialog1 = await screen.findByRole("alertdialog");
    await user.click(within(dialog1).getByRole("button", { name: "cancel" }));
    expect(api.syncRemoteInstallationsToTools).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Actions" }));
    const syncAllBtn2 = await screen.findByRole("button", { name: "Sync All Installed" });
    await user.click(syncAllBtn2);
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
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Actions" }));
    const syncAllBtn = await screen.findByRole("button", { name: "Sync All Installed" });
    await user.click(syncAllBtn);
    const dialog = await screen.findByRole("alertdialog");
    await user.click(within(dialog).getByRole("button", { name: "Sync All Active" }));

    await waitFor(() => expect(listenCallbacks["market:sync-progress"]).toBeDefined());
    listenCallbacks["market:sync-progress"]?.({ payload: { completed: 1, total: 3, current: "alpha" } });
    await waitFor(() => {
      const dialog = screen.getByRole("alertdialog");
      expect(within(dialog).getByText("1 / 3")).toBeInTheDocument();
      const progressCurrent = dialog.querySelector(".progress-current");
      expect(progressCurrent).toBeTruthy();
      expect(progressCurrent?.textContent).toContain("alpha");
    });

    resolveSync();
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
  });

  it("T4: clicking a card opens the detail Modal with description, file tree, and hash row", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    const chip = await screen.findByText("alice/alpha-market");
    await user.click(chip);

    await waitFor(() => {
      expect(screen.getByText("demo-skill")).toBeInTheDocument();
    });
    const card = await screen.findByRole("button", { name: "Open detail for demo-skill" });
    await user.click(card);

    const dialog = await screen.findByRole("dialog", { name: "Skill detail" });
    expect(dialog).toBeInTheDocument();
    expect(within(dialog).getByText("Demonstrates the T4 detail Modal.")).toBeInTheDocument();
    expect(within(dialog).getByText("references/spec.md")).toBeInTheDocument();
    expect(within(dialog).getByText("scripts/build.sh")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Copy full hash" })).toBeInTheDocument();
  });

  it("T4: pressing Enter on a focused card opens the detail Modal", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    const chip = await screen.findByText("alice/alpha-market");
    await user.click(chip);

    const card = await screen.findByRole("button", { name: "Open detail for demo-skill" });
    card.focus();
    await user.keyboard("{Enter}");

    expect(await screen.findByRole("dialog", { name: "Skill detail" })).toBeInTheDocument();
  });

  it("T4: Esc closes the detail Modal", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    const chip = await screen.findByText("alice/alpha-market");
    await user.click(chip);

    const card = await screen.findByRole("button", { name: "Open detail for demo-skill" });
    await user.click(card);

    const dialog = await screen.findByRole("dialog", { name: "Skill detail" });
    expect(dialog).toBeInTheDocument();

    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Skill detail" })).not.toBeInTheDocument());
  });

  it("T4: clicking the hash-row prefix expands the full local hash with a Copy button", async () => {
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );

    const user = userEvent.setup();
    const chip = await screen.findByText("alice/alpha-market");
    await user.click(chip);

    const card = await screen.findByRole("button", { name: "Open detail for demo-skill" });
    await user.click(card);

    const dialog = await screen.findByRole("dialog", { name: "Skill detail" });
    expect(within(dialog).getByText("fedcba")).toBeInTheDocument();
    expect(within(dialog).queryByText(/^fedcba0987654321/)).not.toBeInTheDocument();

    await user.click(within(dialog).getByText("fedcba"));
    expect(within(dialog).getByText(/^fedcba0987654321/)).toBeInTheDocument();

    expect(within(dialog).getAllByRole("button", { name: "Copy full hash" })).toHaveLength(2);
  });

  it("binds the market shortcuts: ? opens the cheat sheet, / focuses search", async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <SkillMarketPanel
        projects={[]}
        projectPaths={{}}
        tools={[]}
        onRemoteInstallationsChanged={vi.fn()}
        onSkillsChanged={vi.fn()}
        onMarketsChanged={vi.fn()}
        defaultProjectId={0}
        t={makeT}
        addToast={toast}
      />,
    );
    await waitFor(() => expect(api.listMarkets).toHaveBeenCalled());

    await user.keyboard("?");
    const sheet = await screen.findByRole("dialog", { name: "Keyboard shortcuts" });
    expect(within(sheet).getByText("Ctrl / ⌘ + K")).toBeInTheDocument();
    expect(within(sheet).getAllByText("Search skills")).toHaveLength(2);

    await user.keyboard("{Escape}");
    await waitFor(() =>
      expect(screen.queryByRole("dialog", { name: "Keyboard shortcuts" })).toBeNull(),
    );

    const search = screen.getByPlaceholderText("Search skills (name or description)…");
    await user.keyboard("/");
    expect(document.activeElement).toBe(search);
  });
});




