use async_trait::async_trait;

use gitr_core::error::GitrError;
use gitr_core::models::host::HostKind;

use crate::{ForkSyncStatus, HostProvider, RateLimitInfo, RemoteBranch, RemoteRepo};

pub struct GitLabProvider;

#[async_trait]
impl HostProvider for GitLabProvider {
    async fn validate_credentials(&self) -> Result<bool, GitrError> {
        Err(GitrError::ProviderNotImplemented { kind: "gitlab".into() })
    }

    async fn list_repos(&self) -> Result<Vec<RemoteRepo>, GitrError> {
        Err(GitrError::ProviderNotImplemented { kind: "gitlab".into() })
    }

    async fn get_repo(&self, _owner: &str, _name: &str) -> Result<Option<RemoteRepo>, GitrError> {
        Err(GitrError::ProviderNotImplemented { kind: "gitlab".into() })
    }

    async fn list_branches(&self, _owner: &str, _name: &str) -> Result<Vec<RemoteBranch>, GitrError> {
        Err(GitrError::ProviderNotImplemented { kind: "gitlab".into() })
    }

    async fn fork_sync_status(&self, _owner: &str, _name: &str) -> Result<Vec<ForkSyncStatus>, GitrError> {
        Err(GitrError::ProviderNotImplemented { kind: "gitlab".into() })
    }

    async fn rate_limit_status(&self) -> Result<RateLimitInfo, GitrError> {
        Err(GitrError::ProviderNotImplemented { kind: "gitlab".into() })
    }

    fn kind(&self) -> HostKind {
        HostKind::GitLab
    }
}
