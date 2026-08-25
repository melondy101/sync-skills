// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SettingsPanel } from "./SettingsPanel";
import type { Settings } from "../types";
import * as api from "../api";

const t = (key: string) => key;

// Helper: every Settings shape in these tests starts from the same baseline
// and only overrides what a particular test cares about. Keeps the new
// Phase 4 / T1 fields (`view_market_diff_before_update`,
// `auto_sync_on_file_change`) consistent across fixtures.
function makeSettings(overrides: Partial<Settings> = {}): Settings {
  return {
    sync_mode: "semi-auto",
    prefer_symlink: false,
    theme: "light",
    language: "zh",
    close_action: "tray",
    use_system_proxy: true,
    use_proxy: true,
    proxy_url: "http://127.0.0.1:10090",
    view_market_diff_before_update: true,
    auto_sync_on_file_change: false,
    ...overrides,
  };
}

describe("SettingsPanel", () => {
  it("shows the minimize-to-tray option and a check updates button", async () => {
    vi.spyOn(api, "getSettings").mockResolvedValue(makeSettings() as any);
    const user = userEvent.setup();
    render(
      <SettingsPanel
        t={t}
        settings={makeSettings()}
        onChange={() => {}}
        onSave={() => {}}
        onBack={() => {}}
      />,
    );

    // close_action is on the appearance section (default visible)
    expect(screen.getByRole("combobox")).toHaveValue("tray");
    // The check-updates button lives on the update section — switch to it.
    await user.click(screen.getByRole("button", { name: "appUpdateSection" }));
    expect(screen.getByRole("button", { name: "checkAppUpdate" })).toBeEnabled();
  });

  it("shows a proxy-friendly error when the update check fails", async () => {
    vi.spyOn(api, "checkAppUpdate").mockRejectedValue(
      new Error("HTTP 403; check your network/proxy or set a manual proxy URL"),
    );

    const user = userEvent.setup();
    render(
      <SettingsPanel
        t={t}
        settings={makeSettings({ use_system_proxy: false })}
        onChange={() => {}}
        onSave={() => {}}
        onBack={() => {}}
      />,
    );

    await user.click(screen.getByRole("button", { name: "appUpdateSection" }));
    await user.click(screen.getByRole("button", { name: "checkAppUpdate" }));
    expect(screen.getByText(/updateCheckFailed:/)).toHaveTextContent(
      "updateCheckFailed: Error: HTTP 403; check your network/proxy or set a manual proxy URL",
    );
  });

  it("shows a manual-proxy TLS hint when the update check fails with a https proxy", async () => {
    vi.spyOn(api, "checkAppUpdate").mockRejectedValue(
      new Error("HTTP 403; the proxy itself may also need its certificate trusted"),
    );

    const user = userEvent.setup();
    render(
      <SettingsPanel
        t={t}
        settings={makeSettings({ use_system_proxy: false, proxy_url: "https://127.0.0.1:10090" })}
        onChange={() => {}}
        onSave={() => {}}
        onBack={() => {}}
      />,
    );

    await user.click(screen.getByRole("button", { name: "appUpdateSection" }));
    await user.click(screen.getByRole("button", { name: "checkAppUpdate" }));
    expect(screen.getByText(/updateCheckFailed:/)).toHaveTextContent(
      "updateCheckFailed: Error: HTTP 403; the proxy itself may also need its certificate trusted",
    );
  });

  it("exposes the Phase 4 / T1 toggles with both checked-by-default values", async () => {
    const user = userEvent.setup();
    render(
      <SettingsPanel
        t={t}
        settings={makeSettings()}
        onChange={() => {}}
        onSave={() => {}}
        onBack={() => {}}
      />,
    );

    // Sync section holds both toggles.
    await user.click(screen.getByRole("button", { name: "syncMode" }));

    const marketDiffToggle = screen.getByRole("switch", {
      name: "viewMarketDiffBeforeUpdate",
    });
    const autoSyncToggle = screen.getByRole("switch", {
      name: "autoSyncOnFileChange",
    });

    expect(marketDiffToggle).toHaveAttribute("aria-checked", "true");
    expect(autoSyncToggle).toHaveAttribute("aria-checked", "false");
    // Both controls remain independently editable; the spec's "mirrors" rule
    // is enforced by App.tsx (when sync_mode changes, auto_sync_on_file_change
    // follows) and by the backend helper `effective_auto_sync_on_file_change`
    // (canonical OR of the two fields for downstream consumers).
    expect(autoSyncToggle).not.toHaveAttribute("aria-disabled", "true");
  });

  it("renders auto_sync_on_file_change as a normal switch in full-auto", async () => {
    // Under full-auto, the explicit flag still drives the switch display.
    // The watcher (Phase 4 / M12) consumes the backend helper that ORs the
    // flag with sync_mode, so the UI does not need to conflate them.
    const user = userEvent.setup();
    render(
      <SettingsPanel
        t={t}
        settings={makeSettings({ sync_mode: "full-auto" })}
        onChange={() => {}}
        onSave={() => {}}
        onBack={() => {}}
      />,
    );

    await user.click(screen.getByRole("button", { name: "syncMode" }));
    const autoSyncToggle = screen.getByRole("switch", {
      name: "autoSyncOnFileChange",
    });
    expect(autoSyncToggle).toHaveAttribute("aria-checked", "false");
    expect(autoSyncToggle).not.toHaveAttribute("aria-disabled", "true");
  });
});

