// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

/// AI coding tool (e.g., Claude Code, Codex CLI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: i64,
    pub name: String,
    pub global_path: String,
    pub project_rel_path: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A user project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub created_at: String,
}

/// A discovered skill — identity is (name, project_id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub source_path: String,
    pub content_hash: String,
    pub core_hash: String,
    pub project_id: i64,
    pub ssot_updated_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Skill with installation status for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillView {
    #[serde(flatten)]
    pub skill: Skill,
    pub installed_tools: Vec<InstallationInfo>,
    pub install_count: usize,
    pub has_update: bool,
}

/// Installation info for a single tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationInfo {
    pub tool_id: i64,
    pub tool_name: String,
    pub status: String,
    pub synced_at: Option<String>,
    pub installation_synced_at: Option<String>,
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub skill_id: i64,
    pub skill_name: String,
    pub synced_to: usize,
    pub errors: Vec<String>,
}

/// Update info for a skill (used by check_updates)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpdate {
    pub skill_id: i64,
    pub skill_name: String,
    pub source_path: String,
    pub old_hash: String,
    pub new_hash: String,
    /// Which tool directory has the change (None if source_path changed)
    pub changed_tool: Option<String>,
    /// The tool's DB ID (for dismiss operations)
    pub changed_tool_id: Option<i64>,
}

/// A sync log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncLog {
    pub id: i64,
    pub skill_id: Option<i64>,
    pub skill_name: Option<String>,
    pub tool_id: Option<i64>,
    pub tool_name: Option<String>,
    pub project_id: i64,
    pub action: String,
    pub direction: Option<String>,
    pub status: String,
    pub detail: Option<String>,
    pub created_at: String,
}

/// Detail of a single skill found during scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanDetail {
    pub skill_name: String,
    pub tool_name: String,
    pub scope: String,       // "Global" or project name
    pub status: String,      // "new" or "updated"
    pub source_path: String,
}

/// Result of a scan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub skills_found: usize,
    pub skills_new: usize,
    pub skills_updated: usize,
    pub errors: Vec<String>,
    pub details: Vec<ScanDetail>,
}

/// A discovered skill during scanning (before DB insert)
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    pub name: String,
    pub description: Option<String>,
    pub source_path: String,
    pub content_hash: String,
    pub core_hash: String,
}

// ==================== M5: Conflict types ====================

/// A version of a conflicted skill from a specific tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictVersion {
    pub tool_id: i64,
    pub tool_name: String,
    pub core_hash: String,
    pub source_path: String,
}

/// An unresolved conflict for a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictView {
    pub id: i64,
    pub skill_id: i64,
    pub skill_name: String,
    pub detected_at: String,
    pub versions: Vec<ConflictVersion>,
}

// ==================== Skill Market ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTemplate {
    pub id: String,
    pub label: String,
    pub description: String,
    pub provider: String,
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub kind: String,
    pub root_skill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: i64,
    pub provider: String,
    pub owner: String,
    pub name: String,
    pub branch: String,
    pub enabled: bool,
    pub last_indexed_at: Option<String>,
    pub last_checked_at: Option<String>,
    pub last_commit_sha: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSkill {
    pub id: i64,
    pub market_id: i64,
    pub skill_name: String,
    pub description: Option<String>,
    pub remote_url: String,
    pub ssot_path: String,
    pub remote_content_hash: String,
    pub remote_core_hash: String,
    pub is_installed: bool,
    pub installed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSkillUpdate {
    pub id: i64,
    pub market_id: i64,
    pub skill_name: String,
    pub remote_url: String,
    pub ssot_path: String,
    pub old_hash: String,
    pub new_hash: String,
    pub has_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSyncResult {
    pub market_id: i64,
    pub skills_found: usize,
    pub skills_new: usize,
    pub skills_updated: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSkillInstalledResult {
    pub updated: i64,
}

/// A market whose remote repo has a new commit since the last index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCommitUpdate {
    pub market_id: i64,
    pub market_title: String,
    pub last_commit_sha: Option<String>,
    pub new_commit_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInstallation {
    pub id: i64,
    pub remoteSkillId: i64,
    pub skillName: String,
    pub remoteUrl: String,
    pub ssotPath: String,
    pub installedAt: Option<String>,
    pub marketId: i64,
    pub marketTitle: String,
    pub projectId: i64,
    pub scope: String,
}
