// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Validation for paths the user typed, per PRD §14.
//!
//! Two tiers, because the two failure kinds need different handling:
//! a shape the application can never resolve (relative input, `%VAR%`) is a hard
//! error at the boundary; an absolute path that does not exist yet is not — the
//! tool may create its directory later, and PRD §15.1 wants the scanner to
//! report it rather than the form to refuse it. So the second tier comes back as
//! a `PathCheck` flag the UI renders as a red hint.
//!
//! Error strings are stable ASCII codes with the offending input appended, so
//! the frontend can localize them (`src/i18n::localize_api_error`).

use std::path::PathBuf;

use serde::Serialize;

use crate::scanner::expand_path;

/// Result of checking a user-entered path: what it resolves to, and whether that
/// target is currently a directory on disk.
#[derive(Debug, Clone, Serialize)]
pub struct PathCheck {
    pub input: String,
    pub expanded: String,
    pub exists: bool,
    pub is_dir: bool,
}

/// Hard-reject anything the resolver could never make sense of and report what a
/// resolvable path currently looks like on disk.
pub fn check_user_path(raw: &str) -> Result<PathCheck, String> {
    let resolved = require_resolvable_path(raw)?;
    let is_dir = resolved.is_dir();
    Ok(PathCheck {
        input: raw.trim().to_string(),
        expanded: resolved.to_string_lossy().to_string(),
        exists: resolved.exists(),
        is_dir,
    })
}

/// Same rules as [`check_user_path`], returning the string to store. Use this on
/// the write path so a relative or unexpandable path cannot reach the database.
pub fn require_resolvable_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty-path".to_string());
    }
    // `%APPDATA%\skills` is a shell-ism; expanding it would silently depend on
    // the process environment, which the app does not control (PRD §14.2).
    if has_env_var_syntax(trimmed) {
        return Err(format!("env-var-unsupported:{}", trimmed));
    }
    if !looks_absolute(trimmed) {
        return Err(format!("relative-path:{}", trimmed));
    }
    // `~/…` is handled by `expand_path`; it also accepts an already-absolute
    // path unchanged, which is what we want after the checks above.
    expand_path(trimmed)
}

/// A tool's project path is relative to the project root by design (PRD §5.1),
/// and blank means "infer it from the tool template", so only the rules that
/// apply regardless of shape are enforced here.
pub fn require_configurable_rel_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if !trimmed.is_empty() && has_env_var_syntax(trimmed) {
        return Err(format!("env-var-unsupported:{}", trimmed));
    }
    Ok(trimmed.to_string())
}

/// `%NAME%` (Windows) — a pair of percent signs with at least one name character
/// between them. Two adjacent percents is an escaped percent in some shells and
/// is not treated as a variable.
fn has_env_var_syntax(path: &str) -> bool {
    let bytes = path.as_bytes();
    let mut start = None;
    for (i, byte) in bytes.iter().enumerate() {
        if *byte == b'%' {
            match start {
                // `%` immediately after `%` closes an empty name: not a variable.
                Some(s) if i == s + 1 => start = None,
                Some(_) => return true,
                None => start = Some(i),
            }
        }
    }
    false
}

/// Whether the input claims to be absolute on at least one platform.
///
/// `Path::is_absolute` is host-specific: on Windows a Unix path like
/// `/home/xx/.claude/skills` has no prefix and reads as relative, yet PRD §13.2
/// wants it treated as an absolute path that simply does not exist there.
fn looks_absolute(path: &str) -> bool {
    // Home-rooted input is absolute by definition once `~` expands.
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        return true;
    }
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') || normalized.starts_with("//") {
        return true;
    }
    match drive_len(&normalized) {
        // `C:/…` is absolute; `C:foo` is drive-relative and cannot be resolved
        // without knowing the drive's current directory.
        Some(len) => normalized.as_bytes().get(len) == Some(&b'/'),
        None => false,
    }
}

/// Length of a leading `X:` drive specifier, if any.
fn drive_len(path: &str) -> Option<usize> {
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        Some(2)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        dirs::home_dir().expect("test needs a home directory")
    }

    fn err(raw: &str) -> String {
        require_resolvable_path(raw).expect_err("expected a rejected path")
    }

    #[test]
    fn tilde_paths_expand_to_home() {
        assert_eq!(
            require_resolvable_path("~/skills").unwrap(),
            home().join("skills")
        );
        assert_eq!(
            require_resolvable_path("~").unwrap(),
            home()
        );
        assert_eq!(
            require_resolvable_path("~\\.claude\\skills\\").unwrap(),
            home().join(".claude").join("skills")
        );
    }

    #[test]
    fn absolute_paths_of_every_documented_shape_are_accepted() {
        for raw in [
            "/home/xx/.claude/skills/",
            "C:/Users/xx/.claude/skills/",
            "C:\\Users\\xx\\.claude\\skills\\",
            "\\\\server\\share\\skills\\",
        ] {
            assert!(
                require_resolvable_path(raw).is_ok(),
                "{} must resolve",
                raw
            );
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_before_checking() {
        assert!(require_resolvable_path("  /tmp/skills  ").is_ok());
        assert_eq!(check_user_path("  /tmp/skills  ").unwrap().input, "/tmp/skills");
    }

    #[test]
    fn relative_and_env_var_and_empty_input_are_rejected_by_code() {
        assert_eq!(err("   "), "empty-path");
        assert_eq!(err("./skills/"), "relative-path:./skills/");
        assert_eq!(err("skills/.claude"), "relative-path:skills/.claude");
        assert_eq!(err("../up/skills"), "relative-path:../up/skills");
        // Drive-relative: no separator after the colon.
        assert_eq!(err("C:skills"), "relative-path:C:skills");
        assert_eq!(
            err("%APPDATA%\\skills\\"),
            "env-var-unsupported:%APPDATA%\\skills\\"
        );
    }

    #[test]
    fn a_lone_percent_is_not_a_variable() {
        assert!(require_resolvable_path("/tmp/100%/skills").is_ok());
        assert!(require_resolvable_path("/tmp/a%%b/skills").is_ok());
    }

    #[test]
    fn missing_and_existing_targets_are_distinguished() {
        let missing = check_user_path("/definitely/not/a/real/skill/dir").unwrap();
        assert!(!missing.exists);
        assert!(!missing.is_dir);

        let existing = check_user_path("~/").unwrap();
        assert!(existing.exists, "home must exist: {:?}", existing);
        assert!(existing.is_dir);
    }
}
