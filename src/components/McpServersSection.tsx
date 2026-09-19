// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useEffect, useState } from "react";
import * as api from "../api";
import type { McpSuggestedEntry, McpTargetStatus } from "../types";
import type { TranslateFn } from "../i18n";
import type { AddToastFn } from "../hooks/useToasts";
import { Icon } from "./Icon";

/**
 * Registration panel for the read-only MCP server that ships with this app.
 * Deliberately not a general MCP editor: it only ever touches the
 * `skill-manager` key in each tool's own config, so the user's other servers and
 * unrelated settings are out of scope here by construction.
 */
export function McpServersSection({ t, addToast }: { t: TranslateFn; addToast: AddToastFn }) {
  const [targets, setTargets] = useState<McpTargetStatus[]>([]);
  const [suggestion, setSuggestion] = useState<McpSuggestedEntry | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [suggested, statuses] = await Promise.all([
        api.mcpSuggestedEntry(),
        api.mcpStatus(null),
      ]);
      setSuggestion(suggested);
      setTargets(statuses);
    } catch (e) {
      addToast("error", `${t("mcpCheckFailed")}: ${e}`);
    } finally {
      setLoading(false);
    }
  }, [addToast, t]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const writable = targets.filter((target) => target.registered);
  const anyInstalled = targets.some((target) => target.installed);

  async function handleInstall() {
    setBusy(true);
    try {
      setTargets(await api.mcpInstall(null));
      addToast("success", t("mcpInstalled"));
    } catch (e) {
      addToast("error", `${t("mcpInstallFailed")}: ${e}`);
    } finally {
      setBusy(false);
    }
  }

  async function handleUninstall() {
    setBusy(true);
    try {
      setTargets(await api.mcpUninstall());
      addToast("info", t("mcpRemoved"));
    } catch (e) {
      addToast("error", `${t("mcpRemoveFailed")}: ${e}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="settings-group">
      <label className="settings-label">{t("mcpSection")}</label>
      <p className="settings-hint">{t("mcpSectionHint")}</p>

      {suggestion && !suggestion.command_exists && (
        <p className="settings-hint update-error">
          <Icon name="alert-triangle" size={14} /> {t("mcpBinaryMissing")}: {suggestion.command}
        </p>
      )}

      {suggestion?.command_exists && (
        <p className="settings-hint mcp-command">
          {t("mcpCommandLabel")}: <code>{suggestion.command}</code>
        </p>
      )}

      <div className="mcp-targets" role="region" aria-label={t("mcpSection")}>
        {targets.map((target) => (
          <div className="mcp-target" key={target.tool}>
            <span className="mcp-target-tool">
              {target.tool}
              <span className="mcp-target-path">{target.path}</span>
            </span>
            <span className={`mcp-target-state mcp-target-state-${stateOf(target)}`}>
              {stateLabel(target, t)}
            </span>
          </div>
        ))}
        {loading && <p className="settings-hint">{t("loading")}</p>}
      </div>

      <p className="settings-hint">{t("mcpUnregisteredHint")}</p>

      <div className="mcp-actions">
        <button
          type="button"
          className="btn btn-primary btn-small"
          onClick={handleInstall}
          disabled={busy || loading || writable.length === 0}
        >
          {t("mcpInstall")}
        </button>
        <button
          type="button"
          className="btn btn-secondary btn-small"
          onClick={handleUninstall}
          disabled={busy || loading || !anyInstalled}
        >
          {t("mcpRemove")}
        </button>
        <button
          type="button"
          className="btn btn-secondary btn-small"
          onClick={() => void refresh()}
          disabled={busy || loading}
        >
          {t("refresh")}
        </button>
      </div>
      {writable.length === 0 && !loading && (
        <p className="settings-hint">{t("mcpNoWritableTargets")}</p>
      )}
    </div>
  );
}

type StateKey = "current" | "installed" | "missing" | "unregistered" | "error";

function stateOf(target: McpTargetStatus): StateKey {
  if (target.error) return "error";
  if (!target.registered) return "unregistered";
  if (target.matches) return "current";
  if (target.installed) return "installed";
  return "missing";
}

function stateLabel(target: McpTargetStatus, t: TranslateFn): string {
  switch (stateOf(target)) {
    case "error":
      return target.error ?? t("mcpStateError");
    case "unregistered":
      return t("mcpStateUnregistered");
    case "current":
      return t("mcpStateCurrent");
    case "installed":
      return t("mcpStateOther");
    case "missing":
      return t("mcpStateMissing");
  }
}
