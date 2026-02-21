use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use super::host::HostId;

/// Unique identifier for a repo.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepoId(pub Uuid);

impl RepoId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for RepoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// How a repo was discovered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySource {
    /// Found via API query.
    Api,
    /// Found via local filesystem scan.
    Filesystem,
    /// Manually added by user.
    Manual,
}

impl std::fmt::Display for DiscoverySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoverySource::Api => write!(f, "api"),
            DiscoverySource::Filesystem => write!(f, "filesystem"),
            DiscoverySource::Manual => write!(f, "manual"),
        }
    }
}

impl std::str::FromStr for DiscoverySource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "api" => Ok(DiscoverySource::Api),
            "filesystem" => Ok(DiscoverySource::Filesystem),
            "manual" => Ok(DiscoverySource::Manual),
            _ => Err(format!("unknown discovery source: {s}")),
        }
    }
}

/// A tracked repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: RepoId,
    pub full_name: String,
    pub owner: String,
    pub name: String,
    pub host_id: HostId,
    pub clone_url: String,
    pub local_path: Option<PathBuf>,
    pub is_fork: bool,
    pub upstream_repo_id: Option<RepoId>,
    pub upstream_full_name: Option<String>,
    pub default_branch: String,
    pub discovery_source: DiscoverySource,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Repo {
    pub fn new(
        full_name: String,
        host_id: HostId,
        clone_url: String,
        default_branch: String,
        discovery_source: DiscoverySource,
    ) -> Self {
        let parts: Vec<&str> = full_name.splitn(2, '/').collect();
        let (owner, name) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (String::new(), full_name.clone())
        };
        let now = Utc::now();
        Self {
            id: RepoId::new(),
            full_name,
            owner,
            name,
            host_id,
            clone_url,
            local_path: None,
            is_fork: false,
            upstream_repo_id: None,
            upstream_full_name: None,
            default_branch,
            discovery_source,
            last_synced_at: None,
            created_at: now,
        }
    }
}
