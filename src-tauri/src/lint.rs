// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Skill quality checks (lint). Rules operate on a skill directory's SKILL.md.
//! Issues carry a machine-readable `code` so the frontend can localize messages.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Description length beyond which triggering quality degrades in most agents.
const MAX_DESCRIPTION_LEN: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintIssue {
    /// Machine-readable rule code, e.g. "missing_description"
    pub code: String,
    /// "error" | "warning"
    pub severity: String,
    /// Optional detail parameter (e.g. actual length for description_too_long)
    pub param: Option<String>,
    /// Whether `fix_skill_dir` can repair this issue automatically
    pub fixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLint {
    pub skill_id: i64,
    pub skill_name: String,
    pub issues: Vec<LintIssue>,
}

/// Codes that `fix_skill_dir` can repair without guessing at content.
fn is_fixable(code: &str) -> bool {
    matches!(code, "missing_frontmatter" | "missing_name" | "name_mismatch")
}

fn issue(code: &str, severity: &str, param: Option<String>) -> LintIssue {
    LintIssue {
        code: code.to_string(),
        severity: severity.to_string(),
        param,
        fixable: is_fixable(code),
    }
}

/// Lint a skill directory. Returns an empty Vec when the skill is healthy.
pub fn lint_skill_dir(dir: &Path, skill_name: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    let skill_md = dir.join("SKILL.md");
    if !skill_md.is_file() {
        issues.push(issue("file_missing", "error", None));
        return issues;
    }

    let content = match fs::read_to_string(&skill_md) {
        Ok(c) => c,
        Err(e) => {
            issues.push(issue("file_unreadable", "error", Some(e.to_string())));
            return issues;
        }
    };

    let lines: Vec<&str> = content.lines().collect();

    // Front matter presence
    if lines.is_empty() || lines[0].trim() != "---" {
        issues.push(issue("missing_frontmatter", "warning", None));
        check_body(&content, &mut issues);
        return issues;
    }

    let end_idx = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| l.trim() == "---")
        .map(|(i, _)| i);

    let end_idx = match end_idx {
        Some(i) => i,
        None => {
            issues.push(issue("unclosed_frontmatter", "error", None));
            return issues;
        }
    };

    let yaml_content = lines[1..end_idx].join("\n");
    let yaml: serde_yaml::Value = match serde_yaml::from_str(&yaml_content) {
        Ok(y) => y,
        Err(e) => {
            issues.push(issue("invalid_yaml", "error", Some(e.to_string())));
            return issues;
        }
    };

    // name checks
    match yaml.get("name").and_then(|v| v.as_str()) {
        None => issues.push(issue("missing_name", "warning", None)),
        Some(name) => {
            let dir_name = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !dir_name.is_empty() && name != dir_name && name != skill_name {
                issues.push(issue("name_mismatch", "warning", Some(dir_name)));
            }
        }
    }

    // description checks
    match yaml.get("description").and_then(|v| v.as_str()) {
        None => issues.push(issue("missing_description", "error", None)),
        Some(desc) if desc.trim().is_empty() => {
            issues.push(issue("missing_description", "error", None))
        }
        Some(desc) if desc.chars().count() > MAX_DESCRIPTION_LEN => issues.push(issue(
            "description_too_long",
            "warning",
            Some(desc.chars().count().to_string()),
        )),
        Some(_) => {}
    }

    // Body after front matter. Rebuild from lines rather than byte offsets:
    // `lines()` strips `\r` on CRLF files, so byte arithmetic would land
    // mid-character in multi-byte (e.g. Chinese) content and panic.
    let body = lines[end_idx + 1..].join("\n");
    check_body(&body, &mut issues);

    issues
}

/// Flag skills whose markdown body is empty — nothing for the agent to load.
fn check_body(body: &str, issues: &mut Vec<LintIssue>) {
    if body.trim().is_empty() {
        issues.push(issue("empty_body", "warning", None));
    }
}

/// Apply the safe automatic fixes (see `is_fixable`) to a skill's SKILL.md.
/// Content-level problems (missing description, empty body, broken YAML) are
/// never touched — those need a human in the editor. Returns the fixed codes.
/// Preserves the file's existing CRLF/LF line-ending style.
pub fn fix_skill_dir(dir: &Path, skill_name: &str) -> Result<Vec<String>, String> {
    let skill_md = dir.join("SKILL.md");
    if !skill_md.is_file() {
        return Err(format!("SKILL.md not found in {}", dir.display()));
    }
    let content =
        fs::read_to_string(&skill_md).map_err(|e| format!("Cannot read SKILL.md: {}", e))?;
    let eol = if content.contains("\r\n") { "\r\n" } else { "\n" };
    let dir_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| skill_name.to_string());
    let name_line = format!("name: \"{}\"", dir_name.replace('"', "\\\""));

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut fixed = Vec::new();

    if lines.is_empty() || lines[0].trim() != "---" {
        // Wrap the existing body with a minimal front matter carrying the name.
        let mut new_lines = vec![
            "---".to_string(),
            name_line,
            "---".to_string(),
            String::new(),
        ];
        new_lines.extend(lines);
        lines = new_lines;
        fixed.push("missing_frontmatter".to_string());
    } else {
        let end_idx = lines
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, l)| l.trim() == "---")
            .map(|(i, _)| i);
        let end_idx = match end_idx {
            Some(i) => i,
            // Unclosed front matter: rewriting it risks eating body content.
            None => return Ok(fixed),
        };
        let yaml_content = lines[1..end_idx].join("\n");
        let yaml: serde_yaml::Value = match serde_yaml::from_str(&yaml_content) {
            Ok(y) => y,
            // Broken YAML: cannot edit fields reliably.
            Err(_) => return Ok(fixed),
        };
        match yaml.get("name").and_then(|v| v.as_str()) {
            None => {
                lines.insert(1, name_line);
                fixed.push("missing_name".to_string());
            }
            Some(name) if name != dir_name && name != skill_name => {
                // Replace the top-level `name:` line in place.
                if let Some(idx) = lines[1..end_idx].iter().position(|l| l.starts_with("name:")) {
                    lines[1 + idx] = name_line;
                    fixed.push("name_mismatch".to_string());
                }
            }
            Some(_) => {}
        }
    }

    if !fixed.is_empty() {
        let mut out = lines.join(eol);
        out.push_str(eol);
        fs::write(&skill_md, out).map_err(|e| format!("Cannot write SKILL.md: {}", e))?;
    }
    Ok(fixed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_skill(dir: &Path, content: &str) {
        fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn healthy_skill_has_no_issues() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-skill");
        fs::create_dir(&dir).unwrap();
        write_skill(&dir, "---\nname: my-skill\ndescription: does things\n---\n\n# Body\n");
        assert!(lint_skill_dir(&dir, "my-skill").is_empty());
    }

    #[test]
    fn missing_file_reported() {
        let tmp = TempDir::new().unwrap();
        let issues = lint_skill_dir(tmp.path(), "x");
        assert_eq!(issues[0].code, "file_missing");
    }

    #[test]
    fn missing_description_and_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("s");
        fs::create_dir(&dir).unwrap();
        write_skill(&dir, "# Just a body\n");
        let codes: Vec<_> = lint_skill_dir(&dir, "s").iter().map(|i| i.code.clone()).collect();
        assert!(codes.contains(&"missing_frontmatter".to_string()));

        write_skill(&dir, "---\nname: s\n---\nbody\n");
        let codes: Vec<_> = lint_skill_dir(&dir, "s").iter().map(|i| i.code.clone()).collect();
        assert!(codes.contains(&"missing_description".to_string()));
    }

    #[test]
    fn empty_body_flagged() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("s");
        fs::create_dir(&dir).unwrap();
        write_skill(&dir, "---\nname: s\ndescription: d\n---\n\n");
        let codes: Vec<_> = lint_skill_dir(&dir, "s").iter().map(|i| i.code.clone()).collect();
        assert!(codes.contains(&"empty_body".to_string()));
    }

    #[test]
    fn crlf_with_multibyte_chars_does_not_panic() {
        // Regression: CRLF line endings + Chinese description used to shift the
        // byte-based body offset into the middle of a multi-byte char and panic.
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("exam_cram_session");
        fs::create_dir(&dir).unwrap();
        write_skill(
            &dir,
            "---\r\nname: \"exam_cram_session\"\r\ndescription: \"考试冲刺复习skill：扫描课程材料、提取知识点体系（含优先级+来源标注）、用苏格拉底式交互引导复习。\"\r\ntrigger:\r\n  - \"考试\"\r\n---\r\n\r\n# 正文内容\r\n",
        );
        assert!(lint_skill_dir(&dir, "exam_cram_session").is_empty());
    }

    #[test]
    fn crlf_empty_body_flagged() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("s");
        fs::create_dir(&dir).unwrap();
        write_skill(&dir, "---\r\nname: s\r\ndescription: 中文描述\r\n---\r\n\r\n");
        let codes: Vec<_> = lint_skill_dir(&dir, "s").iter().map(|i| i.code.clone()).collect();
        assert!(codes.contains(&"empty_body".to_string()));
    }

    #[test]
    fn fix_adds_missing_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-skill");
        fs::create_dir(&dir).unwrap();
        write_skill(&dir, "# Just a body\n");
        let fixed = fix_skill_dir(&dir, "my-skill").unwrap();
        assert_eq!(fixed, vec!["missing_frontmatter".to_string()]);
        let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(content.starts_with("---\nname: \"my-skill\"\n---"));
        // Frontmatter is now valid; only the description error should remain.
        let codes: Vec<_> = lint_skill_dir(&dir, "my-skill").iter().map(|i| i.code.clone()).collect();
        assert_eq!(codes, vec!["missing_description".to_string()]);
    }

    #[test]
    fn fix_repairs_name_mismatch_preserving_crlf() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("real-name");
        fs::create_dir(&dir).unwrap();
        write_skill(&dir, "---\r\nname: wrong\r\ndescription: d\r\n---\r\n\r\nbody\r\n");
        let fixed = fix_skill_dir(&dir, "real-name").unwrap();
        assert_eq!(fixed, vec!["name_mismatch".to_string()]);
        let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(content.contains("name: \"real-name\"\r\n"));
        assert!(lint_skill_dir(&dir, "real-name").is_empty());
    }

    #[test]
    fn fix_leaves_content_issues_alone() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("s");
        fs::create_dir(&dir).unwrap();
        // Missing description is not auto-fixable; file must stay unchanged.
        let original = "---\nname: s\n---\nbody\n";
        write_skill(&dir, original);
        let fixed = fix_skill_dir(&dir, "s").unwrap();
        assert!(fixed.is_empty());
        assert_eq!(fs::read_to_string(dir.join("SKILL.md")).unwrap(), original);
    }
}
