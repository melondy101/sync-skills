// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Market indexing orchestration — the domain layer between the thin
//! `commands::market` glue and the [`MarketProvider`](crate::providers::MarketProvider)
//! seam. It resolves the provider for a market, drives discovery, and owns the
//! SQLite transaction that upserts the resulting rows. No Tauri types here.

use crate::db::Database;
use crate::models::{Market, MarketSyncResult};
use crate::providers::{provider_for_market, RemoteSkillProbe};
use rusqlite::params;

/// Scan a single market: pick its provider adapter, discover remote skills,
/// then persist them inside one transaction. Errors from discovery are folded
/// into the result (not returned as `Err`) so callers can render per-market
/// failures alongside the successful batch.
pub async fn scan_market(db: &Database, market: &Market) -> Result<MarketSyncResult, String> {
    let provider = provider_for_market(market)?;
    let probes = match provider.discover_skills(market).await {
        Ok(p) => p,
        Err(errs) => {
            return Ok(MarketSyncResult {
                market_id: market.id,
                skills_found: 0,
                skills_new: 0,
                skills_updated: 0,
                errors: errs,
            });
        }
    };
    persist_probes(db, market, probes)
}

/// Upsert discovered probes into `remote_skills` and stamp the market's
/// `last_indexed_at`, all in one transaction.
fn persist_probes(
    db: &Database,
    market: &Market,
    probes: Vec<RemoteSkillProbe>,
) -> Result<MarketSyncResult, String> {
    let skills_found = probes.len();
    let mut skills_new = 0usize;
    let mut skills_updated = 0usize;

    let conn = db.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    conn.execute_batch("BEGIN").ok();

    for probe in probes {
        let existing: Option<(i64, String, String)> = conn
            .query_row(
                "SELECT id, remote_content_hash, remote_core_hash FROM remote_skills WHERE market_id = ?1 AND skill_name = ?2",
                params![market.id, probe.skill_name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        match existing {
            Some((id, old_content_hash, old_core_hash)) => {
                if old_content_hash != probe.remote_content_hash || old_core_hash != probe.remote_core_hash {
                    skills_updated += 1;
                }
                conn.execute(
                    "UPDATE remote_skills SET remote_url = ?1, ssot_path = ?2, remote_content_hash = ?3, remote_core_hash = ?4, description = ?5, updated_at = datetime('now') WHERE id = ?6",
                    params![probe.remote_url, probe.ssot_path, probe.remote_content_hash, probe.remote_core_hash, probe.description, id],
                ).map_err(|e| format!("Failed to update remote skill: {}", e))?;
            }
            None => {
                skills_new += 1;
                let id = crate::hash::compute_id_hash(&format!("{}:{}", market.id, probe.skill_name));
                conn.execute(
                    "INSERT INTO remote_skills (id, market_id, skill_name, description, remote_url, ssot_path, remote_content_hash, remote_core_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![id, market.id, probe.skill_name, probe.description, probe.remote_url, probe.ssot_path, probe.remote_content_hash, probe.remote_core_hash],
                ).map_err(|e| format!("Failed to insert remote skill: {}", e))?;
            }
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE markets SET last_indexed_at = ?1, updated_at = ?2 WHERE id = ?3",
        params![now, now, market.id],
    ).map_err(|e| format!("Failed to update market: {}", e))?;

    conn.execute_batch("COMMIT").ok();

    Ok(MarketSyncResult {
        market_id: market.id,
        skills_found,
        skills_new,
        skills_updated,
        errors: Vec::new(),
    })
}
