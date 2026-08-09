// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SettingsPanel } from "./SettingsPanel";
import * as api from "../api";

const t = (key: string) => key;

describe("SettingsPanel", () => {
  it("shows the minimize-to-tray option and a check updates button", async () => {
    vi.spyOn(api, "getSettings").mockResolvedValue({
      sync_mode: "semi-auto",
      prefer_symlink: false,
      theme: "light",
      language: "zh",
      close_action: "tray",
      use_system_proxy: true,
      use_proxy: true,
      proxy_url: "http://127.0.0.1:10090",
    } as any);
    render(
      <SettingsPanel
        t={t}
        settings={{
          sync_mode: "semi-auto",
          prefer_symlink: false,
          theme: "light",
          language: "zh",
          close_action: "tray",
          use_system_proxy: true,
          use_proxy: true,
          proxy_url: "http://127.0.0.1:10090",
        }}
        onChange={() => {}}
        onSave={() => {}}
        onBack={() => {}}
      />,
    );

    expect(screen.getAllByRole("combobox")[3]).toHaveValue("tray");
    expect(screen.getByRole("button", { name: "checkAppUpdate" })).toBeEnabled();
  });

  it("shows a proxy-friendly error when the update check fails", async () => {
    vi.spyOn(api, "checkAppUpdate").mockRejectedValue(
      new Error("HTTP 403; check your network/proxy or set a manual proxy URL")
    );

    render(
      <SettingsPanel
        t={t}
        settings={{
          sync_mode: "semi-auto",
          prefer_symlink: false,
          theme: "light",
          language: "zh",
          close_action: "tray",
          use_system_proxy: false,
          use_proxy: true,
          proxy_url: "http://127.0.0.1:10090",
        }}
        onChange={() => {}}
        onSave={() => {}}
        onBack={() => {}}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: "checkAppUpdate" }));
    expect(screen.getByText(/updateCheckFailed:/)).toHaveTextContent(
      "updateCheckFailed: Error: HTTP 403; check your network/proxy or set a manual proxy URL"
    );
  });

  it("shows a manual-proxy TLS hint when the update check fails with a https proxy", async () => {
    vi.spyOn(api, "checkAppUpdate").mockRejectedValue(
      new Error("HTTP 403; the proxy itself may also need its certificate trusted")
    );

    render(
      <SettingsPanel
        t={t}
        settings={{
          sync_mode: "semi-auto",
          prefer_symlink: false,
          theme: "light",
          language: "zh",
          close_action: "tray",
          use_system_proxy: false,
          use_proxy: true,
          proxy_url: "https://127.0.0.1:10090",
        }}
        onChange={() => {}}
        onSave={() => {}}
        onBack={() => {}}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: "checkAppUpdate" }));
    expect(screen.getByText(/updateCheckFailed:/)).toHaveTextContent(
      "updateCheckFailed: Error: HTTP 403; the proxy itself may also need its certificate trusted"
    );
  });
});
