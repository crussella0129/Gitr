use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repo::RepoId;

/// Unique identifier for a sync link.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncLinkId(pub Uuid);

impl SyncLinkId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for SyncLinkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Direction of sync between two repos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    Push,
    Pull,
    Both,
}

impl std::fmt::Display for SyncDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncDirection::Push => write!(f, "push"),
            SyncDirection::Pull => write!(f, "pull"),
            SyncDirection::Both => write!(f, "both"),
        }
    }
}

impl std::str::FromStr for SyncDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "push" => Ok(SyncDirection::Push),
            "pull" => Ok(SyncDirection::Pull),
            "both" => Ok(SyncDirection::Both),
            _ => Err(format!("unknown sync direction: {s}")),
        }
    }
}

/// Strategy for merging upstream changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    FastForward,
    Merge,
    Rebase,
    ForcePush,
}

impl std::fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeStrategy::FastForward => write!(f, "ff"),
            MergeStrategy::Merge => write!(f, "merge"),
            MergeStrategy::Rebase => write!(f, "rebase"),
            MergeStrategy::ForcePush => write!(f, "force_push"),
        }
    }
}

impl std::str::FromStr for MergeStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ff" | "fast_forward" | "fast-forward" => Ok(MergeStrategy::FastForward),
            "merge" => Ok(MergeStrategy::Merge),
            "rebase" => Ok(MergeStrategy::Rebase),
            "force_push" | "force-push" => Ok(MergeStrategy::ForcePush),
            _ => Err(format!("unknown merge strategy: {s}")),
        }
    }
}

/// What triggers a sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTrigger {
    Manual,
    Schedule { cron: String },
    Always,
}

impl std::fmt::Display for SyncTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncTrigger::Manual => write!(f, "manual"),
            SyncTrigger::Schedule { cron } => write!(f, "schedule:{cron}"),
            SyncTrigger::Always => write!(f, "always"),
        }
    }
}

/// Additional sync instructions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncInstructions {
    pub branch_include: Vec<String>,
    pub branch_exclude: Vec<String>,
    pub sync_tags: bool,
}

/// A directed sync edge between two repos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncLink {
    pub id: SyncLinkId,
    pub source_repo_id: RepoId,
    pub target_repo_id: RepoId,
    pub direction: SyncDirection,
    pub merge_strategy: MergeStrategy,
    pub trigger: SyncTrigger,
    pub instructions: SyncInstructions,
    pub enabled: bool,
}

impl SyncLink {
    pub fn new(
        source_repo_id: RepoId,
        target_repo_id: RepoId,
        direction: SyncDirection,
        merge_strategy: MergeStrategy,
    ) -> Self {
        Self {
            id: SyncLinkId::new(),
            source_repo_id,
            target_repo_id,
            direction,
            merge_strategy,
            trigger: SyncTrigger::Manual,
            instructions: SyncInstructions::default(),
            enabled: true,
        }
    }
}
