// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Indented file-tree view of a remote skill's SSOT directory.
 *
 * The backend (`get_remote_skill_detail`) already excludes `SKILL.md`,
 * `local.md`, and dot-prefixed entries, so this component only renders what
 * it is given — no extra filtering is needed. Empty trees render a single
 * muted line so the layout stays stable when there is nothing to show.
 */
export function RemoteSkillFileTree({ files, label }: { files: string[]; label: string }) {
  if (files.length === 0) {
    return <div className="remote-skill-file-tree empty">{emptyHint()}</div>;
  }
  return (
    <ul className="remote-skill-file-tree" aria-label={label}>
      {files.map((rel) => (
        <li key={rel} className="remote-skill-file-tree-entry">
          <span className="remote-skill-file-tree-name">{rel}</span>
        </li>
      ))}
    </ul>
  );
}

function emptyHint() {
  // Keep the hint generic — the backend already excludes SKILL.md, so an
  // empty tree genuinely means "no other files shipped".
  return <span className="remote-skill-file-tree-empty">—</span>;
}