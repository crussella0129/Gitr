pub mod reconcile;
pub mod scanner;

use gitr_core::error::GitrError;
use gitr_core::models::host::Host;
use gitr_host::HostProvider;

use crate::reconcile::{reconcile, ReconcileResult};
use crate::scanner::scan_directory;

/// Discover repos by scanning the filesystem and querying a host API, then reconcile.
pub async fn discover(
    host: &Host,
    provider: &dyn HostProvider,
    scan_paths: &[std::path::PathBuf],
    max_depth: usize,
) -> Result<ReconcileResult, GitrError> {
    // 1. Local filesystem scan
    let mut local_repos = Vec::new();
    for path in scan_paths {
        if path.exists() {
            local_repos.extend(scan_directory(path, max_depth));
        }
    }

    // 2. API query
    let remote_repos = provider.list_repos().await?;

    // 3. Reconcile
    Ok(reconcile(&local_repos, &remote_repos, &host.label))
}
