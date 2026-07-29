// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { DiffHunk, DiffLine, SkillDiff } from "../types";
import type { TranslateFn } from "../i18n";

// P2-1/2/3: shared diff rendering controls (side-by-side toggle + maximize).
export function DiffViewControls({
  t,
  sideBySide,
  onToggleSideBySide,
  maximized,
  onToggleMaximize,
}: {
  t: TranslateFn;
  sideBySide: boolean;
  onToggleSideBySide: () => void;
  maximized: boolean;
  onToggleMaximize: () => void;
}) {
  return (
    <div className="diff-view-controls">
      <div className="diff-view-toggle">
        <button
          className={`btn btn-small${sideBySide ? " active" : ""}`}
          onClick={sideBySide ? undefined : onToggleSideBySide}
        >
          {t("splitView")}
        </button>
        <button
          className={`btn btn-small${!sideBySide ? " active" : ""}`}
          onClick={sideBySide ? onToggleSideBySide : undefined}
        >
          {t("unifiedView")}
        </button>
      </div>
      <button className="btn btn-small" onClick={onToggleMaximize}>
        {maximized ? t("restore") : t("maximize")}
      </button>
    </div>
  );
}

function UnifiedHunk({ hunk }: { hunk: DiffHunk }) {
  return (
    <div className="diff-hunk">
      <div className="diff-hunk-header">
        @@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@
      </div>
      <pre className="diff-lines">
        {hunk.lines.map((line, li) => (
          <div key={li} className={`diff-line diff-line-${line.op === "+" ? "add" : line.op === "-" ? "del" : "ctx"}`}>
            <span className="diff-line-op">{line.op === " " ? " " : line.op}</span>
            <span className="diff-line-content">{line.content}</span>
          </div>
        ))}
      </pre>
    </div>
  );
}

// P2-2: pair deletions/insertions into left (SSOT/old) + right (current/new) columns.
function SideBySideHunk({ hunk }: { hunk: DiffHunk }) {
  const rows: { left?: DiffLine; right?: DiffLine }[] = [];
  let pendL: DiffLine[] = [];
  let pendR: DiffLine[] = [];
  const flush = () => {
    const n = Math.max(pendL.length, pendR.length);
    for (let i = 0; i < n; i++) {
      rows.push({ left: pendL[i], right: pendR[i] });
    }
    pendL = [];
    pendR = [];
  };
  for (const line of hunk.lines) {
    if (line.op === " ") {
      flush();
      rows.push({ left: line, right: line });
    } else if (line.op === "-") {
      pendL.push(line);
    } else if (line.op === "+") {
      pendR.push(line);
    }
  }
  flush();

  const leftClass = (l?: DiffLine) => (l ? (l.op === "-" ? "del" : "ctx") : "empty");
  const rightClass = (r?: DiffLine) => (r ? (r.op === "+" ? "add" : "ctx") : "empty");
  const opSym = (op?: string) => (op === " " ? " " : op ?? " ");

  return (
    <div className="diff-hunk diff-hunk-split">
      <div className="diff-hunk-header">
        @@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@
      </div>
      <div className="diff-split">
        <div className="diff-split-col diff-split-old">
          {rows.map((r, i) => (
            <div key={i} className={`diff-line diff-line-${leftClass(r.left)}`}>
              <span className="diff-line-op">{opSym(r.left?.op)}</span>
              <span className="diff-line-content">{r.left?.content ?? ""}</span>
            </div>
          ))}
        </div>
        <div className="diff-split-col diff-split-new">
          {rows.map((r, i) => (
            <div key={i} className={`diff-line diff-line-${rightClass(r.right)}`}>
              <span className="diff-line-op">{opSym(r.right?.op)}</span>
              <span className="diff-line-content">{r.right?.content ?? ""}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export function DiffFilesView({ diff, sideBySide, noChangesLabel }: { diff: SkillDiff; sideBySide: boolean; noChangesLabel: string }) {
  return (
    <div className="diff-files">
      {diff.files.map((file, fi) => (
        <div key={fi} className="diff-file">
          <div className={`diff-file-header diff-file-${file.change}`}>
            <span className="diff-change-badge">{file.change}</span>
            <span className="diff-file-path">{file.path}</span>
          </div>
          {file.hunks.map((hunk, hi) =>
            sideBySide ? <SideBySideHunk key={hi} hunk={hunk} /> : <UnifiedHunk key={hi} hunk={hunk} />
          )}
        </div>
      ))}
      {!diff.has_changes && <div className="diff-no-changes">{noChangesLabel}</div>}
    </div>
  );
}
