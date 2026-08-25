// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Gradient initial-letter avatar for skill cards.
 * Color is deterministic per skill name (hash-based) so the same skill
 * always gets the same color across sessions.
 *
 * Usage:
 *   <SkillAvatar name="pdf-tools" size={36} />
 *   <SkillAvatar name="code-review" size={32} />
 */

import type { CSSProperties } from "react";

/** 8 gradient presets — mapped by name hash for stable assignment. */
const GRADIENT_PRESETS = [
  "linear-gradient(135deg, #6366f1, #8b5cf6)", // indigo → violet
  "linear-gradient(135deg, #ec4899, #f43f5e)", // pink → rose
  "linear-gradient(135deg, #f59e0b, #ef4444)", // amber → red
  "linear-gradient(135deg, #10b981, #06b6d4)", // emerald → cyan
  "linear-gradient(135deg, #3b82f6, #6366f1)", // blue → indigo
  "linear-gradient(135deg, #8b5cf6, #ec4899)", // violet → pink
  "linear-gradient(135deg, #14b8a6, #22d3ee)", // teal → cyan
  "linear-gradient(135deg, #f97316, #facc15)", // orange → yellow
] as const;

/** Extract up to 2 initials from a skill name. */
function getInitials(name: string): string {
  // Strip common prefixes and split on hyphens/underscores
  const parts = name
    .replace(/^(skill-|plugin-)/, "")
    .split(/[-_\s]+/)
    .filter(Boolean);

  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase();
  }
  // Single word: first two chars
  return name.slice(0, 2).toUpperCase();
}

/** Deterministic hash → preset index. */
function hashToIndex(name: string): number {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = ((hash << 5) - hash + name.charCodeAt(i)) | 0;
  }
  return Math.abs(hash) % GRADIENT_PRESETS.length;
}

interface SkillAvatarProps {
  name: string;
  size?: number;
  style?: CSSProperties;
  className?: string;
}

export function SkillAvatar({ name, size = 36, style, className }: SkillAvatarProps) {
  const initials = getInitials(name);
  const gradientIndex = hashToIndex(name);
  const borderRadius = Math.round(size * 0.22); // ~8px at 36px
  const fontSize = Math.round(size * 0.39);       // ~14px at 36px

  const avatarStyle: CSSProperties = {
    width: size,
    height: size,
    borderRadius,
    background: GRADIENT_PRESETS[gradientIndex],
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    fontSize,
    fontWeight: 600,
    color: "#fff",
    flexShrink: 0,
    letterSpacing: "0.02em",
    lineHeight: 1,
    userSelect: "none",
    ...style,
  };

  return (
    <div className={className} style={avatarStyle} aria-hidden="true">
      {initials}
    </div>
  );
}
