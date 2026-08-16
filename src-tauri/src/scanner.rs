// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hash::{compute_content_hash, compute_core_hash};
use crate::models::DiscoveredSkill;
use std::fs;
use std::path::Path;

/// Recursively scan a directory for SKILL.md files
pub fn scan_directory(path: &Path) -> Result<Vec<DiscoveredSkill>, Vec<String>> {
    let mut skills = Vec::new();
    let mut errors = Vec::new();

    if !path.exists() {
        errors.push(format!("Path does not exist: {:?}", path));
        return Err(errors);
    }

    if !path.is_dir() {
        errors.push(format!("Path is not a directory: {:?}", path));
        return Err(errors);
    }

    scan_recursive(path, &mut skills, &mut errors);

    if skills.is_empty() && !errors.is_empty() {
        Err(errors)
    } else {
        Ok(skills)
    }
}

fn scan_recursive(dir: &Path, skills: &mut Vec<DiscoveredSkill>, errors: &mut Vec<String>) {
    // Check if this directory contains SKILL.md
    let skill_md = dir.join("SKILL.md");
    if skill_md.exists() && skill_md.is_file() {
        match parse_skill(dir, &skill_md) {
            Ok(skill) => {
                skills.push(skill);
                // Found SKILL.md, don't recurse further in this directory
                return;
            }
            Err(e) => {
                errors.push(format!("Failed to parse {:?}: {}", skill_md, e));
                // Still don't recurse further - SKILL.md exists but is malformed
                return;
            }
        }
    }

    // No SKILL.md here, recurse into subdirectories
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            errors.push(format!("Cannot read directory {:?}: {}", dir, e));
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("Dir entry error: {}", e));
                continue;
            }
        };

        let path = entry.path();

        // Skip hidden directories
        if is_hidden(&path) {
            continue;
        }

        if path.is_dir() {
            scan_recursive(&path, skills, errors);
        }
    }
}

/// Parse a SKILL.md file and extract metadata
fn parse_skill(dir: &Path, skill_md_path: &Path) -> Result<DiscoveredSkill, String> {
    let content = fs::read_to_string(skill_md_path)
        .map_err(|e| format!("Cannot read file: {}", e))?;

    // Parse YAML front matter
    let (name, description) = parse_front_matter(&content, dir)?;

    // Compute content hash for the entire directory
    let content_hash = compute_content_hash(dir)
        .map_err(|e| format!("Hash computation failed: {}", e))?;

    // Compute core identity hash (SKILL.md only — stable across copies)
    let core_hash = compute_core_hash(skill_md_path)
        .map_err(|e| format!("Core hash computation failed: {}", e))?;

    let source_path = normalize_path(
        &dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf()),
    );

    Ok(DiscoveredSkill {
        name,
        description,
        source_path,
        content_hash,
        core_hash,
    })
}

/// Parse YAML front matter from SKILL.md content
pub(crate) fn parse_front_matter(content: &str, dir: &Path) -> Result<(String, Option<String>), String> {
    // Look for front matter between --- delimiters
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || lines[0].trim() != "---" {
        // No front matter, use directory name
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        return Ok((name, None));
    }

    // Find the closing ---
    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    let end_idx = match end_idx {
        Some(i) => i,
        None => {
            // No closing ---, use directory name
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return Ok((name, None));
        }
    };

    // Extract YAML content between the --- markers
    let yaml_content = lines[1..end_idx].join("\n");

    // Parse YAML
    let yaml: serde_yaml::Value = serde_yaml::from_str(&yaml_content)
        .map_err(|e| format!("YAML parse error: {}", e))?;

    let name = yaml
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    let description = yaml
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok((name, description))
}

/// Check if a path is hidden
fn is_hidden(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        if name.to_string_lossy().starts_with('.') {
            return true;
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(metadata) = path.metadata() {
            const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
            return metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0;
        }
    }

    false
}

/// Expand ~ to home directory
pub fn expand_path(path: &str) -> Result<std::path::PathBuf, String> {
    let expanded = if path.starts_with("~/") || path.starts_with("~\\") {
        let home = dirs::home_dir().ok_or("Cannot find home directory")?;
        home.join(&path[2..])
    } else if path == "~" {
        dirs::home_dir().ok_or("Cannot find home directory".to_string())?
    } else {
        std::path::PathBuf::from(path)
    };
    Ok(expanded)
}

/// Normalize a path for consistent comparison and storage.
/// On Windows: strips `\\?\` prefix and converts all separators to backslashes.
/// On Unix: no-op (just returns the string as-is).
pub fn normalize_path(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        // Strip \\?\ or \\\\?\\ prefix
        let stripped = match s.strip_prefix(r"\\?\") {
            Some(rest) => rest.to_string(),
            None => s,
        };
        // Convert all forward slashes to backslashes
        stripped.replace('/', "\\")
    }

    #[cfg(not(target_os = "windows"))]
    {
        s
    }
}
