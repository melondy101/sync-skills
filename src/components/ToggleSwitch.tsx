// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Custom toggle switch — replaces native <input type="checkbox"> for
 * settings and inline toggles. Accessible via role="switch" + aria-checked.
 *
 * Usage:
 *   <ToggleSwitch checked={enabled} onChange={setEnabled} />
 *   <ToggleSwitch checked={auto} onChange={setAuto} label={t("autoSync")} />
 *   <ToggleSwitch checked={dark} onChange={setDark} disabled />
 */

import { useCallback } from "react";
import type { CSSProperties, KeyboardEvent } from "react";

interface ToggleSwitchProps {
  checked: boolean;
  onChange: (next: boolean) => void;
  label?: string;
  disabled?: boolean;
  style?: CSSProperties;
  className?: string;
}

export function ToggleSwitch({
  checked,
  onChange,
  label,
  disabled = false,
  style,
  className,
}: ToggleSwitchProps) {
  const handleClick = useCallback(() => {
    if (!disabled) onChange(!checked);
  }, [checked, disabled, onChange]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (disabled) return;
      if (e.key === " " || e.key === "Enter") {
        e.preventDefault();
        onChange(!checked);
      }
    },
    [checked, disabled, onChange],
  );

  const trackStyle: CSSProperties = {
    position: "relative",
    width: 40,
    height: 22,
    borderRadius: 9999,
    background: checked
      ? "var(--accent)"
      : "var(--bg-elevated)",
    border: `1px solid ${checked ? "var(--accent)" : "var(--border)"}`,
    cursor: disabled ? "not-allowed" : "pointer",
    opacity: disabled ? 0.5 : 1,
    transition: "background 0.2s ease, border-color 0.2s ease",
    flexShrink: 0,
    ...style,
  };

  const thumbStyle: CSSProperties = {
    position: "absolute",
    top: 2,
    left: 2,
    width: 16,
    height: 16,
    borderRadius: "50%",
    background: checked ? "#fff" : "var(--text-muted)",
    transition: "transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1), background 0.2s ease",
    transform: checked ? "translateX(18px)" : "translateX(0)",
  };

  if (label) {
    return (
      <label
        className={className}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 8,
          cursor: disabled ? "not-allowed" : "pointer",
          fontSize: 13,
          color: "var(--text-secondary)",
        }}
      >
        <span
          role="switch"
          aria-checked={checked}
          aria-label={label}
          tabIndex={disabled ? -1 : 0}
          onClick={handleClick}
          onKeyDown={handleKeyDown}
          style={trackStyle}
        >
          <span style={thumbStyle} />
        </span>
        <span>{label}</span>
      </label>
    );
  }

  return (
    <span
      role="switch"
      aria-checked={checked}
      tabIndex={disabled ? -1 : 0}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      className={className}
      style={trackStyle}
    >
      <span style={thumbStyle} />
    </span>
  );
}
