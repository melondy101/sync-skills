// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

/// Process-level directory lock manager for skills.
/// Key: "{project_id}:{skill_name}" — locks the skill across ALL tools.
///
/// This is NOT an OS file lock. It prevents concurrent sync/scan operations
/// within the same process from conflicting on the same skill directory.
///
/// All sync/check work runs inside `tokio::task::spawn_blocking` threads,
/// so the acquire methods here are blocking (safe off the async runtime).
pub struct LockManager {
    locks: AsyncMutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

/// Guard that releases the lock when dropped
pub struct SkillLockGuard {
    _guard: OwnedMutexGuard<()>,
    key: String,
}

impl std::fmt::Debug for SkillLockGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillLockGuard")
            .field("key", &self.key)
            .finish()
    }
}

impl LockManager {
    pub fn new() -> Self {
        LockManager {
            locks: AsyncMutex::new(HashMap::new()),
        }
    }

    /// Build the lock key for a skill
    fn key(project_id: i64, skill_name: &str) -> String {
        format!("{}:{}", project_id, skill_name)
    }

    /// Acquire the lock for a skill, blocking the current thread until available.
    /// Must be called from a blocking context (e.g. inside spawn_blocking) —
    /// never from an async task, or the runtime would stall.
    /// The lock is released when the returned guard is dropped.
    pub fn acquire_blocking(&self, project_id: i64, skill_name: &str) -> SkillLockGuard {
        let key = Self::key(project_id, skill_name);

        // Get or create the per-skill mutex
        let mutex = {
            let mut map = self.locks.blocking_lock();
            map.entry(key.clone())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };

        // Acquire the per-skill lock
        let guard = mutex.blocking_lock_owned();
        SkillLockGuard { _guard: guard, key }
    }

    /// Try to acquire the lock without blocking.
    /// Returns None if the lock is already held (skill busy syncing).
    pub fn try_acquire_blocking(&self, project_id: i64, skill_name: &str) -> Option<SkillLockGuard> {
        let key = Self::key(project_id, skill_name);

        let mutex = {
            let mut map = self.locks.blocking_lock();
            map.entry(key.clone())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };

        // Try to acquire without blocking
        match mutex.try_lock_owned() {
            Ok(guard) => Some(SkillLockGuard { _guard: guard, key }),
            Err(_) => None,
        }
    }
}
