// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Market source CRUD.
//! Extracted from db.rs for maintainability.

use super::Database;
use crate::models::{Market, MarketTemplate};
use rusqlite::params;

impl Database {
    pub fn list_markets(&self) -> Result<Vec<Market>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, provider, owner, name, branch, remote_url, enabled, layout,
                        created_at, updated_at, last_indexed_at, last_checked_at, last_commit_sha
                 FROM markets ORDER BY owner, name",
            )
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Market {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    owner: row.get(2)?,
                    name: row.get(3)?,
                    branch: row.get(4)?,
                    remote_url: row.get(5)?,
                    enabled: row.get(6)?,
                    layout: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    last_indexed_at: row.get(10).ok().flatten(),
                    last_checked_at: row.get(11).ok().flatten(),
                    last_commit_sha: row.get(12).ok().flatten(),
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }

    pub fn get_market(&self, market_id: i64) -> Result<Option<Market>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, provider, owner, name, branch, remote_url, enabled, layout,
                        created_at, updated_at, last_indexed_at, last_checked_at, last_commit_sha
                 FROM markets WHERE id = ?1",
            )
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let result = stmt.query_row(params![market_id], |row| {
            Ok(Market {
                id: row.get(0)?,
                provider: row.get(1)?,
                owner: row.get(2)?,
                name: row.get(3)?,
                branch: row.get(4)?,
                remote_url: row.get(5)?,
                enabled: row.get(6)?,
                layout: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                last_indexed_at: row.get(10).ok().flatten(),
                last_checked_at: row.get(11).ok().flatten(),
                last_commit_sha: row.get(12).ok().flatten(),
            })
        });
        match result {
            Ok(market) => Ok(Some(market)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to get market: {}", e)),
        }
    }

    pub fn get_market_by_key(
        &self,
        provider: &str,
        owner: &str,
        name: &str,
        branch: &str,
    ) -> Result<Option<Market>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, provider, owner, name, branch, remote_url, enabled, layout,
                        created_at, updated_at, last_indexed_at, last_checked_at, last_commit_sha
                 FROM markets WHERE provider = ?1 AND owner = ?2 AND name = ?3 AND branch = ?4",
            )
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let result = stmt.query_row(
            params![provider, owner, name, branch],
            |row| {
                Ok(Market {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    owner: row.get(2)?,
                    name: row.get(3)?,
                    branch: row.get(4)?,
                    remote_url: row.get(5)?,
                    enabled: row.get(6)?,
                    layout: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    last_indexed_at: row.get(10).ok().flatten(),
                    last_checked_at: row.get(11).ok().flatten(),
                    last_commit_sha: row.get(12).ok().flatten(),
                })
            },
        );
        match result {
            Ok(market) => Ok(Some(market)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to get market: {}", e)),
        }
    }

    pub fn insert_market(
        &self,
        provider: &str,
        owner: &str,
        name: &str,
        branch: &str,
        remote_url: &str,
        layout: &str,
    ) -> Result<Market, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let key = format!("{}/{}/{}@{}", provider, owner, name, branch);
        let id = crate::hash::compute_id_hash(&key);
        conn.execute(
            "INSERT INTO markets (id, provider, owner, name, branch, remote_url, enabled, layout)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)",
            params![id, provider, owner, name, branch, remote_url, layout],
        )
        .map_err(|e| format!("Failed to insert market: {}", e))?;
        Ok(Market {
            id,
            provider: provider.to_string(),
            owner: owner.to_string(),
            name: name.to_string(),
            branch: branch.to_string(),
            remote_url: remote_url.to_string(),
            enabled: true,
            layout: layout.to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            last_indexed_at: None,
            last_checked_at: None,
            last_commit_sha: None,
        })
    }

    pub fn upsert_market(
        &self,
        provider: &str,
        owner: &str,
        name: &str,
        branch: &str,
        remote_url: &str,
        layout: &str,
    ) -> Result<Market, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let key = format!("{}/{}/{}@{}", provider, owner, name, branch);
        let id = crate::hash::compute_id_hash(&key);
        conn.execute(
            "INSERT INTO markets (id, provider, owner, name, branch, remote_url, enabled, layout)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)
             ON CONFLICT(id) DO UPDATE SET
             remote_url = excluded.remote_url, layout = excluded.layout, updated_at = datetime('now')",
            params![id, provider, owner, name, branch, remote_url, layout],
        )
        .map_err(|e| format!("Failed to upsert market: {}", e))?;
        Ok(Market {
            id,
            provider: provider.to_string(),
            owner: owner.to_string(),
            name: name.to_string(),
            branch: branch.to_string(),
            remote_url: remote_url.to_string(),
            enabled: true,
            layout: layout.to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            last_indexed_at: None,
            last_checked_at: None,
            last_commit_sha: None,
        })
    }

    pub fn update_market(
        &self,
        market_id: i64,
        enabled: bool,
        last_indexed_at: Option<&str>,
        last_checked_at: Option<&str>,
        last_commit_sha: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE markets SET enabled = ?1, last_indexed_at = ?2, last_checked_at = ?3,
             last_commit_sha = ?4, updated_at = datetime('now') WHERE id = ?5",
            params![enabled as i64, last_indexed_at, last_checked_at, last_commit_sha, market_id],
        )
        .map_err(|e| format!("Failed to update market: {}", e))?;
        Ok(())
    }

    pub fn delete_market(&self, market_id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute("DELETE FROM markets WHERE id = ?1", params![market_id])
            .map_err(|e| format!("Failed to delete market: {}", e))?;
        Ok(())
    }

    pub fn set_market_layout(&self, market_id: i64, layout: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        conn.execute(
            "UPDATE markets SET layout = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![layout, market_id],
        )
        .map_err(|e| format!("Failed to set market layout: {}", e))?;
        Ok(())
    }

    pub fn list_market_templates(&self) -> Result<Vec<MarketTemplate>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut stmt = conn
            .prepare("SELECT id, provider, owner, name, branch, remote_url, layout FROM markets ORDER BY owner, name")
            .map_err(|e| format!("Query prepare failed: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(MarketTemplate {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    owner: row.get(2)?,
                    name: row.get(3)?,
                    branch: row.get(4)?,
                    remote_url: row.get(5)?,
                    layout: row.get(6)?,
                })
            })
            .map_err(|e| format!("Query failed: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row read failed: {}", e))
    }
}