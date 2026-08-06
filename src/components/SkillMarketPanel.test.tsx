import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import SkillMarketPanel from "./SkillMarketPanel";
import * as api from "../api";

vi.mock("../api");

const markets = [
  { id: 1, provider: "github", owner: "alice", name: "alpha-market", branch: "main", enabled: true, last_indexed_at: null, last_checked_at: null, created_at: "", updated_at: "" },
  { id: 2, provider: "github", owner: "bob", name: "beta-market", branch: "main", enabled: true, last_indexed_at: null, last_checked_at: null, created_at: "", updated_at: "" },
];

const templates = [
  { id: "anthropic-skills", owner: "anthropics", name: "skills", branch: "main", kind: "builtin", description: "", label: "Builtin", provider: "anthropics", root_skill: false },
];

const toast = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.listMarkets).mockResolvedValue(markets);
  vi.mocked(api.listMarketTemplates).mockResolvedValue(templates);
  vi.mocked(api.listRemoteSkills).mockResolvedValue([]);
  vi.mocked(api.listRemoteInstallations).mockResolvedValue([]);
  vi.mocked(api.scanAllRemoteRepositories).mockResolvedValue([]);
});

function makeT(key: string) {
  const map: Record<string, string> = {
    marketTab: "Markets",
    remoteInstallsTab: "Installs",
    skillsTab: "Skills",
    allMarkets: "All markets",
    filterByMarket: "Filter by market",
    installed: "Installed",
    all: "All",
    scanAllRemote: "Scan all remote",
    checkRemoteUpdates: "Check remote updates",
    remoteNoUpdate: "No remote updates",
    downloadToSsot: "Download to SSOT",
    syncToTools: "Sync to tools",
  };
  return map[key] ?? key;
}

describe("SkillMarketPanel", () => {
  it("renders a market filter dropdown in the Markets and Installs tabs", async () => {
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
    await user.click(screen.getByRole("button", { name: "Markets" }));
    await waitFor(() => expect(screen.getByText("Filter by market")).toBeInTheDocument());

    const instalsButton = screen.getByRole("button", { name: "Installs" });
    instalsButton.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
    instalsButton.dispatchEvent(new MouseEvent("pointerup", { bubbles: true }));
    instalsButton.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await waitFor(() => expect(screen.getByText("Filter by market")).toBeInTheDocument());
  });

  it("uses the selected market filter when loading installations", async () => {
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
    await user.click(screen.getByRole("button", { name: "Installs" }));
    await waitFor(() => expect(screen.getByText("Filter by market")).toBeInTheDocument());

    const [marketsFilter, installsFilter] = screen.getAllByRole("combobox");
    await user.selectOptions(installsFilter, "1");
    expect(api.listRemoteInstallations).toHaveBeenCalledWith(0, 1);

    await user.selectOptions(marketsFilter, "2");
    expect(api.listRemoteSkills).toHaveBeenCalledWith(2);
  });

  it("switches remote update checks between installed skills and all remote skills", async () => {
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
    await user.click(screen.getByRole("button", { name: "Skills" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Check remote updates" })).toBeEnabled());

    const [, , modeSelect] = screen.getAllByRole("combobox");
    await user.selectOptions(modeSelect, "all");
    await user.click(screen.getByRole("button", { name: "Check remote updates" }));

    await waitFor(() => {
      expect(api.checkRemoteUpdates).toHaveBeenCalled();
      expect(api.checkRemoteSsoUpdates).not.toHaveBeenCalled();
    });
  });
});
