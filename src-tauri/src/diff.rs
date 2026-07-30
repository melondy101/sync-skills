// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Simple directory diff for comparing skill source vs SSOT copies.

use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// A single file-level diff entry.
#[derive(Debug, Clone, Serialize)]
pub struct FileDiff {
    /// Relative path within the skill directory.
    pub path: String,
    /// Type of change.
    pub change: FileChange,
    /// Unified diff hunks (empty for added/deleted binary files).
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChange {
    Added,
    Deleted,
    Modified,
}

/// A unified diff hunk with context lines.
#[derive(Debug, Clone, Serialize)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffLine {
    /// "+", "-", or " " (context)
    pub op: String,
    pub content: String,
}

/// Result of comparing two skill directories.
#[derive(Debug, Clone, Serialize)]
pub struct SkillDiff {
    pub skill_name: String,
    pub source_path: String,
    pub ssot_path: String,
    pub files: Vec<FileDiff>,
    pub has_changes: bool,
}

const CONTEXT_LINES: usize = 3;

/// Compute the diff between a skill's source directory and its SSOT copy.
pub fn compute_skill_diff(source: &Path, ssot: &Path, skill_name: &str) -> Result<SkillDiff, String> {
    let source_files = collect_files(source)?;
    let ssot_files = collect_files(ssot)?;

    let mut all_paths: Vec<String> = source_files
        .keys()
        .chain(ssot_files.keys())
        .cloned()
        .collect();
    all_paths.sort();
    all_paths.dedup();

    let mut files = Vec::new();

    for rel_path in &all_paths {
        let in_source = source_files.contains_key(rel_path);
        let in_ssot = ssot_files.contains_key(rel_path);

        match (in_source, in_ssot) {
            (true, false) => {
                // File only in source = added
                files.push(FileDiff {
                    path: rel_path.clone(),
                    change: FileChange::Added,
                    hunks: match as_text(source_files.get(rel_path).unwrap()) {
                        Some(text) => make_full_hunk(text, "+"),
                        None => make_binary_hunk(None, source_files.get(rel_path).map(|b| b.len())),
                    },
                });
            }
            (false, true) => {
                // File only in SSOT = deleted
                files.push(FileDiff {
                    path: rel_path.clone(),
                    change: FileChange::Deleted,
                    hunks: match as_text(ssot_files.get(rel_path).unwrap()) {
                        Some(text) => make_full_hunk(text, "-"),
                        None => make_binary_hunk(ssot_files.get(rel_path).map(|b| b.len()), None),
                    },
                });
            }
            (true, true) => {
                let src_bytes = source_files.get(rel_path).unwrap();
                let ssot_bytes = ssot_files.get(rel_path).unwrap();
                if src_bytes != ssot_bytes {
                    // Compare raw bytes so binary changes are never missed;
                    // fall back to a size-only hunk when either side is not UTF-8.
                    let hunks = match (as_text(ssot_bytes), as_text(src_bytes)) {
                        (Some(old_text), Some(new_text)) => {
                            let mut hunks = compute_unified_diff(old_text, new_text);
                            if hunks.is_empty() {
                                // Bytes differ but lines are identical — the change is
                                // line endings (CRLF/LF) or a trailing newline. Emit an
                                // informational hunk so the UI isn't a blank "modified".
                                hunks = make_format_only_hunk(old_text, new_text);
                            }
                            hunks
                        }
                        _ => make_binary_hunk(Some(ssot_bytes.len()), Some(src_bytes.len())),
                    };
                    files.push(FileDiff {
                        path: rel_path.clone(),
                        change: FileChange::Modified,
                        hunks,
                    });
                }
                // If identical, skip (no entry)
            }
            (false, false) => unreachable!(),
        }
    }

    let has_changes = !files.is_empty();

    Ok(SkillDiff {
        skill_name: skill_name.to_string(),
        source_path: source.to_string_lossy().to_string(),
        ssot_path: ssot.to_string_lossy().to_string(),
        files,
        has_changes,
    })
}

/// Collect all non-hidden files in a directory, returning (relative_path → raw bytes).
/// Raw bytes (not lossy text) so binary file changes are detected reliably.
fn collect_files(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut map = BTreeMap::new();
    if !dir.exists() {
        return Ok(map);
    }
    collect_recursive(dir, dir, &mut map);
    Ok(map)
}

fn collect_recursive(base: &Path, current: &Path, map: &mut BTreeMap<String, Vec<u8>>) {
    let entries = match fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Skip hidden files/dirs and sync metadata
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "local.md" {
                continue;
            }
        }
        if path.is_dir() {
            collect_recursive(base, &path, map);
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            // Read raw bytes; text/binary decision happens at diff time
            let content = fs::read(&path).unwrap_or_default();
            map.insert(rel, content);
        }
    }
}

/// Interpret raw bytes as UTF-8 text; None means binary.
fn as_text(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes).ok()
}

/// Placeholder hunk for binary files: shows size change instead of line diff.
fn make_binary_hunk(old_size: Option<usize>, new_size: Option<usize>) -> Vec<DiffHunk> {
    let mut lines = Vec::new();
    if let Some(s) = old_size {
        lines.push(DiffLine {
            op: "-".to_string(),
            content: format!("[binary file, {} bytes]", s),
        });
    }
    if let Some(s) = new_size {
        lines.push(DiffLine {
            op: "+".to_string(),
            content: format!("[binary file, {} bytes]", s),
        });
    }
    if lines.is_empty() {
        return vec![];
    }
    vec![DiffHunk {
        old_start: 1,
        old_count: old_size.map(|_| 1).unwrap_or(0),
        new_start: 1,
        new_count: new_size.map(|_| 1).unwrap_or(0),
        lines,
    }]
}

/// Describe the text formatting traits (line endings, trailing newline) of a file.
fn describe_text_format(text: &str) -> String {
    let eol = if text.contains("\r\n") { "CRLF" } else { "LF" };
    let trailing = if text.ends_with('\n') {
        "with trailing newline"
    } else {
        "no trailing newline"
    };
    format!("[text format: {} line endings, {}]", eol, trailing)
}

/// Informational hunk for changes invisible to a line-based diff
/// (line-ending conversion or trailing-newline change).
fn make_format_only_hunk(old_text: &str, new_text: &str) -> Vec<DiffHunk> {
    vec![DiffHunk {
        old_start: 1,
        old_count: 1,
        new_start: 1,
        new_count: 1,
        lines: vec![
            DiffLine {
                op: "-".to_string(),
                content: describe_text_format(old_text),
            },
            DiffLine {
                op: "+".to_string(),
                content: describe_text_format(new_text),
            },
        ],
    }]
}

/// Create a full hunk showing all lines as added (+) or deleted (-).
fn make_full_hunk(content: &str, op: &str) -> Vec<DiffHunk> {
    let lines: Vec<DiffLine> = content
        .lines()
        .map(|l| DiffLine {
            op: op.to_string(),
            content: l.to_string(),
        })
        .collect();
    if lines.is_empty() {
        return vec![];
    }
    let count = lines.len();
    vec![DiffHunk {
        old_start: 1,
        old_count: if op == "-" { count } else { 0 },
        new_start: 1,
        new_count: if op == "+" { count } else { 0 },
        lines,
    }]
}

/// Simple unified diff: LCS-based for small files, falls back to full replace for large ones.
fn compute_unified_diff(old_text: &str, new_text: &str) -> Vec<DiffHunk> {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();

    // For very large files (>5000 lines combined), use a simple full-replace hunk
    // to avoid O(m*n) LCS memory/time costs.
    if old_lines.len() + new_lines.len() > 5000 {
        return make_full_replace(&old_lines, &new_lines);
    }

    let lcs = longest_common_subsequence(&old_lines, &new_lines);
    build_hunks_from_lcs(&old_lines, &new_lines, &lcs)
}

fn make_full_replace(old_lines: &[&str], new_lines: &[&str]) -> Vec<DiffHunk> {
    let mut lines = Vec::new();
    for l in old_lines {
        lines.push(DiffLine { op: "-".to_string(), content: l.to_string() });
    }
    for l in new_lines {
        lines.push(DiffLine { op: "+".to_string(), content: l.to_string() });
    }
    vec![DiffHunk {
        old_start: 1,
        old_count: old_lines.len(),
        new_start: 1,
        new_count: new_lines.len(),
        lines,
    }]
}

/// Compute LCS table for two line arrays.
fn longest_common_subsequence(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    let m = a.len();
    let n = b.len();

    // Build DP table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }

    // Backtrack to find matching pairs
    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

/// Build unified diff hunks from LCS matches, with context lines.
fn build_hunks_from_lcs(old: &[&str], new: &[&str], lcs: &[(usize, usize)]) -> Vec<DiffHunk> {
    // Generate edit script
    struct Edit {
        op: char, // ' ', '-', '+'
        old_idx: Option<usize>,
        new_idx: Option<usize>,
    }

    let mut edits: Vec<Edit> = Vec::new();
    let mut oi = 0usize;
    let mut ni = 0usize;

    for &(lo, ln) in lcs {
        // Lines deleted from old before this match
        while oi < lo {
            edits.push(Edit { op: '-', old_idx: Some(oi), new_idx: None });
            oi += 1;
        }
        // Lines added in new before this match
        while ni < ln {
            edits.push(Edit { op: '+', old_idx: None, new_idx: Some(ni) });
            ni += 1;
        }
        // Context (matching) line
        edits.push(Edit { op: ' ', old_idx: Some(oi), new_idx: Some(ni) });
        oi += 1;
        ni += 1;
    }

    // Remaining lines
    while oi < old.len() {
        edits.push(Edit { op: '-', old_idx: Some(oi), new_idx: None });
        oi += 1;
    }
    while ni < new.len() {
        edits.push(Edit { op: '+', old_idx: None, new_idx: Some(ni) });
        ni += 1;
    }

    // Find change regions and extract hunks with context
    let change_indices: Vec<usize> = edits
        .iter()
        .enumerate()
        .filter(|(_, e)| e.op != ' ')
        .map(|(i, _)| i)
        .collect();

    if change_indices.is_empty() {
        return vec![];
    }

    // Group changes into regions (merging close ones)
    let mut regions: Vec<(usize, usize)> = Vec::new(); // (start, end) in edits
    let mut start = change_indices[0].saturating_sub(CONTEXT_LINES);
    let mut end = (change_indices[0] + CONTEXT_LINES + 1).min(edits.len());

    for &ci in &change_indices[1..] {
        let cs = ci.saturating_sub(CONTEXT_LINES);
        let ce = (ci + CONTEXT_LINES + 1).min(edits.len());
        if cs <= end {
            end = ce;
        } else {
            regions.push((start, end));
            start = cs;
            end = ce;
        }
    }
    regions.push((start, end));

    // Build hunks
    let mut hunks = Vec::new();
    for (rs, re) in regions {
        let mut lines = Vec::new();
        let mut old_start = 0usize;
        let mut new_start = 0usize;
        let mut old_count = 0usize;
        let mut new_count = 0usize;
        let mut first = true;

        for e in &edits[rs..re.min(edits.len())] {
            if first && e.op == ' ' {
                // First context line sets the start
                if let Some(oi) = e.old_idx { old_start = oi + 1; }
                if let Some(ni) = e.new_idx { new_start = ni + 1; }
                first = false;
            } else if first {
                if let Some(oi) = e.old_idx { old_start = oi + 1; }
                if let Some(ni) = e.new_idx { new_start = ni + 1; }
                first = false;
            }

            let content = match e.op {
                '-' => old.get(e.old_idx.unwrap()).unwrap_or(&"").to_string(),
                '+' => new.get(e.new_idx.unwrap()).unwrap_or(&"").to_string(),
                ' ' => old.get(e.old_idx.unwrap()).unwrap_or(&"").to_string(),
                _ => String::new(),
            };

            match e.op {
                '-' => old_count += 1,
                '+' => new_count += 1,
                ' ' => { old_count += 1; new_count += 1; }
                _ => {}
            }

            lines.push(DiffLine {
                op: e.op.to_string(),
                content,
            });
        }

        hunks.push(DiffHunk {
            old_start,
            old_count,
            new_start,
            new_count,
            lines,
        });
    }

    hunks
}
