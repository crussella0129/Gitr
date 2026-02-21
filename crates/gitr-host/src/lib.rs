pub mod github;
pub mod gitlab;
pub mod gitea;
pub mod bitbucket;
pub mod azure_devops;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gitr_core::error::GitrError;
use gitr_core::models::host::HostKind;

/// A repo as returned by a hosting API.
#[derive(Debug, Clone)]
pub struct RemoteRepo {
    pub full_name: String,
    pub owner: String,
    pub name: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub is_fork: bool,
    pub upstream_full_name: Option<String>,
    pub upstream_clone_url: Option<String>,
    pub description: Option<String>,
    pub is_private: bool,
    pub is_archived: bool,
    pub updated_at: Option<DateTime<Utc>>,
}

/// A branch as returned by a hosting API.
#[derive(Debug, Clone)]
pub struct RemoteBranch {
    pub name: String,
    pub sha: String,
    pub is_default: bool,
}

/// Fork sync status for a single branch.
#[derive(Debug, Clone)]
pub struct ForkSyncStatus {
    pub branch: String,
    pub behind_by: u32,
    pub ahead_by: u32,
}

/// Rate limit information.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset_at: DateTime<Utc>,
}

/// Trait for interacting with a git hosting provider.
#[async_trait]
pub trait HostProvider: Send + Sync {
    /// Validate that the stored credentials are valid.
    async fn validate_credentials(&self) -> Result<bool, GitrError>;

    /// List all repos for the configured user (handles pagination).
    async fn list_repos(&self) -> Result<Vec<RemoteRepo>, GitrError>;

    /// Get a specific repo by owner/name.
    async fn get_repo(&self, owner: &str, name: &str) -> Result<Option<RemoteRepo>, GitrError>;

    /// List branches for a repo.
    async fn list_branches(&self, owner: &str, name: &str) -> Result<Vec<RemoteBranch>, GitrError>;

    /// Get fork sync status (behind/ahead) for each branch.
    async fn fork_sync_status(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<Vec<ForkSyncStatus>, GitrError>;

    /// Get current rate limit status.
    async fn rate_limit_status(&self) -> Result<RateLimitInfo, GitrError>;

    /// The kind of host this provider handles.
    fn kind(&self) -> HostKind;
}

/// Create a HostProvider for the given host kind.
pub fn create_provider(
    kind: &HostKind,
    api_url: &url::Url,
    token: &str,
    username: &str,
) -> Result<Box<dyn HostProvider>, GitrError> {
    match kind {
        HostKind::GitHub => Ok(Box::new(github::GitHubProvider::new(
            api_url.clone(),
            token.to_string(),
            username.to_string(),
        ))),
        other => Err(GitrError::ProviderNotImplemented {
            kind: other.to_string(),
        }),
    }
}
