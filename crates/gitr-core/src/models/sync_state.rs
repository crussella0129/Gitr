use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repo::RepoId;
use super::sync_link::SyncLinkId;

/// Status of a sync operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Success,
    PartialSuccess,
    Failed,
    Skipped,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Success => write!(f, "success"),
            SyncStatus::PartialSuccess => write!(f, "partial_success"),
            SyncStatus::Failed => write!(f, "failed"),
            SyncStatus::Skipped => write!(f, "skipped"),
        }
    }
}

impl std::str::FromStr for SyncStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "success" => Ok(SyncStatus::Success),
            "partial_success" => Ok(SyncStatus::PartialSuccess),
            "failed" => Ok(SyncStatus::Failed),
            "skipped" => Ok(SyncStatus::Skipped),
            _ => Err(format!("unknown sync status: {s}")),
        }
    }
}

/// Record of a completed sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub id: Uuid,
    pub repo_id: RepoId,
    pub sync_link_id: Option<SyncLinkId>,
    pub branches_synced: u32,
    pub branches_failed: u32,
    pub commits_transferred: u32,
    pub status: SyncStatus,
    pub errors: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

impl SyncRecord {
    pub fn new(repo_id: RepoId) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            repo_id,
            sync_link_id: None,
            branches_synced: 0,
            branches_failed: 0,
            commits_transferred: 0,
            status: SyncStatus::Success,
            errors: Vec::new(),
            started_at: now,
            finished_at: now,
        }
    }
}

/// Snapshot of a branch's state for behind/ahead tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSnapshot {
    pub repo_id: RepoId,
    pub branch: String,
    pub local_sha: Option<String>,
    pub remote_sha: Option<String>,
    pub upstream_sha: Option<String>,
    pub behind_count: u32,
    pub ahead_count: u32,
    pub updated_at: DateTime<Utc>,
}
